/* Independent libjxl normalized RGBA16 / actual output ICC reference. */
#include <jxl/decode.h>
#include <jxl/encode.h>
#include <jxl/version.h>
#include <jxl/color_encoding.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#define CHECK(x) do {if(!(x)){fprintf(stderr,"failed: %s\n",#x);return 1;}}while(0)
int main(int argc,char**argv){
 if(argc==2&&!strcmp(argv[1],"--version")){printf("runtime=%u header=%u\n",JxlDecoderVersion(),JPEGXL_MAJOR_VERSION*1000000+JPEGXL_MINOR_VERSION*1000+JPEGXL_PATCH_VERSION);return 0;}
 CHECK(argc==4||argc==5);FILE*f=fopen(argv[1],"rb");CHECK(f);CHECK(!fseek(f,0,SEEK_END));long n=ftell(f);CHECK(n>0&&n<=1024*1024);rewind(f);unsigned char*input=malloc(n);CHECK(input);CHECK(fread(input,1,n,f)==(size_t)n);fclose(f);
 JxlDecoder*d=JxlDecoderCreate(NULL);CHECK(d);CHECK(JxlDecoderSetRenderSpotcolors(d,JXL_FALSE)==JXL_DEC_SUCCESS);CHECK(JxlDecoderSetUnpremultiplyAlpha(d,argc==5&&!strcmp(argv[4],"straight"))==JXL_DEC_SUCCESS);CHECK(JxlDecoderSubscribeEvents(d,JXL_DEC_BASIC_INFO|JXL_DEC_COLOR_ENCODING|JXL_DEC_FULL_IMAGE)==JXL_DEC_SUCCESS);CHECK(JxlDecoderSetInput(d,input,n)==JXL_DEC_SUCCESS);JxlDecoderCloseInput(d);
 JxlBasicInfo info;JxlPixelFormat format={4,JXL_TYPE_UINT16,JXL_LITTLE_ENDIAN,0};unsigned char*out=NULL;size_t bytes=0;int full=0;
 for(;;){JxlDecoderStatus status=JxlDecoderProcessInput(d);
  if(status==JXL_DEC_BASIC_INFO){CHECK(JxlDecoderGetBasicInfo(d,&info)==JXL_DEC_SUCCESS);CHECK(info.xsize<=1024&&info.ysize<=1024);
   fprintf(stderr,"image %u %u %u %u\n",info.bits_per_sample,info.alpha_bits,info.num_color_channels,info.orientation);
   for(uint32_t i=0;i<info.num_extra_channels;i++){JxlExtraChannelInfo ec;CHECK(JxlDecoderGetExtraChannelInfo(d,i,&ec)==JXL_DEC_SUCCESS);fprintf(stderr,"extra %u %u %u %u %u %u\n",i,ec.type,ec.bits_per_sample,ec.exponent_bits_per_sample,ec.dim_shift,ec.alpha_premultiplied);}}
  else if(status==JXL_DEC_COLOR_ENCODING){
   if(!info.uses_original_profile){JxlColorEncoding color;JxlColorEncodingSetToSRGB(&color,JXL_FALSE);color.rendering_intent=JXL_RENDERING_INTENT_RELATIVE;CHECK(JxlDecoderSetPreferredColorProfile(d,&color)==JXL_DEC_SUCCESS);}
   size_t size=0;CHECK(JxlDecoderGetICCProfileSize(d,JXL_COLOR_PROFILE_TARGET_DATA,&size)==JXL_DEC_SUCCESS);unsigned char*icc=malloc(size);CHECK(icc);CHECK(JxlDecoderGetColorAsICCProfile(d,JXL_COLOR_PROFILE_TARGET_DATA,icc,size)==JXL_DEC_SUCCESS);f=fopen(argv[3],"wb");CHECK(f);CHECK(fwrite(icc,1,size,f)==size);fclose(f);free(icc);
  }else if(status==JXL_DEC_NEED_IMAGE_OUT_BUFFER){CHECK(!out);CHECK(JxlDecoderImageOutBufferSize(d,&format,&bytes)==JXL_DEC_SUCCESS);out=malloc(bytes);CHECK(out);CHECK(JxlDecoderSetImageOutBuffer(d,&format,out,bytes)==JXL_DEC_SUCCESS);JxlBitDepth depth={JXL_BIT_DEPTH_FROM_PIXEL_FORMAT,0,0};CHECK(JxlDecoderSetImageOutBitDepth(d,&depth)==JXL_DEC_SUCCESS);
  }else if(status==JXL_DEC_FULL_IMAGE){CHECK(!full);full=1;}
  else if(status==JXL_DEC_SUCCESS)break;else{fprintf(stderr,"decoder status %d\n",status);return 1;}
 }
 CHECK(full&&out);CHECK(bytes==(size_t)info.xsize*info.ysize*8);f=fopen(argv[2],"wb");CHECK(f);CHECK(fwrite(out,1,bytes,f)==bytes);fclose(f);printf("%u %u\n",info.xsize,info.ysize);free(out);free(input);JxlDecoderDestroy(d);return 0;
}
