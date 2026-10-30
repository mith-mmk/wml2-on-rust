/* Independent, small progressive VarDCT inputs and RGBA16 oracle. */
#include <jxl/encode.h>
#include <jxl/decode.h>
#include <jxl/color_encoding.h>
#include <jxl/version.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define CK(x) do { if (!(x)) {fprintf(stderr,"failed: %s\n",#x);return 1;} } while(0)
int main(int argc,char **argv){
 CK(argc==4); CK(JxlDecoderVersion()==11002 && JPEGXL_MAJOR_VERSION*1000000+JPEGXL_MINOR_VERSION*1000+JPEGXL_PATCH_VERSION==11002);
 if(!strcmp(argv[1],"encode")){
  JxlEncoder *e=JxlEncoderCreate(NULL); CK(e); JxlBasicInfo b; JxlEncoderInitBasicInfo(&b);b.xsize=32;b.ysize=24;b.bits_per_sample=8;b.num_color_channels=3;b.uses_original_profile=JXL_FALSE;CK(JxlEncoderSetBasicInfo(e,&b)==JXL_ENC_SUCCESS);
  JxlColorEncoding color;JxlColorEncodingSetToSRGB(&color,JXL_FALSE);CK(JxlEncoderSetColorEncoding(e,&color)==JXL_ENC_SUCCESS);JxlEncoderFrameSettings *s=JxlEncoderFrameSettingsCreate(e,NULL);CK(s);CK(JxlEncoderSetFrameDistance(s,1.0)==JXL_ENC_SUCCESS);CK(JxlEncoderFrameSettingsSetOption(s,JXL_ENC_FRAME_SETTING_EFFORT,3)==JXL_ENC_SUCCESS);
  CK(JxlEncoderFrameSettingsSetOption(s,JXL_ENC_FRAME_SETTING_PROGRESSIVE_AC,1)==JXL_ENC_SUCCESS);CK(JxlEncoderFrameSettingsSetOption(s,JXL_ENC_FRAME_SETTING_QPROGRESSIVE_AC,1)==JXL_ENC_SUCCESS);CK(JxlEncoderFrameSettingsSetOption(s,JXL_ENC_FRAME_SETTING_PROGRESSIVE_DC,atoi(argv[3]))==JXL_ENC_SUCCESS);
  uint8_t pixels[32*24*3];for(unsigned y=0;y<24;y++)for(unsigned x=0;x<32;x++)for(unsigned c=0;c<3;c++)pixels[(y*32+x)*3+c]=(x*(17+c*11)+y*(31-c*5)+(x*y%13)*19)%256;
  JxlPixelFormat f={3,JXL_TYPE_UINT8,JXL_NATIVE_ENDIAN,0};CK(JxlEncoderAddImageFrame(s,&f,pixels,sizeof(pixels))==JXL_ENC_SUCCESS);JxlEncoderCloseInput(e);uint8_t output[65536],*next=output;size_t left=sizeof(output);CK(JxlEncoderProcessOutput(e,&next,&left)==JXL_ENC_SUCCESS);FILE *file=fopen(argv[2],"wb");CK(file);CK(fwrite(output,1,sizeof(output)-left,file)==sizeof(output)-left);fclose(file);JxlEncoderDestroy(e);return 0;
 }
 CK(!strcmp(argv[1],"decode"));FILE *file=fopen(argv[2],"rb");CK(file);uint8_t input[65536];size_t n=fread(input,1,sizeof(input),file);CK(n && !ferror(file) && feof(file));fclose(file);
 JxlDecoder *d=JxlDecoderCreate(NULL);CK(d);CK(JxlDecoderSubscribeEvents(d,JXL_DEC_BASIC_INFO|JXL_DEC_COLOR_ENCODING|JXL_DEC_FULL_IMAGE)==JXL_DEC_SUCCESS);CK(JxlDecoderSetInput(d,input,n)==JXL_DEC_SUCCESS);JxlDecoderCloseInput(d);uint16_t pixels[32*24*4];JxlPixelFormat f={4,JXL_TYPE_UINT16,JXL_LITTLE_ENDIAN,0};int full=0;
 for(;;){JxlDecoderStatus s=JxlDecoderProcessInput(d);if(s==JXL_DEC_BASIC_INFO){JxlBasicInfo b;CK(JxlDecoderGetBasicInfo(d,&b)==JXL_DEC_SUCCESS);CK(b.xsize==32&&b.ysize==24&&!b.have_animation&&!b.have_preview);}else if(s==JXL_DEC_COLOR_ENCODING){JxlColorEncoding color;JxlColorEncodingSetToSRGB(&color,JXL_FALSE);color.rendering_intent=JXL_RENDERING_INTENT_RELATIVE;CK(JxlDecoderSetPreferredColorProfile(d,&color)==JXL_DEC_SUCCESS);}else if(s==JXL_DEC_NEED_IMAGE_OUT_BUFFER){size_t bytes;CK(JxlDecoderImageOutBufferSize(d,&f,&bytes)==JXL_DEC_SUCCESS&&bytes==sizeof(pixels));CK(JxlDecoderSetImageOutBuffer(d,&f,pixels,sizeof(pixels))==JXL_DEC_SUCCESS);JxlBitDepth depth={JXL_BIT_DEPTH_FROM_PIXEL_FORMAT,0,0};CK(JxlDecoderSetImageOutBitDepth(d,&depth)==JXL_DEC_SUCCESS);}else if(s==JXL_DEC_FULL_IMAGE){CK(!full);full=1;}else if(s==JXL_DEC_SUCCESS)break;else{fprintf(stderr,"decode status %d\n",s);JxlDecoderDestroy(d);return 1;}}
 CK(full);file=fopen(argv[3],"wb");CK(file);CK(fwrite(pixels,1,sizeof(pixels),file)==sizeof(pixels));fclose(file);JxlDecoderDestroy(d);return 0;
}
