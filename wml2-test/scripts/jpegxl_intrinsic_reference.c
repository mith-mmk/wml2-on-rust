#include <jxl/decode.h>
#include <stdio.h>
#include <stdlib.h>
#define CHECK(x)do{if(!(x)){fprintf(stderr,"failed %s\n",#x);return 1;}}while(0)
int main(int argc,char**argv){CHECK(argc==3);FILE*f=fopen(argv[1],"rb");CHECK(f);fseek(f,0,SEEK_END);size_t size=ftell(f);CHECK(size>0&&size<16*1024*1024);rewind(f);unsigned char*data=malloc(size);CHECK(data);CHECK(fread(data,1,size,f)==size);fclose(f);JxlDecoder*d=JxlDecoderCreate(NULL);CHECK(d);CHECK(JxlDecoderSetKeepOrientation(d,atoi(argv[2]))==JXL_DEC_SUCCESS);CHECK(JxlDecoderSubscribeEvents(d,JXL_DEC_BASIC_INFO)==JXL_DEC_SUCCESS);CHECK(JxlDecoderSetInput(d,data,size)==JXL_DEC_SUCCESS);JxlDecoderCloseInput(d);CHECK(JxlDecoderProcessInput(d)==JXL_DEC_BASIC_INFO);JxlBasicInfo info;CHECK(JxlDecoderGetBasicInfo(d,&info)==JXL_DEC_SUCCESS);printf("%u %u %u %u %u\n",info.xsize,info.ysize,info.orientation,info.intrinsic_xsize,info.intrinsic_ysize);JxlDecoderDestroy(d);free(data);return 0;}
