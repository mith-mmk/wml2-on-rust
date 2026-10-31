/* Independent libjxl oracle for generated, bounded evaluation inputs only. */
#include <jxl/decode.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

int main(int argc, char **argv) {
  if (argc != 4) return 2;
  FILE *input = fopen(argv[1], "rb");
  if (!input || fseek(input, 0, SEEK_END)) return 3;
  long length = ftell(input);
  if (length < 0 || length > 16 * 1024 * 1024 || fseek(input, 0, SEEK_SET)) return 4;
  uint8_t *data = malloc((size_t)length);
  if (!data || fread(data, 1, length, input) != (size_t)length) return 5;
  fclose(input);
  JxlDecoder *dec = JxlDecoderCreate(NULL);
  if (!dec) return 6;
  JxlDecoderSetKeepOrientation(dec, JXL_TRUE);
  JxlDecoderSetUnpremultiplyAlpha(dec, JXL_FALSE);
  JxlDecoderSubscribeEvents(dec, JXL_DEC_BASIC_INFO | JXL_DEC_COLOR_ENCODING | JXL_DEC_FULL_IMAGE);
  JxlDecoderSetInput(dec, data, (size_t)length);
  JxlDecoderCloseInput(dec);
  JxlBasicInfo info = {0};
  JxlColorEncoding color = {0};
  JxlPixelFormat format = {0, atoi(argv[3]) == 16 ? JXL_TYPE_UINT16 : JXL_TYPE_UINT8, JXL_NATIVE_ENDIAN, 0};
  uint8_t *out = NULL;
  size_t size = 0;
  int full = 0, encoded_profile = 0;
  for (;;) {
    JxlDecoderStatus status = JxlDecoderProcessInput(dec);
    if (status == JXL_DEC_BASIC_INFO) {
      if (JxlDecoderGetBasicInfo(dec, &info) != JXL_DEC_SUCCESS) return 7;
      format.num_channels = info.num_color_channels + (info.alpha_bits != 0);
    } else if (status == JXL_DEC_COLOR_ENCODING) {
      encoded_profile = JxlDecoderGetColorAsEncodedProfile(dec, JXL_COLOR_PROFILE_TARGET_ORIGINAL, &color) == JXL_DEC_SUCCESS;
    } else if (status == JXL_DEC_NEED_IMAGE_OUT_BUFFER) {
      if (JxlDecoderImageOutBufferSize(dec, &format, &size) != JXL_DEC_SUCCESS || size > 32 * 1024 * 1024) return 8;
      out = malloc(size);
      if (!out || JxlDecoderSetImageOutBuffer(dec, &format, out, size) != JXL_DEC_SUCCESS) return 9;
    } else if (status == JXL_DEC_FULL_IMAGE) {
      full++;
    } else if (status == JXL_DEC_SUCCESS) break;
    else { fprintf(stderr, "libjxl decode status %d\n", status); return 10; }
  }
  if (full != 1 || !out) return 11;
  FILE *output = fopen(argv[2], "wb");
  if (!output || fwrite(out, 1, size, output) != size || fclose(output)) return 12;
  printf("{\"decoder_version\":%u,\"width\":%u,\"height\":%u,\"bits\":%u,\"alpha_bits\":%u,\"alpha_premultiplied\":%d,\"orientation\":%d,\"uses_original_profile\":%d,\"channels\":%u,\"structured_profile\":%d,\"color_space\":%d,\"white_point\":%d,\"primaries\":%d,\"transfer_function\":%d,\"decoded_bytes\":%zu}\n",
    JxlDecoderVersion(), info.xsize, info.ysize, info.bits_per_sample, info.alpha_bits,
    info.alpha_premultiplied, info.orientation, info.uses_original_profile, format.num_channels,
    encoded_profile, color.color_space, color.white_point, color.primaries, color.transfer_function, size);
  JxlDecoderDestroy(dec); free(out); free(data);
  return 0;
}
