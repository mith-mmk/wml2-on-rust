# TIFF拡張

対象ブランチは `tiff-extend`。既存の自前LZW/JPEG/CCITTを利用し、TIFFコンテナの解析、画像ブロック、Predictor、サンプル復元を分離する。

## デコード

- Classic TIFF / BigTIFF、Little Endian / Big Endian、複数IFD。
- None / LZW / Deflate（8・32946）/ PackBitsのstrip・tile共通処理。末端tileの格納寸法と描画寸法を区別する。
- JPEG new-style、CCITTは既存デコーダを利用する。
- Gray / RGB / RGBA / Palette / Device CMYK。符号なし整数サンプルと8/16-bit Predictor 2、planar画像を扱う。
- IFDの循環、サイズ計算のオーバーフロー、壊れたブロック配列、入力外オフセット、切り詰められたブロックを拒否する。

内部のoffset/countは `u64`。画像寸法は既存APIに合わせ `u32` のまま検証する。BigTIFFを読めることによって `DecodeLimits` が解除されることはない。4GiB超の疎な入力を読む場合も、呼び出し側が入力上限を明示する必要がある。

公開 `DataPack` の互換性を保つため、BigTIFF固有のLONG8/SLONG8/IFD8は公開メタデータでは元のバイト列として保持する。内部IFDには元の型・要素数があるが、公開メタデータ経由で独自の64-bitタグを再エンコードした場合、その型はUNDEFINEDに変わる。画像オフセット等の構造タグはエンコーダが再生成する。

ClassicとBigTIFFの差は `ifd.rs` で吸収し、`page.rs` でIFDごとのページを生成する。`block.rs` はstrip/tileの位置・格納寸法・描画寸法・planeを保持する。`decoder/compression.rs`、`predictor.rs`、`sample.rs`、`color.rs` が画素処理を分担する。

## 高色深度API

`high-bit-depth` と `tiff` featureを有効にする。従来の `image_load()`、`ImageBuffer`、`DrawCallback` はRGBA8のまま。

```rust
use wml2::highres::{PixelBuffer, tiff};
use wml2::limits::DecodeLimits;

fn read_tiff16(bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let frame = tiff::decode_native(bytes, &DecodeLimits::default())?;
    if let PixelBuffer::U16(planes) = frame.pixels() {
        let samples = planes.as_slice()[0].samples();
        println!("{} native samples", samples.len());
    }
    Ok(())
}
```

`decode_native()` は最初の通常ページ、`decode_native_pages()` は通常ページをすべて返す。縮小画像とマスクを除外し、旧SubfileType=1/3とNewSubfileTypeのページビットを通常ページとして扱う。対象は符号なしGray16、GrayAlpha16、RGB16、RGBA16。Predictorを16-bitで復元した後の値を保持する。WhiteIsZeroのGray値は黒0へ正規化する。ExtraSamplesのassociated alphaは `Premultiplied`、unassociated alphaは `Straight` として保持する。用途未指定の追加チャンネル（ExtraSamples=0またはタグなし）は現在のtypedモデルで意味を表現できないため、native APIでは `NoSupportFormat` を返す。legacy出力ではalphaと解釈せず、不透明として扱う。

ICC、EXIF、TIFFタグとorientationはメタデータに保持する。Orientation 2〜8による自動回転は行わない。Device CMYKの表示は `(1-C)*(1-K)` 等の基本近似で、CMYK ICC変換やnative CMYK16保持は対象外。

RGBA8出力の元ICCは `Source ICC Profile` とTIFFヘッダー内に保持する。`ICC Profile` は現在のRGB画素に適用できるプロファイルだけに使用し、Gray/CMYKの元ICCをRGB出力へ自動転記しない。InkSet=2などCMYK以外の色分解は明示的に未対応とする。

CMYKはExtraSamples=2のunassociated alphaを保持する。CMYKのassociated alpha（1）とPaletteのalpha（1/2）は色モデル固有の処理が未実装のため、ページ解析時に`NoSupportFormat`で拒否する。透明度を捨てて成功させない。用途未指定の追加サンプル（0またはタグなし）は不透明として扱う。

汎用block描画は復号済みバッファとColorMapを借用し、描画のためだけに全体を複製しない。公開のraw描画APIでは、Predictorやplanar復元が必要な場合に限り書き込み用バッファを確保する。`expanded_bytes`は個々の展開データや画像バッファの検査上限であり、入力・出力・一時バッファを合算したプロセスのピークメモリ上限ではない。

従来のRGBA8出力ではRGB/Grayのassociated alphaをstraight alphaへ変換する。alphaが0の画素のRGBは0とする。native出力は関連付け済みサンプルと `Premultiplied` の意味をそのまま保持する。

## エンコード

`EncodeOptions.options` の追加指定:

| キー | 値 | 動作 |
| --- | --- | --- |
| `compression` | `DataMap::Ascii("deflate".into())` | ブロックをzlib/Deflateで圧縮 |
| `predictor` | `DataMap::Ascii("none".into())` / `"horizontal"` | 圧縮前に水平差分を適用 |
| `bigtiff` | `DataMap::UInt(1)` | 明示的にBigTIFFを出力 |

既存のNone/LZW/JPEG出力は引き続き利用できる。Classicの範囲を超えた出力を暗黙にBigTIFFへ変更しない。出力のhorizontal PredictorはLZW/Deflateで使用できる。EXIFの公開serializerはClassic TIFF形式を出力し、BigTIFFのIFD writerとは分ける。

sample converter:

```text
cargo run -p wml2-test --example converter -- input.png -o <temporary-output> -f tiff -c deflate --predictor horizontal --bigtiff
cargo run -p wml2-test --example metadata -- <temporary-output>/input.png.tiff
```

エンコーダへの入力は従来のRGBA8契約。native U16入力のエンコード、tile出力、DNG現像、浮動小数点、Old-style JPEG、LogLuv等は今回の必須範囲に含まない。JPEGを使わないraw YCbCrは従来同様に未対応。

## テストと外部データ

`wml2/tests/tiff_extend.rs` の生成fixtureは外部画像ファイルに依存しない。正常系、異常入力、4GiB超の疑似readerを検証する。既存の `tiff_encode` とEXIF・他形式の回帰も維持する。

外部スイートの出所とライセンスは [調査記録](tiff-extend-research.md) と [samples/README.md](../samples/README.md) に記載している。実行確認にはsampleのconverter/metadataを含める。

`wml2-test/scripts/tiff_oracle.py` はPillowでタグを調べ、converterとmetadataを実行し、ImageMagickによるRGBA8参照値との差分をJSONで返す。JPEGはPillow/libjpegを参照にする。`--converter`、`--metadata`、`--corpus`、`--output-dir` で場所を指定する。実行成功、画素一致、未対応を分けて報告する。

レビュー後の修正、追加画像、32-bitの実行結果は [レビュー修正記録](tiff-extend-review-fixes.md) を参照してください。

### 初期拡張の検証結果（a08bf058、2026-09-12、Windows）

- ライブラリ全体: `cargo test -p wml2 --lib --tests --features high-bit-depth --locked` は169件成功、既存の1件がignored。
- 最小feature構成22件、EXIF単独18件、doc test 12件、converter引数テスト13件成功。Clippyは成功（既存コードを含む警告あり）。Linux/macOS・32-bit・WasmのCIはこのローカル検証では未実行。
- 生成fixtureはClassic/BigTIFFのLE/BE、strip/tileの5圧縮、planar16、Predictorの桁上がり、RGBA/Gray alpha、4GiB超のIFD/strip/tile/次IFD、ICC/EXIF、循環・切り詰め・上限を検証する。
- JPEGの横2tile（tile高が画像高以上）、右端・下端・右下partial、2×2配置を5形状、JPEGTables有無で検証。全画素一致。Palette LE/BEとCCITT RLE/G3/G4も自己完結fixtureで確認する。
- 外部コーパス154件中、validの145件を調べ、整数TIFF対象76件でconverter/metadataを実行。両方成功69件、ImageMagick/Pillowと画素完全一致39件、各チャンネル差1以内63件。
- 外部encoder比較はClassic/BigTIFF × None/LZW/DeflateおよびLZW/Deflate Predictor 2の10通りで全画素・ICC（588 bytes）が完全一致。EXIF Makeとページ数も保持。JPEG出力2通りはstandalone JPEGと最大差6、平均差1.20486。

### 外部コーパスの例外

成功69件のうち、JPEG 3件の差はchroma補間方式が主因。抽出JPEGブロックをImageMagickの `jpeg:fancy-upsampling=off` で復号すると最大差3〜4、平均RGB差0.2075〜0.4228になる。RGB JPEGは最大差1。`strike.tif` の最大差は透明画素のRGBによる（RGBA全体の平均差約0.03）。

`testfax3_bug54_1dnoEOL.tif` と `testfax3_bug_513.tiff` は既存CCITTデコーダで外部実装との差が残る。前者はPillow自身もロードを拒否する。今回のCCITT対応は既存codecの共通ブロック接続と回帰維持であり、これら特殊なfax画像の修正を含まない。

拒否7件は次のとおり。

- `cramps-tile.tif` / `quad-tile.tif`: stripとtile配列の同時指定。曖昧な入力として拒否。
- `deflate-last-strip-extra-data.tiff`: 最終stripの可視寸法を超える展開データ。ブロック展開上限により拒否。
- `dscf0013.tif` / `ycbcr-cat.tif`: JPEG以外のYCbCr。
- `fax4.tiff`: 既存CCITTの2D extension 7は未対応。
- `lzw-single-strip.tiff`: EOI欠落。独立解析で可視画素はPillowと一致するが、54,049コード中EOIは0回で、最後に5bitしか残っていない。[TIFF 6.0 p61](https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf#page=61)に従い不完全なストリームを拒否する。
