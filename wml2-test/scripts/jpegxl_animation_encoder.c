/* Independent, small C libjxl animation qualification inputs. */
#include <jxl/encode.h>
#include <jxl/color_encoding.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#define CHECK(x) do {if(!(x)){fprintf(stderr,"failed: %s\n",#x);return 1;}}while(0)
int main(int argc,char**argv){
 CHECK(argc==9);const char*mode=argv[2];unsigned bits=atoi(argv[3]),orientation=atoi(argv[4]);CHECK((bits==8||bits==16)&&orientation>=1&&orientation<=8);
 JxlEncoder*enc=JxlEncoderCreate(NULL);CHECK(enc);JxlBasicInfo info;JxlEncoderInitBasicInfo(&info);info.xsize=!strcmp(mode,"preview-one")?2:7;info.ysize=!strcmp(mode,"preview-one")?3:5;info.bits_per_sample=bits;info.num_color_channels=3;info.num_extra_channels=1;info.alpha_bits=bits;info.uses_original_profile=JXL_TRUE;info.orientation=(JxlOrientation)orientation;info.have_animation=JXL_TRUE;info.animation.tps_numerator=strtoul(argv[5],NULL,10);info.animation.tps_denominator=strtoul(argv[6],NULL,10);info.animation.num_loops=strtoul(argv[7],NULL,10);info.animation.have_timecodes=!strcmp(mode,"timecodes");CHECK(JxlEncoderSetBasicInfo(enc,&info)==JXL_ENC_SUCCESS);
 if(!strcmp(argv[8],"-")){JxlColorEncoding color;JxlColorEncodingSetToSRGB(&color,JXL_FALSE);CHECK(JxlEncoderSetColorEncoding(enc,&color)==JXL_ENC_SUCCESS);}else{FILE*f=fopen(argv[8],"rb");CHECK(f);CHECK(!fseek(f,0,SEEK_END));long n=ftell(f);CHECK(n>0&&n<65536);rewind(f);unsigned char*p=malloc(n);CHECK(p);CHECK(fread(p,1,n,f)==(size_t)n);fclose(f);CHECK(JxlEncoderSetICCProfile(enc,p,n)==JXL_ENC_SUCCESS);free(p);}
 unsigned frames=!strcmp(mode,"preview-one")?1:(!strcmp(mode,"internal")?4:3);
 for(unsigned i=0;i<frames;i++){
  JxlEncoderFrameSettings*s=JxlEncoderFrameSettingsCreate(enc,NULL);CHECK(s);CHECK(JxlEncoderSetFrameLossless(s,JXL_TRUE)==JXL_ENC_SUCCESS);CHECK(JxlEncoderFrameSettingsSetOption(s,JXL_ENC_FRAME_SETTING_EFFORT,3)==JXL_ENC_SUCCESS);CHECK(JxlEncoderFrameSettingsSetOption(s,JXL_ENC_FRAME_SETTING_KEEP_INVISIBLE,1)==JXL_ENC_SUCCESS);
  JxlFrameHeader frame;JxlEncoderInitFrameHeader(&frame);frame.duration=(uint32_t[]){5,17,3,0}[i];if(!strcmp(mode,"long"))frame.duration=UINT32_MAX-i;if(!strcmp(mode,"internal"))frame.duration=(uint32_t[]){0,5,17,0}[i];if(info.animation.have_timecodes)frame.timecode=i+1;
  if(!strcmp(mode,"preview-one")){frame.duration=0;}
  unsigned w=info.xsize,h=info.ysize;
  if(!strcmp(mode,"reference")||!strcmp(mode,"internal")){
   frame.layer_info.save_as_reference=(i%2)+1;
   if(i>0&&i<3){w=3;h=2;frame.layer_info.have_crop=JXL_TRUE;frame.layer_info.crop_x0=i==1?1:-1;frame.layer_info.crop_y0=i==1?1:2;frame.layer_info.xsize=w;frame.layer_info.ysize=h;frame.layer_info.blend_info.blendmode=JXL_BLEND_BLEND;frame.layer_info.blend_info.source=((i-1)%2)+1;frame.layer_info.blend_info.alpha=0;frame.layer_info.blend_info.clamp=JXL_TRUE;}
  }
  CHECK(JxlEncoderSetFrameHeader(s,&frame)==JXL_ENC_SUCCESS);if(frame.layer_info.blend_info.blendmode==JXL_BLEND_BLEND)CHECK(JxlEncoderSetExtraChannelBlendInfo(s,0,&frame.layer_info.blend_info)==JXL_ENC_SUCCESS);
  char name[32];snprintf(name,sizeof(name),"coded-layer-%u",i);CHECK(JxlEncoderSetFrameName(s,name)==JXL_ENC_SUCCESS);
  uint16_t words[7*5*4];unsigned char bytes[7*5*4];
  for(unsigned p=0;p<w*h;p++)for(unsigned c=0;c<4;c++){unsigned v=c==3?((unsigned[]){255,128,0,1,254})[(p+i)%5]:(17+p*(31+c*19)+i*(73+c*11))%256;if(i==0&&(!strcmp(mode,"reference")||!strcmp(mode,"internal"))&&c==3)v=255;bytes[p*4+c]=v;words[p*4+c]=(uint16_t)(v*257+(bits==16&&c<3&&v<255?i+1:0));}
  JxlPixelFormat format={4,bits==8?JXL_TYPE_UINT8:JXL_TYPE_UINT16,JXL_LITTLE_ENDIAN,0};JxlBitDepth depth={JXL_BIT_DEPTH_FROM_PIXEL_FORMAT,0,0};CHECK(JxlEncoderSetFrameBitDepth(s,&depth)==JXL_ENC_SUCCESS);CHECK(JxlEncoderAddImageFrame(s,&format,bits==8?(void*)bytes:(void*)words,w*h*4*(bits==8?1:2))==JXL_ENC_SUCCESS);
 }
 JxlEncoderCloseInput(enc);size_t size=1<<20,left=size;unsigned char*output=malloc(size),*next=output;CHECK(output);CHECK(JxlEncoderProcessOutput(enc,&next,&left)==JXL_ENC_SUCCESS);FILE*f=fopen(argv[1],"wb");CHECK(f);CHECK(fwrite(output,1,size-left,f)==size-left);fclose(f);free(output);JxlEncoderDestroy(enc);return 0;
}
