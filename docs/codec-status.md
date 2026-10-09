# TGA / PCX / DDS 対応状況

EXは`feat/image-buffer-ex-compat`に分離し、このcodec branchには含めません。`jpeg2000`は同じmainから独立したbranchで、backendは未選択です。

対象は `codex/p1-retro-limits` の通常の RGBA8 callback 経路です。既存の公開 callback、options の field、既定 feature は変更していません。以下の形式は feature を有効にした場合だけ利用でき、`noretoro` では無効になります。

| 形式 | 通常の decode / encode | 通常の入力で残る未対応・解釈の制約 | 次段階の追加機能 |
|---|---|---|---|
| TGA | raw/RLE、indexed/true-color/gray、origin・旧 interleave を decode。true-color 24/32、gray 8/16、exact indexed を encode。TGA 2.0 alpha 0/1/3/4 と payload 境界を処理 | 属性2（未定義値を保持する指定）・未知属性を明示拒否。色補正・gamma等の extension 内容は描画へ適用しない。extension なしは既存の alpha 解釈を維持 | その他 metadata、15/16-bit encode、16-bit index encode、旧 Huffman type32/33 |
| PCX | RLE と低bit palette・8-bit RGB/RGBAを decode。旧 EGA/CGA を下記の規則で解釈。version5 RGB / exact indexed を encode | CGA C=1 monochrome を明示拒否。PaletteInfo=2 による特別な grayscale 解釈は行わない。1-bit単一planeは既存の白0/黒1。旧画面装置の判別はできない | 低bit / grayscale encode、DCX（別の複数page container） |
| DDS | legacy mask / DX10 の対応済み8-bit形式、BC1–5・BC7全modeを decode。単一2Dの先頭mipのみ。RGBA8をlegacy BGRA8へ encode。DX10 alpha-modeを処理 | BC6H/HDR、その他の数値・packed DXGI形式、BC7 TYPELESS は未対応。custom第四channel・未知alpha-modeを明示拒否 | 圧縮encode、mipmap選択・生成、array/cube/volume等のtexture API |

## 解釈を固定した箇所

- TGA属性0/1は格納alphaを無視してRGBを保持、3はstraight、4はpremultipliedからstraightへ丸め・飽和付きで変換します。alpha=0の有効alpha画素はRGB=0にします。仕様の属性表は4をpremultipliedとしていますが、続く例の文章は3と記述しています。このdecoderは表を採用します。属性2の未使用値保持はstraight RGBA8では表現できません。[Truevision TGA 2.0](https://raw.githubusercontent.com/DEAKSoftware/Truevision-TGA/master/truevision-tga.pdf)
- PCX version0/3の低bit色画像はEGA既定palette、version0/2/3のpacked 2bit×1planeはCGA background/P/I、version4/5はheader RGB paletteとする規則です。version/layoutだけでは元の装置を一意に識別できません。C=1についてはRGBを推測しません。[ZSoft manual](https://msxcomputermagazine.nl/mccm/millennium/milc/formats/topic_0.htm)
- DX10 alpha-mode0は既存のstraight仮定、1はstraight、2はpremultipliedをstraightへ変換、3はalpha=255にします。4のcustom第四channelと予約flagsは拒否します。sRGB形式も格納channel値を維持し、暗黙のtransfer変換は行いません。[Microsoft DDS_HEADER_DXT10](https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-header-dxt10)

## BC6H / HDR と EX

現在のEXはGray/RGBAの整数8/12/16bitで、HDR浮動小数点値、1を超える値や負値の保存契約ではありません。BC6Hにはsigned/unsignedのdecodeと、浮動小数点値を保つ受け渡し、または明示的なHDR→表示変換方針が必要です。既存`highres::PixelBuffer::F32`を再利用する余地はありますが、整数RGBA16のEXを接続するだけではHDR保存を満たしません。EX/HDR方針が保留の間、BC6Hを暗黙にRGBA8へ縮小する実装は追加していません。

## 制限と検証範囲

`input_bytes`は通常入口とretroの直接decodeで入力確保前に検査します。pixel/output制限、PCX scanline、各encoderのheader/RLE/paletteを含む出力量も検査します。`expanded_bytes`は個々の展開buffer・出力サイズの制限で、同時に存在する全bufferやallocator overheadを含む合計peak heap/RSS上限ではありません。

BC7は独立Pillow fixtureで全mode・partition・anchor・p-bitを比較しました。TGA属性0/1/3と現行PCX RGB/indexedは外部decoderとの全画素一致を確認しました。TGA属性4と旧EGA/CGAは仕様vectorと実際のWML表示で確認しています。ImageMagickはこれらの解釈に対応しないため、その不一致を外部oracle成功とは扱っていません。JPEG2000の依存やbackendは追加していません。
