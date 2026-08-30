use avif_codec::RichAvifInfo;
use std::mem::size_of;

use super::allocation::ConstructionLedger;
use crate::highres::ProcessingError;
use crate::highres::ownership::{
    copy_owner_with_ledger, fresh_owner_with_ledger, reserve_provenance_with_ledger,
    reserve_unknown_with_ledger,
};
use crate::highres::{
    Av1Description, CleanAperture, ColorInformationSet, ColorProvenance, FrameMetadata,
    GeometryOperation, IccColorType, NclxColorInformation, PixelChannelInformation,
    PixelInformation, PixelSubsampling, RawRational, Rotation,
};

fn add_bytes(total: &mut usize, value: usize) -> Result<(), ProcessingError> {
    *total = total
        .checked_add(value)
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?;
    Ok(())
}

pub(super) fn metadata_final_owned_bytes(rich: &RichAvifInfo) -> Result<usize, ProcessingError> {
    let source = &rich.color_information;
    let mut total = 2usize
        .checked_mul(size_of::<(u32, u32)>())
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?;
    let geometry_count = usize::from(rich.info.rotation.is_some())
        .checked_add(usize::from(rich.info.mirror.is_some()))
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?;
    add_bytes(
        &mut total,
        geometry_count
            .checked_mul(2)
            .and_then(|count| count.checked_mul(size_of::<crate::highres::GeometryOperation>()))
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?,
    )?;
    let mut color = 0usize;
    if let Some(profile) = &source.icc_profile {
        add_bytes(&mut color, profile.len())?;
        add_bytes(&mut color, size_of::<ColorProvenance>())?;
    }
    if source.nclx.is_some() {
        add_bytes(&mut color, 8 + size_of::<ColorProvenance>())?;
    }
    add_bytes(
        &mut color,
        source
            .unknown_colr
            .len()
            .checked_mul(size_of::<crate::highres::UnknownColorInformation>())
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?,
    )?;
    for unknown in &source.unknown_colr {
        add_bytes(&mut color, unknown.payload.len())?;
    }
    add_bytes(
        &mut total,
        color
            .checked_mul(2)
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?,
    )?;
    if let Some(pixi) = &rich.info.pixel_information {
        add_bytes(&mut total, pixi.bits_per_channel.len())?;
        if let Some(channels) = &pixi.extended_channels {
            add_bytes(
                &mut total,
                channels
                    .len()
                    .checked_mul(size_of::<PixelChannelInformation>())
                    .ok_or_else(|| {
                        ProcessingError::ResourceLimit("metadata size overflows".into())
                    })?,
            )?;
        }
    }
    Ok(total)
}

pub(super) fn metadata_borrowed_bytes(rich: &RichAvifInfo) -> Result<usize, ProcessingError> {
    let mut source_total = 0;
    let source = &rich.color_information;
    if let Some(projected) = &rich.info.color_information {
        add_bytes(&mut source_total, size_of::<avif_codec::ColorInformation>())?;
        add_bytes(&mut source_total, projected.payload.capacity())?;
    }
    if let Some(profile) = &source.icc_profile {
        add_bytes(&mut source_total, profile.capacity())?;
    }
    if source.icc_color_type.is_some() {
        add_bytes(&mut source_total, size_of::<[u8; 4]>())?;
    }
    if source.nclx.is_some() {
        add_bytes(
            &mut source_total,
            size_of::<avif_codec::NclxColorInformation>(),
        )?;
    }
    add_bytes(
        &mut source_total,
        source
            .unknown_colr
            .capacity()
            .checked_mul(size_of::<avif_codec::ColorInformation>())
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?,
    )?;
    for unknown in &source.unknown_colr {
        add_bytes(&mut source_total, unknown.payload.capacity())?;
    }
    if let Some(pixi) = &rich.info.pixel_information {
        add_bytes(&mut source_total, pixi.bits_per_channel.capacity())?;
        if let Some(channels) = &pixi.extended_channels {
            add_bytes(
                &mut source_total,
                channels
                    .capacity()
                    .checked_mul(size_of::<avif_codec::PixelChannelInformation>())
                    .ok_or_else(|| {
                        ProcessingError::ResourceLimit("metadata size overflows".into())
                    })?,
            )?;
        }
    }
    if rich.info.clean_aperture.is_some() {
        add_bytes(&mut source_total, size_of::<avif_codec::CleanAperture>())?;
    }

    Ok(source_total)
}

/// Bytes owned by the colour metadata copy produced from a RichAvifInfo.
/// This is a length-based projection; borrowed capacities are accounted for
/// separately by `metadata_borrowed_bytes`.
pub(super) fn projected_color_bytes(
    rich: &RichAvifInfo,
    include_av1: bool,
) -> Result<usize, ProcessingError> {
    let source = &rich.color_information;
    let mut total = 0usize;
    let mut provenance = 0usize;
    if let Some(profile) = &source.icc_profile {
        add_bytes(&mut total, profile.len())?;
        provenance = provenance
            .checked_add(1)
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?;
    }
    if source.nclx.is_some() {
        add_bytes(&mut total, 8)?;
        provenance = provenance
            .checked_add(1)
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?;
    }
    add_bytes(
        &mut total,
        source
            .unknown_colr
            .len()
            .checked_mul(size_of::<crate::highres::UnknownColorInformation>())
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?,
    )?;
    for unknown in &source.unknown_colr {
        add_bytes(&mut total, unknown.payload.len())?;
    }
    if include_av1 {
        add_bytes(&mut total, crate::highres::AV1_COLOR_INFORMATION_BYTES)?;
        provenance = provenance
            .checked_add(1)
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata size overflows".into()))?;
    }
    add_bytes(&mut total, provenance)?;
    Ok(total)
}

#[allow(dead_code)]
pub(super) fn color_information(
    rich: &RichAvifInfo,
) -> Result<ColorInformationSet, ProcessingError> {
    color_information_impl(rich, &mut None)
}

fn color_information_impl(
    rich: &RichAvifInfo,
    ledger: &mut Option<&mut ConstructionLedger>,
) -> Result<ColorInformationSet, ProcessingError> {
    let mut set = ColorInformationSet::new();
    let source = &rich.color_information;
    if let Some(profile) = &source.icc_profile {
        let copied = copy_owner_with_ledger(
            crate::highres::output_plan::OwnerKey::IccProfile,
            ledger,
            profile,
        )?;
        reserve_provenance_with_ledger(&mut set, ledger)?;
        set.set_icc_profile(copied).map_err(ProcessingError::from)?;
        if let Some(kind) = source.icc_color_type {
            if kind == *b"prof" {
                set.set_icc_color_type(IccColorType::Prof);
            } else if kind == *b"rICC" {
                set.set_icc_color_type(IccColorType::Ricc);
            } else {
                return Err(ProcessingError::Unsupported(
                    "unsupported ICC colour type".into(),
                ));
            }
        }
    } else if let Some(kind) = source.icc_color_type
        && kind != *b"prof"
        && kind != *b"rICC"
    {
        return Err(ProcessingError::Unsupported(
            "unsupported ICC colour type".into(),
        ));
    }
    if let Some(nclx) = source.nclx {
        reserve_provenance_with_ledger(&mut set, ledger)?;
        set.set_nclx(NclxColorInformation::new(
            nclx.color_primaries,
            nclx.transfer_characteristics,
            nclx.matrix_coefficients,
            nclx.full_range_flag,
        ));
    }
    reserve_unknown_with_ledger(&mut set, source.unknown_colr.len(), ledger)?;
    for (index, unknown) in source.unknown_colr.iter().enumerate() {
        let payload = copy_owner_with_ledger(
            crate::highres::output_plan::OwnerKey::UnknownPayload(index),
            ledger,
            &unknown.payload,
        )?;
        set.push_unknown_colr(crate::highres::UnknownColorInformation {
            color_type: unknown.color_type,
            payload,
        });
    }
    Ok(set)
}

#[allow(dead_code)]
pub(super) fn frame_metadata(rich: &RichAvifInfo) -> Result<FrameMetadata, ProcessingError> {
    frame_metadata_impl(rich, &mut None)
}

pub(super) fn frame_metadata_with_ledger(
    rich: &RichAvifInfo,
    ledger: &mut ConstructionLedger,
) -> Result<FrameMetadata, ProcessingError> {
    frame_metadata_impl(rich, &mut Some(ledger))
}

fn frame_metadata_impl(
    rich: &RichAvifInfo,
    ledger: &mut Option<&mut ConstructionLedger>,
) -> Result<FrameMetadata, ProcessingError> {
    let colors = color_information_impl(rich, ledger)?;
    let mut metadata = FrameMetadata::new(colors);
    let info = &rich.info;
    if let Some(pixi) = &info.pixel_information {
        let channels = pixi
            .extended_channels
            .as_ref()
            .map(
                |source| -> Result<Vec<PixelChannelInformation>, ProcessingError> {
                    let mut channels = fresh_owner_with_ledger(
                        crate::highres::output_plan::OwnerKey::PixiExtended,
                        ledger,
                        source.len(),
                    )?;
                    for channel in source {
                        channels.push(PixelChannelInformation::new(
                            channel.channel_idc,
                            channel.component_format,
                            channel.subsampling.map(|subsampling| {
                                PixelSubsampling::new(
                                    subsampling.subsampling_type,
                                    subsampling.subsampling_location,
                                )
                            }),
                        ));
                    }
                    Ok(channels)
                },
            )
            .transpose()?;
        let bits = copy_owner_with_ledger(
            crate::highres::output_plan::OwnerKey::PixiBits,
            ledger,
            &pixi.bits_per_channel,
        )?;
        let pixel_information =
            PixelInformation::new(bits, channels).map_err(ProcessingError::from)?;
        metadata.set_pixel_information(Some(pixel_information));
    }
    let operation_count = usize::from(info.rotation.is_some())
        .checked_add(usize::from(info.mirror.is_some()))
        .ok_or_else(|| ProcessingError::ResourceLimit("geometry count overflows".into()))?;
    let mut coded = Vec::new();
    coded
        .try_reserve_exact(operation_count)
        .map_err(|_| ProcessingError::Allocation("geometry allocation failed".into()))?;
    if let Some(crop) = info.clean_aperture {
        let aperture = CleanAperture::new(
            RawRational::new(crop.width_n, crop.width_d).map_err(ProcessingError::from)?,
            RawRational::new(crop.height_n, crop.height_d).map_err(ProcessingError::from)?,
            RawRational::new(crop.horizontal_offset_n, crop.horizontal_offset_d)
                .map_err(ProcessingError::from)?,
            RawRational::new(crop.vertical_offset_n, crop.vertical_offset_d)
                .map_err(ProcessingError::from)?,
        );
        metadata.set_clean_aperture(Some(aperture));
    }
    if let Some(rotation) = info.rotation {
        let rotation = match rotation.angle % 4 {
            0 => Rotation::None,
            1 => Rotation::Degrees90,
            2 => Rotation::Degrees180,
            _ => Rotation::Degrees270,
        };
        metadata.set_rotation(rotation);
        coded.push(GeometryOperation::Rotate(rotation));
    }
    if let Some(mirror) = info.mirror {
        if mirror.axis == 0 {
            metadata.set_mirror(true, false);
            coded.push(GeometryOperation::MirrorHorizontal);
        } else {
            metadata.set_mirror(false, true);
            coded.push(GeometryOperation::MirrorVertical);
        }
    }
    let render = copy_owner_with_ledger(
        crate::highres::output_plan::OwnerKey::RenderGeometry,
        ledger,
        &coded,
    )?;
    metadata.set_coded_geometry(coded);
    metadata.set_render_geometry(render);
    Ok(metadata)
}

pub(super) fn av1_description(frame: &avif_codec::DecodedFrame) -> Av1Description {
    let config = frame.color_config;
    let position = config.chroma_sample_position.map(|value| match value {
        avif_codec::av1::ChromaSamplePosition::Unknown => 0,
        avif_codec::av1::ChromaSamplePosition::Vertical => 1,
        avif_codec::av1::ChromaSamplePosition::Colocated => 2,
        avif_codec::av1::ChromaSamplePosition::Reserved => 3,
    });
    Av1Description::new(true).with_flags(
        Some(config.monochrome),
        Some(matches!(
            config.color_range,
            avif_codec::av1::ColorRange::Full
        )),
        Some(config.subsampling_x),
        Some(config.subsampling_y),
        position,
    )
}
