/*
 * tiff/tags.rs  Mith@mmk (C) 2022
 * use MIT License
 */

use super::super::io::read_string;
use super::header::DataPack;
use super::util::print_data;
pub fn gps_mapper(tag :u16, data: &DataPack) -> (String,String){
    let tagname;
    let s;
    match tag {
            0x0000 => {
            tagname = "GPSVersionID";
            s = print_data(&data);
        },
            0x0001 => {
            tagname = "GPSLatitudeRef";
            s = print_data(&data);
        },
           
            0x0002 => {
            tagname = "GPSLatitude";
            s = print_data(&data);
        },
            0x0003 => {
            tagname = "GPSLongitudeRef";
            s = print_data(&data);
        },
            0x0004 => {
            tagname = "GPSLongitude";
            s = print_data(&data);
        },
            0x0005 => {
            tagname = "GPSAltitudeRef";
            s = print_data(&data);
        },
                       
            0x0006 => {
            tagname = "GPSAltitude";
            s = print_data(&data);
        },
            0x0007 => {
            tagname = "GPSTimeStamp";
            s = print_data(&data);
        },
            0x0008 => {
            tagname = "GPSSatellites";
            s = print_data(&data);
        },
            0x0009 => {
            tagname = "GPSStatus";
            s = print_data(&data);
        },
            0x000a => {
            tagname = "GPSMeasureMode";
            s = print_data(&data);
        },
            0x000b => {
            tagname = "GPSDOP";
            s = print_data(&data);
        },
            0x000c => {
            tagname = "GPSSpeedRef";
            s = print_data(&data);
        },               
            0x000d => {
            tagname = "GPSSpeed";
            s = print_data(&data);
        },
            0x000e => {
            tagname = "GPSTrackRef";
            s = print_data(&data);
        },
            0x000f => {
            tagname = "GPSTrack";
            s = print_data(&data);
        },
            0x0010 => {
            tagname = "GPSImgDirectionRef";
            s = print_data(&data);
        },
            0x0011 => {
            tagname = "GPSImgDirection";
            s = print_data(&data);
        },
            0x0012 => {
            tagname = "GPSMapDatum";
            s = print_data(&data);
        },
            0x0013 => {
            tagname = "GPSDestLatitudeRef";
            s = print_data(&data);
        },
            0x0014 => {
            tagname = "GPSDestLatitude";
            s = print_data(&data);
        },
            0x0015 => {
            tagname = "GPSDestLongitudeRef";
            s = print_data(&data);
        },
            0x0016 => {
            tagname = "GPSDestLongitude";
            s = print_data(&data);
        },
            0x0017 => {
            tagname = "GPSDestBearingRef";
            s = print_data(&data);
        },
            0x0018 => {
            tagname = "GPSDestBearing";
            s = print_data(&data);
        },
            0x0019 => {
            tagname = "GPSDestDistanceRef";
            s = print_data(&data);
        },
            0x001a => {
            tagname = "GPSDestDistance";
            s = print_data(&data);
        },
            0x001b => {
            tagname = "GPSProcessingMethod";
            s = print_data(&data);
        },
            0x001c => {
            tagname = "GPSAreaInformation";
            s = print_data(&data);
        },
            0x001d => {
            tagname = "GPSDateStamp";
            s = print_data(&data);
        },
            0x001e => {
            tagname = "GPSDifferential";
            s = print_data(&data);
        },
            0x001f => {
            tagname = "GPSHPositioningError";
            s = print_data(&data);
        },
        _=> {
             tagname = "UnKnown";
             s = print_data(&data);
        },
    }  
    (tagname.to_string(),s)
}
pub fn tag_mapper(tag :u16, data: &DataPack) -> (String,String) {
    let tagname;
    let s;
    match tag {
        0x0001 => {
            tagname = "InteropIndex";
            s = print_data(&data);
     },
        0x0002 => {
            tagname = "InteropVersion";
            s = print_data(&data);
     },
        0x000b => {
            tagname = "ProcessingSoftware";
            s = print_data(&data);
     
     },
        0x00fe => {
            tagname = "SubfileType";
            s = print_data(&data);
     
     },
        0x00ff => {
            tagname = "OldSubfileType";
            s = print_data(&data);
     
     },
        0x0100 => {
            tagname = "ImageWidth";
            s = print_data(&data);
     
     },
        0x0101 => {
            tagname = "ImageHeight";
            s = print_data(&data);
     
     },
        0x0102 => {
            tagname = "BitsPerSample";
            s = print_data(&data);
     
     },
        0x0103 => {
            tagname = "Compression";
            s = print_data(&data);
     
     },
        0x0106 => {
            tagname = "PhotometricInterpretation";
            s = print_data(&data);
     
     },
        0x0107 => {
            tagname = "Thresholding";
            s = print_data(&data);
     
     },
        0x0108 => {
            tagname = "CellWidth";
            s = print_data(&data);
     
     },
        0x0109 => {
            tagname = "CellLength";
            s = print_data(&data);
     
     },
        0x010a => {
            tagname = "FillOrder";
            s = print_data(&data);
     
     },
        0x010d => {
            tagname = "DocumentName";
            s = print_data(&data);
     
     },
        0x010e => {
            tagname = "ImageDescription";
            s = print_data(&data);
     
     },
        0x010f => {
            tagname = "Make";
            s = print_data(&data);
     
     },
        0x0110 => {
            tagname = "Model";
            s = print_data(&data);
     
     },
        0x0111 => {
            tagname = "StripOffsets";
            s = print_data(&data);
     
     },
        0x0112 => {
            tagname = "Orientation";
            s = print_data(&data);
     
     },
        0x0115 => {
            tagname = "SamplesPerPixel";
            s = print_data(&data);
     
     },
        0x0116 => {
            tagname = "RowsPerStrip";
            s = print_data(&data);
     
     },
        0x0117 => {
            tagname = "StripByteCounts";
            s = print_data(&data);
     
     },
        0x0118 => {
            tagname = "MinSampleValue";
            s = print_data(&data);
     
     },
        0x0119 => {
            tagname = "MaxSampleValue";
            s = print_data(&data);
     
     },
        0x011a => {
            tagname = "XResolution";
            s = print_data(&data);
     
     },
        0x011b => {
            tagname = "YResolution";
            s = print_data(&data);
     
     },
        0x011c => {
            tagname = "PlanarConfiguration";
            s = print_data(&data);
     
     },
        0x011d => {
            tagname = "PageName";
            s = print_data(&data);
     
     },
        0x011e => {
            tagname = "XPosition";
            s = print_data(&data);
     
     },
        0x011f => {
            tagname = "YPosition";
            s = print_data(&data);
     
     },
        0x0120 => {
            tagname = "FreeOffsets";
            s = print_data(&data);
     
     },
        0x0121 => {
            tagname = "FreeByteCounts";
            s = print_data(&data);
     
     },
        0x0122 => {
            tagname = "GrayResponseUnit";
            s = print_data(&data);
     
     },
        0x0123 => {
            tagname = "GrayResponseCurve";
            s = print_data(&data);
     
     },
        0x0124 => {
            tagname = "T4Options";
            s = print_data(&data);
     
     },
        0x0125 => {
            tagname = "T6Options";
            s = print_data(&data);
     
     },
        0x0128 => {
            tagname = "ResolutionUnit";
            s = print_data(&data);
     
     },
        0x0129 => {
            tagname = "PageNumber";
            s = print_data(&data);
     
     },
        0x012c => {
            tagname = "ColorResponseUnit";
            s = print_data(&data);
     
     },
        0x012d => {
            tagname = "TransferFunction";
            s = print_data(&data);
     
     },
        0x0131 => {
            tagname = "Software";
            s = print_data(&data);
     
     },
        0x0132 => {
            tagname = "ModifyDate";
            s = print_data(&data);
     
     },
        0x013b => {
            tagname = "Artist";
            s = print_data(&data);
     
     },
        0x013c => {
            tagname = "HostComputer";
            s = print_data(&data);
     
     },
        0x013d => {
            tagname = "Predictor";
            s = print_data(&data);
     
     },
        0x013e => {
            tagname = "WhitePoint";
            s = print_data(&data);
     
     },
        0x013f => {
            tagname = "PrimaryChromaticities";
            s = print_data(&data);
     
     },
        0x0140 => {
            tagname = "ColorMap";
            s = print_data(&data);
     
     },
        0x0141 => {
            tagname = "HalftoneHints";
            s = print_data(&data);
     
     },
        0x0142 => {
            tagname = "TileWidth";
            s = print_data(&data);
     
     },
        0x0143 => {
            tagname = "TileLength";
            s = print_data(&data);
     
     },
        0x0144 => {
            tagname = "TileOffsets";
            s = print_data(&data);
     
     },
        0x0145 => {
            tagname = "TileByteCounts";
            s = print_data(&data);
     
     },
        0x0146 => {
            tagname = "BadFaxLines";
            s = print_data(&data);
     
     },
        0x0147 => {
            tagname = "CleanFaxData";
            s = print_data(&data);
     
     },
        0x0148 => {
            tagname = "ConsecutiveBadFaxLines";
            s = print_data(&data);
     
     },
        0x014a => {
            tagname = "SubIFD";
            s = print_data(&data);
     
     },
        0x014c => {
            tagname = "InkSet";
            s = print_data(&data);
     
     },
        0x014d => {
            tagname = "InkNames";
            s = print_data(&data);
     
     },
        0x014e => {
            tagname = "NumberofInks";
            s = print_data(&data);
     
     },
        0x0150 => {
            tagname = "DotRange";
            s = print_data(&data);
     
     },
        0x0151 => {
            tagname = "TargetPrinter";
            s = print_data(&data);
     
     },
        0x0152 => {
            tagname = "ExtraSamples";
            s = print_data(&data);
     
     },
        0x0153 => {
            tagname = "SampleFormat";
            s = print_data(&data);
     
     },
        0x0154 => {
            tagname = "SMinSampleValue";
            s = print_data(&data);
     
     },
        0x0155 => {
            tagname = "SMaxSampleValue";
            s = print_data(&data);
     
     },
        0x0156 => {
            tagname = "TransferRange";
            s = print_data(&data);
     
     },
        0x0157 => {
            tagname = "ClipPath";
            s = print_data(&data);
     
     },
        0x0158 => {
            tagname = "XClipPathUnits";
            s = print_data(&data);
     
     },
        0x0159 => {
            tagname = "YClipPathUnits";
            s = print_data(&data);
     
     },
        0x015a => {
            tagname = "Indexed";
            s = print_data(&data);
     
     },
        0x015b => {
            tagname = "JPEGTables";
            s = print_data(&data);
     
     },
        0x015f => {
            tagname = "OPIProxy";
            s = print_data(&data);
     
     },
        0x0190 => {
            tagname = "GlobalParametersIFD";
            s = print_data(&data);
     
     },
        0x0191 => {
            tagname = "ProfileType";
            s = print_data(&data);
     
     },
        0x0192 => {
            tagname = "FaxProfile";
            s = print_data(&data);
     
     },
        0x0193 => {
            tagname = "CodingMethods";
            s = print_data(&data);
     
     },
        0x0194 => {
            tagname = "VersionYear";
            s = print_data(&data);
     
     },
        0x0195 => {
            tagname = "ModeNumber";
            s = print_data(&data);
     
     },
        0x01b1 => {
            tagname = "Decode";
            s = print_data(&data);
     
     },
        0x01b2 => {
            tagname = "DefaultImageColor";
            s = print_data(&data);
     
     },
        0x01b3 => {
            tagname = "T82Options";
            s = print_data(&data);
     
     },
        0x01b5 => {
            tagname = "JPEGTables";
            s = print_data(&data);
     
     },
        0x0200 => {
            tagname = "JPEGProc";
            s = print_data(&data);
     
     },
        0x0201 => {
            tagname = "ThumbnailOffset";
            s = print_data(&data);
     
     },
        0x0202 => {
            tagname = "ThumbnailLength";
            s = print_data(&data);
     
     },
        0x0203 => {
            tagname = "JPEGRestartInterval";
            s = print_data(&data);
     
     },
        0x0205 => {
            tagname = "JPEGLosslessPredictors";
            s = print_data(&data);
     
     },
        0x0206 => {
            tagname = "JPEGPointTransforms";
            s = print_data(&data);
     
     },
        0x0207 => {
            tagname = "JPEGQTables";
            s = print_data(&data);
     
     },
        0x0208 => {
            tagname = "JPEGDCTables";
            s = print_data(&data);
     
     },
        0x0209 => {
            tagname = "JPEGACTables";
            s = print_data(&data);
     
     },
        0x0211 => {
            tagname = "YCbCrCoefficients";
            s = print_data(&data);
     
     },
        0x0212 => {
            tagname = "YCbCrSubSampling";
            s = print_data(&data);
     
     },
        0x0213 => {
            tagname = "YCbCrPositioning";
            s = print_data(&data);
     
     },
        0x0214 => {
            tagname = "ReferenceBlackWhite";
            s = print_data(&data);
     
     },
        0x022f => {
            tagname = "StripRowCounts";
            s = print_data(&data);
     
     },
        0x02bc => {
            tagname = "ApplicationNotes";
            s = print_data(&data);
     
     },
        0x03e7 => {
            tagname = "USPTOMiscellaneous";
            s = print_data(&data);
     
     },
        0x1000 => {
            tagname = "RelatedImageFileFormat";
            s = print_data(&data);
     
     },
        0x1001 => {
            tagname = "RelatedImageWidth";
            s = print_data(&data);
     
     },
        0x1002 => {
            tagname = "RelatedImageHeight";
            s = print_data(&data);
     
     },
        0x4746 => {
            tagname = "Rating";
            s = print_data(&data);
     
     },
        0x4747 => {
            tagname = "XP_DIP_XML";
            s = print_data(&data);
     
     },
        0x4748 => {
            tagname = "StitchInfo";
            s = print_data(&data);
     
     },
        0x4749 => {
            tagname = "RatingPercent";
            s = print_data(&data);
     
     },
        0x7000 => {
            tagname = "SonyRawFileType";
            s = print_data(&data);
     
     },
        0x7010 => {
            tagname = "SonyToneCurve";
            s = print_data(&data);
     
     },
        0x7031 => {
            tagname = "VignettingCorrection";
            s = print_data(&data);
     
     },
        0x7032 => {
            tagname = "VignettingCorrParams";
            s = print_data(&data);
     
     },
        0x7034 => {
            tagname = "ChromaticAberrationCorrection";
            s = print_data(&data);
     
     },
        0x7035 => {
            tagname = "ChromaticAberrationCorrParams";
            s = print_data(&data);
     
     },
        0x7036 => {
            tagname = "DistortionCorrection";
            s = print_data(&data);
     
     },
        0x7037 => {
            tagname = "DistortionCorrParams";
            s = print_data(&data);
     
     },
        0x74c7 => {
            tagname = "SonyCropTopLeft";
            s = print_data(&data);
     
     },
        0x74c8 => {
            tagname = "SonyCropSize";
            s = print_data(&data);
     
     },
        0x800d => {
            tagname = "ImageID";
            s = print_data(&data);
     
     },
        0x80a3 => {
            tagname = "WangTag1";
            s = print_data(&data);
     
     },
        0x80a4 => {
            tagname = "WangAnnotation";
            s = print_data(&data);
     
     },
        0x80a5 => {
            tagname = "WangTag3";
            s = print_data(&data);
     
     },
        0x80a6 => {
            tagname = "WangTag4";
            s = print_data(&data);
     
     },
        0x80b9 => {
            tagname = "ImageReferencePoints";
            s = print_data(&data);
     
     },
        0x80ba => {
            tagname = "RegionXformTackPoint";
            s = print_data(&data);
     
     },
        0x80bb => {
            tagname = "WarpQuadrilateral";
            s = print_data(&data);
     
     },
        0x80bc => {
            tagname = "AffineTransformMat";
            s = print_data(&data);
     
     },
        0x80e3 => {
            tagname = "Matteing";
            s = print_data(&data);
     
     },
        0x80e4 => {
            tagname = "DataType";
            s = print_data(&data);
     
     },
        0x80e5 => {
            tagname = "ImageDepth";
            s = print_data(&data);
     
     },
        0x80e6 => {
            tagname = "TileDepth";
            s = print_data(&data);
     
     },
        0x8214 => {
            tagname = "ImageFullWidth";
            s = print_data(&data);
     
     },
        0x8215 => {
            tagname = "ImageFullHeight";
            s = print_data(&data);
     
     },
        0x8216 => {
            tagname = "TextureFormat";
            s = print_data(&data);
     
     },
        0x8217 => {
            tagname = "WrapModes";
            s = print_data(&data);
     
     },
        0x8218 => {
            tagname = "FovCot";
            s = print_data(&data);
     
     },
        0x8219 => {
            tagname = "MatrixWorldToScreen";
            s = print_data(&data);
     
     },
        0x821a => {
            tagname = "MatrixWorldToCamera";
            s = print_data(&data);
     
     },
        0x827d => {
            tagname = "Model2";
            s = print_data(&data);
     
     },
        0x828d => {
            tagname = "CFARepeatPatternDim";
            s = print_data(&data);
     
     },
        0x828e => {
            tagname = "CFAPattern2";
            s = print_data(&data);
     
     },
        0x828f => {
            tagname = "BatteryLevel";
            s = print_data(&data);
     
     },
        0x8290 => {
            tagname = "KodakIFD";
            s = print_data(&data);
     
     },
        0x8298 => {
            tagname = "Copyright";
            s = print_data(&data);
     
     },
        0x829a => {
            tagname = "ExposureTime";
            s = print_data(&data);
     
     },
        0x829d => {
            tagname = "FNumber";
            s = print_data(&data);
     
     },
        0x82a5 => {
            tagname = "MDFileTag";
            s = print_data(&data);
     
     },
        0x82a6 => {
            tagname = "MDScalePixel";
            s = print_data(&data);
     
     },
        0x82a7 => {
            tagname = "MDColorTable";
            s = print_data(&data);
     
     },
        0x82a8 => {
            tagname = "MDLabName";
            s = print_data(&data);
     
     },
        0x82a9 => {
            tagname = "MDSampleInfo";
            s = print_data(&data);
     
     },
        0x82aa => {
            tagname = "MDPrepDate";
            s = print_data(&data);
     
     },
        0x82ab => {
            tagname = "MDPrepTime";
            s = print_data(&data);
     
     },
        0x82ac => {
            tagname = "MDFileUnits";
            s = print_data(&data);
     
     },
        0x830e => {
            tagname = "PixelScale";
            s = print_data(&data);
     
     },
        0x8335 => {
            tagname = "AdventScale";
            s = print_data(&data);
     
     },
        0x8336 => {
            tagname = "AdventRevision";
            s = print_data(&data);
     
     },
        0x835c => {
            tagname = "UIC1Tag";
            s = print_data(&data);
     
     },
        0x835d => {
            tagname = "UIC2Tag";
            s = print_data(&data);
     
     },
        0x835e => {
            tagname = "UIC3Tag";
            s = print_data(&data);
     
     },
        0x835f => {
            tagname = "UIC4Tag";
            s = print_data(&data);
     
     },
        0x83bb => {
            tagname = "IPTC-NAA";
            s = print_data(&data);
     
     },
        0x847e => {
            tagname = "IntergraphPacketData";
            s = print_data(&data);
     
     },
        0x847f => {
            tagname = "IntergraphFlagRegisters";
            s = print_data(&data);
     
     },
        0x8480 => {
            tagname = "IntergraphMatrix";
            s = print_data(&data);
     
     },
        0x8481 => {
            tagname = "INGRReserved";
            s = print_data(&data);
     
     },
        0x8482 => {
            tagname = "ModelTiePoint";
            s = print_data(&data);
     
     },
        0x84e0 => {
            tagname = "Site";
            s = print_data(&data);
     
     },
        0x84e1 => {
            tagname = "ColorSequence";
            s = print_data(&data);
     
     },
        0x84e2 => {
            tagname = "IT8Header";
            s = print_data(&data);
     
     },
        0x84e3 => {
            tagname = "RasterPadding";
            s = print_data(&data);
     
     },
        0x84e4 => {
            tagname = "BitsPerRunLength";
            s = print_data(&data);
     
     },
        0x84e5 => {
            tagname = "BitsPerExtendedRunLength";
            s = print_data(&data);
     
     },
        0x84e6 => {
            tagname = "ColorTable";
            s = print_data(&data);
     
     },
        0x84e7 => {
            tagname = "ImageColorIndicator";
            s = print_data(&data);
     
     },
        0x84e8 => {
            tagname = "BackgroundColorIndicator";
            s = print_data(&data);
     
     },
        0x84e9 => {
            tagname = "ImageColorValue";
            s = print_data(&data);
     
     },
        0x84ea => {
            tagname = "BackgroundColorValue";
            s = print_data(&data);
     
     },
        0x84eb => {
            tagname = "PixelIntensityRange";
            s = print_data(&data);
     
     },
        0x84ec => {
            tagname = "TransparencyIndicator";
            s = print_data(&data);
     
     },
        0x84ed => {
            tagname = "ColorCharacterization";
            s = print_data(&data);
     
     },
        0x84ee => {
            tagname = "HCUsage";
            s = print_data(&data);
     
     },
        0x84ef => {
            tagname = "TrapIndicator";
            s = print_data(&data);
     
     },
        0x84f0 => {
            tagname = "CMYKEquivalent";
            s = print_data(&data);
     
     },
        0x8546 => {
            tagname = "SEMInfo";
            s = print_data(&data);
     
     },
        0x8568 => {
            tagname = "AFCP_IPTC";
            s = print_data(&data);
     
     },
        0x85b8 => {
            tagname = "PixelMagicJBIGOptions";
            s = print_data(&data);
     
     },
        0x85d7 => {
            tagname = "JPLCartoIFD";
            s = print_data(&data);
     
     },
        0x85d8 => {
            tagname = "ModelTransform";
            s = print_data(&data);
     
     },
        0x8602 => {
            tagname = "WB_GRGBLevels";
            s = print_data(&data);
     
     },
        0x8606 => {
            tagname = "LeafData";
            s = print_data(&data);
     
     },
        0x8649 => {
            tagname = "PhotoshopSettings";
            s = print_data(&data);
     
     },
        0x8769 => {
            tagname = "ExifOffset";
            s = print_data(&data);
     
     },
        0x8773 => {
            tagname = "ICC_Profile";
            s = print_data(&data);
     
     },
        0x877f => {
            tagname = "TIFF_FXExtensions";
            s = print_data(&data);
     
     },
        0x8780 => {
            tagname = "MultiProfiles";
            s = print_data(&data);
     
     },
        0x8781 => {
            tagname = "SharedData";
            s = print_data(&data);
     
     },
        0x8782 => {
            tagname = "T88Options";
            s = print_data(&data);
     
     },
        0x87ac => {
            tagname = "ImageLayer";
            s = print_data(&data);
     
     },
        0x87af => {
            tagname = "GeoTiffDirectory";
            s = print_data(&data);
     
     },
        0x87b0 => {
            tagname = "GeoTiffDoubleParams";
            s = print_data(&data);
     
     },
        0x87b1 => {
            tagname = "GeoTiffAsciiParams";
            s = print_data(&data);
     
     },
        0x87be => {
            tagname = "JBIGOptions";
            s = print_data(&data);
     
     },
        0x8822 => {
            tagname = "ExposureProgram";
            s = print_data(&data);
     
     },
        0x8824 => {
            tagname = "SpectralSensitivity";
            s = print_data(&data);
     
     },
        0x8825 => {
            tagname = "GPSInfo";
            s = print_data(&data);
     
     },
        0x8827 => {
            tagname = "ISO";
            s = print_data(&data);
     
     },
        0x8828 => {
            tagname = "Opto-ElectricConvFactor";
            s = print_data(&data);
     
     },
        0x8829 => {
            tagname = "Interlace";
            s = print_data(&data);
     
     },
        0x882a => {
            tagname = "TimeZoneOffset";
            s = print_data(&data);
     
     },
        0x882b => {
            tagname = "SelfTimerMode";
            s = print_data(&data);
     
     },
        0x8830 => {
            tagname = "SensitivityType";
            s = print_data(&data);
     
     },
        0x8831 => {
            tagname = "StandardOutputSensitivity";
            s = print_data(&data);
     
     },
        0x8832 => {
            tagname = "RecommendedExposureIndex";
            s = print_data(&data);
     
     },
        0x8833 => {
            tagname = "ISOSpeed";
            s = print_data(&data);
     
     },
        0x8834 => {
            tagname = "ISOSpeedLatitudeyyy";
            s = print_data(&data);
     
     },
        0x8835 => {
            tagname = "ISOSpeedLatitudezzz";
            s = print_data(&data);
     
     },
        0x885c => {
            tagname = "FaxRecvParams";
            s = print_data(&data);
     
     },
        0x885d => {
            tagname = "FaxSubAddress";
            s = print_data(&data);
     
     },
        0x885e => {
            tagname = "FaxRecvTime";
            s = print_data(&data);
     
     },
        0x8871 => {
            tagname = "FedexEDR";
            s = print_data(&data);
     
     },
        0x888a => {
            tagname = "LeafSubIFD";
            s = print_data(&data);
     
     },
        0x9000 => {
            tagname = "ExifVersion";
            s = print_data(&data);
     
     },
        0x9003 => {
            tagname = "DateTimeOriginal";
            s = print_data(&data);
     
     },
        0x9004 => {
            tagname = "CreateDate";
            s = print_data(&data);
     
     },
        0x9009 => {
            tagname = "GooglePlusUploadCode";
            s = print_data(&data);
     
     },
        0x9010 => {
            tagname = "OffsetTime";
            s = print_data(&data);
     
     },
        0x9011 => {
            tagname = "OffsetTimeOriginal";
            s = print_data(&data);
     
     },
        0x9012 => {
            tagname = "OffsetTimeDigitized";
            s = print_data(&data);
     
     },
        0x9101 => {
            tagname = "ComponentsConfiguration";
            s = print_data(&data);
     
     },
        0x9102 => {
            tagname = "CompressedBitsPerPixel";
            s = print_data(&data);
     
     },
        0x9201 => {
            tagname = "ShutterSpeedValue";
            s = print_data(&data);
     
     },
        0x9202 => {
            tagname = "ApertureValue";
            s = print_data(&data);
     
     },
        0x9203 => {
            tagname = "BrightnessValue";
            s = print_data(&data);
     
     },
        0x9204 => {
            tagname = "ExposureCompensation";
            s = print_data(&data);
     
     },
        0x9205 => {
            tagname = "MaxApertureValue";
            s = print_data(&data);
     
     },
        0x9206 => {
            tagname = "SubjectDistance";
            s = print_data(&data);
     
     },
        0x9207 => {
            tagname = "MeteringMode";
            s = print_data(&data);
     
     },
        0x9208 => {
            tagname = "LightSource";
            s = print_data(&data);
     
     },
        0x9209 => {
            tagname = "Flash";
            s = print_data(&data);
     
     },
        0x920a => {
            tagname = "FocalLength";
            s = print_data(&data);
     
     },
        0x920b => {
            tagname = "FlashEnergy";
            s = print_data(&data);
     
     },
        0x920c => {
            tagname = "SpatialFrequencyResponse";
            s = print_data(&data);
     
     },
        0x920d => {
            tagname = "Noise";
            s = print_data(&data);
     
     },
        0x920e => {
            tagname = "FocalPlaneXResolution";
            s = print_data(&data);
     
     },
        0x920f => {
            tagname = "FocalPlaneYResolution";
            s = print_data(&data);
     
     },
        0x9210 => {
            tagname = "FocalPlaneResolutionUnit";
            s = print_data(&data);
     
     },
        0x9211 => {
            tagname = "ImageNumber";
            s = print_data(&data);
     
     },
        0x9212 => {
            tagname = "SecurityClassification";
            s = print_data(&data);
     
     },
        0x9213 => {
            tagname = "ImageHistory";
            s = print_data(&data);
     
     },
        0x9214 => {
            tagname = "SubjectArea";
            s = print_data(&data);
     
     },
        0x9215 => {
            tagname = "ExposureIndex";
            s = print_data(&data);
     
     },
        0x9216 => {
            tagname = "TIFF-EPStandardID";
            s = print_data(&data);
     
     },
        0x9217 => {
            tagname = "SensingMethod";
            s = print_data(&data);
     
     },
        0x923a => {
            tagname = "CIP3DataFile";
            s = print_data(&data);
     
     },
        0x923b => {
            tagname = "CIP3Sheet";
            s = print_data(&data);
     
     },
        0x923c => {
            tagname = "CIP3Side";
            s = print_data(&data);
     
     },
        0x923f => {
            tagname = "StoNits";
            s = print_data(&data);
     
     },
        0x927c => {
            tagname = "MakerNoteApple";
        match data {
            DataPack::Undef(d) => {
                s = read_string(d, 0, d.len());
            },
            _ => {
                s = "".to_string();
            }
        }
     },
        0x9286 => {
            tagname = "UserComment";
            s = print_data(&data);
     },
        0x9290 => {
            tagname = "SubSecTime";
            s = print_data(&data);
     
     },
        0x9291 => {
            tagname = "SubSecTimeOriginal";
            s = print_data(&data);
     
     },
        0x9292 => {
            tagname = "SubSecTimeDigitized";
            s = print_data(&data);
     
     },
        0x932f => {
            tagname = "MSDocumentText";
            s = print_data(&data);
     
     },
        0x9330 => {
            tagname = "MSPropertySetStorage";
            s = print_data(&data);
     
     },
        0x9331 => {
            tagname = "MSDocumentTextPosition";
            s = print_data(&data);
     
     },
        0x935c => {
            tagname = "ImageSourceData";
            s = print_data(&data);
     
     },
        0x9400 => {
            tagname = "AmbientTemperature";
            s = print_data(&data);
     
     },
        0x9401 => {
            tagname = "Humidity";
            s = print_data(&data);
     
     },
        0x9402 => {
            tagname = "Pressure";
            s = print_data(&data);
     
     },
        0x9403 => {
            tagname = "WaterDepth";
            s = print_data(&data);
     
     },
        0x9404 => {
            tagname = "Acceleration";
            s = print_data(&data);
     
     },
        0x9405 => {
            tagname = "CameraElevationAngle";
            s = print_data(&data);
     
     },
        0x9c9b => {
            tagname = "XPTitle";
            s = print_data(&data);
     
     },
        0x9c9c => {
            tagname = "XPComment";
            s = print_data(&data);
     
     },
        0x9c9d => {
            tagname = "XPAuthor";
            s = print_data(&data);
     
     },
        0x9c9e => {
            tagname = "XPKeywords";
            s = print_data(&data);
     
     },
        0x9c9f => {
            tagname = "XPSubject";
            s = print_data(&data);
     
     },
        0xa000 => {
            tagname = "FlashpixVersion";
            s = print_data(&data);
     
     },
        0xa001 => {
            tagname = "ColorSpace";
            s = print_data(&data);
     
     },
        0xa002 => {
            tagname = "ExifImageWidth";
            s = print_data(&data);
     
     },
        0xa003 => {
            tagname = "ExifImageHeight";
            s = print_data(&data);
     
     },
        0xa004 => {
            tagname = "RelatedSoundFile";
            s = print_data(&data);
     
     },
        0xa005 => {
            tagname = "InteropOffset";
            s = print_data(&data);
     
     },
        0xa010 => {
            tagname = "SamsungRawPointersOffset";
            s = print_data(&data);
     
     },
        0xa011 => {
            tagname = "SamsungRawPointersLength";
            s = print_data(&data);
     
     },
        0xa101 => {
            tagname = "SamsungRawByteOrder";
            s = print_data(&data);
     
     },
        0xa102 => {
            tagname = "SamsungRawUnknown?";
            s = print_data(&data);
     
     },
        0xa20b => {
            tagname = "FlashEnergy";
            s = print_data(&data);
     
     },
        0xa20c => {
            tagname = "SpatialFrequencyResponse";
            s = print_data(&data);
     
     },
        0xa20d => {
            tagname = "Noise";
            s = print_data(&data);
     
     },
        0xa20e => {
            tagname = "FocalPlaneXResolution";
            s = print_data(&data);
     
     },
        0xa20f => {
            tagname = "FocalPlaneYResolution";
            s = print_data(&data);
     
     },
        0xa210 => {
            tagname = "FocalPlaneResolutionUnit";
            s = print_data(&data);
     
     },
        0xa211 => {
            tagname = "ImageNumber";
            s = print_data(&data);
     
     },
        0xa212 => {
            tagname = "SecurityClassification";
            s = print_data(&data);
     
     },
        0xa213 => {
            tagname = "ImageHistory";
            s = print_data(&data);
     
     },
        0xa214 => {
            tagname = "SubjectLocation";
            s = print_data(&data);
     
     },
        0xa215 => {
            tagname = "ExposureIndex";
            s = print_data(&data);
     
     },
        0xa216 => {
            tagname = "TIFF-EPStandardID";
            s = print_data(&data);
     
     },
        0xa217 => {
            tagname = "SensingMethod";
            s = print_data(&data);
     
     },
        0xa300 => {
            tagname = "FileSource";
            s = print_data(&data);
     
     },
        0xa301 => {
            tagname = "SceneType";
            s = print_data(&data);
     
     },
        0xa302 => {
            tagname = "CFAPattern";
            s = print_data(&data);
     
     },
        0xa401 => {
            tagname = "CustomRendered";
            s = print_data(&data);
     
     },
        0xa402 => {
            tagname = "ExposureMode";
            s = print_data(&data);
     
     },
        0xa403 => {
            tagname = "WhiteBalance";
            s = print_data(&data);
     
     },
        0xa404 => {
            tagname = "DigitalZoomRatio";
            s = print_data(&data);
     
     },
        0xa405 => {
            tagname = "FocalLengthIn35mmFormat";
            s = print_data(&data);
     
     },
        0xa406 => {
            tagname = "SceneCaptureType";
            s = print_data(&data);
     
     },
        0xa407 => {
            tagname = "GainControl";
            s = print_data(&data);
     
     },
        0xa408 => {
            tagname = "Contrast";
            s = print_data(&data);
     
     },
        0xa409 => {
            tagname = "Saturation";
            s = print_data(&data);
     
     },
        0xa40a => {
            tagname = "Sharpness";
            s = print_data(&data);
     
     },
        0xa40b => {
            tagname = "DeviceSettingDescription";
            s = print_data(&data);
     
     },
        0xa40c => {
            tagname = "SubjectDistanceRange";
            s = print_data(&data);
     
     },
        0xa420 => {
            tagname = "ImageUniqueID";
            s = print_data(&data);
     
     },
        0xa430 => {
            tagname = "OwnerName";
            s = print_data(&data);
     
     },
        0xa431 => {
            tagname = "SerialNumber";
            s = print_data(&data);
     
     },
        0xa432 => {
            tagname = "LensInfo";
            s = print_data(&data);
     
     },
        0xa433 => {
            tagname = "LensMake";
            s = print_data(&data);
     
     },
        0xa434 => {
            tagname = "LensModel";
            s = print_data(&data);
     
     },
        0xa435 => {
            tagname = "LensSerialNumber";
            s = print_data(&data);
     
     },
        0xa460 => {
            tagname = "CompositeImage";
            s = print_data(&data);
     
     },
        0xa461 => {
            tagname = "CompositeImageCount";
            s = print_data(&data);
     
     },
        0xa462 => {
            tagname = "CompositeImageExposureTimes";
            s = print_data(&data);
     
     },
        0xa480 => {
            tagname = "GDALMetadata";
            s = print_data(&data);
     
     },
        0xa481 => {
            tagname = "GDALNoData";
            s = print_data(&data);
     
     },
        0xa500 => {
            tagname = "Gamma";
            s = print_data(&data);
     
     },
        0xafc0 => {
            tagname = "ExpandSoftware";
            s = print_data(&data);
     
     },
        0xafc1 => {
            tagname = "ExpandLens";
            s = print_data(&data);
     
     },
        0xafc2 => {
            tagname = "ExpandFilm";
            s = print_data(&data);
     
     },
        0xafc3 => {
            tagname = "ExpandFilterLens";
            s = print_data(&data);
     
     },
        0xafc4 => {
            tagname = "ExpandScanner";
            s = print_data(&data);
     
     },
        0xafc5 => {
            tagname = "ExpandFlashLamp";
            s = print_data(&data);
     
     },
        0xb4c3 => {
            tagname = "HasselbladRawImage";
            s = print_data(&data);
     
     },
        0xbc01 => {
            tagname = "PixelFormat";
            s = print_data(&data);
     
     },
        0xbc02 => {
            tagname = "Transformation";
            s = print_data(&data);
     
     },
        0xbc03 => {
            tagname = "Uncompressed";
            s = print_data(&data);
     
     },
        0xbc04 => {
            tagname = "ImageType";
            s = print_data(&data);
     
     },
        0xbc80 => {
            tagname = "ImageWidth";
            s = print_data(&data);
     
     },
        0xbc81 => {
            tagname = "ImageHeight";
            s = print_data(&data);
     
     },
        0xbc82 => {
            tagname = "WidthResolution";
            s = print_data(&data);
     
     },
        0xbc83 => {
            tagname = "HeightResolution";
            s = print_data(&data);
     
     },
        0xbcc0 => {
            tagname = "ImageOffset";
            s = print_data(&data);
     
     },
        0xbcc1 => {
            tagname = "ImageByteCount";
            s = print_data(&data);
     
     },
        0xbcc2 => {
            tagname = "AlphaOffset";
            s = print_data(&data);
     
     },
        0xbcc3 => {
            tagname = "AlphaByteCount";
            s = print_data(&data);
     
     },
        0xbcc4 => {
            tagname = "ImageDataDiscard";
            s = print_data(&data);
     
     },
        0xbcc5 => {
            tagname = "AlphaDataDiscard";
            s = print_data(&data);
     
     },
        0xc427 => {
            tagname = "OceScanjobDesc";
            s = print_data(&data);
     
     },
        0xc428 => {
            tagname = "OceApplicationSelector";
            s = print_data(&data);
     
     },
        0xc429 => {
            tagname = "OceIDNumber";
            s = print_data(&data);
     
     },
        0xc42a => {
            tagname = "OceImageLogic";
            s = print_data(&data);
     
     },
        0xc44f => {
            tagname = "Annotations";
            s = print_data(&data);
     
     },
        0xc4a5 => {
            tagname = "PrintIM";
            s = print_data(&data);
     
     },
        0xc51b => {
            tagname = "HasselbladExif";
            s = print_data(&data);
     
     },
        0xc573 => {
            tagname = "OriginalFileName";
            s = print_data(&data);
     
     },
        0xc580 => {
            tagname = "USPTOOriginalContentType";
            s = print_data(&data);
     
     },
        0xc5e0 => {
            tagname = "CR2CFAPattern";
            s = print_data(&data);
     
     },
        0xc612 => {
            tagname = "DNGVersion";
            s = print_data(&data);
     
     },
        0xc613 => {
            tagname = "DNGBackwardVersion";
            s = print_data(&data);
     
     },
        0xc614 => {
            tagname = "UniqueCameraModel";
            s = print_data(&data);
     
     },
        0xc615 => {
            tagname = "LocalizedCameraModel";
            s = print_data(&data);
     
     },
        0xc616 => {
            tagname = "CFAPlaneColor";
            s = print_data(&data);
     
     },
        0xc617 => {
            tagname = "CFALayout";
            s = print_data(&data);
     
     },
        0xc618 => {
            tagname = "LinearizationTable";
            s = print_data(&data);
     
     },
        0xc619 => {
            tagname = "BlackLevelRepeatDim";
            s = print_data(&data);
     
     },
        0xc61a => {
            tagname = "BlackLevel";
            s = print_data(&data);
     
     },
        0xc61b => {
            tagname = "BlackLevelDeltaH";
            s = print_data(&data);
     
     },
        0xc61c => {
            tagname = "BlackLevelDeltaV";
            s = print_data(&data);
     
     },
        0xc61d => {
            tagname = "WhiteLevel";
            s = print_data(&data);
     
     },
        0xc61e => {
            tagname = "DefaultScale";
            s = print_data(&data);
     
     },
        0xc61f => {
            tagname = "DefaultCropOrigin";
            s = print_data(&data);
     
     },
        0xc620 => {
            tagname = "DefaultCropSize";
            s = print_data(&data);
     
     },
        0xc621 => {
            tagname = "ColorMatrix1";
            s = print_data(&data);
     
     },
        0xc622 => {
            tagname = "ColorMatrix2";
            s = print_data(&data);
     
     },
        0xc623 => {
            tagname = "CameraCalibration1";
            s = print_data(&data);
     
     },
        0xc624 => {
            tagname = "CameraCalibration2";
            s = print_data(&data);
     
     },
        0xc625 => {
            tagname = "ReductionMatrix1";
            s = print_data(&data);
     
     },
        0xc626 => {
            tagname = "ReductionMatrix2";
            s = print_data(&data);
     
     },
        0xc627 => {
            tagname = "AnalogBalance";
            s = print_data(&data);
     
     },
        0xc628 => {
            tagname = "AsShotNeutral";
            s = print_data(&data);
     
     },
        0xc629 => {
            tagname = "AsShotWhiteXY";
            s = print_data(&data);
     
     },
        0xc62a => {
            tagname = "BaselineExposure";
            s = print_data(&data);
     
     },
        0xc62b => {
            tagname = "BaselineNoise";
            s = print_data(&data);
     
     },
        0xc62c => {
            tagname = "BaselineSharpness";
            s = print_data(&data);
     
     },
        0xc62d => {
            tagname = "BayerGreenSplit";
            s = print_data(&data);
     
     },
        0xc62e => {
            tagname = "LinearResponseLimit";
            s = print_data(&data);
     
     },
        0xc62f => {
            tagname = "CameraSerialNumber";
            s = print_data(&data);
     
     },
        0xc630 => {
            tagname = "DNGLensInfo";
            s = print_data(&data);
     
     },
        0xc631 => {
            tagname = "ChromaBlurRadius";
            s = print_data(&data);
     
     },
        0xc632 => {
            tagname = "AntiAliasStrength";
            s = print_data(&data);
     
     },
        0xc633 => {
            tagname = "ShadowScale";
            s = print_data(&data);
     
     },
        0xc634 => {
            tagname = "SR2Private";
            s = print_data(&data);
     
     },
        0xc635 => {
            tagname = "MakerNoteSafety";
            s = print_data(&data);
     
     },
        0xc640 => {
            tagname = "RawImageSegmentation";
            s = print_data(&data);
     
     },
        0xc65a => {
            tagname = "CalibrationIlluminant1";
            s = print_data(&data);
     
     },
        0xc65b => {
            tagname = "CalibrationIlluminant2";
            s = print_data(&data);
     
     },
        0xc65c => {
            tagname = "BestQualityScale";
            s = print_data(&data);
     
     },
        0xc65d => {
            tagname = "RawDataUniqueID";
            s = print_data(&data);
     
     },
        0xc660 => {
            tagname = "AliasLayerMetadata";
            s = print_data(&data);
     
     },
        0xc68b => {
            tagname = "OriginalRawFileName";
            s = print_data(&data);
     
     },
        0xc68c => {
            tagname = "OriginalRawFileData";
            s = print_data(&data);
     
     },
        0xc68d => {
            tagname = "ActiveArea";
            s = print_data(&data);
     
     },
        0xc68e => {
            tagname = "MaskedAreas";
            s = print_data(&data);
     
     },
        0xc68f => {
            tagname = "AsShotICCProfile";
            s = print_data(&data);
     
     },
        0xc690 => {
            tagname = "AsShotPreProfileMatrix";
            s = print_data(&data);
     
     },
        0xc691 => {
            tagname = "CurrentICCProfile";
            s = print_data(&data);
     
     },
        0xc692 => {
            tagname = "CurrentPreProfileMatrix";
            s = print_data(&data);
     
     },
        0xc6bf => {
            tagname = "ColorimetricReference";
            s = print_data(&data);
     
     },
        0xc6c5 => {
            tagname = "SRawType";
            s = print_data(&data);
     
     },
        0xc6d2 => {
            tagname = "PanasonicTitle";
            s = print_data(&data);
     
     },
        0xc6d3 => {
            tagname = "PanasonicTitle2";
            s = print_data(&data);
     
     },
        0xc6f3 => {
            tagname = "CameraCalibrationSig";
            s = print_data(&data);
     
     },
        0xc6f4 => {
            tagname = "ProfileCalibrationSig";
            s = print_data(&data);
     
     },
        0xc6f5 => {
            tagname = "ProfileIFD";
            s = print_data(&data);
     
     },
        0xc6f6 => {
            tagname = "AsShotProfileName";
            s = print_data(&data);
     
     },
        0xc6f7 => {
            tagname = "NoiseReductionApplied";
            s = print_data(&data);
     
     },
        0xc6f8 => {
            tagname = "ProfileName";
            s = print_data(&data);
     
     },
        0xc6f9 => {
            tagname = "ProfileHueSatMapDims";
            s = print_data(&data);
     
     },
        0xc6fa => {
            tagname = "ProfileHueSatMapData1";
            s = print_data(&data);
     
     },
        0xc6fb => {
            tagname = "ProfileHueSatMapData2";
            s = print_data(&data);
     
     },
        0xc6fc => {
            tagname = "ProfileToneCurve";
            s = print_data(&data);
     
     },
        0xc6fd => {
            tagname = "ProfileEmbedPolicy";
            s = print_data(&data);
     
     },
        0xc6fe => {
            tagname = "ProfileCopyright";
            s = print_data(&data);
     
     },
        0xc714 => {
            tagname = "ForwardMatrix1";
            s = print_data(&data);
     
     },
        0xc715 => {
            tagname = "ForwardMatrix2";
            s = print_data(&data);
     
     },
        0xc716 => {
            tagname = "PreviewApplicationName";
            s = print_data(&data);
     
     },
        0xc717 => {
            tagname = "PreviewApplicationVersion";
            s = print_data(&data);
     
     },
        0xc718 => {
            tagname = "PreviewSettingsName";
            s = print_data(&data);
     
     },
        0xc719 => {
            tagname = "PreviewSettingsDigest";
            s = print_data(&data);
     
     },
        0xc71a => {
            tagname = "PreviewColorSpace";
            s = print_data(&data);
     
     },
        0xc71b => {
            tagname = "PreviewDateTime";
            s = print_data(&data);
     
     },
        0xc71c => {
            tagname = "RawImageDigest";
            s = print_data(&data);
     
     },
        0xc71d => {
            tagname = "OriginalRawFileDigest";
            s = print_data(&data);
     
     },
        0xc71e => {
            tagname = "SubTileBlockSize";
            s = print_data(&data);
     
     },
        0xc71f => {
            tagname = "RowInterleaveFactor";
            s = print_data(&data);
     
     },
        0xc725 => {
            tagname = "ProfileLookTableDims";
            s = print_data(&data);
     
     },
        0xc726 => {
            tagname = "ProfileLookTableData";
            s = print_data(&data);
     
     },
        0xc740 => {
            tagname = "OpcodeList1";
            s = print_data(&data);
     
     },
        0xc741 => {
            tagname = "OpcodeList2";
            s = print_data(&data);
     
     },
        0xc74e => {
            tagname = "OpcodeList3";
            s = print_data(&data);
     
     },
        0xc761 => {
            tagname = "NoiseProfile";
            s = print_data(&data);
     
     },
        0xc763 => {
            tagname = "TimeCodes";
            s = print_data(&data);
     
     },
        0xc764 => {
            tagname = "FrameRate";
            s = print_data(&data);
     
     },
        0xc772 => {
            tagname = "TStop";
            s = print_data(&data);
     
     },
        0xc789 => {
            tagname = "ReelName";
            s = print_data(&data);
     
     },
        0xc791 => {
            tagname = "OriginalDefaultFinalSize";
            s = print_data(&data);
     
     },
        0xc792 => {
            tagname = "OriginalBestQualitySize";
            s = print_data(&data);
     
     },
        0xc793 => {
            tagname = "OriginalDefaultCropSize";
            s = print_data(&data);
     
     },
        0xc7a1 => {
            tagname = "CameraLabel";
            s = print_data(&data);
     
     },
        0xc7a3 => {
            tagname = "ProfileHueSatMapEncoding";
            s = print_data(&data);
     
     },
        0xc7a4 => {
            tagname = "ProfileLookTableEncoding";
            s = print_data(&data);
     
     },
        0xc7a5 => {
            tagname = "BaselineExposureOffset";
            s = print_data(&data);
     
     },
        0xc7a6 => {
            tagname = "DefaultBlackRender";
            s = print_data(&data);
     
     },
        0xc7a7 => {
            tagname = "NewRawImageDigest";
            s = print_data(&data);
     
     },
        0xc7a8 => {
            tagname = "RawToPreviewGain";
            s = print_data(&data);
     
     },
        0xc7aa => {
            tagname = "CacheVersion";
            s = print_data(&data);
     
     },
        0xc7b5 => {
            tagname = "DefaultUserCrop";
            s = print_data(&data);
     
     },
        0xc7d5 => {
            tagname = "NikonNEFInfo";
            s = print_data(&data);
     
     },
        0xc7e9 => {
            tagname = "DepthFormat";
            s = print_data(&data);
     
     },
        0xc7ea => {
            tagname = "DepthNear";
            s = print_data(&data);
     
     },
        0xc7eb => {
            tagname = "DepthFar";
            s = print_data(&data);
     
     },
        0xc7ec => {
            tagname = "DepthUnits";
            s = print_data(&data);
     
     },
        0xc7ed => {
            tagname = "DepthMeasureType";
            s = print_data(&data);
     
     },
        0xc7ee => {
            tagname = "EnhanceParams";
            s = print_data(&data);
     
     },
        0xcd2d => {
            tagname = "ProfileGainTableMap";
            s = print_data(&data);
     
     },
        0xcd2e => {
            tagname = "SemanticName";
            s = print_data(&data);
     
     },
        0xcd30 => {
            tagname = "SemanticInstanceIFD";
            s = print_data(&data);
     
     },
        0xcd31 => {
            tagname = "CalibrationIlluminant3";
            s = print_data(&data);
     
     },
        0xcd32 => {
            tagname = "CameraCalibration3";
            s = print_data(&data);
     
     },
        0xcd33 => {
            tagname = "ColorMatrix3";
            s = print_data(&data);
     
     },
        0xcd34 => {
            tagname = "ForwardMatrix3";
            s = print_data(&data);
     
     },
        0xcd35 => {
            tagname = "IlluminantData1";
            s = print_data(&data);
     
     },
        0xcd36 => {
            tagname = "IlluminantData2";
            s = print_data(&data);
     
     },
        0xcd37 => {
            tagname = "IlluminantData3";
            s = print_data(&data);
     
     },
        0xcd38 => {
            tagname = "MaskSubArea";
            s = print_data(&data);
     
     },
        0xcd39 => {
            tagname = "ProfileHueSatMapData3";
            s = print_data(&data);
     
     },
        0xcd3a => {
            tagname = "ReductionMatrix3";
            s = print_data(&data);
     
     },
        0xcd3b => {
            tagname = "RGBTables";
            s = print_data(&data);
     
     },
        0xea1c => {
            tagname = "Padding";
            s = print_data(&data);
     
     },
        0xea1d => {
            tagname = "OffsetSchema";
            s = print_data(&data);
     
     },
        0xfde8 => {
            tagname = "OwnerName";
            s = print_data(&data);
     
     },
        0xfde9 => {
            tagname = "SerialNumber";
            s = print_data(&data);
     
     },
        0xfdea => {
            tagname = "Lens";
            s = print_data(&data);
     
     },
        0xfe00 => {
            tagname = "KDC_IFD";
            s = print_data(&data);
     
     },
        0xfe4c => {
            tagname = "RawFile";
            s = print_data(&data);
     
     },
        0xfe4d => {
            tagname = "Converter";
            s = print_data(&data);
     
     },
        0xfe4e => {
            tagname = "WhiteBalance";
            s = print_data(&data);
     
     },
        0xfe51 => {
            tagname = "Exposure";
            s = print_data(&data);
     
     },
        0xfe52 => {
            tagname = "Shadows";
            s = print_data(&data);
     
     },
        0xfe53 => {
            tagname = "Brightness";
            s = print_data(&data);
     
     },
        0xfe54 => {
            tagname = "Contrast";
            s = print_data(&data);
     
     },
        0xfe55 => {
            tagname = "Saturation";
            s = print_data(&data);
     
     },
        0xfe56 => {
            tagname = "Sharpness";
            s = print_data(&data);
     
     },
        0xfe57 => {
            tagname = "Smoothness";
            s = print_data(&data);
     
     },
        0xfe58 => {
            tagname = "MoireFilter";
            s = print_data(&data);
     
     },
    _ => {
            tagname = "Unknown";
            s = print_data(&data);       
    },
    }
    (tagname.to_string(), s)
}
