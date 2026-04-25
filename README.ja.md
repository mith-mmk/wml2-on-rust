[![crates.io](https://img.shields.io/crates/v/wml2)](https://crates.io/crates/wml2) ![license](https://img.shields.io/crates/l/wml2)
[英語](./README.md)

> [!NOTE]
> WML2Viewerはリポジトリごと移転しました [WML2Viwer](https://github.com/mith-mmk/wml2viewer)


# WML2 - Web graphic Multi format Library To Rust

`wml2` は callback ベースの画像 I/O ライブラリです。

- メモリ上で画像を decode / encode
- 組み込みの RGBA `ImageBuffer` と独自 callback の両方に対応
- 静止画に加えて、一部のアニメーション / マルチページ形式にも対応
- 非 WASM ターゲットではファイル I/O helper も利用可能

## 実装例

`wml2-test/examples` を参照してください。

### BMP へ変換する例

```console
$ cargo run -p wml2-test --example to_bmp --release -- <inputfile> <output_dir>
```

### メタデータ表示の例

```console
$ cargo run -p wml2-test --example metadata --release -- <inputfile>
```

### converter の例

```console
$ cargo run -p wml2-test --example converter -- <inputfiles...> -o <output_dir> [-f gif|png|jpeg|bmp|tiff|webp] [-q <quality>] [-z <0-9>] [-c <none|lzw|lzw_msb|lzw_lsb|jpeg|lossy|lossless>] [--exif copy] [--split]
```

## サポートフォーマット (`0.0.20`)

| フォーマット | enc | dec | 備考 |
| --- | --- | --- | --- |
| BMP | O | O | encoder は無圧縮 BMP を出力 |
| JPEG | O | O | encoder は baseline のみ、decoder は baseline と Huffman progressive に対応 |
| GIF | O | O | パレット/LZW encoder、animation 対応 |
| ICO | x | O | BMP/PNG 内包の icon image を decode |
| PNG | O | O | PNG/APNG 対応、encoder は RGBA truecolor を出力 |
| TIFF | O | O | encode: none/LZW/JPEG(new)、decode: none/LZW/PackBits/JPEG(new)/Adobe Deflate/CCITT Huffman RLE/CCITT Group 3/4 Fax |
| WEBP | O | O | Pure Rust の静止画/アニメーション decoder と静止画/アニメーション encoder、lossless/lossy 出力に対応 |
| MAG | x | O | 日本の旧画像形式。`noretoro` 指定時は無効 |
| MAKI | x | O | 日本の旧画像形式。`noretoro` 指定時は無効 |
| PI | x | O | 日本の旧画像形式。`noretoro` 指定時は無効 |
| PIC | x | O | 日本の旧画像形式。`noretoro` 指定時は無効 |
| VSP/DAT | x | O | 日本の旧画像形式/コンテナ。`noretoro` 指定時は無効 |
| PCD | x | O | Photo CD base4 decode。`noretoro` 指定時は無効 |

## Feature

- `default`: 標準の decoder/encoder、EXIF 対応、埋め込みフォーマット bridge、`idct_llm` を有効化
- フォーマット feature: `bmp`, `gif`, `ico`, `jpeg`, `png`, `tiff`, `webp`, `mag`, `maki`, `pcd`, `pi`, `pic`, `vsp`
- metadata feature: `exif`
- 埋め込みフォーマット bridge feature: `bmp-jpeg`, `bmp-png`, `tiff-jpeg`, `ico-bmp`, `ico-png`
- JPEG IDCT feature: `idct_llm` (default), `idct_aan`, `idct_slower` のいずれか 1 つを選択
- JPEG encoder 用 toggle: `fdct_slower`
- その他の toggle: `multithread`, `SJIS`, `noretoro`
- `noretoro`: これで gate されている旧フォーマット decoder、`MAG`, `MAKI`, `PCD`, `PI`, `PIC`, `VSP/DAT` を無効化

```toml
[dependencies]
wml2 = "0.0.20"
```

```toml
[dependencies]
wml2 = { version = "0.0.20", features = ["noretoro"] }
```

```toml
[dependencies]
wml2 = { version = "0.0.20", default-features = false, features = ["jpeg", "png", "exif", "idct_aan"] }
```

## エンコードと変換オプション

`draw::image_to()` は `ImageBuffer` を直接 `Vec<u8>` に encode します。

`draw::convert()` は出力拡張子から encoder を選択します。
対応拡張子は `.gif`, `.png`, `.apng`, `.jpg`, `.jpeg`, `.bmp`, `.tif`, `.tiff`, `.webp` です。

`EncodeOptions::options` / `draw::convert(..., options)` で使える主なキー:

- JPEG: `quality`
- TIFF: `compression = none|lzw|lzw_msb|lzw_lsb|jpeg`
- TIFF で `compression=jpeg`: `quality`
- WebP: `optimize` (`0..=9`)
- WebP lossy: `quality`
- PNG/JPEG/TIFF/WebP: `exif`
  - `DataMap::Raw` で生 EXIF バイト列
  - `DataMap::Exif` で TIFF 形式の EXIF
  - `DataMap::Ascii("copy".to_string())` で `convert()` 時に入力 EXIF をコピー

`wml2-test/examples/converter` の主なオプション:

- `-q`: JPEG quality、WebP lossy quality、または TIFF JPEG quality
- `-z`: WebP optimize level
- `-c`: TIFF compression または WebP `lossy|lossless`
- `--exif copy`: PNG/JPEG/TIFF/WebP 出力へ入力 EXIF をコピー
- `--split`: GIF/PNG/TIFF/WebP の animation / multi-frame を分割出力

## TIFF 圧縮

encode 対応:

- 無圧縮
- LZW
- JPEG (new-style TIFF JPEG, RGB のみ)

decode 対応:

- 無圧縮
- LZW
- PackBits
- JPEG (new-style TIFF JPEG)
- Adobe Deflate
- CCITT Huffman RLE
- CCITT Group 3 Fax
- CCITT Group 4 Fax

## 基本的な decode

メモリ上の画像をそのまま decode するなら `draw::image_load()`、
ネイティブ環境でファイルから読むなら `draw::image_from_file()` を使います。

```rust
use std::error::Error;
use wml2::draw::{PickCallback, image_from_file};

fn main() -> Result<(), Box<dyn Error>> {
    let mut image = image_from_file("foo.webp".to_string())?;
    println!("{}x{}", image.width, image.height);

    if let Some(metadata) = image.metadata()? {
        if let Some(format) = metadata.get("Format") {
            println!("format: {:?}", format);
        }
    }

    Ok(())
}
```

独自の `DrawCallback` を使いたい場合は `draw::image_loader()` または
`draw::image_reader()` を使います。

`ImageBuffer` は `DrawCallback` を実装しており、decoder から次を受け取れます。

- `init`: canvas 初期化
- `draw`: RGBA 矩形の書き込み
- `next`: animation / multi-image の切り替え情報 (`NextOptions`)
- `terminate`: decode 終了処理
- `verbose`: decoder 固有の debug 出力
- `set_metadata`: decode 済み metadata

## 基本的な encode

`ImageBuffer` を encode するだけなら `draw::image_to()`、
独自の `PickCallback` を使うなら `draw::image_encoder()` /
`draw::image_writer()` を使います。

```rust
use std::collections::HashMap;
use std::error::Error;
use wml2::draw::{ImageBuffer, image_to};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

fn main() -> Result<(), Box<dyn Error>> {
    let mut image = ImageBuffer::from_buffer(1, 1, vec![255, 0, 0, 255]);
    let mut options = HashMap::new();
    options.insert("quality".to_string(), DataMap::UInt(90));

    let jpeg = image_to(&mut image, ImageFormat::Jpeg, Some(options))?;
    assert!(jpeg.starts_with(&[0xff, 0xd8]));
    Ok(())
}
```

`ImageBuffer` は `PickCallback` も実装しており、encoder から次が呼ばれます。

- `encode_start`
- `encode_pick`
- `encode_end`
- `metadata`

## Metadata

metadata は `HashMap<String, DataMap>` で表現されます。

- `metadata::exif` で TIFF 形式の EXIF/GPS タグを parse / serialize / edit できます。
- `c2pa` feature が有効な場合、PNG `caBX` と JPEG APP11 の C2PA manifest store は `"C2PA"` の `DataMap::JSON` と `"C2PA Raw"` の生バイト列として出力します。署名・証明書検証は上位の C2PA validator に任せます。`metadata::c2pa::c2pa_to_text()` は claim generator 名と actions だけを残し、bytes/base64/hash/signature を省いた簡易表示を返します。

```rust
use std::error::Error;
use wml2::draw::{PickCallback, image_from_file};
use wml2::metadata::DataMap;

fn main() -> Result<(), Box<dyn Error>> {
    let mut image = image_from_file("foo.jpg".to_string())?;
    let metadata = image.metadata()?.unwrap_or_default();

    if let Some(DataMap::ICCProfile(profile)) = metadata.get("ICC Profile") {
        println!("ICC profile: {} bytes", profile.len());
    }

    Ok(())
}
```

## テストサンプル

- テストデータは権利上の問題が生じるため、リポジトリには含まれていません。適切なサンプルを用意してください
- 統合テストでは `sample.mki`, `sample.pi`, `sample.pic`, `sample.dat` などの汎用名を使います
- 元の sample 名は公開テストコードでは直接参照しません
- 外部 sample のパスは `wml2/tests/test_samples.txt` に設定できます
- `wml2/tests/test_samples.txt` は `.gitignore` 対象で、雛形は `wml2/tests/test_samples.example.txt` です
- `wm2/tests/error_samples.rs` はエラー処理テスト用のファイルを置いたフォルダを指定した環境変数 `WML2_ERROR_SAMPLES_DIR` を指定していることを前提としています .env ファイルなどで設定してください
- C2PA/EXIF のローカル sample は `WML2_C2PA_EXIF_SAMPLES_DIR` または `.test/c2pa+exif` が存在する場合に読みます。ディレクトリが無い場合、該当テストは skip します

## debug flag

### JPEG

- `0x01`: basic header
- `0x02`: Huffman table
- `0x04`: extracted Huffman table
- `0x08`: quantization table
- `0x10`: EXIF
- `0x20`: ICC profile header
- `0x40`: ICC profile detail
- `0x60`: ICC profile all

## 更新履歴

- `0.0.1`: baseline JPEG
- `0.0.2`: BMP OS/2 / Windows RGB/RLE4/RLE8/bit fields、baseline JPEG
- `0.0.3`: GIF decoder
- `0.0.4`: error message 更新
- `0.0.5`: reader と error propagation の変更
- `0.0.6`: RST marker read bug fix
- `0.0.7`: PNG decoder
- `0.0.8`: pipelined JPEG、BMP saver、animation GIF 周辺
- `0.0.9`: PNG saver
- `0.0.10`: progressive JPEG fix
- `0.0.11`: TIFF Group 3/4 Fax と multipage TIFF decode 改善
- `0.0.12`: encode option 変更
- `0.0.13`: MAG decoder
- `0.0.14`: MAKI/PI/PIC/VSP(DAT)/PCD decoder と `noretoro`
- `0.0.15`: baseline JPEG encoder
- `0.0.16`: Pure Rust WebP decoder と APNG encoder
- `0.0.17`: Pure Rust WebP encoder と animated WebP encode
- `0.0.18`: GIF encoder、TIFF encoder、EXIF writer
- `0.0.19`: ICO decoder、features の整理、JPEG decoder の IDCT アルゴリズムは LL&M が default に変更
- `0.0.20`: boudary checkの強化、パレット付きpngのデコードバグfix
- `0.0.21`: gif decoderのバグフィックス
- `0.0.22`: png decoderのエンバグフィックス
- `0.0.23`: c2pa manifest store の decode を追加

## License
MIT License (C) 2022-2026

## Author

MITH@mmk https://mith-mmk.github.io/
