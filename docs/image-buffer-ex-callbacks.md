# ImageBufferEx Callback 実装と実行サンプル

入力宣言、内部保存、エンコーダへの出力要求を分けて扱う。
ICC 適用と色変換は利用者の Callback／後処理の責務であり、EX の
Exact／Round は整数精度変換だけを行う。この文書は EX ブランチの
`9ec7f072` の実装を確認した現行仕様で、末尾の「将来提案」は未実装。
WML2 0.0.35 の既存 API とランタイムを変更しない。

## 入力・保存・出力の三つの契約

| 段階 | 決定する側 | 現行の意味 |
| --- | --- | --- |
| `init(w,h,legacy)` | デコーダ／画素を送る側 | 以後の `draw` 入力は packed RGBA8 |
| `init_ex(w,h,Some(InitOptionsEx))` | デコーダ／画素を送る側 | `source_format` と `source_stride` で以後の入力バイトを宣言 |
| `with_storage(format,policy)` | Callback 実装／利用者 | 内部保存形式と入力→保存の精度変換方針を選択 |
| `encode_start_ex` | 保存側 | 保存済み canvas の形式・stride を報告。エンコーダ能力の交渉ではない |
| `encode_pick_ex(...,PixelOutputEx,...)` | 出力を消費する側 | エンコーダ等が受け付ける形式を確認し、出力形式・stride・精度変換を明示 |

`init_ex` は「デコーダに好きな出力形式を要求する」関数ではない。
Callback は宣言を受理するかエラーで拒否する。受理後、送る側は同じ
宣言で各 draw のバイトを渡し、Callback はそれを解釈する。入力形式を
変える場合は、新たに初期化する。`None` の InitOptionsEx は packed RGBA8。
通常の `init` も、保存形式を RGBA16 に選んだからといって入力まで
RGBA16 にはならない。

`source_format()` は Callback が受け取った画素バイトの宣言で、元ファイルの
色空間・ICC・encoded color type そのものではない。PNG の RGB／palette／
GrayAlpha はデコーダ内で RGBA に展開され、packed Gray は Gray8 に展開
されることがある。EX の PNG／TIFF デコードは現在 unsigned Gray8/16・
RGBA8/16 の静止画が対象。RGBA12 の Callback 契約は利用できるが、
「PNG／TIFF が RGBA12 を直接宣言する」機能は追加しない。

この責任分離をカスタム Callback にも適用する。内蔵 ImageBufferEx の
変換・検証・失敗時の性質が、任意の Callback に自動実装されるわけではない。

## バイト配置と精度

Gray/RGBA は interleaved。RGBA は **straight alpha**。8bit は1byte/sample、
12/16bit は **little-endian の2bytes/sample** で、12bit は下位12bitだけを
使用し、上位4bitはゼロでなければならない。12bit packed ビット列ではない。
alignment を仮定しないため、奇数の byte stride も許される。

`source_stride` の単位は **byte** で、canvas 全体ではなく各 draw 矩形に適用
される。`None` は `矩形幅 × bytes_per_pixel`。明示値はその矩形の1行以上
必要で、最終行を含む全行の padding 込みで `data.len() == stride × 高さ`。
padding は sample として解釈しない。tile 用 stride は canvas 全幅の行より
短くてもよいが、その stride で収まらない幅の draw は拒否する。保存側の
`stride()` と入力側の `source_stride()` は別である。pick の出力 stride も
byte 単位で、最終行の padding を含み、出力 padding はゼロになる。

source sample を `s`、source 最大値を `Ms = 2^source_bits−1`、保存／出力最大値を
`Md = 2^destination_bits−1` とすると、正規化式は `s × Md / Ms`。

- **Exact**：`(s × Md) mod Ms == 0` のときだけ整数として受理。単なる bit shift
  ではなく、bit 数を増やす場合も拒否しうる。
- **Round**：最も近い整数に丸め、同距離なら上側。実装式は整数演算で
  `floor((s × Md + floor(Ms/2)) / Ms)`。
- 同形式の RGBA12→RGBA12 では精度変換が不要で、sample をそのまま保持する。
  12bit 上位ゼロや長さの検証は引き続き行う。

式は **alpha にも適用**。透明画素の RGB を消したり、premultiply、背景との
合成、gamma/ICC 変換をしたりしない。Gray→RGBA は Gray を RGB に複製し
opaque alpha を補う。RGBA→Gray は明示的な色変換が必要なため拒否する。
legacy 背景色は引き続き RGBA8 宣言で、保存時に同じ精度方針で変換される。

RGBA16→RGBA12 の Exact は source が `4369×k` の場合に `273×k` として成功
する（`k=0..15`、各色と alpha を個別に判定）。たとえば4369→273、
21845→1365、65535→4095。一方1は Exact で拒否される。32768 は Round で
2048になる。Round が成功しても、不正な寸法・stride・長さ・sample・
リソース制限まで免除されるわけではない。

## 動くサンプル

[ex_callback_contracts.rs](../wml2/examples/ex_callback_contracts.rs) は次の5ケースを
一つのプログラムにまとめ、結果を assert する。

1. 通常 `init` の RGBA8 入力→RGBA16 保存。1→257など、全正規化範囲を保持。
2. `init_ex` で RGBA16 入力→RGBA12 保存。Exact 成功、最後の alpha での
   Exact 失敗、Round 成功を示す。
3. RGBA12→RGBA12 を保持し、`PickCallbackEx` で同形式の Exact 出力と
   packed RGBA8 の Round 出力を明示要求。元 canvas は保持。
4. `BufferedIccCallback` が画素と metadata を収集し、成功確認後に外部処理へ
   所有権を渡す。例の ICC marker は有効な profile ではなく、ICC エンジンも
   実装しない。実際の色管理を挿す位置をコメントで示す。
5. `init_ex` を override しない旧 Callback が RGBA16 を拒否し、packed
   RGBA8 は受け付ける。

```sh
# codec 不要。Callback の5ケースを実行する
cargo run -p wml2 --offline --no-default-features --features image-buffer-ex --example ex_callback_contracts

# 現行 PNG エンコーダへの明示 RGBA8 adapter と decode round trip も実行
cargo run -p wml2 --offline --no-default-features --features image-buffer-ex,png,tiff --example ex_callback_contracts

# 任意の対応 PNG/TIFF：デコード結果・警告を確認してから外部処理位置へ進む
cargo run -p wml2 --offline --no-default-features --features image-buffer-ex,png,tiff --example ex_callback_contracts -- wml2/tests/fixtures/ex-access-rgba16.tif
```

現行 `image_encoder` の codec は legacy `PickCallback` の packed RGBA8 を
読む。`PickCallbackEx` を直接受け取る高精度 PNG/TIFF encoder や、能力を
自動照会する API は未実装。サンプルは `png` feature がある場合だけ
`legacy_picker(Round)` を渡し、現行 RGBA8 PNG エンコードを実行する。
これをネイティブ RGBA12／16 PNG エンコードと解釈しない。
別の EX 対応 consumer を作る場合は、その consumer の入力契約に合わせて
`PixelOutputEx` を指定し、返却された `PixelBytesEx` の実レイアウトを使う。
この段階の精度変換方針は入力→保存時の方針とは独立している。

AVIF には追加の [EX adapter](image-buffer-ex-avif.md) がある。
`image_encoder_ex(EncodeOptionsEx)` は8/12bitを明示要求し、元の storage を保持する。
AVIF の EX decode も ICC は draw 後の metadata として渡す。上記の高精度
PNG/TIFF encoder と能力交渉 API は引き続き未実装である。

## ICC と metadata の配信順

現在の EX 対応 codec を実コードで確認すると、正常終了時は次の順序になる。
これは現行実装の記録であり、全 codec 共通の「draw 前 metadata 完了」保証
ではない。metadata map 内の key 間順序にも依存しない。

| 現行経路 | 実際の Callback 配信 |
| --- | --- |
| PNG EX | bKGD があれば `PNG Background` → `init_ex` → pixel draw → 最後に metadata map（`Source ICC Profile` を含む）→ `terminate` |
| TIFF EX | `init_ex` → 各行 draw → Format／Orientation／Tiff headers／任意の `Source ICC Profile` → `terminate` |
| AVIF EX | `init_ex`（RGBA16LE）→ 全矩形 draw → Format／source depth／CICP／任意の `Source ICC Profile` → `terminate` |

根拠は基準 commit `9ec7f072` の
[PNG 初期化](https://github.com/mith-mmk/wml2-on-rust/blob/9ec7f072c300edb0e318a169f466bdd6a17f5a7e/wml2/src/png/decoder.rs#L518)、
[PNG 最終 metadata](https://github.com/mith-mmk/wml2-on-rust/blob/9ec7f072c300edb0e318a169f466bdd6a17f5a7e/wml2/src/png/decoder.rs#L803)、
[TIFF draw と metadata](https://github.com/mith-mmk/wml2-on-rust/blob/9ec7f072c300edb0e318a169f466bdd6a17f5a7e/wml2/src/tiff/decoder/ex.rs#L113)。
iCCP が PNG の IDAT より前にあること、TIFF IFD に ICC があることと、
Callback へいつ配信されるかは別である。追加テストは実際の public decode
で両形式の `init → draw → ICC → terminate` を観測する。後からより早い
配信を保証する実装へ移す場合は、この観測テストも意図的に更新する。

AVIF の追加経路は [EX adapter](../wml2/src/avif/ex.rs) と
[AVIF EX テスト](../wml2/tests/avif_ex.rs) で順序を確認する。

ICC は `DataMap::ICCProfile(Vec<u8>)`。EX の PNG／TIFF／AVIF は
`"Source ICC Profile"` というキーで元の profile を渡す。一般の
`"ICC Profile"` を変換後出力の profile として使う場合とは区別する。
EX 精度変換は profile を解析・検証・適用しない。ICC がないときに sRGB
とみなすか、色管理なしにするか、拒否するかも利用者が選ぶ。

**最初の draw から profile が必要な逐次色変換は、このままでは成立しない。**
サンプルの成立する手順は、1画像につき新しい wrapper を作り、入力精度を
失わない canvas と metadata を収集し、decode 関数の成功・警告方針を確認して
から一括で外部色管理へ渡すことである。init 前の metadata も保持する。
サンプルは全 metadata を別 map に保持し、出力 encoder へ自動転記しない。

PNG は末尾の読み込み失敗時に `terminate` して警告付きで早期 return する
経路があり、最終 metadata 配信に達しないことがある
（[decoder.rs:785–795](https://github.com/mith-mmk/wml2-on-rust/blob/9ec7f072c300edb0e318a169f466bdd6a17f5a7e/wml2/src/png/decoder.rs#L785)）。
したがって `terminate` だけで成功・metadata 完了とは判断しない。
例の `after_decode(actual_result, stage)` は Err、警告あり、未終了を拒否する。
この wrapper 自身は Abort を返さない。任意の Callback が Abort した場合は
guard が正常結果として返す経路もあるため、自分の中止状態を別に追跡する。
一般化された metadata 完了通知は現行 API にはない。

外部 CMM を挿す場合、入力 layout と profile の色モデル、ターゲット profile、
rendering intent、精度と alpha 方針を確認する。Gray を RGBA に複製しただけ
の canvas に、Gray ICC を RGB profile として適用してはいけない。必要なら
Gray 保存を選ぶか、宣言に応じて native layout を保持する独自 Callback を
用意する。変換後の RGB と straight alpha を明示出力し、出力 metadata は
ターゲット profile に対応させる。元 ICC をそのまま出力 ICC と改名しない。

この例は全 canvas と metadata を保持する。大画像では source／storage と
一時変換分の制限を適切に選び、独自 Callback の追加バッファにも独自の
予算を設ける。複数画像で wrapper を使い回して metadata を混在させない。
既存 highres 色管理 API は別系統で、Callback への暗黙接続ではない。

## 失敗時の性質

ImageBufferEx の形式変換を伴う `draw` は矩形全体を一時変換し、成功後に
canvas へ copy する。同形式の場合も全入力 sample の検証を済ませてから
copy する。したがって **1回の draw の画素更新は atomic**。途中まで検証が
進んだ後、最後の色や alpha が失敗しても、その draw の変更は残らない。
以前成功した draw は残る。canvas 外に clip される入力部分も検証される。
これは全 decode の rollback や任意のカスタム Callback の atomicity の
保証ではない。例の後処理は decode が成功するまで部分画像を公開しない。

一方、ImageBufferEx の `init`／`init_ex` が内部 initialize に入ると、最初に
古い buffer を無効化する。**初期化失敗で以前の canvas を保持する契約では
ない**。`bytes()` と `source_format()` は None になり、draw／pick は再初期化
まで失敗する。寸法・stride・metadata 等がすべてゼロ／空に戻るとは約束しない。
decoder guard が Callback 呼出しより前に拒否した場合は、この initialize
自体に到達せず、既存 Callback 状態が残る場合もある。

旧 Callback の既定 `init_ex` は別で、非 RGBA8 または明示 stride を
**旧 init を呼ぶ前に拒否**する。既存の旧 buffer はその拒否では再初期化
されない。この違いも契約テストで確認する。

## 将来提案（未実装・今回変更しない）

draw 前に ICC が必須な用途には、全 codec の既存 `set_metadata` 順序を
暗黙変更するより、色関連 metadata の準備完了を示す opt-in 通知または
明示的な事前解析 API を別途設計する余地がある。対象となる metadata、
missing/invalid profile、正常終了／警告／Abort をどう表すかを決める必要がある。
既存 Callback の必須メソッド追加や `init_ex` の出力交渉への転用は行わない。
この用途では現行の「蓄積して成功後に処理」が使えるため、ランタイム変更は不要。

## 検証

[契約テスト](../wml2/tests/image_buffer_ex_callback_contracts.rs) はサンプルそのものを
コンパイル・実行し、RGBA16→12 の遅延失敗、各色／alpha、clip、byte stride、
初期化失敗、旧 Callback、metadata の前後配信と後処理の中止を追加確認する。
PNG/TIFF が有効なら実デコーダの ICC 配信順も検証する。

```sh
cargo test -p wml2 --offline --no-default-features --features image-buffer-ex,png,tiff,high-bit-depth --test image_buffer_ex_callback_contracts
```

実行結果と未実行範囲は [今回の検証記録](image-buffer-ex-callback-validation.json)
に保存する。以前の TIFF 回帰や WASM 結果はその時点の履歴であり、今回の
再実行結果と混同しない。

Rust/Cargo 1.91.0、専用 target directory で実行した結果：

| 今回の確認 | 結果 |
| --- | --- |
| サンプル：最小 EX／PNG・TIFF 有効＋実 TIFF 読み込み | 両方実行成功。5ケース成功、ICC 色変換は実施しない |
| 新規契約テスト：EX＋PNG＋TIFF＋high-bit-depth | 9成功、0失敗 |
| PNG-only／TIFF-only の新規契約テスト | 各8成功、0失敗 |
| release workspace：EX＋high-bit-depth＋optional retro codecs | 417成功、0失敗、既存 ignored 13（51 targets） |
| 最小 EX／feature-off | 77成功／39成功、0失敗 |
| fmt・diff・サンプル／新規テストの Clippy | 成功。Clippy は既存 library 警告あり、新規2ファイルの診断なし |

今回、ランタイム・decoder・encoder は変更していないため、画像表示・
benchmark・WASM・他 OS・全 optional feature／CI matrix は再実行していない。
main／JPEG2000 の検証や実行プロセスにも手を加えていない。
