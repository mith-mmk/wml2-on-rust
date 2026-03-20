# wml2viewer 0.0.11 preview

`egui` と `wml2` を使った軽量ネイティブ画像ビューアです。

## 主な機能

- 画像表示とマンガ見開きモード
- 一覧 / サムネイル / 詳細を切り替えられるファイラー
- ZIP / WML(ファイルリスト) の仮想ブラウズ
- ロケール連動の UI リソースとフォントフォールバック
- 保存形式を選べる保存ダイアログ
- `internal / system / ffmpeg / susie64` の優先順位で動く plugin decode 土台

## 起動

```powershell
cargo run --manifest-path wml2viewer/Cargo.toml -- <path>
```

## コマンドライン
- `wml2viewer` デフォルトのファイルを見ます
- `wml2viewer [path]` 画像を指定して起動します
- `wml2viewer --config <path> [path]` 設定ファイルを指定します
- `wml2viewer --clean system`　設定を消します

## ヘルプ
- https://mith-mmk.github.io/wml2/help.html

## 設定

設定は OS ごとの設定ディレクトリに保存されます。

大容量 / ネットワーク ZIP 向けワークアラウンド例:

```toml
[runtime.workaround.archive.zip]
threshold_mb = 256
local_cache = true
```

plugin 設定例:

```toml
[plugins.ffmpeg]
enable = true
search_path = ["../test/plugins/ffmpeg"]

[plugins.susie64]
enable = true
search_path = ["../test/plugins/susie64"]
```

## メモ

- 大きい ZIP やネットワーク上の ZIP では low-I/O ワークアラウンドが有効になります。
- Windows では設定画面から拡張子関連付けを操作できます。
- `ffmpeg` は現状 `ffmpeg.exe` を起動して decode します。
- `susie64` は Windows 専用で、今は image plugin decode まで入っています。
- `system` は設定と優先順位スロットまで先に入り、OS codec 本体は今後の拡張対象です。
