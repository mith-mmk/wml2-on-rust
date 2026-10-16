# AVIF / avifenc と ImageBufferEx

AVIF の WML2 adapter を EX の入力宣言・保存・明示出力要求へ接続する。
既存 `image_decoder` / `image_encoder`、native highres API、依存 crate の
公開 API と gitlink は変更しない。新 crate、wmlcore 分離、paintcore 統合は行わない。

## 契約と対応範囲

| 経路 | 入力／出力の契約 |
| --- | --- |
| 従来 `image_decoder` | packed RGBA8 を通常 init→draw。既存 AVIF の色処理・animation 挙動を維持 |
| `image_decoder_ex` / `image_from_ex`＋avif | bounded native still → codec の色再構成 → **RGBA16LE** を init_ex で宣言。ICC を適用しない |
| `image_encoder_ex`＋avifenc | `EncodeOptionsEx.output` を `PickCallbackEx` に明示要求。Gray/RGBA8 または12を符号化 |
| RGBA16 保存からの encode | 保存は16bitのまま、出力を8/12bitと Exact/Round で選択。16bit AV1出力は拒否 |
| 既存 highres native API | YUV、subsampling、10bit、明示 RightShift 等の既存契約を維持。EX の正規化 Round と同一視しない |

EX バッファの形式は Gray/RGBA8/12/16、unsigned interleaved、straight alpha。
12/16bit は LE の2bytes/sample、12bit の上位4bitはゼロ。stride は各矩形の byte
単位で、最終行 padding も含む。AVIF decode の source_format は RGBA16 で、
元ファイルの coded depth や色空間そのものではない。`AVIF Source Bit Depth` は
元の8/10/12bitを別途伝える。

YUV→RGB、range、chroma の再構成は依存 codec の責務。EX 共通バッファに
YUV plane や AVIF 型を追加しない。decode は codec が生成する正規化 RGBA16 を
そのまま宣言し、storage への変換は Callback が行う。10bit の EX enum は作らない。
YUV→RGB の丸め・clamp と、EX の入力→保存の精度変換を区別する。

固定依存 avifenc-rust0.0.7 の writer は alpha の auxl を primary1→alpha2 と書く。
そのままの出力では Pillow/libavif が alpha を認識せず、透明度が失われることを
外部照合で確認した。WML2 の3つの encode 入口（従来／EX／highres native）は
共通 helper で、この生成済みコンテナの参照だけを alpha2→primary1 に正規化する。
[libavif の writer](https://github.com/AOMediaCodec/libavif/blob/main/src/write.c)も alpha
item から color item への参照を生成している。box 境界と既知の producer schema を
検証し、ICC／AV1 payload や item offset は変更しない。既に正しい参照はそのまま。
依存 crate・gitlink・旧 API の引数や画素変換方針は変更しない。

12bit identity RGB を EX の RGBA12 に戻す場合も、入力宣言は正規化 RGBA16。
非整数スケーリングを許可する `Round` が必要になる場合がある。`Exact` は
`s * destination_max % source_max == 0` だけを受理し、alpha も同じ式で判定する。
EX encode の `lossless` は codec の量子化方針であり、EX で選択した減精度を
取り消さない。RGBA12→RGBA12 pick は無精度変換で、12bit sample のまま backend へ渡る。

EX decode は依存の strict native still profile（通常 av01 と対応する bounded
Sample Transform）に限定する。AVIS、grid、geometry transform、premultiplied alpha、
HDR 等の display-transfer 変換が必要な入力は明示拒否。従来入口の対応範囲を
この制限へ縮小しない。F32、signed、YUV を EX のバイト列と誤解釈しない。

## 色 metadata と責務

EX decode は通常の `DecodedFrame::to_rgba16` をそのまま呼ばない。この helper は
ICC／一部 display transfer を適用するためである。EX adapter は codec の
再構成 primitive を、表示変換を伴わない transfer に限定して呼ぶ。
対応 transfer は absent または1/2/6/7/11/12/13/14/15。

順序は **init_ex → draw → metadata → terminate**。元 ICC は
`Source ICC Profile`、AV1 の signaling は `Source AV1 CICP` / `Source AV1 Full Range`、
container は `Source AVIF nclx` で保持する。metadata が最初の draw 前に届く保証はない。
[既存の収集 Callback](image-buffer-ex-callbacks.md)のように、成功後に外部色管理へ渡す。

EX encode は RGB を codec の identity GBR plane 順へ並べ替えるだけで、
ICC や RGB→YUV を共通バッファに実装しない。現 backend の identity RGB に
必要な nclx は BT.709/sRGB、matrix0、full range（Gray は matrix1）。この
signaling と入力 sample の色意味が整合するように用意するのは caller の責務。
`ICC Profile: DataMap::ICCProfile` だけを**明示的な出力 profile**として opaque に
signal する。`Source ICC Profile` を自動転記・改名・適用しない。custom CICP や
YUV の native plane を指定する用途は既存 native API を使う。

## API と例

```rust
# use wml2::draw::*;
# use wml2::util::ImageFormat;
# fn example(image: &mut ImageBufferEx) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
image_encoder_ex(&mut EncodeOptionsEx {
    debug_flag: 0,
    drawer: image,
    output: PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Round),
    options: None,
}, ImageFormat::Avif)
# }
```

[ex_avif.rs](../wml2/examples/ex_avif.rs) は RGBA16 storage から12bit Exact/lossless を
明示し、全 sample の roundtrip と非表現可能値の Exact 拒否／Round 成功を検証する。

```sh
cargo run -p wml2 --no-default-features --features avifenc,image-buffer-ex --example ex_avif -- .test-output.avif
cargo test -p wml2 --no-default-features --features avif,image-buffer-ex --test avif_ex
cargo test -p wml2 --no-default-features --features avifenc,image-buffer-ex,high-bit-depth --lib --tests
```

`EncodeOptionsEx` は既存 `EncodeOptions` と別型。既存 encoder は従来どおり
packed RGBA8 の PickCallback を使う。現時点の `image_encoder_ex` は AVIF のみで、
PNG/TIFF 等は明示的な legacy adapter を使う。入力 format の能力交渉は行わない。

## 失敗と資源制限

Callback の precision/pick エラーはそのまま返し、成功した storage を変更しない。
要求と異なる format／寸法／stride の picker 結果を拒否し、8/12bit の範囲も再検証。
encode_start_ex が profile を返した後は、pick／codec 失敗でも encode_end_ex を1回呼ぶ。
未対応 output16 は start 前に拒否する。init／draw の atomicity は EX の既存契約を維持。
Abort の後は残りの draw／metadata を転送せず、terminate は1回。

EX decode は入力・pixel・frame・metadata と native plane の limit を codec に渡す。
幅／高さの profile 上限は16,384、items128、properties512、derived depth16。
native plane と RGBA16 の同時保持、RGBA16→LE copy の workspace を明示予算で確認する。
encode も requested bytes＋native sample planes を pick 前に確認する。
`image_encoder_ex_with_limits` と既存 decode-with-limits を使える。
依存 native limits が未保証の deep entropy scratch まで含めた総 RSS 上限とは主張しない。

## 分離の検討メモ

AVIF 型は adapter 内に留め、ImageBufferEx の core 実装に依存を追加しない。
将来分離を検討する場合、既存の draw callback 型、limits、metadata の DataMap
（Exif では TIFF 型）、optional highres 型との結合が整理対象になる。
今回それらの分離・移動は実装しない。

## 検証結果（2026-10-16）

Rust/Cargo1.91.0、Linux x86_64、WML2 0.0.36 で実行した。
[検証記録](image-buffer-ex-avif-validation.json)にコマンド、source／log の hash、
依存 source の照合、外部照合、WASM、未実行範囲を保存する。

| 実行範囲 | harness の集計（passed / failed / ignored） |
| --- | --- |
| workspace、既定 codec＋AVIF encoder／EX／high-bit-depth／JPEG2000／TGA／PCX／DDS／Q4 | 510 / 0 / 14 |
| no-default AVIF encoder＋EX＋high-bit-depth、lib／tests | 112 / 0 / 1 |
| 同構成の重点再実行（lib、EX、旧 encode、native highres） | 50 / 0 / 1 |
| no-default 旧 AVIF encoder＋high-bit-depth、EX なし | 61 / 0 / 1 |
| multithread、EX と既存 native highres | 17 / 0 / 0 |
| no-default AVIF decoder＋EX | 73 / 0 / 0 |
| no-default EX のみ／feature off | 78 / 0 / 0、40 / 0 / 0 |

**新規 EX の12テストと alpha-reference helper の2単体テストは実行済み。**
表は Rust test harness の報告値で、外部素材不足による条件付き early return も
passed に含む。外部 AVIS3素材がないため、animation の3テストと PNG 有効時の
APNG 化1テストは実処理を検証していない。workspace でも外部 error corpus、
C2PA、任意の legacy/WebP/GIF sample などは不足分をスキップしている。
14 ignored には手動 benchmark／表示 artifact／外部 corpus 用の既存テストを含む。
したがって全素材・全 codec の510ケースを独立照合したという意味ではない。

`filter-disabled-gbr.avif` は初回不足したが、PowerShell がないため依存の
`generate_filter_disabled_oracle.ps1` と同じ FFmpeg recipe で生成して再実行した。
必要な WML2 fixture のみを用意し、依存 avif-rust 自体の全 oracle bootstrap／
全 standalone test corpus は実行していない。

RGBA16 sample から生成した2×2 lossless12bit AVIF は、Pillow12.3.0 の RGBA8
全成分と、FFmpeg7.1.5 の `gbrp12le` 色3plane／`gray12le` alpha plane に完全一致した。
透明 pixel の RGB も比較した。[900×900 の EX preview](image-buffer-ex-avif-display.png)
と [alpha 照合 preview](image-buffer-ex-avif-alpha.png) は目視でも可読性を確認した。
ICC や HDR の表示色精度を検証したという意味ではない。

### WASM の実行手順

既存 EX smoke 手順の `cargo build → wasm32 → Node` を使う。
基盤の [既存 helper](../wml2-test/scripts/verify_image_buffer_ex_wasm.py) は82ケース成功。
追加の [AVIF helper](../wml2-test/scripts/verify_avif_ex_wasm.py) は4ケース成功した。
後者はRGBA12の全 sample、透明RGB／alpha、16bit storage の Exact／Round、
旧 RGBA8 encode/decode を実行する。Linux の Node24.19.0 での結果であり、
ブラウザ実機や WASM multithread の結果ではない。

AVIF encoder の `rav1e → v_frame → wasm-bindgen` は実際の binding を必要とする。
raw WASM を空 import で instantiate する初回手順は失敗したが、同じ crate version の
公式 CLI で Node 用 binding を生成して解決した。dummy import は使わない。
製品の Cargo manifest や feature は変更せず、検証 helper が `.test*` 内だけに
一時 Cargo package と binding を生成する。これは wmlcore 等の新製品 crate ではない。

```sh
# Rust wasm32 target、Node、および生成された Cargo.lock に対応する CLI が必要。
# 今回の lock は wasm-bindgen0.2.126。
cargo install wasm-bindgen-cli --version 0.2.126 --locked --root .test-wasm-tools
python3 -B wml2-test/scripts/verify_image_buffer_ex_wasm.py --offline --output .test-ex-wasm
python3 -B wml2-test/scripts/verify_avif_ex_wasm.py --offline --wasm-bindgen .test-wasm-tools/bin/wasm-bindgen --output .test-avif-ex-wasm
```

### 性能と未実行範囲

[性能記録](image-buffer-ex-avif-benchmark.json)は基準 `f93e334` と同一 dependency lock／
同一 `review_bench.rs`、release、単一 CPU 固定で比較した。既存 decode3入力の中央値は
約+0.8〜1.0%、allocation 数・総割当 bytes・peak live bytes と画素 hash は同一だった。
alpha付き2×2旧 encoder の初回中央値は約+3.3%。順序を反転し各 process1000回へ
増やした追試では約+0.4%で、どちらも割当量は同一だった。初回値も記録に残す。
encoded hash の変化は alpha 参照2byte の修正によるもの。局所的な小入力の計測であり、
全サイズ・全 CPU の性能を保証しない。新規 EX decode は基準側に対応経路がないため
絶対値のみを記録し、速度向上の比較には使わない。

Clippy、fmt、CI YAML／sample 条件をローカルで検査した。既存 Clippy warning は残る。
Windows／macOS、stable toolchain、追加した CI matrix の実行、外部 AVIS の本体比較、
依存 standalone 全 corpus は未実行。CI の終了待ちや継続監視は行っていない。
AVIS／grid／geometry／premultiplied alpha／表示変換が必要な transfer の EX 対応、
AV1 の16bit出力、wmlcore 分離／paintcore 統合は本変更に含めない。
