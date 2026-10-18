/* Independent fixture encoder using libjxl, never WML2 or jxl-rs.
 * Build/run through generate_jpegxl_fixtures.py. */
#include <jxl/encode.h>
#include <jxl/color_encoding.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr,"failed: %s\n",#x); return 1; } } while(0)
int main(int argc,char **argv) {
  CHECK(argc==9);
  const char *mode=argv[7];
  unsigned w=atoi(argv[3]),h=atoi(argv[4]),channels=atoi(argv[5]),bits=atoi(argv[6]);
  size_t n=(size_t)w*h*channels*(bits==8?1:2);
  unsigned char *pixels=malloc(n);CHECK(pixels);
  FILE *f=fopen(argv[1],"rb");CHECK(f);CHECK(fread(pixels,1,n,f)==n);fclose(f);
  JxlEncoder *enc=JxlEncoderCreate(NULL);CHECK(enc);
  JxlBasicInfo info;JxlEncoderInitBasicInfo(&info);
  info.xsize=w;info.ysize=h;info.bits_per_sample=bits;
  if(!strcmp(mode,"float")) info.exponent_bits_per_sample=5;
  info.num_color_channels=channels<=2?1:3;
  info.uses_original_profile=strcmp(mode,"lossy")!=0;
  if(channels==2||channels==4) {info.num_extra_channels=1;info.alpha_bits=bits;info.alpha_premultiplied=!strcmp(mode,"premult");}
  if(!strcmp(mode,"animation")) {info.have_animation=JXL_TRUE;info.animation.tps_numerator=100;info.animation.tps_denominator=1;}
  if(!strcmp(mode,"mixed-alpha")) info.alpha_bits=8;
  if(!strcmp(mode,"orientation")) info.orientation=JXL_ORIENT_ROTATE_90_CW;
  CHECK(JxlEncoderSetBasicInfo(enc,&info)==JXL_ENC_SUCCESS);
  if(channels==2||channels==4) {
    JxlExtraChannelInfo alpha;JxlEncoderInitExtraChannelInfo(JXL_CHANNEL_ALPHA,&alpha);
    alpha.bits_per_sample=!strcmp(mode,"mixed-alpha")?8:bits;alpha.alpha_premultiplied=!strcmp(mode,"premult");
    CHECK(JxlEncoderSetExtraChannelInfo(enc,0,&alpha)==JXL_ENC_SUCCESS);
  }
  JxlColorEncoding color;JxlColorEncodingSetToSRGB(&color,channels<=2);
  if(!strcmp(mode,"hdr")) color.transfer_function=JXL_TRANSFER_FUNCTION_PQ;
  if(strcmp(argv[8],"-")==0) CHECK(JxlEncoderSetColorEncoding(enc,&color)==JXL_ENC_SUCCESS);
  else {
    f=fopen(argv[8],"rb");CHECK(f);fseek(f,0,SEEK_END);long len=ftell(f);rewind(f);unsigned char *icc=malloc(len);CHECK(icc);CHECK(fread(icc,1,len,f)==(size_t)len);fclose(f);
    CHECK(JxlEncoderSetICCProfile(enc,icc,len)==JXL_ENC_SUCCESS);free(icc);
  }
  JxlEncoderFrameSettings *settings=JxlEncoderFrameSettingsCreate(enc,NULL);
  CHECK(JxlEncoderSetFrameDistance(settings,!strcmp(mode,"lossy")?1.0:0.0)==JXL_ENC_SUCCESS);
  CHECK(JxlEncoderSetFrameLossless(settings,strcmp(mode,"lossy")!=0)==JXL_ENC_SUCCESS);
  CHECK(JxlEncoderFrameSettingsSetOption(settings,JXL_ENC_FRAME_SETTING_EFFORT,3)==JXL_ENC_SUCCESS);
  CHECK(JxlEncoderFrameSettingsSetOption(settings,JXL_ENC_FRAME_SETTING_KEEP_INVISIBLE,1)==JXL_ENC_SUCCESS);
  JxlBitDepth depth={JXL_BIT_DEPTH_FROM_CODESTREAM,0,0};
  CHECK(JxlEncoderSetFrameBitDepth(settings,&depth)==JXL_ENC_SUCCESS);
  JxlPixelFormat format={channels,!strcmp(mode,"float")?JXL_TYPE_FLOAT16:(bits==8?JXL_TYPE_UINT8:JXL_TYPE_UINT16),JXL_LITTLE_ENDIAN,0};
  if(info.have_animation) {JxlFrameHeader fh;JxlEncoderInitFrameHeader(&fh);fh.duration=5;CHECK(JxlEncoderSetFrameHeader(settings,&fh)==JXL_ENC_SUCCESS);}
  CHECK(JxlEncoderAddImageFrame(settings,&format,pixels,n)==JXL_ENC_SUCCESS);
  if(info.have_animation) CHECK(JxlEncoderAddImageFrame(settings,&format,pixels,n)==JXL_ENC_SUCCESS);
  JxlEncoderCloseInput(enc);
  size_t cap=1024*1024,avail=cap;unsigned char *out=malloc(cap),*next=out;CHECK(out);
  CHECK(JxlEncoderProcessOutput(enc,&next,&avail)==JXL_ENC_SUCCESS);
  f=fopen(argv[2],"wb");CHECK(f);CHECK(fwrite(out,1,cap-avail,f)==cap-avail);fclose(f);
  JxlEncoderDestroy(enc);free(out);free(pixels);return 0;
}
