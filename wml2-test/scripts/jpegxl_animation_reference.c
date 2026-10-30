/* Independent C libjxl coalesced frames, exact tick metadata and output ICC. */
#include <jxl/decode.h>
#include <jxl/version.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#define CHECK(x) do {if(!(x)){fprintf(stderr,"failed: %s\n",#x);return 1;}}while(0)
static int save(const char*name,const void*data,size_t size){FILE*f=fopen(name,"wb");if(!f)return 0;int ok=fwrite(data,1,size,f)==size;fclose(f);return ok;}
int main(int argc,char**argv){
 if(argc==2&&!strcmp(argv[1],"--version")){printf("runtime=%u header=%u\n",JxlDecoderVersion(),JPEGXL_MAJOR_VERSION*1000000+JPEGXL_MINOR_VERSION*1000+JPEGXL_PATCH_VERSION);return 0;}
 CHECK(argc==3||argc==4);int layers=argc==4&&!strcmp(argv[3],"--layers");int want_preview=argc==4&&!layers;FILE*f=fopen(argv[1],"rb");CHECK(f);CHECK(!fseek(f,0,SEEK_END));long n=ftell(f);CHECK(n>0&&n<1<<20);rewind(f);unsigned char*input=malloc(n);CHECK(input);CHECK(fread(input,1,n,f)==(size_t)n);fclose(f);
 JxlDecoder*d=JxlDecoderCreate(NULL);CHECK(d);CHECK(JxlDecoderSetCoalescing(d,!layers)==JXL_DEC_SUCCESS);if(layers)CHECK(JxlDecoderSetKeepOrientation(d,JXL_TRUE)==JXL_DEC_SUCCESS);CHECK(JxlDecoderSetRenderSpotcolors(d,JXL_FALSE)==JXL_DEC_SUCCESS);CHECK(JxlDecoderSubscribeEvents(d,JXL_DEC_BASIC_INFO|JXL_DEC_COLOR_ENCODING|JXL_DEC_FRAME|JXL_DEC_FULL_IMAGE|(want_preview?JXL_DEC_PREVIEW_IMAGE:0))==JXL_DEC_SUCCESS);CHECK(JxlDecoderSetInput(d,input,n)==JXL_DEC_SUCCESS);JxlDecoderCloseInput(d);
 JxlBasicInfo info;JxlFrameHeader frame;JxlPixelFormat format={4,JXL_TYPE_UINT16,JXL_LITTLE_ENDIAN,0};unsigned index=0;size_t size=0;unsigned char*out=NULL,*preview=NULL;size_t preview_size=0;unsigned previews=0;char path[4096];
 for(;;){JxlDecoderStatus status=JxlDecoderProcessInput(d);
  if(status==JXL_DEC_BASIC_INFO){CHECK(JxlDecoderGetBasicInfo(d,&info)==JXL_DEC_SUCCESS);CHECK(info.xsize<=64&&info.ysize<=64&&info.have_animation);printf("image %u %u %u %u %u %u %u %u %u\n",info.xsize,info.ysize,info.bits_per_sample,info.alpha_bits,info.animation.tps_numerator,info.animation.tps_denominator,info.animation.num_loops,info.animation.have_timecodes,info.have_preview);}
  else if(status==JXL_DEC_COLOR_ENCODING){size_t count;CHECK(JxlDecoderGetICCProfileSize(d,JXL_COLOR_PROFILE_TARGET_DATA,&count)==JXL_DEC_SUCCESS);CHECK(count<65536);unsigned char*icc=malloc(count);CHECK(icc);CHECK(JxlDecoderGetColorAsICCProfile(d,JXL_COLOR_PROFILE_TARGET_DATA,icc,count)==JXL_DEC_SUCCESS);snprintf(path,sizeof(path),"%s.icc",argv[2]);CHECK(save(path,icc,count));free(icc);}
  else if(status==JXL_DEC_NEED_PREVIEW_OUT_BUFFER){CHECK(want_preview&&info.have_preview);CHECK(JxlDecoderPreviewOutBufferSize(d,&format,&preview_size)==JXL_DEC_SUCCESS);CHECK(preview_size>0&&preview_size<=64*64*8);preview=malloc(preview_size);CHECK(preview);CHECK(JxlDecoderSetPreviewOutBuffer(d,&format,preview,preview_size)==JXL_DEC_SUCCESS);}
  else if(status==JXL_DEC_PREVIEW_IMAGE){CHECK(want_preview&&!previews++);CHECK(save(argv[3],preview,preview_size));}
  else if(status==JXL_DEC_FRAME){CHECK(JxlDecoderGetFrameHeader(d,&frame)==JXL_DEC_SUCCESS);printf("frame %u %u %u %u %u %u %d %d %u %u %u\n",index,frame.duration,frame.timecode,frame.is_last,frame.layer_info.xsize,frame.layer_info.ysize,frame.layer_info.crop_x0,frame.layer_info.crop_y0,frame.layer_info.blend_info.blendmode,frame.layer_info.blend_info.source,frame.layer_info.save_as_reference);}
  else if(status==JXL_DEC_NEED_IMAGE_OUT_BUFFER){CHECK(index<16);CHECK(JxlDecoderImageOutBufferSize(d,&format,&size)==JXL_DEC_SUCCESS);CHECK(size>0&&size<=64*64*8);free(out);out=malloc(size);CHECK(out);CHECK(JxlDecoderSetImageOutBuffer(d,&format,out,size)==JXL_DEC_SUCCESS);JxlBitDepth depth={JXL_BIT_DEPTH_FROM_PIXEL_FORMAT,0,0};CHECK(JxlDecoderSetImageOutBitDepth(d,&depth)==JXL_DEC_SUCCESS);}
  else if(status==JXL_DEC_FULL_IMAGE){snprintf(path,sizeof(path),"%s.%u.rgba16le",argv[2],index);CHECK(save(path,out,size));index++;}
  else if(status==JXL_DEC_SUCCESS)break;else{fprintf(stderr,"decoder status %d\n",status);return 1;}
 }
 printf("frames %u\n",index);if(want_preview)CHECK(previews==1);free(preview);free(out);free(input);JxlDecoderDestroy(d);return 0;
}
