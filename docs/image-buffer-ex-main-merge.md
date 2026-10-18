# main → ImageBufferEx continuation merge

2026-10-15 時点の `origin/main`
`5b12608a90026a913ddafd90bfb3f8ec53d14c83` を、EX 作業ブランチ
`codex/image-buffer-ex-continuation-20261013` の
`c201461e93b84b6284c953fe68ac31d80dc77319` に通常 merge した。
main 自体への書き込み、rebase、force push、release、新規 PR 作成、CI 監視は行わない。
作業前の EX はローカル・リモート SHA が一致し、未保存変更はなかった。

## 競合と保持した実装

競合は `.github/workflows/ci.yml` と `README.ja.md` の2ファイル。
CI の feature 一覧は EX と JPEG2000 の両方を残して解消した。
日本語 README は両機能の節を併記した。README と EX の現行案内は main 由来の
0.0.36 を明示し、EX の追加機能自体は未リリースであることを維持した。
これは main の既存バージョン更新の取り込みであり、今回の公開操作ではない。

Rust ソースの競合はない。JPEG2000 の各実装、共有 limits、error、lib、util と
JPEG2000 テストは取り込んだ main とバイト一致する。EX の画素 storage、
callback guard、PNG/TIFF、実行サンプルと新規契約テストは EX 親とバイト一致。
共通 draw dispatcher には main の opt-in JPEG2000 分岐が自動統合されている。

[Callback 入力・保存・出力の契約](image-buffer-ex-callbacks.md)、ICC metadata を
蓄積してから外部処理する手順、Exact/Round と矩形更新の atomicity は維持する。
JPEG2000 の native EX／高ビット深度対応などの新機能は追加しない。

## 今回の検証

結果・コマンド・実行 profile・ソース／ログの hash は
[今回の検証記録](image-buffer-ex-main-merge-validation.json)に保存する。
旧検証記録の数字を今回の実行結果に数えない。

| 最終マージ内容に対する今回の実行 | 結果 |
| --- | --- |
| release workspace、JPEG2000＋EX＋high-bit-depth＋optional retro codecs | 474成功、0失敗、既存 ignored 13（52 targets、doctests含む） |
| 上記に含まれる JPEG2000／EX Callback 契約 | 48成功／9成功 |
| JPEG2000-only、release、lib＋tests | 84成功、0失敗（JPEG2000 target は48成功） |
| 最小 EX／feature-off、debug | 78成功／40成功、0失敗 |
| 実 WASM、Node v24.19.0 | EX82グループ、JPEG2000拡張72ケース成功 |
| EX 実行サンプル | 最小とJPEG2000/PNG/TIFF併用で5ケース成功、実TIFF読み込みも成功 |
| fmt／diff／workspace all-targets Clippy | 成功。Clippy の既存警告は維持し、新規 Callback サンプル／テストの診断なし |
| release metadata | 6テスト成功、main の0.0.36 release notes 検証成功 |

追加の一時ハーネスでは JPEG2000 の通常 `image_decoder` から EX RGBA16 保存に
渡し、全 sample が legacy RGBA8×257 と一致することを確認した。
入力宣言は packed RGBA8 のままで、ネイティブ高精度 JPEG2000 デコードを意味しない。
EX 専用入口 `image_from_ex` は JPEG2000 を引き続き拒否する。

取り込んだ main と同じ release helper・最小 JPEG2000 features を使った
限定的な [paired benchmark](image-buffer-ex-main-merge-benchmark.json) と、
[JPEG2000／EX の表示確認](image-buffer-ex-main-merge-display.png)も記録した。
JPEG2000 の18回の export は3ケースとも全RGB画素が参照と一致した。
EX RGBA16 の8,580 sample も元の独立生成パターンと一致した。
benchmark の平均変化は16×16Grayが−6.51%、65×49RGBが+0.44%、
複数品質レイヤー32×32RGBが−0.78%。両バイナリとも最小 JPEG2000 feature で、
EX を無効にした既存経路の限定測定である。
共有 host の測定は全 workload や他 OS の速度保証ではない。

JPEG2000-only の追加 debug 実行は大型 corpus で長時間となり、自分の実行
セッションを中断した。完全な feature 単独結果は release 実行で確認した。
補助 benchmark 用の main checkout は、空の optional path dependency directory を
既存の同一 dependency source で準備した後に build した。準備前の build 失敗は
マージコードの失敗と区別して記録する。

他 OS、全 optional feature／外部 corpus／CI matrix、実 ICC 色変換と
高ビット深度 JPEG2000 は今回の検証・追加実装の対象外。
以前の結果を実行した扱いにせず、CI 待ちも行わない。
