# Q4 ローカル調査の引き継ぎ

対象branchは`codex/p2-q4-boundaries`、baseは`187ef7cb72cdaf8d98550ecc0e5daf58920a7646`です。詳細とcommit経緯は[状態記録](q4-status.md)および`git log`を参照してください。まだpush・merge・releaseしていません。この文書はローカル作業の準備で、環境移動を実施したものではありません。

**横帯は修正済みです。** `ifxld4.spi`の逆アセンブルで短縮runが直前の明示run色を継続すると確認し、RECOIL 6.4.5の28画像とRGB全画素が一致しました。解析根拠は[`ifxld4.spi`解析](q4-ifxld4-analysis.md)を参照してください。性的内容を含むサンプルはチャット・Driveへ提示しません。画像と外部バイナリはrepoに同梱しません。

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

`q4-local-audit.py`は数値のみを記録し、画像を表示・作成しません。入力hash、header、bitcode／辞書展開、RLE位置、run長、出力位置、明示色と短縮runの関係を扱います。RLE規則は`ifxld4.spi`の解析結果に合わせていますが、このscript単独は独立画像比較の代替ではありません。

## 現在の出力と独立oracleの比較

まず現在のdecoderでraw RGBA8を保存します。出力の寸法・alphaは検査しますが、画素の正しさは保証しません。

```bash
cargo build -p wml2 --release --no-default-features --features q4 \
  --example q4_dump_rgba
cargo run -p wml2 --release --no-default-features --features q4 \
  --example q4_dump_rgba -- .test-q4-input/example.Q4 .test-q4-current.rgba
```

RECOIL 6.4.5のユーザー直接添付debは179580bytesで、次のSHA256が公式掲載値と一致しました。ローカルで展開・実行し、28画像を比較済みです。再実行時も配置したファイルを照合してください。

```text
3248d027c1ceb894aba180c6b6fbd899197d4d4cf440e63ff2cbf877fc0901f3
```

```bash
sha256sum recoil-2png_6.4.5-1_amd64.deb
```

展開後のCLIは`recoil2png -o OUTPUT.png INPUT.Q4`、versionは6.4.5でした。同じ入力から独立PNGを作り、`.test-q4-oracle/`に`入力stem.png`という名前で置きます。実行versionとbinary hashを記録し、oracle画像を別のWML2出力で代用しません。今回使った`recoil2png`のSHA256は`4ade6d82068381dd1ea500324a564e8b3c764607e5b2de626f6122f5df7cd2a7`です。

```bash
python3 docs/q4-compare-local.py \
  --input-dir .test-q4-input --output-dir .test-q4-current \
  --dump target/release/examples/q4_dump_rgba --repo . \
  --oracle-dir .test-q4-oracle \
  --oracle-binary /absolute/path/to/recoil-2png --oracle-version 6.4.5
```

`CARGO_TARGET_DIR`を設定している場合は`--dump`をそのbuild出力に変更します。比較は640×400のRGB全画素で、resize・補間・ICC変換はしません。相違画素数、最大channel差、最初の相違座標／RGB、入力・出力・binary hash、repo HEADとdirty状態をJSONに記録します。oracleがないと`oracle_missing`、比較完了判定はfalseです。raw RGBAとreportだけを作り、現在の画像PNGを自動で公開・表示しません。

## 切り分け結果と次の焦点

- 別構造LZWとの196stream一致とRECOIL全画素一致により、辞書層・bit解釈・RLEの有効corpus互換性を確認しました。
- `ifxld4.spi`のRLE routineは短縮runを直前の明示run色で展開します。literalは保持色を更新せず、保持色はimage blockごとに0へ戻ります。
- 修正前の前行コピー仮説、初行補完、block履歴持越しは棄却しました。
- decoderの安全境界はpluginの未検査挙動を模倣せず、既存の予算・checked arithmetic・run/block/canvas上限を維持します。

元出力・独立PNG・比較reportは別の`.test*`directoryへ保存します。decoder変更時はrepoのAGENTS.mdに従い表示確認とbenchmarkも行います。
