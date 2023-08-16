/*
 * tiff/tags.rs  Mith@mmk (C) 2022
 * use MIT License
 */

 use bin_rs::io::read_string;
use crate::metadata::DataMap;
use super::header::DataPack;
use super::util::convert;
use super::util::convert_utf16;
use super::util::convert_utf16_le;

pub(super) fn convert_windows_tag(data: DataPack, length: usize) -> DataMap {
    match data {
        DataPack::Bytes(d) => {
            DataMap::I18NString(convert_utf16_le(d.to_vec()))
        },
        DataPack::Undef(d,_) => {
            DataMap::I18NString(convert_utf16_le(d.to_vec()))
        },
        _ => {
            convert(&data, length)
        }
    }
}


pub fn gps_mapper(tag :u16, data: &DataPack,length: usize) -> (String,DataMap){
    let tagname;
    let s = match tag {
            0x0000 => {
            tagname = "GPSVersionID";
            convert(&data,length)
        },
            0x0001 => {
            tagname = "GPSLatitudeRef";
            convert(&data,length)
        },
           
            0x0002 => {
            tagname = "GPSLatitude";
            convert(&data,length)
        },
            0x0003 => {
            tagname = "GPSLongitudeRef";
            convert(&data,length)
        },
            0x0004 => {
            tagname = "GPSLongitude";
            convert(&data,length)
        },
            0x0005 => {
            tagname = "GPSAltitudeRef";
            convert(&data,length)
        },
                       
            0x0006 => {
            tagname = "GPSAltitude";
            convert(&data,length)
        },
            0x0007 => {
            tagname = "GPSTimeStamp";
            convert(&data,length)
        },
            0x0008 => {
            tagname = "GPSSatellites";
            convert(&data,length)
        },
            0x0009 => {
            tagname = "GPSStatus";
            convert(&data,length)
        },
            0x000a => {
            tagname = "GPSMeasureMode";
            convert(&data,length)
        },
            0x000b => {
            tagname = "GPSDOP";
            convert(&data,length)
        },
            0x000c => {
            tagname = "GPSSpeedRef";
            convert(&data,length)
        },               
            0x000d => {
            tagname = "GPSSpeed";
            convert(&data,length)
        },
            0x000e => {
            tagname = "GPSTrackRef";
            convert(&data,length)
        },
            0x000f => {
            tagname = "GPSTrack";
            convert(&data,length)
        },
            0x0010 => {
            tagname = "GPSImgDirectionRef";
            convert(&data,length)
        },
            0x0011 => {
            tagname = "GPSImgDirection";
            convert(&data,length)
        },
            0x0012 => {
            tagname = "GPSMapDatum";
            convert(&data,length)
        },
            0x0013 => {
            tagname = "GPSDestLatitudeRef";
            convert(&data,length)
        },
            0x0014 => {
            tagname = "GPSDestLatitude";
            convert(&data,length)
        },
            0x0015 => {
            tagname = "GPSDestLongitudeRef";
            convert(&data,length)
        },
            0x0016 => {
            tagname = "GPSDestLongitude";
            convert(&data,length)
        },
            0x0017 => {
            tagname = "GPSDestBearingRef";
            convert(&data,length)
        },
            0x0018 => {
            tagname = "GPSDestBearing";
            convert(&data,length)
        },
            0x0019 => {
            tagname = "GPSDestDistanceRef";
            convert(&data,length)
        },
            0x001a => {
            tagname = "GPSDestDistance";
            convert(&data,length)
        },
            0x001b => {
            tagname = "GPSProcessingMethod";
            convert(&data,length)
        },
            0x001c => {
            tagname = "GPSAreaInformation";
            convert(&data,length)
        },
            0x001d => {
            tagname = "GPSDateStamp";
            convert(&data,length)
        },
            0x001e => {
            tagname = "GPSDifferential";
            convert(&data,length)
        },
            0x001f => {
            tagname = "GPSHPositioningError";
            convert(&data,length)
        },
        _=> {
             tagname = "UnKnown";
             convert(&data,length)
        },
    };
    (tagname.to_string(),s)
}

pub fn tag_mapper(tag :u16, data: &DataPack,length: usize) -> (String,DataMap) {
    let tagname;
    let s = match tag {
        0x0001 => {
            tagname = "InteropIndex";
            convert(&data,length)
        },
        0x0002 => {
            tagname = "InteropVersion";
            convert(&data,length)
        },
        0x000b => {
            tagname = "ProcessingSoftware";
            convert(&data,length)
     
        },
        0x00fe => {
            tagname = "SubfileType";
            convert(&data,length)
     
        },
        0x00ff => {
            tagname = "OldSubfileType";
            convert(&data,length)
     
        },
        0x0100 => {
            tagname = "ImageWidth";
            convert(&data,length)
     
        },
        0x0101 => {
            tagname = "ImageHeight";
            convert(&data,length)
     
        },
        0x0102 => {
            tagname = "BitsPerSample";
            convert(&data,length)
     
        },
        0x0103 => {
            tagname = "Compression";
            convert(&data,length)
     
        },
        0x0106 => {
            tagname = "PhotometricInterpretation";
            convert(&data,length)
     
        },
        0x0107 => {
            tagname = "Thresholding";
            convert(&data,length)
     
        },
        0x0108 => {
            tagname = "CellWidth";
            convert(&data,length)
     
        },
        0x0109 => {
            tagname = "CellLength";
            convert(&data,length)
     
        },
        0x010a => {
            tagname = "FillOrder";
            convert(&data,length)
     
        },
        0x010d => {
            tagname = "DocumentName";
            convert(&data,length)
     
        },
        0x010e => {
            tagname = "ImageDescription";
            convert(&data,length)
     
        },
        0x010f => {
            tagname = "Make";
            convert(&data,length)
     
        },
        0x0110 => {
            tagname = "Model";
            convert(&data,length)
     
        },
        0x0111 => {
            tagname = "StripOffsets";
            convert(&data,length)
     
        },
        0x0112 => {
            tagname = "Orientation";
            convert(&data,length)
     
        },
        0x0115 => {
            tagname = "SamplesPerPixel";
            convert(&data,length)
     
        },
        0x0116 => {
            tagname = "RowsPerStrip";
            convert(&data,length)
     
        },
        0x0117 => {
            tagname = "StripByteCounts";
            convert(&data,length)
     
        },
        0x0118 => {
            tagname = "MinSampleValue";
            convert(&data,length)
     
        },
        0x0119 => {
            tagname = "MaxSampleValue";
            convert(&data,length)
     
        },
        0x011a => {
            tagname = "XResolution";
            convert(&data,length)
     
        },
        0x011b => {
            tagname = "YResolution";
            convert(&data,length)
     
        },
        0x011c => {
            tagname = "PlanarConfiguration";
            convert(&data,length)
     
        },
        0x011d => {
            tagname = "PageName";
            convert(&data,length)
     
        },
        0x011e => {
            tagname = "XPosition";
            convert(&data,length)
     
        },
        0x011f => {
            tagname = "YPosition";
            convert(&data,length)
     
        },
        0x0120 => {
            tagname = "FreeOffsets";
            convert(&data,length)
     
        },
        0x0121 => {
            tagname = "FreeByteCounts";
            convert(&data,length)
     
        },
        0x0122 => {
            tagname = "GrayResponseUnit";
            convert(&data,length)
     
        },
        0x0123 => {
            tagname = "GrayResponseCurve";
            convert(&data,length)
     
        },
        0x0124 => {
            tagname = "T4Options";
            convert(&data,length)
     
        },
        0x0125 => {
            tagname = "T6Options";
            convert(&data,length)
     
        },
        0x0128 => {
            tagname = "ResolutionUnit";
            convert(&data,length)
     
        },
        0x0129 => {
            tagname = "PageNumber";
            convert(&data,length)
     
        },
        0x012c => {
            tagname = "ColorResponseUnit";
            convert(&data,length)
     
        },
        0x012d => {
            tagname = "TransferFunction";
            convert(&data,length)
     
        },
        0x0131 => {
            tagname = "Software";
            convert(&data,length)
     
        },
        0x0132 => {
            tagname = "ModifyDate";
            convert(&data,length)
     
        },
        0x013b => {
            tagname = "Artist";
            convert(&data,length)
     
        },
        0x013c => {
            tagname = "HostComputer";
            convert(&data,length)
     
        },
        0x013d => {
            tagname = "Predictor";
            convert(&data,length)
     
        },
        0x013e => {
            tagname = "WhitePoint";
            convert(&data,length)
     
        },
        0x013f => {
            tagname = "PrimaryChromaticities";
            convert(&data,length)
     
        },
        0x0140 => {
            tagname = "ColorMap";
            convert(&data,length)
     
        },
        0x0141 => {
            tagname = "HalftoneHints";
            convert(&data,length)
     
        },
        0x0142 => {
            tagname = "TileWidth";
            convert(&data,length)
     
        },
        0x0143 => {
            tagname = "TileLength";
            convert(&data,length)
     
        },
        0x0144 => {
            tagname = "TileOffsets";
            convert(&data,length)
     
        },
        0x0145 => {
            tagname = "TileByteCounts";
            convert(&data,length)
     
        },
        0x0146 => {
            tagname = "BadFaxLines";
            convert(&data,length)
     
        },
        0x0147 => {
            tagname = "CleanFaxData";
            convert(&data,length)
     
        },
        0x0148 => {
            tagname = "ConsecutiveBadFaxLines";
            convert(&data,length)
     
        },
        0x014a => {
            tagname = "SubIFD";
            convert(&data,length)
     
        },
        0x014c => {
            tagname = "InkSet";
            convert(&data,length)
     
        },
        0x014d => {
            tagname = "InkNames";
            convert(&data,length)
     
        },
        0x014e => {
            tagname = "NumberofInks";
            convert(&data,length)
     
        },
        0x0150 => {
            tagname = "DotRange";
            convert(&data,length)
     
        },
        0x0151 => {
            tagname = "TargetPrinter";
            convert(&data,length)
     
        },
        0x0152 => {
            tagname = "ExtraSamples";
            convert(&data,length)
     
        },
        0x0153 => {
            tagname = "SampleFormat";
            convert(&data,length)
     
        },
        0x0154 => {
            tagname = "SMinSampleValue";
            convert(&data,length)
     
        },
        0x0155 => {
            tagname = "SMaxSampleValue";
            convert(&data,length)
     
        },
        0x0156 => {
            tagname = "TransferRange";
            convert(&data,length)
     
        },
        0x0157 => {
            tagname = "ClipPath";
            convert(&data,length)
     
        },
        0x0158 => {
            tagname = "XClipPathUnits";
            convert(&data,length)
     
        },
        0x0159 => {
            tagname = "YClipPathUnits";
            convert(&data,length)
     
        },
        0x015a => {
            tagname = "Indexed";
            convert(&data,length)
     
        },
        0x015b => {
            tagname = "JPEGTables";
            convert(&data,length)
     
        },
        0x015f => {
            tagname = "OPIProxy";
            convert(&data,length)
     
        },
        0x0190 => {
            tagname = "GlobalParametersIFD";
            convert(&data,length)
     
        },
        0x0191 => {
            tagname = "ProfileType";
            convert(&data,length)
     
        },
        0x0192 => {
            tagname = "FaxProfile";
            convert(&data,length)
     
        },
        0x0193 => {
            tagname = "CodingMethods";
            convert(&data,length)
     
        },
        0x0194 => {
            tagname = "VersionYear";
            convert(&data,length)
     
        },
        0x0195 => {
            tagname = "ModeNumber";
            convert(&data,length)
     
        },
        0x01b1 => {
            tagname = "Decode";
            convert(&data,length)
     
        },
        0x01b2 => {
            tagname = "DefaultImageColor";
            convert(&data,length)
     
        },
        0x01b3 => {
            tagname = "T82Options";
            convert(&data,length)
     
        },
        0x01b5 => {
            tagname = "JPEGTables";
            convert(&data,length)
     
        },
        0x0200 => {
            tagname = "JPEGProc";
            convert(&data,length)
     
        },
        0x0201 => {
            tagname = "ThumbnailOffset";
            convert(&data,length)
     
        },
        0x0202 => {
            tagname = "ThumbnailLength";
            convert(&data,length)
     
        },
        0x0203 => {
            tagname = "JPEGRestartInterval";
            convert(&data,length)
     
        },
        0x0205 => {
            tagname = "JPEGLosslessPredictors";
            convert(&data,length)
     
        },
        0x0206 => {
            tagname = "JPEGPointTransforms";
            convert(&data,length)
     
        },
        0x0207 => {
            tagname = "JPEGQTables";
            convert(&data,length)
     
        },
        0x0208 => {
            tagname = "JPEGDCTables";
            convert(&data,length)
     
        },
        0x0209 => {
            tagname = "JPEGACTables";
            convert(&data,length)
     
        },
        0x0211 => {
            tagname = "YCbCrCoefficients";
            convert(&data,length)
     
        },
        0x0212 => {
            tagname = "YCbCrSubSampling";
            convert(&data,length)
     
        },
        0x0213 => {
            tagname = "YCbCrPositioning";
            convert(&data,length)
     
        },
        0x0214 => {
            tagname = "ReferenceBlackWhite";
            convert(&data,length)
     
        },
        0x022f => {
            tagname = "StripRowCounts";
            convert(&data,length)
     
        },
        0x02bc => {
            tagname = "ApplicationNotes";
            convert(&data,length)
     
        },
        0x03e7 => {
            tagname = "USPTOMiscellaneous";
            convert(&data,length)
     
        },
        0x1000 => {
            tagname = "RelatedImageFileFormat";
            convert(&data,length)
     
        },
        0x1001 => {
            tagname = "RelatedImageWidth";
            convert(&data,length)
     
        },
        0x1002 => {
            tagname = "RelatedImageHeight";
            convert(&data,length)
     
        },
        0x4746 => {
            tagname = "Rating";
            convert(&data,length)
     
        },
        0x4747 => {
            tagname = "XP_DIP_XML";
            convert(&data,length)
     
        },
        0x4748 => {
            tagname = "StitchInfo";
            convert(&data,length)
     
        },
        0x4749 => {
            tagname = "RatingPercent";
            convert(&data,length)
     
        },
        0x7000 => {
            tagname = "SonyRawFileType";
            convert(&data,length)
     
        },
        0x7010 => {
            tagname = "SonyToneCurve";
            convert(&data,length)
     
        },
        0x7031 => {
            tagname = "VignettingCorrection";
            convert(&data,length)
     
        },
        0x7032 => {
            tagname = "VignettingCorrParams";
            convert(&data,length)
     
        },
        0x7034 => {
            tagname = "ChromaticAberrationCorrection";
            convert(&data,length)
     
        },
        0x7035 => {
            tagname = "ChromaticAberrationCorrParams";
            convert(&data,length)
     
        },
        0x7036 => {
            tagname = "DistortionCorrection";
            convert(&data,length)
     
        },
        0x7037 => {
            tagname = "DistortionCorrParams";
            convert(&data,length)
     
        },
        0x74c7 => {
            tagname = "SonyCropTopLeft";
            convert(&data,length)
     
        },
        0x74c8 => {
            tagname = "SonyCropSize";
            convert(&data,length)
     
        },
        0x800d => {
            tagname = "ImageID";
            convert(&data,length)
     
        },
        0x80a3 => {
            tagname = "WangTag1";
            convert(&data,length)
     
        },
        0x80a4 => {
            tagname = "WangAnnotation";
            convert(&data,length)
     
        },
        0x80a5 => {
            tagname = "WangTag3";
            convert(&data,length)
     
        },
        0x80a6 => {
            tagname = "WangTag4";
            convert(&data,length)
     
        },
        0x80b9 => {
            tagname = "ImageReferencePoints";
            convert(&data,length)
     
        },
        0x80ba => {
            tagname = "RegionXformTackPoint";
            convert(&data,length)
     
        },
        0x80bb => {
            tagname = "WarpQuadrilateral";
            convert(&data,length)
     
        },
        0x80bc => {
            tagname = "AffineTransformMat";
            convert(&data,length)
     
        },
        0x80e3 => {
            tagname = "Matteing";
            convert(&data,length)
     
        },
        0x80e4 => {
            tagname = "DataType";
            convert(&data,length)
     
        },
        0x80e5 => {
            tagname = "ImageDepth";
            convert(&data,length)
     
        },
        0x80e6 => {
            tagname = "TileDepth";
            convert(&data,length)
     
        },
        0x8214 => {
            tagname = "ImageFullWidth";
            convert(&data,length)
     
        },
        0x8215 => {
            tagname = "ImageFullHeight";
            convert(&data,length)
     
        },
        0x8216 => {
            tagname = "TextureFormat";
            convert(&data,length)
     
        },
        0x8217 => {
            tagname = "WrapModes";
            convert(&data,length)
     
        },
        0x8218 => {
            tagname = "FovCot";
            convert(&data,length)
     
        },
        0x8219 => {
            tagname = "MatrixWorldToScreen";
            convert(&data,length)
     
        },
        0x821a => {
            tagname = "MatrixWorldToCamera";
            convert(&data,length)
     
        },
        0x827d => {
            tagname = "Model2";
            convert(&data,length)
     
        },
        0x828d => {
            tagname = "CFARepeatPatternDim";
            convert(&data,length)
     
        },
        0x828e => {
            tagname = "CFAPattern2";
            convert(&data,length)
     
        },
        0x828f => {
            tagname = "BatteryLevel";
            convert(&data,length)
     
        },
        0x8290 => {
            tagname = "KodakIFD";
            convert(&data,length)
     
        },
        0x8298 => {
            tagname = "Copyright";
            convert(&data,length)
     
        },
        0x829a => {
            tagname = "ExposureTime";
            convert(&data,length)
     
        },
        0x829d => {
            tagname = "FNumber";
            convert(&data,length)
     
        },
        0x82a5 => {
            tagname = "MDFileTag";
            convert(&data,length)
     
        },
        0x82a6 => {
            tagname = "MDScalePixel";
            convert(&data,length)
     
        },
        0x82a7 => {
            tagname = "MDColorTable";
            convert(&data,length)
     
        },
        0x82a8 => {
            tagname = "MDLabName";
            convert(&data,length)
     
        },
        0x82a9 => {
            tagname = "MDSampleInfo";
            convert(&data,length)
     
        },
        0x82aa => {
            tagname = "MDPrepDate";
            convert(&data,length)
     
        },
        0x82ab => {
            tagname = "MDPrepTime";
            convert(&data,length)
     
        },
        0x82ac => {
            tagname = "MDFileUnits";
            convert(&data,length)
     
        },
        0x830e => {
            tagname = "PixelScale";
            convert(&data,length)
     
        },
        0x8335 => {
            tagname = "AdventScale";
            convert(&data,length)
     
        },
        0x8336 => {
            tagname = "AdventRevision";
            convert(&data,length)
     
        },
        0x835c => {
            tagname = "UIC1Tag";
            convert(&data,length)
     
        },
        0x835d => {
            tagname = "UIC2Tag";
            convert(&data,length)
     
        },
        0x835e => {
            tagname = "UIC3Tag";
            convert(&data,length)
     
        },
        0x835f => {
            tagname = "UIC4Tag";
            convert(&data,length)
     
        },
        0x83bb => {
            tagname = "IPTC-NAA";
            convert(&data,length)
     
        },
        0x847e => {
            tagname = "IntergraphPacketData";
            convert(&data,length)
     
        },
        0x847f => {
            tagname = "IntergraphFlagRegisters";
            convert(&data,length)
     
        },
        0x8480 => {
            tagname = "IntergraphMatrix";
            convert(&data,length)
     
        },
        0x8481 => {
            tagname = "INGRReserved";
            convert(&data,length)
     
        },
        0x8482 => {
            tagname = "ModelTiePoint";
            convert(&data,length)
     
        },
        0x84e0 => {
            tagname = "Site";
            convert(&data,length)
     
        },
        0x84e1 => {
            tagname = "ColorSequence";
            convert(&data,length)
     
        },
        0x84e2 => {
            tagname = "IT8Header";
            convert(&data,length)
     
        },
        0x84e3 => {
            tagname = "RasterPadding";
            convert(&data,length)
     
        },
        0x84e4 => {
            tagname = "BitsPerRunLength";
            convert(&data,length)
     
        },
        0x84e5 => {
            tagname = "BitsPerExtendedRunLength";
            convert(&data,length)
     
        },
        0x84e6 => {
            tagname = "ColorTable";
            convert(&data,length)
     
        },
        0x84e7 => {
            tagname = "ImageColorIndicator";
            convert(&data,length)
     
        },
        0x84e8 => {
            tagname = "BackgroundColorIndicator";
            convert(&data,length)
     
        },
        0x84e9 => {
            tagname = "ImageColorValue";
            convert(&data,length)
     
        },
        0x84ea => {
            tagname = "BackgroundColorValue";
            convert(&data,length)
     
        },
        0x84eb => {
            tagname = "PixelIntensityRange";
            convert(&data,length)
     
        },
        0x84ec => {
            tagname = "TransparencyIndicator";
            convert(&data,length)
     
        },
        0x84ed => {
            tagname = "ColorCharacterization";
            convert(&data,length)
     
        },
        0x84ee => {
            tagname = "HCUsage";
            convert(&data,length)
     
        },
        0x84ef => {
            tagname = "TrapIndicator";
            convert(&data,length)
     
        },
        0x84f0 => {
            tagname = "CMYKEquivalent";
            convert(&data,length)
     
        },
        0x8546 => {
            tagname = "SEMInfo";
            convert(&data,length)
     
        },
        0x8568 => {
            tagname = "AFCP_IPTC";
            convert(&data,length)
     
        },
        0x85b8 => {
            tagname = "PixelMagicJBIGOptions";
            convert(&data,length)
     
        },
        0x85d7 => {
            tagname = "JPLCartoIFD";
            convert(&data,length)
     
        },
        0x85d8 => {
            tagname = "ModelTransform";
            convert(&data,length)
     
        },
        0x8602 => {
            tagname = "WB_GRGBLevels";
            convert(&data,length)
     
        },
        0x8606 => {
            tagname = "LeafData";
            convert(&data,length)
     
        },
        0x8649 => {
            tagname = "PhotoshopSettings";
            convert(&data,length)
     
        },
        0x8769 => {
            tagname = "ExifOffset";
            convert(&data,length)
     
        },
        0x8773 => {
            tagname = "ICC_Profile";
            if let DataPack::Undef(data,_) = data {
                DataMap::ICCProfile(data.to_vec())
            } else {
                convert(&data,length)
            }
     
        },
        0x877f => {
            tagname = "TIFF_FXExtensions";
            convert(&data,length)
     
        },
        0x8780 => {
            tagname = "MultiProfiles";
            convert(&data,length)
     
        },
        0x8781 => {
            tagname = "SharedData";
            convert(&data,length)
     
        },
        0x8782 => {
            tagname = "T88Options";
            convert(&data,length)
     
        },
        0x87ac => {
            tagname = "ImageLayer";
            convert(&data,length)
     
        },
        0x87af => {
            tagname = "GeoTiffDirectory";
            convert(&data,length)
     
        },
        0x87b0 => {
            tagname = "GeoTiffDoubleParams";
            convert(&data,length)
     
        },
        0x87b1 => {
            tagname = "GeoTiffAsciiParams";
            convert(&data,length)
     
        },
        0x87be => {
            tagname = "JBIGOptions";
            convert(&data,length)
     
        },
        0x8822 => {
            tagname = "ExposureProgram";
            convert(&data,length)
     
        },
        0x8824 => {
            tagname = "SpectralSensitivity";
            convert(&data,length)
     
        },
        0x8825 => {
            tagname = "GPSInfo";
            convert(&data,length)
     
        },
        0x8827 => {
            tagname = "ISO";
            convert(&data,length)
     
        },
        0x8828 => {
            tagname = "Opto-ElectricConvFactor";
            convert(&data,length)
     
        },
        0x8829 => {
            tagname = "Interlace";
            convert(&data,length)
     
        },
        0x882a => {
            tagname = "TimeZoneOffset";
            convert(&data,length)
     
        },
        0x882b => {
            tagname = "SelfTimerMode";
            convert(&data,length)
     
        },
        0x8830 => {
            tagname = "SensitivityType";
            convert(&data,length)
     
        },
        0x8831 => {
            tagname = "StandardOutputSensitivity";
            convert(&data,length)
     
        },
        0x8832 => {
            tagname = "RecommendedExposureIndex";
            convert(&data,length)
     
        },
        0x8833 => {
            tagname = "ISOSpeed";
            convert(&data,length)
     
        },
        0x8834 => {
            tagname = "ISOSpeedLatitudeyyy";
            convert(&data,length)
     
        },
        0x8835 => {
            tagname = "ISOSpeedLatitudezzz";
            convert(&data,length)
     
        },
        0x885c => {
            tagname = "FaxRecvParams";
            convert(&data,length)
     
        },
        0x885d => {
            tagname = "FaxSubAddress";
            convert(&data,length)
     
        },
        0x885e => {
            tagname = "FaxRecvTime";
            convert(&data,length)
     
        },
        0x8871 => {
            tagname = "FedexEDR";
            convert(&data,length)
     
        },
        0x888a => {
            tagname = "LeafSubIFD";
            convert(&data,length)
     
        },
        0x9000 => {
            tagname = "ExifVersion";
            convert(&data,length)
     
        },
        0x9003 => {
            tagname = "DateTimeOriginal";
            convert(&data,length)
     
        },
        0x9004 => {
            tagname = "CreateDate";
            convert(&data,length)
     
        },
        0x9009 => {
            tagname = "GooglePlusUploadCode";
            convert(&data,length)
     
        },
        0x9010 => {
            tagname = "OffsetTime";
            convert(&data,length)
     
        },
        0x9011 => {
            tagname = "OffsetTimeOriginal";
            convert(&data,length)
     
        },
        0x9012 => {
            tagname = "OffsetTimeDigitized";
            convert(&data,length)
     
        },
        0x9101 => {
            tagname = "ComponentsConfiguration";
            convert(&data,length)
     
        },
        0x9102 => {
            tagname = "CompressedBitsPerPixel";
            convert(&data,length)
     
        },
        0x9201 => {
            tagname = "ShutterSpeedValue";
            convert(&data,length)
     
        },
        0x9202 => {
            tagname = "ApertureValue";
            convert(&data,length)
     
        },
        0x9203 => {
            tagname = "BrightnessValue";
            convert(&data,length)
     
        },
        0x9204 => {
            tagname = "ExposureCompensation";
            convert(&data,length)
     
        },
        0x9205 => {
            tagname = "MaxApertureValue";
            convert(&data,length)
     
        },
        0x9206 => {
            tagname = "SubjectDistance";
            convert(&data,length)
     
        },
        0x9207 => {
            tagname = "MeteringMode";
            convert(&data,length)
     
        },
        0x9208 => {
            tagname = "LightSource";
            convert(&data,length)
     
        },
        0x9209 => {
            tagname = "Flash";
            convert(&data,length)
     
        },
        0x920a => {
            tagname = "FocalLength";
            convert(&data,length)
     
        },
        0x920b => {
            tagname = "FlashEnergy";
            convert(&data,length)
     
        },
        0x920c => {
            tagname = "SpatialFrequencyResponse";
            convert(&data,length)
     
        },
        0x920d => {
            tagname = "Noise";
            convert(&data,length)
     
        },
        0x920e => {
            tagname = "FocalPlaneXResolution";
            convert(&data,length)
     
        },
        0x920f => {
            tagname = "FocalPlaneYResolution";
            convert(&data,length)
     
        },
        0x9210 => {
            tagname = "FocalPlaneResolutionUnit";
            convert(&data,length)
     
        },
        0x9211 => {
            tagname = "ImageNumber";
            convert(&data,length)
     
        },
        0x9212 => {
            tagname = "SecurityClassification";
            convert(&data,length)
     
        },
        0x9213 => {
            tagname = "ImageHistory";
            convert(&data,length)
     
        },
        0x9214 => {
            tagname = "SubjectArea";
            convert(&data,length)
     
        },
        0x9215 => {
            tagname = "ExposureIndex";
            convert(&data,length)
     
        },
        0x9216 => {
            tagname = "TIFF-EPStandardID";
            convert(&data,length)
     
        },
        0x9217 => {
            tagname = "SensingMethod";
            convert(&data,length)
     
        },
        0x923a => {
            tagname = "CIP3DataFile";
            convert(&data,length)
     
        },
        0x923b => {
            tagname = "CIP3Sheet";
            convert(&data,length)
     
        },
        0x923c => {
            tagname = "CIP3Side";
            convert(&data,length)
     
        },
        0x923f => {
            tagname = "StoNits";
            convert(&data,length)
     
        },
        0x927c => {
            tagname = "MakerNoteApple";
            match data {
                DataPack::Undef(d,_) => {
                    DataMap::Ascii(read_string(d, 0, d.len()))
                },
                _ => {
                    convert(&data,length)
                }
            }
        },
        0x9286 => {
            tagname = "UserComment";
            // get endien?
            
            match data {
                DataPack::Undef(d, endien) => {
                    // get 8byte
                    let d = d.to_vec();
                    let mut d8 = vec![0;8];
                    d8.copy_from_slice(&d[0..8]);
                    let d8 = String::from_utf8(d8).unwrap();
                    match d8.as_str() {
                        "ASCII\0\0\0" => {
                            DataMap::Ascii(read_string(&d, 8, d.len()))
                        },
                        "UNICODE\0" => {
                            let mut d = d.to_vec();
                            d.drain(0..8);
                            // utf16 to utf8
                            let d16:String = convert_utf16(d, *endien);
                            // print!("{}", d16);        
                            DataMap::I18NString(d16)
                        },
                        // "JIS\0\0\0\0\0" => {
                        // noimpl
                        // },
                        _ => {
                            convert(&data,length)
                        }
                    }
                },
                _ => {
                    convert(&data,length)
                }   
            }
        },
        0x9290 => {
            tagname = "SubSecTime";
            convert(&data,length)
     
        },
        0x9291 => {
            tagname = "SubSecTimeOriginal";
            convert(&data,length)
     
        },
        0x9292 => {
            tagname = "SubSecTimeDigitized";
            convert(&data,length)
     
        },
        0x932f => {
            tagname = "MSDocumentText";
            convert(&data,length)
     
        },
        0x9330 => {
            tagname = "MSPropertySetStorage";
            convert(&data,length)
     
        },
        0x9331 => {
            tagname = "MSDocumentTextPosition";
            convert(&data,length)
     
        },
        0x935c => {
            tagname = "ImageSourceData";
            convert(&data,length)
     
        },
        0x9400 => {
            tagname = "AmbientTemperature";
            convert(&data,length)
     
        },
        0x9401 => {
            tagname = "Humidity";
            convert(&data,length)
     
        },
        0x9402 => {
            tagname = "Pressure";
            convert(&data,length)
     
        },
        0x9403 => {
            tagname = "WaterDepth";
            convert(&data,length)
     
        },
        0x9404 => {
            tagname = "Acceleration";
            convert(&data,length)
     
        },
        0x9405 => {
            tagname = "CameraElevationAngle";
            convert(&data,length)
     
        },
        0x9c9b => {
            tagname = "XPTitle";
            convert_windows_tag(data.clone(), length)
        },
        0x9c9c => {
            tagname = "XPComment";
            convert_windows_tag(data.clone(), length)
        },
        0x9c9d => {
            tagname = "XPAuthor";
            convert_windows_tag(data.clone(), length)
        },
        0x9c9e => {
            tagname = "XPKeywords";
            convert_windows_tag(data.clone(), length)
        },
        0x9c9f => {
            tagname = "XPSubject";
            convert_windows_tag(data.clone(), length)
        },
        0xa000 => {
            tagname = "FlashpixVersion";
            convert(&data,length)     
        },
        0xa001 => {
            tagname = "ColorSpace";
            convert(&data,length)
     
        },
        0xa002 => {
            tagname = "ExifImageWidth";
            convert(&data,length)
     
        },
        0xa003 => {
            tagname = "ExifImageHeight";
            convert(&data,length)
     
        },
        0xa004 => {
            tagname = "RelatedSoundFile";
            convert(&data,length)
     
        },
        0xa005 => {
            tagname = "InteropOffset";
            convert(&data,length)
     
        },
        0xa010 => {
            tagname = "SamsungRawPointersOffset";
            convert(&data,length)
     
        },
        0xa011 => {
            tagname = "SamsungRawPointersLength";
            convert(&data,length)
     
        },
        0xa101 => {
            tagname = "SamsungRawByteOrder";
            convert(&data,length)
     
        },
        0xa102 => {
            tagname = "SamsungRawUnknown?";
            convert(&data,length)
     
        },
        0xa20b => {
            tagname = "FlashEnergy";
            convert(&data,length)
     
        },
        0xa20c => {
            tagname = "SpatialFrequencyResponse";
            convert(&data,length)
     
        },
        0xa20d => {
            tagname = "Noise";
            convert(&data,length)
     
        },
        0xa20e => {
            tagname = "FocalPlaneXResolution";
            convert(&data,length)
     
        },
        0xa20f => {
            tagname = "FocalPlaneYResolution";
            convert(&data,length)
     
        },
        0xa210 => {
            tagname = "FocalPlaneResolutionUnit";
            convert(&data,length)
     
        },
        0xa211 => {
            tagname = "ImageNumber";
            convert(&data,length)
     
        },
        0xa212 => {
            tagname = "SecurityClassification";
            convert(&data,length)
     
        },
        0xa213 => {
            tagname = "ImageHistory";
            convert(&data,length)
     
        },
        0xa214 => {
            tagname = "SubjectLocation";
            convert(&data,length)
     
        },
        0xa215 => {
            tagname = "ExposureIndex";
            convert(&data,length)
     
        },
        0xa216 => {
            tagname = "TIFF-EPStandardID";
            convert(&data,length)
     
        },
        0xa217 => {
            tagname = "SensingMethod";
            convert(&data,length)
     
        },
        0xa300 => {
            tagname = "FileSource";
            convert(&data,length)
     
        },
        0xa301 => {
            tagname = "SceneType";
            convert(&data,length)
     
        },
        0xa302 => {
            tagname = "CFAPattern";
            convert(&data,length)
     
        },
        0xa401 => {
            tagname = "CustomRendered";
            convert(&data,length)
     
        },
        0xa402 => {
            tagname = "ExposureMode";
            convert(&data,length)
     
        },
        0xa403 => {
            tagname = "WhiteBalance";
            convert(&data,length)
     
        },
        0xa404 => {
            tagname = "DigitalZoomRatio";
            convert(&data,length)
     
        },
        0xa405 => {
            tagname = "FocalLengthIn35mmFormat";
            convert(&data,length)
     
        },
        0xa406 => {
            tagname = "SceneCaptureType";
            convert(&data,length)
     
        },
        0xa407 => {
            tagname = "GainControl";
            convert(&data,length)
     
        },
        0xa408 => {
            tagname = "Contrast";
            convert(&data,length)
     
        },
        0xa409 => {
            tagname = "Saturation";
            convert(&data,length)
     
        },
        0xa40a => {
            tagname = "Sharpness";
            convert(&data,length)
     
        },
        0xa40b => {
            tagname = "DeviceSettingDescription";
            convert(&data,length)
     
        },
        0xa40c => {
            tagname = "SubjectDistanceRange";
            convert(&data,length)
     
        },
        0xa420 => {
            tagname = "ImageUniqueID";
            convert(&data,length)
     
        },
        0xa430 => {
            tagname = "OwnerName";
            convert(&data,length)
     
        },
        0xa431 => {
            tagname = "SerialNumber";
            convert(&data,length)
     
        },
        0xa432 => {
            tagname = "LensInfo";
            convert(&data,length)
     
        },
        0xa433 => {
            tagname = "LensMake";
            convert(&data,length)
     
        },
        0xa434 => {
            tagname = "LensModel";
            convert(&data,length)
     
        },
        0xa435 => {
            tagname = "LensSerialNumber";
            convert(&data,length)
     
        },
        0xa460 => {
            tagname = "CompositeImage";
            convert(&data,length)
     
        },
        0xa461 => {
            tagname = "CompositeImageCount";
            convert(&data,length)
     
        },
        0xa462 => {
            tagname = "CompositeImageExposureTimes";
            convert(&data,length)
     
        },
        0xa480 => {
            tagname = "GDALMetadata";
            convert(&data,length)
     
        },
        0xa481 => {
            tagname = "GDALNoData";
            convert(&data,length)
     
        },
        0xa500 => {
            tagname = "Gamma";
            convert(&data,length)
     
        },
        0xafc0 => {
            tagname = "ExpandSoftware";
            convert(&data,length)
     
        },
        0xafc1 => {
            tagname = "ExpandLens";
            convert(&data,length)
     
        },
        0xafc2 => {
            tagname = "ExpandFilm";
            convert(&data,length)
     
        },
        0xafc3 => {
            tagname = "ExpandFilterLens";
            convert(&data,length)
     
        },
        0xafc4 => {
            tagname = "ExpandScanner";
            convert(&data,length)
     
        },
        0xafc5 => {
            tagname = "ExpandFlashLamp";
            convert(&data,length)
     
        },
        0xb4c3 => {
            tagname = "HasselbladRawImage";
            convert(&data,length)
     
        },
        0xbc01 => {
            tagname = "PixelFormat";
            convert(&data,length)
     
        },
        0xbc02 => {
            tagname = "Transformation";
            convert(&data,length)
     
        },
        0xbc03 => {
            tagname = "Uncompressed";
            convert(&data,length)
     
        },
        0xbc04 => {
            tagname = "ImageType";
            convert(&data,length)
     
        },
        0xbc80 => {
            tagname = "ImageWidth";
            convert(&data,length)
     
        },
        0xbc81 => {
            tagname = "ImageHeight";
            convert(&data,length)
     
        },
        0xbc82 => {
            tagname = "WidthResolution";
            convert(&data,length)
     
        },
        0xbc83 => {
            tagname = "HeightResolution";
            convert(&data,length)
     
        },
        0xbcc0 => {
            tagname = "ImageOffset";
            convert(&data,length)
     
        },
        0xbcc1 => {
            tagname = "ImageByteCount";
            convert(&data,length)
     
        },
        0xbcc2 => {
            tagname = "AlphaOffset";
            convert(&data,length)
     
        },
        0xbcc3 => {
            tagname = "AlphaByteCount";
            convert(&data,length)
     
        },
        0xbcc4 => {
            tagname = "ImageDataDiscard";
            convert(&data,length)
     
        },
        0xbcc5 => {
            tagname = "AlphaDataDiscard";
            convert(&data,length)
     
        },
        0xc427 => {
            tagname = "OceScanjobDesc";
            convert(&data,length)
     
        },
        0xc428 => {
            tagname = "OceApplicationSelector";
            convert(&data,length)
     
        },
        0xc429 => {
            tagname = "OceIDNumber";
            convert(&data,length)
     
        },
        0xc42a => {
            tagname = "OceImageLogic";
            convert(&data,length)
     
        },
        0xc44f => {
            tagname = "Annotations";
            convert(&data,length)
     
        },
        0xc4a5 => {
            tagname = "PrintIM";
            convert(&data,length)
     
        },
        0xc51b => {
            tagname = "HasselbladExif";
            convert(&data,length)
     
        },
        0xc573 => {
            tagname = "OriginalFileName";
            convert(&data,length)
     
        },
        0xc580 => {
            tagname = "USPTOOriginalContentType";
            convert(&data,length)
     
        },
        0xc5e0 => {
            tagname = "CR2CFAPattern";
            convert(&data,length)
     
        },
        0xc612 => {
            tagname = "DNGVersion";
            convert(&data,length)
     
        },
        0xc613 => {
            tagname = "DNGBackwardVersion";
            convert(&data,length)
     
        },
        0xc614 => {
            tagname = "UniqueCameraModel";
            convert(&data,length)
     
        },
        0xc615 => {
            tagname = "LocalizedCameraModel";
            convert(&data,length)
     
        },
        0xc616 => {
            tagname = "CFAPlaneColor";
            convert(&data,length)
     
        },
        0xc617 => {
            tagname = "CFALayout";
            convert(&data,length)
     
        },
        0xc618 => {
            tagname = "LinearizationTable";
            convert(&data,length)
     
        },
        0xc619 => {
            tagname = "BlackLevelRepeatDim";
            convert(&data,length)
     
        },
        0xc61a => {
            tagname = "BlackLevel";
            convert(&data,length)
     
        },
        0xc61b => {
            tagname = "BlackLevelDeltaH";
            convert(&data,length)
     
        },
        0xc61c => {
            tagname = "BlackLevelDeltaV";
            convert(&data,length)
     
        },
        0xc61d => {
            tagname = "WhiteLevel";
            convert(&data,length)
     
        },
        0xc61e => {
            tagname = "DefaultScale";
            convert(&data,length)
     
        },
        0xc61f => {
            tagname = "DefaultCropOrigin";
            convert(&data,length)
     
        },
        0xc620 => {
            tagname = "DefaultCropSize";
            convert(&data,length)
     
        },
        0xc621 => {
            tagname = "ColorMatrix1";
            convert(&data,length)
     
        },
        0xc622 => {
            tagname = "ColorMatrix2";
            convert(&data,length)
     
        },
        0xc623 => {
            tagname = "CameraCalibration1";
            convert(&data,length)
     
        },
        0xc624 => {
            tagname = "CameraCalibration2";
            convert(&data,length)
     
        },
        0xc625 => {
            tagname = "ReductionMatrix1";
            convert(&data,length)
     
        },
        0xc626 => {
            tagname = "ReductionMatrix2";
            convert(&data,length)
     
        },
        0xc627 => {
            tagname = "AnalogBalance";
            convert(&data,length)
     
        },
        0xc628 => {
            tagname = "AsShotNeutral";
            convert(&data,length)
     
        },
        0xc629 => {
            tagname = "AsShotWhiteXY";
            convert(&data,length)
     
        },
        0xc62a => {
            tagname = "BaselineExposure";
            convert(&data,length)
     
        },
        0xc62b => {
            tagname = "BaselineNoise";
            convert(&data,length)
     
        },
        0xc62c => {
            tagname = "BaselineSharpness";
            convert(&data,length)
     
        },
        0xc62d => {
            tagname = "BayerGreenSplit";
            convert(&data,length)
     
        },
        0xc62e => {
            tagname = "LinearResponseLimit";
            convert(&data,length)
     
        },
        0xc62f => {
            tagname = "CameraSerialNumber";
            convert(&data,length)
     
        },
        0xc630 => {
            tagname = "DNGLensInfo";
            convert(&data,length)
     
        },
        0xc631 => {
            tagname = "ChromaBlurRadius";
            convert(&data,length)
     
        },
        0xc632 => {
            tagname = "AntiAliasStrength";
            convert(&data,length)
     
        },
        0xc633 => {
            tagname = "ShadowScale";
            convert(&data,length)
     
        },
        0xc634 => {
            tagname = "SR2Private";
            convert(&data,length)
     
        },
        0xc635 => {
            tagname = "MakerNoteSafety";
            convert(&data,length)
     
        },
        0xc640 => {
            tagname = "RawImageSegmentation";
            convert(&data,length)
     
        },
        0xc65a => {
            tagname = "CalibrationIlluminant1";
            convert(&data,length)
     
        },
        0xc65b => {
            tagname = "CalibrationIlluminant2";
            convert(&data,length)
     
        },
        0xc65c => {
            tagname = "BestQualityScale";
            convert(&data,length)
     
        },
        0xc65d => {
            tagname = "RawDataUniqueID";
            convert(&data,length)
     
        },
        0xc660 => {
            tagname = "AliasLayerMetadata";
            convert(&data,length)
     
        },
        0xc68b => {
            tagname = "OriginalRawFileName";
            convert(&data,length)
     
        },
        0xc68c => {
            tagname = "OriginalRawFileData";
            convert(&data,length)
     
        },
        0xc68d => {
            tagname = "ActiveArea";
            convert(&data,length)
     
        },
        0xc68e => {
            tagname = "MaskedAreas";
            convert(&data,length)
     
        },
        0xc68f => {
            tagname = "AsShotICCProfile";
            convert(&data,length)
     
        },
        0xc690 => {
            tagname = "AsShotPreProfileMatrix";
            convert(&data,length)
     
        },
        0xc691 => {
            tagname = "CurrentICCProfile";
            convert(&data,length)
     
        },
        0xc692 => {
            tagname = "CurrentPreProfileMatrix";
            convert(&data,length)
     
        },
        0xc6bf => {
            tagname = "ColorimetricReference";
            convert(&data,length)
     
        },
        0xc6c5 => {
            tagname = "SRawType";
            convert(&data,length)
     
        },
        0xc6d2 => {
            tagname = "PanasonicTitle";
            convert(&data,length)
     
        },
        0xc6d3 => {
            tagname = "PanasonicTitle2";
            convert(&data,length)
     
        },
        0xc6f3 => {
            tagname = "CameraCalibrationSig";
            convert(&data,length)
     
        },
        0xc6f4 => {
            tagname = "ProfileCalibrationSig";
            convert(&data,length)
     
        },
        0xc6f5 => {
            tagname = "ProfileIFD";
            convert(&data,length)
     
        },
        0xc6f6 => {
            tagname = "AsShotProfileName";
            convert(&data,length)
     
        },
        0xc6f7 => {
            tagname = "NoiseReductionApplied";
            convert(&data,length)
     
        },
        0xc6f8 => {
            tagname = "ProfileName";
            convert(&data,length)
     
        },
        0xc6f9 => {
            tagname = "ProfileHueSatMapDims";
            convert(&data,length)
     
        },
        0xc6fa => {
            tagname = "ProfileHueSatMapData1";
            convert(&data,length)
     
        },
        0xc6fb => {
            tagname = "ProfileHueSatMapData2";
            convert(&data,length)
     
        },
        0xc6fc => {
            tagname = "ProfileToneCurve";
            convert(&data,length)
     
        },
        0xc6fd => {
            tagname = "ProfileEmbedPolicy";
            convert(&data,length)
     
        },
        0xc6fe => {
            tagname = "ProfileCopyright";
            convert(&data,length)
     
        },
        0xc714 => {
            tagname = "ForwardMatrix1";
            convert(&data,length)
     
        },
        0xc715 => {
            tagname = "ForwardMatrix2";
            convert(&data,length)
     
        },
        0xc716 => {
            tagname = "PreviewApplicationName";
            convert(&data,length)
     
        },
        0xc717 => {
            tagname = "PreviewApplicationVersion";
            convert(&data,length)
     
        },
        0xc718 => {
            tagname = "PreviewSettingsName";
            convert(&data,length)
     
        },
        0xc719 => {
            tagname = "PreviewSettingsDigest";
            convert(&data,length)
     
        },
        0xc71a => {
            tagname = "PreviewColorSpace";
            convert(&data,length)
     
        },
        0xc71b => {
            tagname = "PreviewDateTime";
            convert(&data,length)
     
        },
        0xc71c => {
            tagname = "RawImageDigest";
            convert(&data,length)
     
        },
        0xc71d => {
            tagname = "OriginalRawFileDigest";
            convert(&data,length)
     
        },
        0xc71e => {
            tagname = "SubTileBlockSize";
            convert(&data,length)
     
        },
        0xc71f => {
            tagname = "RowInterleaveFactor";
            convert(&data,length)
     
        },
        0xc725 => {
            tagname = "ProfileLookTableDims";
            convert(&data,length)
     
        },
        0xc726 => {
            tagname = "ProfileLookTableData";
            convert(&data,length)
     
        },
        0xc740 => {
            tagname = "OpcodeList1";
            convert(&data,length)
     
        },
        0xc741 => {
            tagname = "OpcodeList2";
            convert(&data,length)
     
        },
        0xc74e => {
            tagname = "OpcodeList3";
            convert(&data,length)
     
        },
        0xc761 => {
            tagname = "NoiseProfile";
            convert(&data,length)
     
        },
        0xc763 => {
            tagname = "TimeCodes";
            convert(&data,length)
     
        },
        0xc764 => {
            tagname = "FrameRate";
            convert(&data,length)
     
        },
        0xc772 => {
            tagname = "TStop";
            convert(&data,length)
     
        },
        0xc789 => {
            tagname = "ReelName";
            convert(&data,length)
     
        },
        0xc791 => {
            tagname = "OriginalDefaultFinalSize";
            convert(&data,length)
     
        },
        0xc792 => {
            tagname = "OriginalBestQualitySize";
            convert(&data,length)
     
        },
        0xc793 => {
            tagname = "OriginalDefaultCropSize";
            convert(&data,length)
     
        },
        0xc7a1 => {
            tagname = "CameraLabel";
            convert(&data,length)
     
        },
        0xc7a3 => {
            tagname = "ProfileHueSatMapEncoding";
            convert(&data,length)
     
        },
        0xc7a4 => {
            tagname = "ProfileLookTableEncoding";
            convert(&data,length)
     
        },
        0xc7a5 => {
            tagname = "BaselineExposureOffset";
            convert(&data,length)
     
        },
        0xc7a6 => {
            tagname = "DefaultBlackRender";
            convert(&data,length)
     
        },
        0xc7a7 => {
            tagname = "NewRawImageDigest";
            convert(&data,length)
     
        },
        0xc7a8 => {
            tagname = "RawToPreviewGain";
            convert(&data,length)
     
        },
        0xc7aa => {
            tagname = "CacheVersion";
            convert(&data,length)
     
        },
        0xc7b5 => {
            tagname = "DefaultUserCrop";
            convert(&data,length)
     
        },
        0xc7d5 => {
            tagname = "NikonNEFInfo";
            convert(&data,length)
     
        },
        0xc7e9 => {
            tagname = "DepthFormat";
            convert(&data,length)
     
        },
        0xc7ea => {
            tagname = "DepthNear";
            convert(&data,length)
     
        },
        0xc7eb => {
            tagname = "DepthFar";
            convert(&data,length)
     
        },
        0xc7ec => {
            tagname = "DepthUnits";
            convert(&data,length)
     
        },
        0xc7ed => {
            tagname = "DepthMeasureType";
            convert(&data,length)
     
        },
        0xc7ee => {
            tagname = "EnhanceParams";
            convert(&data,length)
     
        },
        0xcd2d => {
            tagname = "ProfileGainTableMap";
            convert(&data,length)
     
        },
        0xcd2e => {
            tagname = "SemanticName";
            convert(&data,length)
     
        },
        0xcd30 => {
            tagname = "SemanticInstanceIFD";
            convert(&data,length)
     
        },
        0xcd31 => {
            tagname = "CalibrationIlluminant3";
            convert(&data,length)
     
        },
        0xcd32 => {
            tagname = "CameraCalibration3";
            convert(&data,length)
     
        },
        0xcd33 => {
            tagname = "ColorMatrix3";
            convert(&data,length)
     
        },
        0xcd34 => {
            tagname = "ForwardMatrix3";
            convert(&data,length)
     
        },
        0xcd35 => {
            tagname = "IlluminantData1";
            convert(&data,length)
     
        },
        0xcd36 => {
            tagname = "IlluminantData2";
            convert(&data,length)
     
        },
        0xcd37 => {
            tagname = "IlluminantData3";
            convert(&data,length)
     
        },
        0xcd38 => {
            tagname = "MaskSubArea";
            convert(&data,length)
     
        },
        0xcd39 => {
            tagname = "ProfileHueSatMapData3";
            convert(&data,length)
     
        },
        0xcd3a => {
            tagname = "ReductionMatrix3";
            convert(&data,length)
     
        },
        0xcd3b => {
            tagname = "RGBTables";
            convert(&data,length)
     
        },
        0xea1c => {
            tagname = "Padding";
            convert(&data,length)
     
        },
        0xea1d => {
            tagname = "OffsetSchema";
            convert(&data,length)
     
        },
        0xfde8 => {
            tagname = "OwnerName";
            convert(&data,length)
        },
        0xfde9 => {
            tagname = "SerialNumber";
            convert(&data,length)
     
        },
        0xfdea => {
            tagname = "Lens";
            convert(&data,length)
     
        },
        0xfe00 => {
            tagname = "KDC_IFD";
            convert(&data,length)
     
        },
        0xfe4c => {
            tagname = "RawFile";
            convert(&data,length)
     
        },
        0xfe4d => {
            tagname = "Converter";
            convert(&data,length)
     
        },
        0xfe4e => {
            tagname = "WhiteBalance";
            convert(&data,length)
     
        },
        0xfe51 => {
            tagname = "Exposure";
            convert(&data,length)
        },
        0xfe52 => {
            tagname = "Shadows";
            convert(&data,length)
     
        },
        0xfe53 => {
            tagname = "Brightness";
            convert(&data,length)
     
        },
        0xfe54 => {
            tagname = "Contrast";
            convert(&data,length)
     
        },
        0xfe55 => {
            tagname = "Saturation";
            convert(&data,length)
     
        },
        0xfe56 => {
            tagname = "Sharpness";
            convert(&data,length)
     
        },
        0xfe57 => {
            tagname = "Smoothness";
            convert(&data,length)
     
         },
        0xfe58 => {
            tagname = "MoireFilter";
            convert(&data,length)
     
         },
        _ => {
            tagname = "Unknown";
            convert(&data,length)       
       },
    };
    (tagname.to_string(), s)
}
