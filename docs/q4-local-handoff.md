# Q4 ローカル調査の引き継ぎ

対象branchは`codex/p2-q4-boundaries`、baseは`187ef7cb72cdaf8d98550ecc0e5daf58920a7646`です。詳細とcommit経緯は[状態記録](q4-status.md)および`git log`を参照してください。まだpush・merge・releaseしていません。この文書はローカル作業の準備で、環境移動を実施したものではありません。

**正式decoderの出力はユーザー視覚検品に不合格です。** smoke test成功は正しい復号を意味しません。前行コピーは診断仮説で、正式実装に採用していません。性的内容を含むサンプルや誤復号で内容が判断できない画像は、チャット・Driveへ提示しません。画像と外部バイナリはrepoに同梱しません。

## 入力とテスト

Rust 1.91、Python 3を使用します。独立PNGを比較するときだけPillowが必要です。利用権限のあるQ4を自分のローカル`.test-q4-input/`に配置してください。28件の既存corpusを使う場合は[出所とarchive SHA256](q4-status.md)を確認し、再配布条件を別途確認してください。Q4Dは作品説明であり形式仕様ではありません。

repo rootで実行します。submoduleを別途必要とするfeatureはこの調査に使いません。

```bash
cargo test -p wml2 --no-default-features --features q4 --lib --test q4_formats
python3 docs/q4-local-audit.py --self-test
python3 docs/q4-local-audit.py \
  --input-dir .test-q4-input --output .test-q4-audit.json
```

実画像smoke testは既定でignoredです。実行する場合、既存の`samples/`を持つrootを`WML2_Q4_SAMPLE_ROOT`に指定してください（testはそのrootに`samples`を追加します）。Q4が直接そのrootに入っていても見つかりません。未配置・空directoryは失敗し、`noretoro`ではこのtestを無効にします。

```bash
WML2_Q4_SAMPLE_ROOT=/absolute/path/to/sample-root \
  cargo test -p wml2 --no-default-features --features q4 \
  --test q4_formats -- --include-ignored
```

`q4-local-audit.py`は数値のみを記録し、画像を表示・作成しません。入力hash、header、bitcode／辞書展開、RLE位置、run長、出力位置と予測参照候補、初行・行跨ぎ・block履歴の条件を扱います。RLEの予測意味は仮定として分離します。小テストは仮定の計算確認で、形式互換の証明ではありません。

## 現在の出力と独立oracleの比較

まず現在のdecoderでraw RGBA8を保存します。出力の寸法・alphaは検査しますが、画素の正しさは保証しません。

```bash
cargo build -p wml2 --release --no-default-features --features q4 \
  --example q4_dump_rgba
cargo run -p wml2 --release --no-default-features --features q4 \
  --example q4_dump_rgba -- .test-q4-input/example.Q4 .test-q4-current.rgba
```

RECOIL 6.4.5のユーザー直接添付debは親環境で179580bytes、次のSHA256が公式掲載値と一致しました。WML2環境では展開・実行・比較していません。ローカルに配置したファイルを改めて照合してください。

```text
3248d027c1ceb894aba180c6b6fbd899197d4d4cf440e63ff2cbf877fc0901f3
```

```bash
sha256sum recoil-2png_6.4.5-1_amd64.deb
```

必要なバイナリの展開・実行はユーザーのローカル作業として行います。展開した`recoil-2png`のhelpに従って、同じ入力から独立PNGを作り、`.test-q4-oracle/`に`入力stem.png`という名前で置いてください。CLIのflagsはこの環境で確認していないため、ここでは推測しません。実行versionとbinary hashを記録し、oracle画像を別のWML2出力で代用しません。

```bash
python3 docs/q4-compare-local.py \
  --input-dir .test-q4-input --output-dir .test-q4-current \
  --dump target/release/examples/q4_dump_rgba --repo . \
  --oracle-dir .test-q4-oracle \
  --oracle-binary /absolute/path/to/recoil-2png --oracle-version 6.4.5
```

`CARGO_TARGET_DIR`を設定している場合は`--dump`をそのbuild出力に変更します。比較は640×400のRGB全画素で、resize・補間・ICC変換はしません。相違画素数、最大channel差、最初の相違座標／RGB、入力・出力・binary hash、repo HEADとdirty状態をJSONに記録します。oracleがないと`oracle_missing`、比較完了判定はfalseです。raw RGBAとreportだけを作り、現在の画像PNGを自動で公開・表示しません。

## 切り分け結果と次の焦点

- 別構造LZWとの196stream一致は辞書層の内部整合性を支持します。bit解釈の独立証明ではありません。
- 今回のRLE値は0〜16で、signed8解釈との差はありません。countの一律±1は宣言画素数に合いません。
- 非zero-repeatをindex0で埋める処理が横帯に対応し、前行コピー仮説で改善します。初行補完・block履歴持越し・順次コピーの意味は未確定です。
- 原資料、Susie命令解析、独立oracleのいずれも手元にありません。初行やblock条件を見た目だけで決めず、独立PNGの最初の相違を命令トレースと突き合わせてください。

元出力を保持したまま、修正版は別の`.test*`出力directoryへ保存します。命令／予測規則に根拠を得てから最小修正と回帰testを入れ、全28画像の相違と表示可能な資料を検品します。decoder変更時はrepoのAGENTS.mdに従い表示確認とbenchmarkも行います。
