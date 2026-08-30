use super::super::{
    ChannelModel, ChannelRole, ChromaPhase, HighresError, ImageFrame, MatrixCoefficients,
    PixelBuffer, PixelFormat, Plane, PlaneDescriptor, ProcessingError, SampleRange, Subsampling,
};
#[cfg(test)]
use super::super::{ColorConvertOptions, ResourceLimits};
use super::plan::ConversionPlan;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum NativeColor {
    Gray(f32),
    Rgb([f32; 3]),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NativePixel {
    pub(crate) color: NativeColor,
    pub(crate) alpha: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
struct PlaneAddress {
    plane: usize,
    offset: usize,
    subsampling: Subsampling,
    meaningful_bits: u8,
}

#[derive(Debug, Clone, Copy, Default)]
struct RoleAddresses {
    gray: Option<PlaneAddress>,
    red: Option<PlaneAddress>,
    green: Option<PlaneAddress>,
    blue: Option<PlaneAddress>,
    y: Option<PlaneAddress>,
    cb: Option<PlaneAddress>,
    cr: Option<PlaneAddress>,
    alpha: Option<PlaneAddress>,
}

/// Borrowed native pixel access planned once from an immutable frame.
///
/// The reader contains no image-sized allocation and never applies ICC,
/// transfer, primaries, alpha association, or output quantization.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NativePixelReader<'a> {
    source: &'a ImageFrame,
    plan: ConversionPlan<'a>,
    roles: RoleAddresses,
    model: ChannelModel,
}

impl<'a> NativePixelReader<'a> {
    #[cfg(test)]
    pub(crate) fn inspect(
        source: &'a ImageFrame,
        options: &ColorConvertOptions<'a>,
        limits: &ResourceLimits,
    ) -> std::result::Result<Self, ProcessingError> {
        let plan = ConversionPlan::inspect(source, options, limits)?;
        Self::from_plan(source, plan)
    }

    pub(crate) fn from_plan(
        source: &'a ImageFrame,
        plan: ConversionPlan<'a>,
    ) -> std::result::Result<Self, ProcessingError> {
        let model = source.descriptor().model();
        if model == ChannelModel::YCbCr && source.pixels().format() == PixelFormat::F32 {
            return Err(ProcessingError::Unsupported(
                "F32 YCbCr native reading is not defined".into(),
            ));
        }
        let roles = build_role_addresses(source, model)?;
        validate_reader_layout(source, model, roles)?;
        if matches!(source.pixels().format(), PixelFormat::U8 | PixelFormat::U16)
            && plan.native_sample_encoding().is_none()
        {
            return Err(ProcessingError::Unsupported(
                "integer native reading requires resolved sample encoding".into(),
            ));
        }
        Ok(Self {
            source,
            plan,
            roles,
            model,
        })
    }

    pub(crate) fn pixel(&self, x: u32, y: u32) -> std::result::Result<NativePixel, HighresError> {
        if x >= self.source.descriptor().width() || y >= self.source.descriptor().height() {
            return Err(HighresError::InvalidDimensions(
                "native pixel coordinate is outside the frame".into(),
            ));
        }
        let alpha = self
            .roles
            .alpha
            .map(|address| self.read_role_with_range(x, y, address, false, Some(SampleRange::Full)))
            .transpose()?
            .map(|value| value as f32);
        match self.model {
            ChannelModel::Gray => Ok(NativePixel {
                color: NativeColor::Gray(self.read_color_role(ChannelRole::Gray, x, y)? as f32),
                alpha,
            }),
            ChannelModel::RGB => Ok(NativePixel {
                color: NativeColor::Rgb([
                    self.read_color_role(ChannelRole::Red, x, y)? as f32,
                    self.read_color_role(ChannelRole::Green, x, y)? as f32,
                    self.read_color_role(ChannelRole::Blue, x, y)? as f32,
                ]),
                alpha,
            }),
            ChannelModel::YCbCr => {
                let y_value = self.read_color_role(ChannelRole::Y, x, y)?;
                let cb = self.interpolate_chroma(ChannelRole::Cb, x, y)?;
                let cr = self.interpolate_chroma(ChannelRole::Cr, x, y)?;
                let matrix = self
                    .plan
                    .native_sample_encoding()
                    .ok_or_else(|| HighresError::Unsupported("native matrix is missing".into()))?
                    .matrix();
                let (kr, kb) = match matrix {
                    MatrixCoefficients::Bt709 => (0.2126, 0.0722),
                    MatrixCoefficients::Bt601 => (0.299, 0.114),
                    MatrixCoefficients::Bt2020 => (0.2627, 0.0593),
                    MatrixCoefficients::Identity | MatrixCoefficients::Unspecified(_) => {
                        return Err(HighresError::Unsupported(
                            "YCbCr matrix is not reconstructible".into(),
                        ));
                    }
                };
                let red = y_value + 2.0 * (1.0 - kr) * cr;
                let blue = y_value + 2.0 * (1.0 - kb) * cb;
                let green = (y_value - kr * red - kb * blue) / (1.0 - kr - kb);
                Ok(NativePixel {
                    color: NativeColor::Rgb([red as f32, green as f32, blue as f32]),
                    alpha,
                })
            }
        }
    }

    fn read_color_role(
        &self,
        role: ChannelRole,
        x: u32,
        y: u32,
    ) -> std::result::Result<f64, HighresError> {
        let address = self.role(role)?;
        self.read_role(
            x,
            y,
            address,
            matches!(role, ChannelRole::Cb | ChannelRole::Cr),
        )
    }

    fn interpolate_chroma(
        &self,
        role: ChannelRole,
        x: u32,
        y: u32,
    ) -> std::result::Result<f64, HighresError> {
        let address = self.role(role)?;
        if address.subsampling == Subsampling::FULL {
            return self.read_color_role(role, x, y);
        }
        let encoding = self
            .plan
            .native_sample_encoding()
            .ok_or_else(|| HighresError::Unsupported("chroma encoding is missing".into()))?;
        let location = encoding.chroma_location().ok_or_else(|| {
            HighresError::Unsupported("subsampled chroma location is missing".into())
        })?;
        let plane = self.plane_descriptor(address.plane)?;
        let (x0, x1, wx) = interpolation_axis(
            x,
            address.subsampling.x(),
            location.x_phase(),
            plane.layout().width(),
        )?;
        let (y0, y1, wy) = interpolation_axis(
            y,
            address.subsampling.y(),
            location.y_phase(),
            plane.layout().height(),
        )?;
        let p00 = self.read_color_role_at(role, x0, y0)?;
        let p10 = self.read_color_role_at(role, x1, y0)?;
        let p01 = self.read_color_role_at(role, x0, y1)?;
        let p11 = self.read_color_role_at(role, x1, y1)?;
        Ok(p00 * (1.0 - wx) * (1.0 - wy)
            + p10 * wx * (1.0 - wy)
            + p01 * (1.0 - wx) * wy
            + p11 * wx * wy)
    }

    fn read_color_role_at(
        &self,
        role: ChannelRole,
        x: usize,
        y: usize,
    ) -> std::result::Result<f64, HighresError> {
        let x = u32::try_from(x)
            .map_err(|_| HighresError::InvalidDimensions("chroma x overflows".into()))?;
        let y = u32::try_from(y)
            .map_err(|_| HighresError::InvalidDimensions("chroma y overflows".into()))?;
        let address = self.role(role)?;
        self.read_role_at(x, y, address, true)
    }

    fn read_role(
        &self,
        x: u32,
        y: u32,
        address: PlaneAddress,
        chroma: bool,
    ) -> std::result::Result<f64, HighresError> {
        self.read_role_with_range(x, y, address, chroma, None)
    }

    fn read_role_with_range(
        &self,
        x: u32,
        y: u32,
        address: PlaneAddress,
        chroma: bool,
        range: Option<SampleRange>,
    ) -> std::result::Result<f64, HighresError> {
        let code = self.sample_value(x, y, address, false)?;
        if self.source.pixels().format() == PixelFormat::F32 {
            return Ok(code);
        }
        let range = match range {
            Some(range) => range,
            None => self.encoding_range()?,
        };
        expand_integer(code, address.meaningful_bits, range, chroma)
    }

    fn read_role_at(
        &self,
        x: u32,
        y: u32,
        address: PlaneAddress,
        chroma: bool,
    ) -> std::result::Result<f64, HighresError> {
        let code = self.sample_value(x, y, address, true)?;
        if self.source.pixels().format() == PixelFormat::F32 {
            return Ok(code);
        }
        expand_integer(
            code,
            address.meaningful_bits,
            self.encoding_range()?,
            chroma,
        )
    }

    fn sample_value(
        &self,
        x: u32,
        y: u32,
        address: PlaneAddress,
        plane_coordinates: bool,
    ) -> std::result::Result<f64, HighresError> {
        match self.source.pixels() {
            PixelBuffer::U8(planes) => {
                let plane = planes
                    .as_slice()
                    .get(address.plane)
                    .ok_or_else(|| HighresError::InvalidLayout("pixel plane is missing".into()))?;
                let sample = if plane_coordinates {
                    sample_at_plane(plane, x, y, address)?
                } else {
                    sample_at(plane, x, y, address)?
                };
                Ok(f64::from(sample))
            }
            PixelBuffer::U16(planes) => {
                let plane = planes
                    .as_slice()
                    .get(address.plane)
                    .ok_or_else(|| HighresError::InvalidLayout("pixel plane is missing".into()))?;
                let sample = if plane_coordinates {
                    sample_at_plane(plane, x, y, address)?
                } else {
                    sample_at(plane, x, y, address)?
                };
                Ok(f64::from(sample))
            }
            PixelBuffer::F32(planes) => {
                let plane = planes
                    .as_slice()
                    .get(address.plane)
                    .ok_or_else(|| HighresError::InvalidLayout("pixel plane is missing".into()))?;
                let sample = if plane_coordinates {
                    sample_at_plane(plane, x, y, address)?
                } else {
                    sample_at(plane, x, y, address)?
                };
                Ok(f64::from(sample))
            }
        }
    }

    fn encoding_range(&self) -> std::result::Result<SampleRange, HighresError> {
        self.plan
            .native_sample_encoding()
            .map(|encoding| encoding.range())
            .ok_or_else(|| HighresError::Unsupported("native range is missing".into()))
    }

    fn role(&self, role: ChannelRole) -> std::result::Result<PlaneAddress, HighresError> {
        let address = match role {
            ChannelRole::Gray => self.roles.gray,
            ChannelRole::Red => self.roles.red,
            ChannelRole::Green => self.roles.green,
            ChannelRole::Blue => self.roles.blue,
            ChannelRole::Y => self.roles.y,
            ChannelRole::Cb => self.roles.cb,
            ChannelRole::Cr => self.roles.cr,
            ChannelRole::Alpha => self.roles.alpha,
        };
        address
            .ok_or_else(|| HighresError::InvalidLayout("required channel role is missing".into()))
    }

    fn plane_descriptor(
        &self,
        index: usize,
    ) -> std::result::Result<&PlaneDescriptor, HighresError> {
        self.source
            .descriptor()
            .planes()
            .get(index)
            .ok_or_else(|| HighresError::InvalidLayout("pixel plane descriptor is missing".into()))
    }
}

fn build_role_addresses(
    source: &ImageFrame,
    model: ChannelModel,
) -> std::result::Result<RoleAddresses, ProcessingError> {
    let mut roles = RoleAddresses::default();
    for (plane, descriptor) in source.descriptor().planes().iter().enumerate() {
        for (index, role) in descriptor.roles().iter().copied().enumerate() {
            let offset = descriptor
                .layout()
                .channel_offsets()
                .get(index)
                .copied()
                .ok_or_else(|| {
                    ProcessingError::Invalid(HighresError::InvalidLayout(
                        "role/channel offset table is inconsistent".into(),
                    ))
                })?;
            let address = PlaneAddress {
                plane,
                offset,
                subsampling: descriptor.layout().subsampling(),
                meaningful_bits: descriptor.meaningful_bits(),
            };
            let slot = match role {
                ChannelRole::Gray => &mut roles.gray,
                ChannelRole::Red => &mut roles.red,
                ChannelRole::Green => &mut roles.green,
                ChannelRole::Blue => &mut roles.blue,
                ChannelRole::Y => &mut roles.y,
                ChannelRole::Cb => &mut roles.cb,
                ChannelRole::Cr => &mut roles.cr,
                ChannelRole::Alpha => &mut roles.alpha,
            };
            if slot.replace(address).is_some() {
                return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                    "channel role is repeated in native reader table".into(),
                )));
            }
        }
    }
    let required = match model {
        ChannelModel::Gray => [ChannelRole::Gray, ChannelRole::Red, ChannelRole::Green],
        ChannelModel::RGB => [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue],
        ChannelModel::YCbCr => [ChannelRole::Y, ChannelRole::Cb, ChannelRole::Cr],
    };
    let required_count = match model {
        ChannelModel::Gray => 1,
        ChannelModel::RGB | ChannelModel::YCbCr => 3,
    };
    for role in required.into_iter().take(required_count) {
        if role_address(roles, role).is_none() {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "required channel role is missing".into(),
            )));
        }
    }
    Ok(roles)
}

fn role_address(roles: RoleAddresses, role: ChannelRole) -> Option<PlaneAddress> {
    match role {
        ChannelRole::Gray => roles.gray,
        ChannelRole::Red => roles.red,
        ChannelRole::Green => roles.green,
        ChannelRole::Blue => roles.blue,
        ChannelRole::Y => roles.y,
        ChannelRole::Cb => roles.cb,
        ChannelRole::Cr => roles.cr,
        ChannelRole::Alpha => roles.alpha,
    }
}

fn validate_reader_layout(
    source: &ImageFrame,
    model: ChannelModel,
    roles: RoleAddresses,
) -> std::result::Result<(), ProcessingError> {
    let descriptors = source.descriptor().planes();
    let subsampling_422 = Subsampling::new(2, 1).map_err(ProcessingError::from)?;
    let subsampling_420 = Subsampling::new(2, 2).map_err(ProcessingError::from)?;
    for descriptor in descriptors {
        let subsampling = descriptor.layout().subsampling();
        let supported = match model {
            ChannelModel::Gray | ChannelModel::RGB => subsampling == Subsampling::FULL,
            ChannelModel::YCbCr => {
                subsampling == Subsampling::FULL
                    || subsampling == subsampling_422
                    || subsampling == subsampling_420
            }
        };
        if !supported {
            return Err(ProcessingError::Unsupported(
                "native reader does not support this subsampling layout".into(),
            ));
        }
    }
    if let Some(alpha) = roles.alpha
        && alpha.subsampling != Subsampling::FULL
    {
        return Err(ProcessingError::Unsupported(
            "native alpha must be full resolution".into(),
        ));
    }
    if model == ChannelModel::YCbCr {
        let y = roles.y.ok_or_else(|| {
            ProcessingError::Invalid(HighresError::InvalidLayout("Y role is missing".into()))
        })?;
        let cb = roles.cb.ok_or_else(|| {
            ProcessingError::Invalid(HighresError::InvalidLayout("Cb role is missing".into()))
        })?;
        let cr = roles.cr.ok_or_else(|| {
            ProcessingError::Invalid(HighresError::InvalidLayout("Cr role is missing".into()))
        })?;
        if y.subsampling != Subsampling::FULL
            || cb.subsampling != cr.subsampling
            || (cb.subsampling != Subsampling::FULL
                && cb.subsampling != subsampling_422
                && cb.subsampling != subsampling_420)
        {
            return Err(ProcessingError::Unsupported(
                "native reader requires equal supported Cb/Cr subsampling".into(),
            ));
        }
    }
    Ok(())
}

fn sample_at<T: Copy>(
    plane: &Plane<T>,
    x: u32,
    y: u32,
    address: PlaneAddress,
) -> std::result::Result<T, HighresError> {
    let x = usize::try_from(x)
        .map_err(|_| HighresError::InvalidDimensions("pixel x does not fit usize".into()))?
        / usize::from(address.subsampling.x());
    let y = usize::try_from(y)
        .map_err(|_| HighresError::InvalidDimensions("pixel y does not fit usize".into()))?
        / usize::from(address.subsampling.y());
    let x = u32::try_from(x)
        .map_err(|_| HighresError::InvalidDimensions("plane x does not fit u32".into()))?;
    let y = u32::try_from(y)
        .map_err(|_| HighresError::InvalidDimensions("plane y does not fit u32".into()))?;
    sample_at_plane(plane, x, y, address)
}

fn sample_at_plane<T: Copy>(
    plane: &Plane<T>,
    x: u32,
    y: u32,
    address: PlaneAddress,
) -> std::result::Result<T, HighresError> {
    let x = usize::try_from(x)
        .map_err(|_| HighresError::InvalidDimensions("plane x does not fit usize".into()))?;
    let y = usize::try_from(y)
        .map_err(|_| HighresError::InvalidDimensions("plane y does not fit usize".into()))?;
    let width = usize::try_from(plane.layout().width())
        .map_err(|_| HighresError::InvalidDimensions("plane width does not fit usize".into()))?;
    let height = usize::try_from(plane.layout().height())
        .map_err(|_| HighresError::InvalidDimensions("plane height does not fit usize".into()))?;
    if x >= width || y >= height {
        return Err(HighresError::InvalidDimensions(
            "pixel coordinate is outside the plane".into(),
        ));
    }
    let row = y
        .checked_mul(plane.layout().row_stride())
        .ok_or_else(|| HighresError::InvalidLayout("row address overflows".into()))?;
    let column = x
        .checked_mul(plane.layout().pixel_stride())
        .and_then(|value| value.checked_add(address.offset))
        .ok_or_else(|| HighresError::InvalidLayout("column address overflows".into()))?;
    let index = row
        .checked_add(column)
        .ok_or_else(|| HighresError::InvalidLayout("sample address overflows".into()))?;
    plane
        .samples()
        .get(index)
        .copied()
        .ok_or_else(|| HighresError::InvalidLayout("sample address exceeds allocation".into()))
}

fn expand_integer(
    code: f64,
    meaningful_bits: u8,
    range: SampleRange,
    chroma: bool,
) -> std::result::Result<f64, HighresError> {
    let max = 1u64
        .checked_shl(u32::from(meaningful_bits))
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| HighresError::InvalidSamples("bit depth is too large".into()))?
        as f64;
    match range {
        SampleRange::Full => {
            let center = if chroma { (max + 1.0) / 2.0 } else { 0.0 };
            Ok((code - center) / max)
        }
        SampleRange::Limited => {
            if meaningful_bits < 8 {
                return Err(HighresError::Unsupported(
                    "limited-range expansion requires at least 8 bits".into(),
                ));
            }
            let scale = 1u64
                .checked_shl(u32::from(meaningful_bits - 8))
                .ok_or_else(|| HighresError::InvalidSamples("range scale is too large".into()))?
                as f64;
            let (offset, denominator) = if chroma {
                (128.0 * scale, 224.0 * scale)
            } else {
                (16.0 * scale, 219.0 * scale)
            };
            Ok((code - offset) / denominator)
        }
    }
}

fn interpolation_axis(
    index: u32,
    factor: u8,
    phase: ChromaPhase,
    extent: u32,
) -> std::result::Result<(usize, usize, f64), HighresError> {
    if extent == 0 || factor == 0 {
        return Err(HighresError::InvalidDimensions(
            "chroma interpolation extent is invalid".into(),
        ));
    }
    if factor == 1 {
        let index = usize::try_from(index).map_err(|_| {
            HighresError::InvalidDimensions("full-resolution index overflows".into())
        })?;
        let extent = usize::try_from(extent).map_err(|_| {
            HighresError::InvalidDimensions("plane extent does not fit usize".into())
        })?;
        if index >= extent {
            return Err(HighresError::InvalidDimensions(
                "full-resolution index exceeds plane".into(),
            ));
        }
        return Ok((index, index, 0.0));
    }
    let phase = match phase {
        ChromaPhase::Zero => 0.0,
        ChromaPhase::Half => 0.5,
        ChromaPhase::One => 1.0,
    };
    let coordinate = (f64::from(index) - phase) / f64::from(factor);
    let base = coordinate.floor();
    let max = f64::from(extent - 1);
    let lower = base.clamp(0.0, max) as usize;
    let upper = (base + 1.0).clamp(0.0, max) as usize;
    Ok((lower, upper, coordinate - base))
}

#[cfg(test)]
#[path = "convert_native_alloc_tests.rs"]
mod allocation_tests;
#[cfg(test)]
pub(crate) use allocation_tests::ActualAllocationDenyGuard;
#[cfg(test)]
pub(crate) use allocation_tests::AllocationGuard;
#[cfg(test)]
pub(crate) use allocation_tests::DeallocationObservationGuard;
#[cfg(all(test, miri))]
pub(crate) use allocation_tests::drop_registered_value;
#[cfg(test)]
#[path = "convert_native_tests.rs"]
mod tests;
