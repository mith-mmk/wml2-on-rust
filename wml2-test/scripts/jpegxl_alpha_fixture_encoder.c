/* Independent libjxl encoder for integer alpha qualification. Pixel input is
 * normalized U16; color and alpha source bit depths are signaled separately. */
#include <jxl/encode.h>
#include <jxl/color_encoding.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr,"failed: %s\n",#x); return 1; } } while(0)
int main(int argc,char**argv) {
  CHECK(argc==11);
  const unsigned w=atoi(argv[3]),h=atoi(argv[4]),color=atoi(argv[5]),bits=atoi(argv[6]),alpha=atoi(argv[7]),orientation=atoi(argv[8]);
  CHECK(w&&h&&w<=1024&&h<=1024&&(color==1||color==3)&&bits>=8&&bits<=16&&alpha>=1&&alpha<=16&&orientation>=1&&orientation<=8);
  size_t size=(size_t)w*h*(color+1)*2;unsigned char*input=malloc(size);CHECK(input);FILE*f=fopen(argv[1],"rb");CHECK(f);CHECK(fread(input,1,size,f)==size);fclose(f);
  JxlEncoder*enc=JxlEncoderCreate(NULL);CHECK(enc);JxlBasicInfo info;JxlEncoderInitBasicInfo(&info);
  info.xsize=w;info.ysize=h;info.bits_per_sample=bits;info.num_color_channels=color;info.num_extra_channels=(!strcmp(argv[9],"multiple")||!strcmp(argv[9],"multiple-alpha"))?2:1;info.alpha_bits=alpha;info.alpha_premultiplied=!strcmp(argv[9],"associated");info.uses_original_profile=JXL_TRUE;info.orientation=(JxlOrientation)orientation;
  const int nonalpha=!strcmp(argv[9],"depth")||!strcmp(argv[9],"spot");
  if(nonalpha) info.alpha_bits=0;
  if(!strcmp(argv[9],"float-alpha")) info.alpha_exponent_bits=5;
  CHECK(JxlEncoderSetBasicInfo(enc,&info)==JXL_ENC_SUCCESS);
  JxlExtraChannelInfo ec;JxlEncoderInitExtraChannelInfo(nonalpha?(!strcmp(argv[9],"depth")?JXL_CHANNEL_DEPTH:JXL_CHANNEL_SPOT_COLOR):JXL_CHANNEL_ALPHA,&ec);ec.bits_per_sample=alpha;if(!strcmp(argv[9],"subsampled"))ec.dim_shift=1;if(!strcmp(argv[9],"float-alpha"))ec.exponent_bits_per_sample=5;if(!strcmp(argv[9],"spot")){ec.spot_color[0]=1;ec.spot_color[3]=1;}ec.alpha_premultiplied=info.alpha_premultiplied;CHECK(JxlEncoderSetExtraChannelInfo(enc,0,&ec)==JXL_ENC_SUCCESS);
  if(info.num_extra_channels==2){JxlExtraChannelInfo extra;JxlEncoderInitExtraChannelInfo(!strcmp(argv[9],"multiple-alpha")?JXL_CHANNEL_ALPHA:JXL_CHANNEL_DEPTH,&extra);extra.bits_per_sample=alpha;CHECK(JxlEncoderSetExtraChannelInfo(enc,1,&extra)==JXL_ENC_SUCCESS);}
  if(!strcmp(argv[10],"-")){JxlColorEncoding color_encoding;JxlColorEncodingSetToSRGB(&color_encoding,color==1);CHECK(JxlEncoderSetColorEncoding(enc,&color_encoding)==JXL_ENC_SUCCESS);}
  else {f=fopen(argv[10],"rb");CHECK(f);CHECK(!fseek(f,0,SEEK_END));long n=ftell(f);CHECK(n>0&&n<=65536);rewind(f);unsigned char*icc=malloc(n);CHECK(icc);CHECK(fread(icc,1,n,f)==(size_t)n);fclose(f);CHECK(JxlEncoderSetICCProfile(enc,icc,n)==JXL_ENC_SUCCESS);free(icc);}
  JxlEncoderFrameSettings*settings=JxlEncoderFrameSettingsCreate(enc,NULL);CHECK(settings);CHECK(JxlEncoderSetFrameLossless(settings,strcmp(argv[9],"subsampled")!=0)==JXL_ENC_SUCCESS);if(!strcmp(argv[9],"subsampled")){CHECK(JxlEncoderSetFrameDistance(settings,0)==JXL_ENC_SUCCESS);CHECK(JxlEncoderFrameSettingsSetOption(settings,JXL_ENC_FRAME_SETTING_MODULAR,1)==JXL_ENC_SUCCESS);CHECK(JxlEncoderFrameSettingsSetOption(settings,JXL_ENC_FRAME_SETTING_EXTRA_CHANNEL_RESAMPLING,2)==JXL_ENC_SUCCESS);}CHECK(JxlEncoderFrameSettingsSetOption(settings,JXL_ENC_FRAME_SETTING_EFFORT,3)==JXL_ENC_SUCCESS);CHECK(JxlEncoderFrameSettingsSetOption(settings,JXL_ENC_FRAME_SETTING_KEEP_INVISIBLE,1)==JXL_ENC_SUCCESS);
  /* Associated qualification uses RGB multiplied by alpha (RGB<=alpha),
   * including RGB=0 at alpha=0; input is normalized little-endian U16. */
  if(!strcmp(argv[9],"associated")){for(size_t i=0;i<(size_t)w*h;i++){unsigned char*p=input+i*(color+1)*2;uint32_t a=p[color*2]|((uint32_t)p[color*2+1]<<8);for(unsigned c=0;c<color;c++){uint32_t v=p[c*2]|((uint32_t)p[c*2+1]<<8);v=(v*a+32767)/65535;p[c*2]=v&255;p[c*2+1]=v>>8;}}}
  if(!strcmp(argv[9],"float-alpha")){for(size_t i=0;i<(size_t)w*h;i++){unsigned char*v=input+(i*(color+1)+color)*2;v[0]=v[1]=(i%2)?255:0;}}
  JxlPixelFormat format={nonalpha?color:color+1,JXL_TYPE_UINT16,JXL_LITTLE_ENDIAN,0};JxlBitDepth depth={JXL_BIT_DEPTH_FROM_PIXEL_FORMAT,0,0};CHECK(JxlEncoderSetFrameBitDepth(settings,&depth)==JXL_ENC_SUCCESS);unsigned char*plane=malloc((size_t)w*h*2);CHECK(plane);
  for(size_t i=0;i<(size_t)w*h;i++)memcpy(plane+i*2,input+(i*(color+1)+color)*2,2);
  if(nonalpha){for(size_t i=0;i<(size_t)w*h;i++)memmove(input+i*color*2,input+i*(color+1)*2,color*2);size=(size_t)w*h*color*2;}
  CHECK(JxlEncoderAddImageFrame(settings,&format,input,size)==JXL_ENC_SUCCESS);
  JxlPixelFormat extra_format={1,JXL_TYPE_UINT16,JXL_LITTLE_ENDIAN,0};
  if(nonalpha||info.num_extra_channels==2)CHECK(JxlEncoderSetExtraChannelBuffer(settings,&extra_format,plane,(size_t)w*h*2,nonalpha?0:1)==JXL_ENC_SUCCESS);
  free(plane);JxlEncoderCloseInput(enc);
  size_t cap=1024*1024,available=cap;unsigned char*out=malloc(cap),*next=out;CHECK(out);CHECK(JxlEncoderProcessOutput(enc,&next,&available)==JXL_ENC_SUCCESS);f=fopen(argv[2],"wb");CHECK(f);CHECK(fwrite(out,1,cap-available,f)==cap-available);fclose(f);JxlEncoderDestroy(enc);free(input);free(out);return 0;
}
