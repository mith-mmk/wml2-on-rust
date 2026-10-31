/* Independent libjxl preview + main pixels and output-profile reference.
 * This fixture utility is not part of the WML2 runtime. */
#include <jxl/decode.h>
#include <jxl/encode.h>
#include <jxl/color_encoding.h>
#include <jxl/version.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define CHECK(x) do {if(!(x)){fprintf(stderr,"failed: %s\n",#x);return 1;}}while(0)
static int save(const char *name,const void *data,size_t size){
 FILE*f=fopen(name,"wb");if(!f)return 0;int ok=fwrite(data,1,size,f)==size;fclose(f);return ok;
}
int main(int argc,char**argv){
 if(argc==2&&!strcmp(argv[1],"--version")){printf("runtime=%u header=%u\n",JxlDecoderVersion(),JPEGXL_MAJOR_VERSION*1000000+JPEGXL_MINOR_VERSION*1000+JPEGXL_PATCH_VERSION);return 0;}
 CHECK(argc==5);int want_preview=strcmp(argv[4],"-")!=0;
 FILE*f=fopen(argv[1],"rb");CHECK(f);CHECK(!fseek(f,0,SEEK_END));long length=ftell(f);CHECK(length>0&&length<16*1024*1024);rewind(f);unsigned char*input=malloc(length);CHECK(input);CHECK(fread(input,1,length,f)==(size_t)length);fclose(f);
 JxlDecoder*d=JxlDecoderCreate(NULL);CHECK(d);CHECK(JxlDecoderSubscribeEvents(d,JXL_DEC_BASIC_INFO|JXL_DEC_COLOR_ENCODING|JXL_DEC_FULL_IMAGE|(want_preview?JXL_DEC_PREVIEW_IMAGE:0))==JXL_DEC_SUCCESS);CHECK(JxlDecoderSetInput(d,input,length)==JXL_DEC_SUCCESS);JxlDecoderCloseInput(d);
 JxlBasicInfo info;JxlPixelFormat format={4,JXL_TYPE_UINT16,JXL_LITTLE_ENDIAN,0};
 unsigned char*main=NULL,*preview=NULL;size_t main_size=0,preview_size=0;int full=0,preview_done=0;
 for(;;){JxlDecoderStatus status=JxlDecoderProcessInput(d);
  if(status==JXL_DEC_BASIC_INFO){CHECK(JxlDecoderGetBasicInfo(d,&info)==JXL_DEC_SUCCESS);}
  else if(status==JXL_DEC_COLOR_ENCODING){
   if(!info.uses_original_profile){JxlColorEncoding c;JxlColorEncodingSetToSRGB(&c,JXL_FALSE);c.rendering_intent=JXL_RENDERING_INTENT_RELATIVE;CHECK(JxlDecoderSetPreferredColorProfile(d,&c)==JXL_DEC_SUCCESS);}
   size_t n=0;CHECK(JxlDecoderGetICCProfileSize(d,JXL_COLOR_PROFILE_TARGET_DATA,&n)==JXL_DEC_SUCCESS);CHECK(n<1024*1024);unsigned char*icc=malloc(n);CHECK(icc);CHECK(JxlDecoderGetColorAsICCProfile(d,JXL_COLOR_PROFILE_TARGET_DATA,icc,n)==JXL_DEC_SUCCESS);CHECK(save(argv[3],icc,n));free(icc);
  }else if(status==JXL_DEC_NEED_PREVIEW_OUT_BUFFER){
   CHECK(want_preview&&info.have_preview&&!preview);CHECK(JxlDecoderPreviewOutBufferSize(d,&format,&preview_size)==JXL_DEC_SUCCESS);CHECK(preview_size>0&&preview_size<=16*1024*1024);preview=malloc(preview_size);CHECK(preview);CHECK(JxlDecoderSetPreviewOutBuffer(d,&format,preview,preview_size)==JXL_DEC_SUCCESS);
  }else if(status==JXL_DEC_PREVIEW_IMAGE){CHECK(preview&&!preview_done);preview_done=1;
  }else if(status==JXL_DEC_NEED_IMAGE_OUT_BUFFER){
   CHECK(!main);CHECK(JxlDecoderImageOutBufferSize(d,&format,&main_size)==JXL_DEC_SUCCESS);CHECK(main_size>0&&main_size<=16*1024*1024);main=malloc(main_size);CHECK(main);CHECK(JxlDecoderSetImageOutBuffer(d,&format,main,main_size)==JXL_DEC_SUCCESS);JxlBitDepth depth={JXL_BIT_DEPTH_FROM_PIXEL_FORMAT,0,0};CHECK(JxlDecoderSetImageOutBitDepth(d,&depth)==JXL_DEC_SUCCESS);
  }else if(status==JXL_DEC_FULL_IMAGE){CHECK(!full);full=1;
  }else if(status==JXL_DEC_SUCCESS)break;
  else{fprintf(stderr,"decoder status %d\n",status);return 1;}
 }
 CHECK(full&&main);CHECK(main_size==(size_t)info.xsize*info.ysize*8);CHECK(save(argv[2],main,main_size));
 if(want_preview){CHECK(preview_done&&preview);CHECK(preview_size==(size_t)info.preview.xsize*info.preview.ysize*8);CHECK(save(argv[4],preview,preview_size));}
 printf("%u %u %d %u %u %u %u\n",info.xsize,info.ysize,info.have_preview,info.preview.xsize,info.preview.ysize,info.bits_per_sample,info.alpha_bits);
 free(main);free(preview);free(input);JxlDecoderDestroy(d);return 0;
}
