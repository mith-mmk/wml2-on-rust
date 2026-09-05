# 2026-09-05 修正計画の実装・検証記録

対象資料: EXECUTION_PLAN.md / WML2_REVIEW_AND_PLAN.md のR01〜R18。基準コミットは `d40fed2e3cd01c4af86eae01b09487c15c1a54e7`、作業ブランチは `codex/review-plan-implementation`。
既存の公開コールバック、RGBA8量子化、feature名、低レベルWebP公開入口を維持した。上限管理・変換器再利用は追加APIで提供する。

## 実装済み項目

| ID | 変更 | 対応する検証 |
| --- | --- | --- |
| R01 | 符号化成功後に同一ディレクトリの一時ファイルへ保存し、renameで確定 | review_regressions: 同一パス、旧出力保持、ハードリンク、Windowsロック、一時ファイル回収 |
| R02 | 再initでアニメーション状態をリセットし、nextの検査・確保を状態更新より先に実施 | review_regressions: 再利用、失敗前後の状態一致、不正current |
| R03 | PNGのチャンク残量・fcTL/fdAT/IEND長・期待走査線長を検査 | review_regressions: 切断、fdAT長0〜3、展開長不足・余剰 |
| R04 | 共通unfilterとAdam7パス復元、GA16と16-bit tRNS修正 | review_regressions: 全対応色モデル/深度、全filter、1×N/N×1、解析的画素期待値 |
| R05 | ICC整数入力をプレーンのmeaningful_bitsで正規化・再量子化 | highres_icc: 8/10/12/16-bit非恒等ガンマ、513×2タイル境界 |
| R06 | ICCでPremultipliedを明示的に拒否 | highres_icc: alpha 0/中間/最大、alpha不変 |
| R07 | AVIFで暗黙の精度拡張を拒否、明示RightShiftだけを縮小に使用 | highres_avif_encode: 8→10/12・10→12拒否、12→10復号サンプル一致 |
| R08 | 独立した資源上限、miniz_oxide 0.7 core APIによる有界・fallible展開 | limits単体試験、review_regressionsの画素/入力/展開/ICC+圧縮テキスト累積、webp_core_compatのフレーム累積 |
| R09 | PNG/JPEGのAbortを成功中断として扱いterminateを一度呼ぶ | callback_abort: init/next/draw/metadata/verbose、反復中断、切断JPEG。PSDの既存中断契約も確認 |
| R10 | ICC変換でtimingを保持 | highres_icc: Noneと非自明なtimingの一致 |
| R11 | AV1色記述省略と明示unspecifiedを区別しfull_rangeを常に保存 | highres_avif_native_bridge: full/limited、nclxなし、競合nclxを独立保持 |
| R12 | PR向けfeature/MSRV/platform/Clippy CI、定期Miri・mutationを追加 | 下記ローカル検証とci.yml、extended-checks.yml |
| R13 | draw/pickを事前検証済み行スライスのコピーへ変更 | 矩形・stride・ゼロ埋め回帰、4K/小矩形ベンチ |
| R14 | ImageBuffer→WebPはフレーム画素を借用、旧Rawメタデータ経路も維持 | webp_core_compat: 両経路一致、3フレーム/48 bytesの境界。24フレーム計測 |
| R15 | ICCの256画素タイルとFrameTransform再利用、AVIFの型/プレーン選択をループ外へ移動 | ICC数値回帰・再利用ベンチ、AVIF planar/interleaved/padded抽出ベンチ |
| R16 | JPEG係数・逆量子化を固定配列化、容量8のキューとjoinする単一ワーカー | callback_abort、worker panic試験、各IDCT、単一/並列ハッシュ・初回draw・メモリ比較 |
| R17 | WebP静止画コアをwebp-rust 0.3.1 legacy入口へ委譲 | 削除前の直接比較、固定golden長/ハッシュ、lossless/quality/method/alpha/EXIF、公開API回帰 |
| R18 | 検証済みPNG/JPEG/WebP重複を除去し依存・実装状態の文書を更新 | feature行列、Clippy差分、Wasm/32-bit、既存レトロ形式回帰、現行依存確認 |

PNG修正前には走査線不足の添字パニック、fdAT減算パニック、GA16/Adam7の誤画素を再現した。資料が言及する外部fixture一式は提示されていないため、Rust合成入力と独立した解析的期待値を追加した。AV1省略色記述はFFmpegで生成した固定fixtureを収録し、手順を [fixture README](../wml2/tests/fixtures/review/README.md) に記載した。

## 追加APIと互換性

`draw::image_from_with_limits` / `image_decoder_with_limits` は `limits::DecodeLimits` を受け取る。従来の入口は次の既定値を使用する。

| 上限 | 既定値 |
| --- | ---: |
| input_bytes | 512 MiB |
| pixels | 128 × 1024 × 1024画素 |
| expanded_bytes | 512 MiB |
| metadata_bytes | 32 MiB |
| frames | 10,000 |
| animation_bytes | 512 MiB |

`DecodeLimits::unlimited()` または各フィールドの `usize::MAX` で明示解除できる。従来読み込めた非常に大きい画像は既定入口ではエラーになる場合がある。RGBAキャンバス、アニメーション累積、PNG/PSD ZIP/TIFF Deflate、PNGのICC/テキストを対象にしており、全依存コーデックの全内部割当を計量する仕組みではない。ネイティブhighres入口や個別形式の低レベルAPIは、汎用ディスパッチと同一の上限範囲を保証しない。

`image_to_with_limits` / `image_encoder_with_limits` は既存アニメーションメタデータ輸送とWebPアニメーションの上限を指定する。汎用的なエンコーダ総メモリ上限ではなく、ユーザーコールバック内の割当は管理しない。WebPの `encode_buffer` はImageBufferのフレームを借用する追加入口。PickCallback/予約メタデータキーも引き続き利用できる。

`highres::icc::FrameTransform::new(...).transform(&frame)` は解析・コンパイル済み変換器を再利用する。alpha/padding/timingを保持し整数値は元の有効深度へ戻す。Premultiplied、YCbCr、明示limited-rangeは自動補正せずエラーにする。ICC/CICP間の推測やトーンマッピングは追加していない。

保存はcreate_new一時ファイルへのwrite/flush/sync_all/close後にrenameする。既存出力を先に削除しない。ハードリンクは宛先エントリだけを置換し、シンボリックリンクもリンク自身を置換する。通常のI/Oエラー時の旧出力保持を検証した。ディレクトリfsyncを含む電源断耐久性や既存ファイルのACL/属性保持は保証しない。

## ローカルで実行・確認済み

Rust 1.95.0 / Windows、依存解決を固定した `--locked` で実行。外部サンプル不在時に正常終了する既存テストは、そのサンプルを検証したとは数えない。

| 検査 | コマンド・範囲 | 結果 |
| --- | --- | --- |
| 既定 | `cargo test -p wml2 --locked --lib --tests` | 106 passed、0 failed、1 ignored |
| 最小 | 上記に `--no-default-features` | 21 passed、0 failed |
| PNG/PSD/TIFF | `--no-default-features --features png,psd,tiff` | 54 passed、0 failed、1 ignored |
| 個別feature | no-defaultでhigh-bit-depth、color-management、avif+high-bit-depth、avifenc+high-bit-depth、同multithread追加 | 各lib/tests成功 |
| JPEG | no-defaultでjpeg,idct_llm / jpeg,idct_aan / jpeg,idct_slower、LLM+multithreadを別実行 | 各lib/tests成功 |
| 複合 | no-defaultでpng,psd,tiff-jpeg,jpeg,idct_llm,multithread,color-management,avifenc | 100 passed、0 failed、2 ignored |
| MSRV | `cargo +1.91.0 test -p wml2 --locked --no-default-features --features png,jpeg,idct_llm,multithread,color-management,avifenc --lib --tests` | 81 passed、0 failed、3 ignored |
| ICC独立オラクル | `cargo test -p wml2 --locked --no-default-features --features color-management --test highres_icc` | 最終追加試験を含め6 passed、0 failed、1 ignored。LittleCMS 2.16の256 RGB階調を8/10/12/16-bitで比較 |
| AVIF実画像 | avif_encode_sampleをavifenc+webp有効で実行、利用可能なFFmpegで検証 | 成功 |
| docs/examples | `cargo test -p wml2 --locked --doc`、`cargo check -p wml2 --locked --examples --features color-management` | 12 doctests成功、examples成功 |
| Clippy | `cargo clippy -p wml2 --locked --lib --tests --features color-management,avifenc,psd --message-format=json` | 成功、既存警告あり。基準とのcode/message/file比較で新規警告なし |
| 書式・差分 | `cargo fmt -p wml2 --check`、`git diff --check` | 成功 |
| Wasm / 32-bit | `cargo check -p wml2 --lib --target <target> --features high-bit-depth,color-management,avifenc,psd` | wasm32-unknown-unknown / i686-unknown-linux-gnu成功、実行試験ではない |
| Miri | `cargo +nightly miri test -p wml2 --no-default-features --test review_regressions` | 境界関数の2試験成功 |
| mutation | `cargo test -p wml2 --no-default-features --features png --test review_regressions bounded_png_mutation_corpus -- --ignored` | 固定seedの10,000入力成功 |

基準Clippyのhard error 3件は同一f32値のSQRT_2定数と数値を変えない添字簡約で解消した。PNGサンプル試験とTIFF/JPEG試験の不足feature条件も修正した。ignoredは明示実行のベンチ/mutationであり、削除して通したものではない。

## CIの設定と未確認範囲

PR CIはRust 1.91/stableの12構成、Windows/macOS保存回帰、Wasm/32-bit check、fmtを定義した。各ジョブで生成したCargo.lockとClippy診断をartifactに保存する。既存方針に合わせCargo.lock自体は追跡しない。相互排他的なIDCTを含むため `--all-features` は使用しない。

この作業ではGitHub Actionsを実行していない。Linux/macOSのランタイム確認はCI実行後に確定する。固定mutationは完全なfuzzer/長時間耐久試験ではない。ICCは解析的非恒等期待値、icc-profile直接呼出し、LittleCMS 2.16の固定期待値と比較した。LittleCMS比較はRGBガンマの相対的測色に限定し、全プロファイル/intentの網羅とはしない。Windowsでシンボリックリンク作成権限を要する保存試験は未実施。

## 性能と現行依存

[性能レポート](review-performance.md) にABBA反復、信頼区間、割当、RSS、初回draw、ネイティブ/Wasmサイズと原CSVを収録した。draw/pick・ICC・AVIF抽出で改善を確認。WebPアニメーションは速度がほぼ同等で、生存ヒープ最大が4.728→1.694 MiBになった。JPEG並列1024×768は初回drawとメモリが改善した一方、総時間が16.610→18.466 ms（約11%増）となった。すべての処理が高速化したとはしない。

上記修正・性能計測時点の依存はicc-profile 0.0.5（registry）、webp-rust 0.3.1（registry、legacy追加）、avif-rust / avifenc-rust 0.0.7（独立サブモジュールへのpath付き）。ICCの一時git依存という旧チェックリストの記述を履歴に変更した。その後、追加依頼によってICCを0.0.6へ更新した。性能CSVの測定条件とCargo.lockハッシュは計測当時のものを保持する。

## ICC 0.0.6公開後の統合確認

2026-09-05の追加依頼に従い、ICCをmainに統合して0.0.6へ更新し、package/dry-run後にcrates.ioへ公開した。公開コミットは `bd5a65e19b26826aa5f031cd6a354e5d38e77de9`。既定masterは残し、mainはその子孫の実装履歴をfast-forward相当で取り込んだ。変換アルゴリズムと公開APIは0.0.5と同じで、旧プロファイルリーダーの符号なし整数 `<= 0` を同値の `== 0` へ修正した。

WML2本体とwml2-testの依存指定を0.0.6に揃え、Cargo.lockを更新した。`cargo info icc-profile@0.0.6 --registry crates-io` で公開版を取得し、`cargo metadata` で `registry+https://github.com/rust-lang/crates.io-index` からの解決を確認した。checksumは `fb960e791dd103a3e696377c2b1e68da54215d68e43698960217666f8a53ff87`。

| 公開版への更新後に実行した検査 | 結果 |
| --- | --- |
| `cargo test --workspace --locked --features wml2/color-management` | 136 passed、0 failed、2 ignored（doc testsを含む） |
| `cargo test -p wml2 --locked --no-default-features --features png,psd,tiff-jpeg,jpeg,idct_llm,multithread,color-management,avifenc --lib --tests` | 103 passed、0 failed、3 ignored |
| `cargo +1.91.0 test -p wml2 --locked --no-default-features --features color-management --lib --tests` | 38 passed、0 failed、1 ignored |
| `cargo check --workspace --locked --examples --features wml2/color-management` | 成功 |
| `cargo clippy -p wml2 --locked --lib --tests --features color-management,avifenc,psd` | 成功、既存警告あり |
| `cargo check -p wml2 --locked --lib --target <target> --no-default-features --features color-management` | wasm32-unknown-unknown / i686-unknown-linux-gnuとも成功 |
| `cargo fmt -p wml2 --check` / `git diff --check` | 成功 |

ICC単体は219 tests成功、通常Clippyとall-targets checkも成功した。ICCの厳格Clippy（208件の既存警告）と全体fmtの既存整形差分は残るため、成功扱いにしていない。ICC mainはpush済み。WML2の作業ブランチは `codex/review-plan-implementation` で、WML2自体の公開は今回行っていない。
