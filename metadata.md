# Metadata

|name|type|comment|
|-----------|---|--------------|
|Format|Ascii|image format|
|version|Ascii|image format version|
|compression|Ascii|compression method|
|comment|Ascii|comment of image|
|gamma|float|gamma value|
|sRGB|float|sRGB value|
|bits per pixel|int|bits per pixel of image|
|width|int|image width pixels|
|height|int|image height pixels|
|EXIF|EXIF|Tiff header + IFD|
|ICC Profile name|Ascii|ICC Profile name|
|ICC Profile|ICCProfile|ICCProfile(raw data)|
|bmp:negative height|Ascii("ture")|Bmp ois if height < 0 top to bottom BMP|
|gif:animation|Ascii|Animation GIF metadata|

- compression  compression type of image
    - NONE - RAW IMAGE 
    - RLE - BMP RLE 
    - Bit fields - BMP Bit fields
    - Jpeg - BMP Jpeg
    - PNG - BMP PNG
    - LZW - GIF
    - etc.. - TIFF