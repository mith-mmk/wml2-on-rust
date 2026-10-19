// Independent fixture/reference tooling; not linked into WML2.
// Requires matching libavif headers/runtime. Override the include for a pinned
// header with -DWML2_AVIF_HEADER='"/path/to/avif.h"'.
#ifndef WML2_AVIF_HEADER
#define WML2_AVIF_HEADER <avif/avif.h>
#endif
#include WML2_AVIF_HEADER
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int decode(const char *input, const char *raw, const char *icc) {
    avifDecoder *decoder = avifDecoderCreate();
    avifImage *image = avifImageCreateEmpty();
    avifResult result = avifDecoderReadFile(decoder, image, input);
    if (result != AVIF_RESULT_OK) {
        fprintf(stderr, "decode: %s\n", avifResultToString(result));
        return 3;
    }
    avifRGBImage rgb;
    avifRGBImageSetDefaults(&rgb, image);
    rgb.depth = 16;
    rgb.format = AVIF_RGB_FORMAT_RGBA;
    rgb.alphaPremultiplied = AVIF_FALSE;
    rgb.chromaUpsampling = AVIF_CHROMA_UPSAMPLING_BEST_QUALITY;
    if (avifRGBImageAllocatePixels(&rgb) != AVIF_RESULT_OK) return 4;
    result = avifImageYUVToRGB(image, &rgb);
    if (result != AVIF_RESULT_OK) {
        fprintf(stderr, "convert: %s\n", avifResultToString(result));
        return 5;
    }
    FILE *file = fopen(raw, "wb");
    if (!file) return 6;
    for (unsigned y = 0; y < rgb.height; ++y) {
        const uint16_t *row = (const uint16_t *)(rgb.pixels + y * rgb.rowBytes);
        for (unsigned x = 0; x < rgb.width * 4; ++x) {
            const unsigned char le[2] = { row[x] & 255, row[x] >> 8 };
            if (fwrite(le, 1, 2, file) != 2) return 7;
        }
    }
    fclose(file);
    file = fopen(icc, "wb");
    if (!file) return 8;
    if (fwrite(image->icc.data, 1, image->icc.size, file) != image->icc.size) return 9;
    fclose(file);
    avifRGBImageFreePixels(&rgb);
    avifImageDestroy(image);
    avifDecoderDestroy(decoder);
    return 0;
}

static int gray_limited(unsigned bits, const char *profile_path, const char *output_path) {
    if (bits != 8 && bits != 10 && bits != 12) return 2;
    FILE *file = fopen(profile_path, "rb");
    if (!file) return 3;
    fseek(file, 0, SEEK_END);
    const long size = ftell(file);
    rewind(file);
    if (size <= 0) return 4;
    unsigned char *profile = malloc(size);
    if (!profile || fread(profile, 1, size, file) != (size_t)size) return 5;
    fclose(file);
    avifImage *image = avifImageCreate(16, 16, bits, AVIF_PIXEL_FORMAT_YUV400);
    image->yuvRange = AVIF_RANGE_LIMITED;
    image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT709;
    image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_LINEAR;
    image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_BT709;
    if (avifImageSetProfileICC(image, profile, size) != AVIF_RESULT_OK) return 6;
    if (avifImageAllocatePlanes(image, AVIF_PLANES_YUV) != AVIF_RESULT_OK) return 7;
    for (unsigned y = 0; y < 16; ++y) for (unsigned x = 0; x < 16; ++x) {
        unsigned value = (16u << (bits - 8)) +
            (((x + y * 16) * 37 % 256) * (219u << (bits - 8)) + 127) / 255;
        if (bits == 8) image->yuvPlanes[0][y * image->yuvRowBytes[0] + x] = value;
        else ((uint16_t *)(image->yuvPlanes[0] + y * image->yuvRowBytes[0]))[x] = value;
    }
    avifEncoder *encoder = avifEncoderCreate();
    encoder->codecChoice = AVIF_CODEC_CHOICE_AOM;
    encoder->speed = AVIF_SPEED_FASTEST;
    encoder->quality = AVIF_QUALITY_LOSSLESS;
    encoder->minQuantizer = encoder->maxQuantizer = 0;
    avifRWData output = AVIF_DATA_EMPTY;
    avifResult result = avifEncoderWrite(encoder, image, &output);
    if (result != AVIF_RESULT_OK) {
        fprintf(stderr, "encode: %s\n", avifResultToString(result));
        return 8;
    }
    file = fopen(output_path, "wb");
    if (!file || fwrite(output.data, 1, output.size, file) != output.size) return 9;
    fclose(file);
    avifRWDataFree(&output);
    avifEncoderDestroy(encoder);
    avifImageDestroy(image);
    free(profile);
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--version")) {
        printf("libavif=%s header=%d.%d.%d\n", avifVersion(),
            AVIF_VERSION_MAJOR, AVIF_VERSION_MINOR, AVIF_VERSION_PATCH);
        return 0;
    }
    if (argc == 5 && !strcmp(argv[1], "--gray-limited"))
        return gray_limited(strtoul(argv[2], NULL, 10), argv[3], argv[4]);
    if (argc != 4) return 2;
    return decode(argv[1], argv[2], argv[3]);
}
