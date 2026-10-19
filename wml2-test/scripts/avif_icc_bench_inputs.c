// Independent development fixtures: identical lossless AV1 payloads with/without ICC.
// Build with libavif1.2.1 and its matching header; not linked into WML2.
#ifndef WML2_AVIF_HEADER
#define WML2_AVIF_HEADER <avif/avif.h>
#endif
#include WML2_AVIF_HEADER
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc != 3) return 2; // sRGB.icc, existing output directory
    FILE *file = fopen(argv[1], "rb");
    if (!file || fseek(file, 0, SEEK_END)) return 3;
    long profile_size = ftell(file);
    rewind(file);
    if (profile_size <= 0) return 3;
    unsigned char *profile = malloc(profile_size);
    if (!profile || fread(profile, 1, profile_size, file) != (size_t)profile_size) return 4;
    fclose(file);
    const char *patterns[] = { "flat", "runs", "varying" };
    for (unsigned size = 32; size <= 128; size *= 4) {
        for (unsigned pattern = 0; pattern < 3; ++pattern) {
            if (size == 128 && pattern == 1) continue;
            avifImage *image = avifImageCreate(size, size, 8, AVIF_PIXEL_FORMAT_YUV444);
            if (!image) return 5;
            image->yuvRange = AVIF_RANGE_FULL;
            image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT709;
            image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_SRGB;
            image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_IDENTITY;
            if (avifImageAllocatePlanes(image, AVIF_PLANES_YUV) != AVIF_RESULT_OK) return 5;
            for (unsigned y = 0; y < size; ++y) for (unsigned x = 0; x < size; ++x) {
                unsigned i = y * size + x;
                unsigned c = pattern == 0 ? 0 : pattern == 1 ? i / 8 : i;
                // Identity YUV444 is encoded G/B/R, with no chroma interpolation.
                image->yuvPlanes[0][y * image->yuvRowBytes[0] + x] = (c * 73 + 131) % 256;
                image->yuvPlanes[1][y * image->yuvRowBytes[1] + x] = (c * 19 + 211) % 256;
                image->yuvPlanes[2][y * image->yuvRowBytes[2] + x] = (c * 37 + 57) % 256;
            }
            for (unsigned icc = 0; icc < 2; ++icc) {
                if (icc && avifImageSetProfileICC(image, profile, profile_size) != AVIF_RESULT_OK) return 6;
                avifEncoder *encoder = avifEncoderCreate();
                encoder->codecChoice = AVIF_CODEC_CHOICE_AOM;
                encoder->speed = AVIF_SPEED_FASTEST;
                encoder->maxThreads = 1;
                encoder->quality = AVIF_QUALITY_LOSSLESS;
                encoder->minQuantizer = encoder->maxQuantizer = 0;
                avifRWData output = AVIF_DATA_EMPTY;
                avifResult result = avifEncoderWrite(encoder, image, &output);
                if (result != AVIF_RESULT_OK) {
                    fprintf(stderr, "encode: %s\n", avifResultToString(result));
                    return 7;
                }
                char path[4096];
                int n = snprintf(path, sizeof(path), "%s/%s-%u-%s.avif", argv[2], patterns[pattern], size, icc ? "icc" : "plain");
                if (n < 0 || n >= (int)sizeof(path)) return 8;
                file = fopen(path, "wb");
                if (!file || fwrite(output.data, 1, output.size, file) != output.size) return 9;
                fclose(file);
                avifRWDataFree(&output);
                avifEncoderDestroy(encoder);
            }
            avifImageDestroy(image);
        }
    }
    free(profile);
    return 0;
}
