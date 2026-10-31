/* Independent libjxl 0.11.2 fixture generator/oracle. Trusted tiny inputs only. */
#include <jxl/decode.h>
#include <jxl/encode.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define CHECK(x) do { if (!(x)) { fprintf(stderr,"failed: %s\n",#x); return 1; } } while(0)
static unsigned char *read_file(const char *name, size_t *n) {
  FILE *f=fopen(name,"rb"); if(!f) return NULL;
  if(fseek(f,0,SEEK_END)) {fclose(f); return NULL;}
  long length=ftell(f); if(length<0||length>1024*1024) {fclose(f); return NULL;}
  rewind(f); *n=(size_t)length; unsigned char *p=malloc(*n ? *n : 1);
  if(!p||fread(p,1,*n,f)!=*n) {free(p);fclose(f);return NULL;} fclose(f);return p;
}
static int save(const char *name,const unsigned char *p,size_t n) {
  FILE *f=fopen(name,"wb");if(!f)return 0;
  int ok=fwrite(p,1,n,f)==n; return fclose(f)==0&&ok;
}
int main(int argc,char **argv) {
  CHECK(argc>=2);
  if(!strcmp(argv[1],"encode")) {
    /* encode raw jxl width height channels storage transfer intensity ICC-or-- */
    CHECK(argc==11); size_t n=0;unsigned char *raw=read_file(argv[2],&n);CHECK(raw);
    unsigned w=atoi(argv[4]),h=atoi(argv[5]),c=atoi(argv[6]);
    int is_float=argv[7][0]=='f', bits=atoi(argv[7]+1);
    CHECK(w&&h&&w<=32&&h<=32&&c>=1&&c<=4&&(bits==16||bits==32));
    CHECK(n==(size_t)w*h*c*(bits/8));
    JxlEncoder *enc=JxlEncoderCreate(NULL);CHECK(enc);
    JxlBasicInfo info;JxlEncoderInitBasicInfo(&info);
    info.xsize=w;info.ysize=h;info.bits_per_sample=bits;
    info.exponent_bits_per_sample=is_float?(bits==16?5:8):0;
    info.num_color_channels=c<=2?1:3;info.uses_original_profile=JXL_TRUE;
    info.intensity_target=(float)atof(argv[9]);
    if(c==2||c==4){info.num_extra_channels=1;info.alpha_bits=bits;info.alpha_exponent_bits=info.exponent_bits_per_sample;}
    CHECK(JxlEncoderSetBasicInfo(enc,&info)==JXL_ENC_SUCCESS);
    if(c==2||c==4){JxlExtraChannelInfo a;JxlEncoderInitExtraChannelInfo(JXL_CHANNEL_ALPHA,&a);
      a.bits_per_sample=bits;a.exponent_bits_per_sample=info.exponent_bits_per_sample;
      CHECK(JxlEncoderSetExtraChannelInfo(enc,0,&a)==JXL_ENC_SUCCESS);}
    JxlColorEncoding color;JxlColorEncodingSetToLinearSRGB(&color,c<=2);
    if(!strcmp(argv[8],"pq")||!strcmp(argv[8],"hlg")){
      color.primaries=JXL_PRIMARIES_2100;color.transfer_function=!strcmp(argv[8],"pq")?JXL_TRANSFER_FUNCTION_PQ:JXL_TRANSFER_FUNCTION_HLG;
    } else if(!strcmp(argv[8],"srgb"))color.transfer_function=JXL_TRANSFER_FUNCTION_SRGB;
    if(!strcmp(argv[10],"-"))CHECK(JxlEncoderSetColorEncoding(enc,&color)==JXL_ENC_SUCCESS);
    else {size_t icc_n=0;unsigned char *icc=read_file(argv[10],&icc_n);CHECK(icc);
      CHECK(JxlEncoderSetICCProfile(enc,icc,icc_n)==JXL_ENC_SUCCESS);free(icc);}
    JxlEncoderFrameSettings *settings=JxlEncoderFrameSettingsCreate(enc,NULL);
    CHECK(JxlEncoderSetFrameLossless(settings,JXL_TRUE)==JXL_ENC_SUCCESS);
    CHECK(JxlEncoderFrameSettingsSetOption(settings,JXL_ENC_FRAME_SETTING_EFFORT,3)==JXL_ENC_SUCCESS);
    CHECK(JxlEncoderFrameSettingsSetOption(settings,JXL_ENC_FRAME_SETTING_KEEP_INVISIBLE,1)==JXL_ENC_SUCCESS);
    JxlPixelFormat format={c,is_float?(bits==16?JXL_TYPE_FLOAT16:JXL_TYPE_FLOAT):JXL_TYPE_UINT16,JXL_LITTLE_ENDIAN,0};
    CHECK(JxlEncoderAddImageFrame(settings,&format,raw,n)==JXL_ENC_SUCCESS);JxlEncoderCloseInput(enc);
    size_t avail=1024*1024;unsigned char *out=malloc(avail),*next=out;CHECK(out);
    CHECK(JxlEncoderProcessOutput(enc,&next,&avail)==JXL_ENC_SUCCESS);
    CHECK(save(argv[3],out,(size_t)(next-out)));JxlEncoderDestroy(enc);free(raw);free(out);return 0;
  }
  /* decode jxl output storage target-nits-or-0 */
  CHECK(!strcmp(argv[1],"decode")&&argc==6);size_t n=0;unsigned char *data=read_file(argv[2],&n);CHECK(data);
  JxlDecoder *dec=JxlDecoderCreate(NULL);CHECK(dec);
  CHECK(JxlDecoderSubscribeEvents(dec,JXL_DEC_BASIC_INFO|JXL_DEC_COLOR_ENCODING|JXL_DEC_FULL_IMAGE)==JXL_DEC_SUCCESS);
  CHECK(JxlDecoderSetKeepOrientation(dec,JXL_TRUE)==JXL_DEC_SUCCESS);
  CHECK(JxlDecoderSetUnpremultiplyAlpha(dec,JXL_FALSE)==JXL_DEC_SUCCESS);
  if(atof(argv[5])>0)CHECK(JxlDecoderSetDesiredIntensityTarget(dec,(float)atof(argv[5]))==JXL_DEC_SUCCESS);
  JxlDecoderSetInput(dec,data,n);JxlDecoderCloseInput(dec);
  JxlBasicInfo info={0};JxlColorEncoding color={0};int structured=0,full=0;
  JxlPixelFormat format={0,!strcmp(argv[4],"f32")?JXL_TYPE_FLOAT:(!strcmp(argv[4],"f16")?JXL_TYPE_FLOAT16:JXL_TYPE_UINT16),JXL_LITTLE_ENDIAN,0};
  unsigned char *out=NULL;size_t out_n=0;
  for(;;){JxlDecoderStatus s=JxlDecoderProcessInput(dec);
    if(s==JXL_DEC_BASIC_INFO){CHECK(JxlDecoderGetBasicInfo(dec,&info)==JXL_DEC_SUCCESS);format.num_channels=info.num_color_channels+(info.alpha_bits!=0);}
    else if(s==JXL_DEC_COLOR_ENCODING){structured=JxlDecoderGetColorAsEncodedProfile(dec,JXL_COLOR_PROFILE_TARGET_ORIGINAL,&color)==JXL_DEC_SUCCESS;
      {size_t icc_n=0;CHECK(JxlDecoderGetICCProfileSize(dec,JXL_COLOR_PROFILE_TARGET_ORIGINAL,&icc_n)==JXL_DEC_SUCCESS&&icc_n<=1024*1024);
        unsigned char *icc=malloc(icc_n);CHECK(icc);CHECK(JxlDecoderGetColorAsICCProfile(dec,JXL_COLOR_PROFILE_TARGET_ORIGINAL,icc,icc_n)==JXL_DEC_SUCCESS);
        char name[4096];CHECK(snprintf(name,sizeof(name),"%s.icc",argv[3])<(int)sizeof(name));CHECK(save(name,icc,icc_n));free(icc);}}
    else if(s==JXL_DEC_NEED_IMAGE_OUT_BUFFER){CHECK(JxlDecoderImageOutBufferSize(dec,&format,&out_n)==JXL_DEC_SUCCESS&&out_n<=1024*1024);
      out=malloc(out_n);CHECK(out);CHECK(JxlDecoderSetImageOutBuffer(dec,&format,out,out_n)==JXL_DEC_SUCCESS);}
    else if(s==JXL_DEC_FULL_IMAGE)full++;
    else if(s==JXL_DEC_SUCCESS)break;
    else {fprintf(stderr,"decoder status %d\n",s);return 2;}}
  CHECK(full==1&&out&&save(argv[3],out,out_n));
  /* Query DATA separately after actual output has completed. ORIGINAL is not
   * a substitute: the decoder may choose another reconstruction profile. */
  {size_t icc_n=0;CHECK(JxlDecoderGetICCProfileSize(dec,JXL_COLOR_PROFILE_TARGET_DATA,&icc_n)==JXL_DEC_SUCCESS&&icc_n<=1024*1024);
    unsigned char *icc=malloc(icc_n);CHECK(icc);CHECK(JxlDecoderGetColorAsICCProfile(dec,JXL_COLOR_PROFILE_TARGET_DATA,icc,icc_n)==JXL_DEC_SUCCESS);
    char name[4096];CHECK(snprintf(name,sizeof(name),"%s.data.icc",argv[3])<(int)sizeof(name));CHECK(save(name,icc,icc_n));free(icc);}
  printf("{\"version\":%u,\"width\":%u,\"height\":%u,\"channels\":%u,\"bits\":%u,\"exponent_bits\":%u,\"alpha_bits\":%u,\"alpha_exponent_bits\":%u,\"uses_original_profile\":%d,\"intensity_target\":%g,\"structured_profile\":%d,\"primaries\":%d,\"transfer\":%d,\"bytes\":%zu}\n",
    JxlDecoderVersion(),info.xsize,info.ysize,format.num_channels,info.bits_per_sample,info.exponent_bits_per_sample,info.alpha_bits,info.alpha_exponent_bits,info.uses_original_profile,info.intensity_target,structured,color.primaries,color.transfer_function,out_n);
  JxlDecoderDestroy(dec);free(data);free(out);return 0;
}
