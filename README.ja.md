Notice: Specification of this library is not decision.

# WML2 - Web graphic Multi format Library To Rust
- 名前はただのこじつけ
- WASMで動作可能 https://github.com/mith-mmk/wasm-paint
- メモリーバッファで動作可能
- callback 関数を使った拡張が可能
- UIはまだない

# 実装例
wml-test/examplesを参考

## BMPリーダーのサンプル
```
$ cargo run --example to_bmp --release <inputfile> <output_dir>
```

## メタデータリーダーのサンプル

```
$ cargo run --example metadata --release <inputfile>
```

# サポートフォーマット 0.0.11

|フォーマット|エンコード|デコード|  |
|------|---|---|--|
|BMP|O|O|エンコーダは無圧縮のみ|
|JPEG|x|O|算術符号には対応していない|
|GIF|x|O|アニメーションGIF対応|
|PNG|O|O|APNG対応|
|TIFF|x|o|無圧縮/LZW/Packbits/Jpeg(new)/3G Fax(1Dのみ)に対応|
|WEBP|x|x|not support|

# 使い方
- バッファ上にあるイメージをロードする
- Callbackを実装することで拡張が可能 (デフォルトで ImageBufferが装備)

```rust
  let image = new ImageBuffer();
  let verbose = 0;
  let mut option = DecodeOptions{
    debug_flag: verbose,  // depended decoder
    drawer: image,
  };

  let r = wml2::jpeg::decoder::decode(data, &mut option);
```

## シンプルデコーダ

```rust
use std::error::Error;
use wml2::draw::ImageBuffer;

pub fn main()-> Result<(),Box<dyn Error>> {
    println!("read");
    let filename = "foo.bmp";
    let mut image = image_from_file(filename.to_string())?;
    let _metadata = image.metadata()?.unwrap();

    Ok(())
}


```
 ImageBufferはDrawCallback トレイトを実装して作成されている。

 - init -> デコーダが最初に呼び出す。関数は引数をみてからバッファを確保することが可能。
 - draw -> デコーダがデータを書き出す時に呼び出す。画像全体とは限らず一部だけ書き出すことが可能
 - verbose -> デバッグ用の詳細情報を返します
 - next -> 複数イメージが存在する場合、次の処理を要求します。
  - アニメーション(GIF/APNG)もしくはマルチイメージフォーマット（Tiffなど）をサポートするときに利用。
 - terminate -> デコーダが終了したときに呼び出され、後処理をおこなう関数
 - set_metadata -> メタデータをセットする(0.0.10移行)

# using saver
```rust
  let image = new ImageBuffer(width,height);

  if let Some(image) = image {
      let option = EncodeOptions {
          debug_flag: 0,
          drawer: &mut image,    
      };
      let data = wml2::bmp::encoder(option);
      if let Ok(data) = data {
          let filename = format!("{}.bmp",filename);
          let f = File::create(&filename).unwrap();
          f.write_all(data).unwrap();
          f.flush().unwrap();
      }
  }
```

 ImageBufferのエンコーダはPickCallbackトレイトを実装している
 
- encode_start エンコーダが開始された時呼び出される
- encode_pick  エンコーダが画像の一部のデータを読み取る時に呼び出れる関数
- encode_end   エンコーダが終了したときに呼び出される関数
- metadata メタデータを要求した時に呼び出される関数。0.0.10移行

# Metadata
 Metadatasは、(Key Value)のHashMapになっている。KeyはString型、ValueはDataMap型で実装されている。.

```rust

// Get ICC Profile
    let filename = "foo.bmp";
    let mut image = image_from_file(filename.to_string())?;
    let metadata = image.metadata()?.unwrap();

    if let Some(icc_profile_data) = metadata.get(&"ICC Profile") {
      if let DataMap::ICCProfile(icc_profile) = icc_profile_data {
        // Read ICC Profile
      }
    }

  

```

# debug flag
## JPEG
-  0x01 basic header
-  0x02 with Huffman Table
-  0x04 with Extract Huffman Table 
-  0x08 with Define Quatization Table
-  0x10 with Exif
-  0x20 with IIC Profile(header)
-  0x40 with IIC Profile(more infomation)
-  0x60 with IIC Profile(all)
-  0x80 ...
# update
- 0.0.1 baseline jpeg
- 0.0.2 bmp OS2/Windows RGB/RLE4/RLE8/bit fields/baseline JPEG
- 0.0.3 add GIF
- 0.0.4 change error message
- 0.0.5 reader change/Error delagation change
- 0.0.6 issue RST maker read bug fix
- 0.0.7 add PNG
- 0.0.8 add Jpeg multithread(pipelined),Progressive Jpeg has bugs(4,1,1) / BMP saver / Animation GIF(alpha)
- 0.0.9 Png Saver
- 0.0.10 Progressive JPEG YUV=4,1,1 fix
- 0.0.11 
  - ICCProfileパーサの除去 -> see https://github.com/mith-mmk/icc_profile に移行
 - TIFF 3G FAX(1Dのみ)サポート

# todo
- Formated Header writer
- other decoder
- color translation
- jpeg encoder

#　License
 MIT License (C) 2022

# Author
 MITH@mmk https://mith-mmk.github.io/