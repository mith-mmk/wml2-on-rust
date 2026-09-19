# WML2 TIFF拡張リサーチ

調査日: 2026-09-19

対象ブランチ: `codex/tiff-interop`
対象: TIFF 6.0、ITU-T T.6、CCITT Group 4 extension、YCbCr、old-style JPEG、外部テストデータ

## 今回の追補

- 仕様照合は [TIFF 6.0 Specification](https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf) と [ITU-T T.6](https://www.itu.int/rec/T-REC-T.6) を使用した。
- Group 4の標準2D extension 7だけを実装した。`fax4.tiff`はビット列上で予約値6/2/1へ到達するため、標準extension 7のfixtureとはみなさず、IBM MMR系を含む未対応拡張として明示的に拒否する。IBM MMRへの暗黙変換やencoder対応は行わない。
- 非JPEG YCbCrは、`YCbCrCoefficients`、`YCbCrSubSampling`、`YCbCrPositioning`、`ReferenceBlackWhite`を検証し、圧縮展開後にData Unit、Predictor 2、chroma upsampling、色変換を行ってRGBA8へ出力する。今回の公開出力は8-bit contiguous 3成分に限定し、native U16 APIは変更しない。
- old-style JPEG（Compression=6）はbaseline sequential・8-bit・3成分YCbCrに限定した。`JPEGInterchangeFormat`または`JPEGQTables`/`JPEGDCTables`/`JPEGACTables`から標準JPEGを再構成し、既存decoderへ渡す。multi-strip、RowsPerStrip省略、tileをfixtureで確認した。
- 外部TIFF本体はGitへ追加しない。manifestに取得元、固定commit、ファイル単位のサイズ、SHA-256、カテゴリ、期待結果、oracle policyを記録し、実サンプルはローカル指定時だけ検証する。CIで使う正常系はRust生成fixtureとする。

### 追補で使用した外部サンプル

`imazen/codec-corpus` の commit `8e10d4d765667c1c49d74413878fc4bfb46dcf8d` を固定し、同リポジトリの `tiff-conformance/SOURCES.md` に記録された各ファイルの出所・ライセンスを参照した。採用した6件は `wml2/tests/fixtures/tiff_interop/manifest.json` に一覧化している。

| サンプル | 用途 | WML2の期待 |
| --- | --- | --- |
| `fax4.tiff` | Group 4 reserved/IBM-compatible extension | 説明付き拒否 |
| `dscf0013.tif` | None、YCbCr、2:1 subsampling | RGBA8 decode |
| `ycbcr-cat.tif` | LZW、YCbCr、subsampling、strip padding | RGBA8 decode |
| `ojpeg_chewey_subsamp21_multi_strip.tiff` | old-style JPEG、interchange、multi-strip | decode |
| `ojpeg_single_strip_no_rowsperstrip.tiff` | old-style JPEG、table form、tag省略 | decode |
| `ojpeg_zackthecat_subsamp22_single_strip.tiff` | old-style JPEG、table form、tile | decode |

ImageMagick/LibTIFF/Pillowは、old-style JPEG、chroma upsampling、予約fax拡張の実装差を含むため、上記の6件では `oracle_policy=expected_only` として診断比較に分離した。Rustの生成fixtureと外部サンプルのWML2 RGBA8/明示エラーが必須検証である。

### 実行対象の境界

Linux/macOS/Windowsのnativeテスト、Windows i686のruntime test、Linux i686とWasmのcompile checkをCIに定義した。Linux i686の実行とWasmブラウザ実行は今回の完了条件に含めない。

## 仕様の確認

- TIFF 6.0仕様の一次資料は [TIFF 6.0 Specification (Final, June 3, 1992)](https://download.osgeo.org/libtiff/doc/TIFF6.pdf)。ヘッダーは`II`/`MM`、Version 42、最初のIFDへの32-bit offsetで構成される。複数ページはIFDのnextポインタで連結され、StripOffsets/StripByteCountsは各stripを独立した圧縮データとして指す。
- TIFF 6.0の [Tile仕様](https://download.osgeo.org/libtiff/doc/TIFF6.pdf) では、tileは個別に圧縮され、右端・下端のpaddingを含めて展開時には同じstoredサイズになる。TileOffsetsは左から右、上から下の順で、PlanarConfiguration=2ではplaneごとのtile列になる。実描画範囲は画像幅・高さで切り詰める必要がある。
- TIFF 6.0のPredictor tag 317は`1`（予測なし）と`2`（horizontal differencing）を定義する。Predictorは圧縮解除後、サンプル値を8/16-bitのサンプル単位で復元する必要がある。行頭で左隣をリセットし、PlanarConfiguration=2では各planeの行を独立して処理する。
- TIFF 6.0のSeparated画像はPhotometricInterpretation=5、CMYKの順序はC/M/Y/K、通常はSamplesPerPixel=4、PlanarConfiguration=1または2である。仕様は印刷機器依存で、RGBとの往復変換に一意の式はない。
- BigTIFFの [LibTIFF BigTIFF Design](https://libtiff.gitlab.io/libtiff/specification/bigtiff.html) はVersion 43、offset size=8、reserved=0、first IFDとnext IFDの64-bit offset、8-byte entry count、20-byte entry、LONG8/SLONG8/IFD8を定義する。Strip/Tile offsetとbyte countにはLONG8を使える。既存のlibtiff実装も内部offsetは64-bitで扱う。
- DeflateはTIFF 6.0本体のBaseline圧縮ではなく拡張である。LibTIFFの [TIFF 6.0 coverage](https://libtiff.gitlab.io/libtiff/specification/coverage.html) はCompression=8のDeflateと32946の旧PKZIP-style Deflateを区別し、両方をDeflate系として記載する。[Java TIFF metadata仕様](https://www.eecs.yorku.ca/teaching/docs/jdk21-api/java.desktop/javax/imageio/metadata/doc-files/tiff_metadata.html) は両値をzlib/Deflateとして説明し、stripまたはtileごとに完全なzlib streamを持つと明記する。したがって全strip連結後の一回inflateではなく、各blockを個別にinflateする。
- DeflateでもPredictor 2を使う根拠は [Adobe PDF Reference 1.3](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/pdfreference1.3.pdf) のLZW/Flate predictor説明にある。同資料はTIFF Predictor 2をFlateにも適用し、各色成分を左隣の同じ成分から予測すると定義する。TIFF 6.0本文が当時LZW中心に記述している点とは区別する。

設計書16.4のCMYK変換は、他の近似案（`255 - min(C + K)`）ではなく、正規化サンプル値`C,M,Y,K ∈ [0,1]`に対する積形式を採用する。

```text
R = (1 - C) * (1 - K)
G = (1 - M) * (1 - K)
B = (1 - Y) * (1 - K)
```

これはDevice CMYKの基本表示近似であり、ICCプロファイルによるCMYK色変換を意味しない。元ICCはメタデータとして保持する。

## 再利用可能なテストスイート

ライセンスと機能一覧を確認できる [imazen/codec-corpus TIFF Conformance](https://github.com/imazen/codec-corpus/tree/main/tiff-conformance) を採用した。取得元コミットは`8e10d4d765667c1c49d74413878fc4bfb46dcf8d`。TIFF部分は154ファイル（valid 145、edge-cases 5、robustness 4、計約12.5 MB）で、配下の[SOURCES.md](https://github.com/imazen/codec-corpus/blob/main/tiff-conformance/SOURCES.md)にファイル単位の出所とライセンスが記録されている。

含まれるライセンスは、image-tiff/image-rsのMIT、libtiffpicのCC0、libtiff test imagesのpermissive libtiff license、Kodakのunrestricted、USDA NAIPと一部ファイルのPublic Domain/CC0である。TIFF全体を単一ライセンスとみなさず、配布時はSOURCES.mdを同梱する。今回の採用範囲はこの一覧で出所と許諾が確認できるファイルに限定した。

配置先は`D:\data\samples\images\tiff`で、既存データを保持したまま次を追加した。

| ディレクトリ | 内容 |
| --- | --- |
| `codec-corpus\valid` | 145件の正常系。Classic/BigTIFF、strip/tile、圧縮、色、bit depth、metadata |
| `codec-corpus\edge-cases` | 5件。multi-page、SubIFD、GeoTIFF等 |
| `codec-corpus\robustness` | 4件。IFD loop、LZW停止系など拒否確認 |
| `bigtiff` | BigTIFF LE/BE、LONG8相当の3件 |
| `deflate` | Deflate stripと64-bit float Deflateの2件 |
| `lzw` / `packbits` | それぞれ圧縮回帰用の3件/2件 |
| `planar` / `tiles` | planar 3件、square/rectangular/oversized tile 5件 |
| `predictor` | 8/16/64-bitおよびCMYK horizontal predictor 4件 |
| `cmyk` | 8-bit/16-bit、contiguous/planarを含む5件 |
| `multipage` | 2 IFDの1件 |
| `orientation-metadata` | EXIF/GPSとXMPの2件 |
| `robustness` | IFD loopとLZW停止系の4件 |

`codec-corpus\SHA256SUMS.txt`に配置したTIFFのSHA-256を保存した。機能ディレクトリはsuite本体のコピーであり、同一ファイルを機能別に複製している。suite本体のREADMEとSOURCESも`codec-corpus`直下に同梱した。

## 既存ツールでの確認

この環境で利用可能なのはImageMagick 7.1.2-21 Q16 x64とPillow 11.1.0。`tiffinfo`/`tiffcp`コマンドとPythonの`tifffile`/`numpy`は利用できない。

ImageMagickでvalid 145件中140件をidentifyできた。失敗した5件は、64-bit整数サンプル2件、ZSTD圧縮1件、old-JPEGの異常ディレクトリ1件、ThunderScan（設計の非目標）1件で、suiteのライセンスやファイル配置の失敗ではない。Pillowは基本的なTIFF確認には使えるが、同suiteの高bit深度、特殊圧縮、planar/floatを多数扱えないため、一次oracleにはしない。WML2のsample converter/metadata実行と外部oracle比較は、実装後のテスト工程で行う。

## 実装への適用メモ

1. offset/count/IFD countは内部でu64として保持し、block読込前に`offset + byte_count <= input.len()`を検証する。
2. StripとTileは`stored_width/stored_height`と`draw_width/draw_height`を分け、圧縮解除はstored、描画はdrawを使う。
3. Predictorはdecompress直後、sample decodeとcolor conversionの前に適用する。16-bitではu16のまま復元し、最後にRGBA8へ量子化する。
4. PlanarConfiguration=2はsample単位でplaneを再構成する。16-bitではbyte単位の並べ替えにしない。
5. TileOffsets/StripOffsetsが同時に存在する場合やoffset数とbyte count数が異なる場合は、曖昧な優先順位で処理せず明示的に拒否する。
6. `Compression=8`と`32946`は同じblock単位のDeflate経路へ接続し、各strip/tileのzlib streamを独立して展開する。
