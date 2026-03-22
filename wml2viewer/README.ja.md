# wml2viewer 0.0.11 preview

`egui` と `wml2` を使った軽量ネイティブ画像ビューアです。

## 主な機能

- UI を先に起動して初回画像 decode をバックグラウンドで行う非同期起動
- Viewer / filer / subfiler の分離レイアウトと下部 status overlay
- システム連携をまとめた `システム` タブ付きの設定ダイアログ
- 設定変更は `適用` を押すまで反映しない staged 方式
- `適用` / `キャンセル` のどちらでも設定ダイアログを閉じます
- plugin 設定で `internal / system / ffmpeg / susie64` の優先度を編集可能
- 横に十分広い時だけ有効になるマンガ見開きモード
- 一覧 / サムネイル / 詳細を切り替えられるファイラーと drive/root 切り替え
- 設定からファイラーの左右ペイン位置を切り替え可能
- ZIP / WML(ファイルリスト) の仮想ブラウズ
- 保存形式を選べる保存ダイアログ
- ロケール連動の UI リソースとフォントフォールバック。設定から locale を変更可能
- `自動` ボタンでシステムロケールを staged 値へ入れられます
- `internal / system / ffmpeg / susie64` の優先順位で動く plugin decode 土台
- ZIP 指定時もウィンドウを先に開いてから中身を解決する非同期起動
- 起動時は filer/filesystem の同期より先に最初の viewer 画像表示を優先します
- startup 後の filesystem 同期は、最初に解決できた実画像 path を優先して行います
- ZIP metadata 読み込みは必要に応じて plain `BufReader<File>` にフォールバックします
- 読み込み中の遷移先を pending で保持し、フォルダ/アーカイブ跨ぎの古い表示を減らしています
- 画像ロード失敗時は前の画像を残さず loading texture に戻します
- ポインタ既定動作は、左クリックで設定、右クリックで次、右ダブルクリックで fit 切り替え、中クリックでメニューです
- render / filer / thumbnail worker が切断時に自動で再生成されます
- アプリ終了時は render worker に明示的に shutdown を送ります

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
local_cache = false

[filesystem.thumbnail]
suppress_large_files = true

[resources]
font_paths = ["C:/Windows/Fonts/NotoSansJP-Regular.otf"]
```

### plugin
 Pluginを使う事で利用可能な画像形式を増やすことが可能です。基本ffmpegとSystemを有効にして置けば良いでしょう。

- susie64はpluginを探して導入してください(Windowsのみ)
- OS SystemはOSがサポートしているフォーマットをそのまま利用します(WindowsとMac OSのみ)
- ffmpegはexeの入っているフォルダを指定してください

 設定例:

```toml
[plugins.ffmpeg]
enable = true
search_path = ["c:/bin/ffmpeg"]

[plugins.susie64]
enable = true
search_path = ["c:/susie64/plugins/"]
```

## メモ

- 大きい ZIP やネットワーク上の ZIP では low-I/O ワークアラウンドが有効になります。
- ZIP の一時ローカルキャッシュは既定で無効にし、ネットワーク/SSD 環境で起動を引きずりにくくしています。
- 大きい BMP / アーカイブのサムネイルは設定から抑制できます。
- サムネイル生成失敗時は pending を解放して再試行できるようにしています。
- ファイラーの更新日時は UTC ではなくローカル時刻で表示します。
- ファイラーの分離ソートで ZIP を folder 扱い / file 扱いに切り替えられます。
- Windows では `設定 -> システム` から拡張子関連付けを操作できます。
- `ffmpeg` は現状 `ffmpeg.exe` を起動して decode します。
- `susie64` は Windows 専用で、今は image plugin decode まで入っています。
- `system` は Windows では WIC decode まで入りました。macOS system codec は今後の拡張対象です。
- provider を有効化すると、`avif` や `jp2` などの拡張子も filer / viewer の対象に入ります。
- plugin 設定変更時は再起動推奨ポップアップを出します。
- マンガモードの見開き相手は現在のフォルダ / 仮想アーカイブ枝の中だけに制限しています。
- `bench_archive` は decode 失敗エントリが混ざっても metadata/read の計測を続けます。
- `ZipCacheReader` は大きめの chunk と tail prefetch を使うようにしています。
- Windows のフォント探索順は `%LOCALAPPDATA%\\Microsoft\\Windows\\Fonts` → `%WINDIR%\\Fonts` です。
- ロケール既定の system font を先頭に使い、`resources.font_paths` で追加フォントを前置できます。

## ベンチマーク

```powershell
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_decode -- .\samples\WML2Viewer.avif 5
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_browser -- .\samples 3
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip default
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip online_cache
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip temp_copy
```

`bench_archive` は非対応入力でも panic せず、通常のエラーメッセージを出して終了します。
