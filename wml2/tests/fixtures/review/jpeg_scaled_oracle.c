#include <stdio.h>
#include <stdlib.h>
#include <jpeglib.h>
int main(int argc,char **argv){
 if(argc!=4)return 2;FILE *in=fopen(argv[1],"rb"),*out=fopen(argv[3],"wb");if(!in||!out)return 3;
 struct jpeg_decompress_struct c;struct jpeg_error_mgr e;c.err=jpeg_std_error(&e);jpeg_create_decompress(&c);jpeg_stdio_src(&c,in);jpeg_read_header(&c,TRUE);
 c.scale_num=1;c.scale_denom=atoi(argv[2]);c.do_fancy_upsampling=FALSE;c.dct_method=JDCT_ISLOW;c.out_color_space=JCS_EXT_RGBA;
 jpeg_start_decompress(&c);fprintf(stderr,"%u %u\n",c.output_width,c.output_height);
 unsigned char *row=malloc(c.output_width*4);while(c.output_scanline<c.output_height){JSAMPROW p=row;jpeg_read_scanlines(&c,&p,1);fwrite(row,4,c.output_width,out);}free(row);jpeg_finish_decompress(&c);jpeg_destroy_decompress(&c);fclose(in);fclose(out);return 0;
}
