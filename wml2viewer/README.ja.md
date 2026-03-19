# wml2viewer

`egui` と `wml2` を使った軽量ネイティブ画像ビューアです。

## 主な機能

- 画像表示とマンガ見開きモード
- 一覧 / サムネイル / 詳細を切り替えられるファイラー
- ZIP / WML の仮想ブラウズ
- ロケール連動の UI リソースとフォントフォールバック
- 保存形式を選べる保存ダイアログ

## 起動

```powershell
cargo run --manifest-path wml2viewer/Cargo.toml -- <path>
```

## コマンドライン

- `wml2viewer [path]`
- `wml2viewer --config <path> [path]`
- `wml2viewer --clean system`

## 設定

設定は OS ごとの設定ディレクトリに保存されます。

大容量 / ネットワーク ZIP 向けワークアラウンド例:

```toml
[runtime.workaround.archive.zip]
threshold_mb = 256
local_cache = true
```

## メモ

- 大きい ZIP やネットワーク上の ZIP では low-I/O ワークアラウンドが有効になります。
- Windows では設定画面から拡張子関連付けを操作できます。
