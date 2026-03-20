# wml2viewer TODO

ステータス
- [x] 確認済み / 安定実装
- [+] 実装済み / 今後の拡張余地あり
- [*] 実装済みだが要再確認 or 既知の不具合あり
- [-] 設計保留
- [ ] 未実装

最終整理日: 2026-03-20

## src/main.rs / src/app.rs
- [x] `wml2viewer <file>` 起動
- [x] `wml2viewer <directory>` 起動
- [x] 起動時に空表示の場合はファイラーを開く
- [x] 起動時ウィンドウサイズを設定から復元
- [x] 起動時ウィンドウ位置を設定から復元
- [x] 設定未指定時は起動スクリーン基準で 60% サイズ + 中央寄せ
- [x] 起動時 fullscreen を無効化するワークアラウンド
- [x] アプリアイコン設定
- [x] `resources/help.html` 出力の土台
- [+] app 起動時の初回 decode worker 化
- [x] `--clean system`
- [-] 二重起動の制限は一旦取り下げ
- [*] フルスクリーン復帰時の安定性確認

## src/options.rs
- [x] ViewerAction / KeyBinding の整理
- [x] `Shift+G` grayscale toggle
- [x] `Shift+C` manga mode toggle
- [x] `Shift+V` subfiler toggle
- [x] `Ctrl+S` 保存ダイアログ起動
- [x] `F1` help 起動
- [ ] キーリマップ UI

## src/configs/config.rs
- [x] config load/save
- [x] startup path load/save
- [x] config import/export
- [x] `--config [path]`
- [x] window / render / resources / navigation の永続化
- [x] storage.path / storage.path_record の永続化
- [x] manga separator / UI theme の永続化
- [x] plugin config の永続化土台
- [x] workaround.archive.zip の永続化
- [ ] config schema のバージョニング

## src/configs/resourses/mod.rs
- [x] システムロケール検出結果を resources へ適用
- [x] `ja_JP.UTF-8 -> ja_JP` 正規化
- [x] `ja_JP -> ja -> en` フォールバック
- [x] `zh_TW -> zh -> en` フォールバック
- [x] locale 別の system font 候補
- [x] emoji font fallback
- [x] `Auto / S / M / L / LL` のフォントサイズ
- [x] DPI / 画面サイズベースの Auto サイズ
- [x] 外部 JSON resource 読み込み

## src/configs/resourses/english.rs
- [-] 外部 resource ローダ導入時に役割を再整理

## src/configs/resourses/japanese.rs
- [-] 外部 resource ローダ導入時に役割を再整理

## src/dependent/mod.rs
- [x] OS 依存 API の窓口整理
- [x] root drive 一覧取得の UI 用ラッパ
- [x] 保存先フォルダ選択ダイアログの窓口
- [x] http/https 共通ダウンロード窓口（reqwest）

## src/dependent/thirdparty/locale_config.rs
- [x] locale 正規化ヘルパ
- [x] resource locale fallback ヘルパ

## src/dependent/thirdparty/directories.rs
- [x] 設定ディレクトリ解決
- [x] 既定ダウンロードディレクトリ解決
- [x] 共通 temp ディレクトリ解決

## src/dependent/windows/mod.rs
- [x] Windows locale 取得
- [x] Windows 向け日本語/繁体字フォント候補
- [x] Windows emoji font 候補
- [x] Windows drive 列挙
- [x] フォルダ選択ダイアログ
- [+] 拡張子関連付け登録
- [+] 拡張子関連付け clean
- [x] winres による exe icon resource 登録

## src/dependent/linux/mod.rs
- [x] locale 環境変数取得
- [x] Linux font fallback 候補
- [*] build
- [ ] フォルダ選択ダイアログ

## src/dependent/darwin/mod.rs
- [x] locale 環境変数取得
- [x] macOS font fallback 候補
- [*] build
- [ ] フォルダ選択ダイアログ

## src/dependent/android/mod.rs
- [ ] Android 依存実装

## src/dependent/ios/mod.rs
- [ ] iOS 依存実装

## src/dependent/other/mod.rs
- [x] その他 OS 向け最低限の fallback

## src/dependent/plugins/mod.rs
- [x] plugin config 構造体
- [x] provider 別 default 設定
- [x] plugin 設定 UI 向けの土台
- [x] search path からの module 走査
- [+] plugin 優先順位の実行ロジック
- [x] MIME / wildcard 判定
- [+] decoder の実行ロジック
- [+] plugin 有効拡張子の列挙
- [ ] encoder / filter の実行ロジック

## src/dependent/plugins/system.rs
- [+] system provider の既定値
- [+] Windows WIC decode 実装
- [ ] macOS system codec 実装

## src/dependent/plugins/ffmpeg.rs
- [+] ffmpeg provider の既定値
- [+] external ffmpeg 実行による decode

## src/dependent/plugins/susie64.rs
- [+] susie64 provider の既定値
- [+] Windows 専用ロード
- [+] image plugin decode 実行
- [ ] archiver plugin 実行

## src/filesystem/mod.rs
- [x] 単一ファイル起動時に親ディレクトリの画像一覧を取得
- [x] `STOP` / `NEXT` / `LOOP` / `RECURSIVE`
- [x] filesystem worker 分離
- [x] directory 単位 cache
- [x] `.wml` / `.zip` を browser container として扱う
- [x] fileviewer から zip の中身を辿れる
- [x] sort order `os_name` / `name` / `date` / `size`
- [+] plugin 有効拡張子を filer / viewer に反映
- [*] `RECURSIVE` の探索コスト最適化
- [ ] filter 条件の filesystem 側統合
- [ ] archive option (`FOLDER` / `SKIP` / `ARCHIVER`)
- [ ] キャッシュのシリアライズ

## src/filesystem/listed_file.rs
- [x] `.wml` 判定
- [x] 相対 path 基準を ListedFile 親ディレクトリにする
- [x] コメント行 `#` を無視
- [-] `@command` / `@(...)` の本実装

## src/filesystem/zip_file.rs
- [x] zip 読み込み
- [x] zip virtual child path
- [x] 途中 entry エラーを飛ばして継続
- [x] zip 名の SJIS fallback decode
- [+] zip entry 自然順ソート
- [+] BufReader ベースの再読込
- [+] 大容量 / ネットワーク zip の low-I/O workaround
- [+] temp へのローカル archive cache
- [+] ZipCacheReader を使った chunk cache
- [ ] zip encoding option
- [ ] `7z` / `rar` / `lzh` / `gzip`

## src/ui/mod.rs
- [x] viewer / render / input / menu / i18n の分離

## src/ui/i18n/mod.rs
- [-] `configs/resourses` への shim のみ

## src/ui/input/dispatch.rs
- [x] key/pointer から action 解決
- [ ] 未実装 action の no-op 整理
- [ ] 動的入力割当

## src/ui/input/mod.rs
- [x] egui input から viewer action dispatch
- [x] settings 表示中は viewer 入力を止める
- [x] text input 中は viewer shortcut を止める
- [x] `P` で settings を閉じる
- [x] 左クリックで次画像
- [x] 右クリックでメニュー
- [x] `F1` help
- [ ] タッチ UI

## src/ui/menu/mod.rs
- [x] menu 名前空間の分離

## src/ui/menu/config/mod.rs
- [x] 設定画面の土台
- [x] viewer / render / window / navigation / plugins / resources タブ
- [x] 閉じるボタン
- [x] 即時適用
- [x] manga separator 設定
- [x] window theme 設定
- [+] plugin 設定画面の土台
- [+] plugin search path 編集
- [+] plugin search path フォルダ選択ダイアログ
- [+] plugin module load test ボタン
- [x] save path 記録設定
- [x] 適用/undo/初期化ボタン
- [+] 拡張子関連付けボタン
- [x] 設定画面の主要文言リソース化
- [+] workaround.archive.zip 設定 UI
- [ ] キーバインド編集 UI

## src/ui/menu/fileviewer/functions.rs
- [ ] Copy
- [ ] Move
- [ ] Trash
- [ ] Convert
- [ ] Similarity

## src/ui/menu/fileviewer/state.rs
- [x] filer state の分離
- [x] root drive 管理
- [x] view mode / sort / filter / URL input state
- [+] thumbnail size 可変 state
- [x] `available_roots` の曖昧 import 解消

## src/ui/menu/fileviewer/icons.rs
- [x] resources/icons の SVG を UI 描画へ接続
- [x] background 反転色での icon 描画
- [ ] SVG icon の共通化と他 menu への展開

## src/ui/menu/fileviewer/worker.rs
- [x] `FilerCommand / FilerResult`
- [x] directory scan の worker 分離
- [x] metadata 収集
- [x] sort / filter / ext filter / dir separate
- [x] 数値を含む自然順ソート
- [+] incremental snapshot preview
- [+] lazy load の段階化
- [ ] OS 準拠 name collation の強化

## src/ui/menu/fileviewer/thumbnail.rs
- [x] サムネイル worker
- [x] virtual zip/listed file のサムネイル生成
- [+] 巨大 zip bmp thumbnail の抑制
- [ ] thumbnail抑制オプション(ALT=image.svgを代用)
- [ ] 永続キャッシュ
- [ ] 失敗キャッシュ

## src/ui/menu/fileviewer/mod.rs
- [x] 一覧表示
- [x] サムネイル表示（小・中・大）
- [x] サムネイル格子グリッド表示
- [x] 詳細表示
- [x] 表示切り替えボタン
- [x] view / sort / dir separate をボタン化
- [x] metadata 表示
- [x] 昇順/降順切り替え
- [x] 名前/更新日時/サイズソート
- [x] フォルダとファイルを混ぜる/分ける
- [x] ファイル名部分一致フィルタ
- [x] 拡張子フィルタ
- [x] ドライブ選択
- [x] zip / archive の内容表示
- [x] URL 入力欄（http/https は reqwest ダウンロードで表示）
- [x] SVG アイコン素材を resources/icons に生成
- [x] SVG アイコンを UI に実表示
- [x] toolbar 文字ボタンの icon 置換
- [x] サブファイラー下部表示
- [x] サブファイラー閉じるボタン
- [x] 詳細表示で更新日時とサイズを表示
- [x] ファイル選択時に filer を閉じる
- [x] サムネイルのフォルダ/アーカイブ icon 縮小
- [x] サムネイル中央の不要な button chrome 削減
- [x] サムネイルペインサイズ可変
- [+] 長いファイル名の中間省略（末尾 7 文字優先）
- [*] filer 表示時のさらなる高速化



## src/ui/render/layout.rs
- [x] 背景描画
- [x] 中央寄せ offset 計算
- [x] manga spread のレイアウト補助

## src/ui/render/texture.rs
- [x] texture upload 補助
- [x] texture size 制限時の downscale
- [ ] 分割 texture による巨大画像対応

## src/ui/render/worker.rs
- [x] render worker
- [x] load / resize request 分離
- [+] preload queue 連携

## src/ui/render/mod.rs
- [x] viewer から render 責務を切り出し
- [*] 変換パイプラインの追加整理

## src/ui/viewer/options.rs
- [x] viewer / render / window option struct
- [x] grayscale option
- [x] manga option
- [x] manga separator option
- [x] window ui theme option

## src/ui/viewer/animation.rs
- [x] アニメーション表示の基礎
- [ ] preload との統合

## src/ui/viewer/mod.rs
- [x] ViewerApp が composition root として worker を束ねる
- [*] 画像 state と viewer state の完全分離
- [+] save / overlay の transient state 分離
- [x] render worker / filesystem worker / filer worker / thumbnail worker を統合
- [x] filer に引きずられない viewer 更新
- [x] manga mode の中央寄せ
- [x] manga mode でフォルダ跨ぎ時の FitScreen 再計算
- [x] resize イベントに寄せた FitScreen 再計算
- [x] filer から画像選択後に次画像移動できる
- [x] filer から画像選択後に FitScreen を再適用
- [x] 保存ダイアログ（保存先フォルダ選択 + 形式選択 + 名前変更）
- [x] 既定ダウンロードフォルダの利用
- [x] 保存完了時に save dialog を閉じる
- [x] 保存中 waiting 表示
- [x] cancel で save dialog を閉じる
- [x] grayscale 表示トグル
- [x] subfiler の明示トグル
- [x] manga separator 描画
- [x] status message の下部表示
- [x] ライトモード時の SVG 線色
- [x] separator shadow gradient
- [x] 起動時の manga Fit 再計算ループの抑制
- [+] low-I/O archive 時は preload 抑制
- [x] filer 表示時の manga レイアウトは実機で継続確認
- [+] app 起動時の初回 decode 完全 worker 化
- [+] preload queue
- [+] message UI 整理

## src/drawers/affine.rs
- [x] resize / interpolation 実装
- [ ] resize 品質と速度の細かな切り替え

## src/drawers/image.rs
- [x] image load
- [+] plugin fallback load
- [x] image save
- [x] SaveFormat 選択
- [ ] 保存オプションの詳細化

## src/drawers/filter.rs
- [x] grayscale 系 filter は存在
- [ ] scaling系 filter
- [ ] エッジ系filter
- [ ] 色系filter
- [ ] viewer のフィルタパイプライン統合

## src/drawers/grayscale.rs
- [x] グレースケール処理の基礎

## src/drawers/canvas.rs
- [x] Canvas 基盤

## src/drawers/draw.rs
- [x] 基本描画

## src/drawers/clear.rs
- [x] クリア処理

## src/drawers/utils.rs
- [x] 補助関数

## src/drawers/error.rs
- [x] 描画エラー型

## src/error/mod.rs
- [x] 共通 error module の土台

## src/graphics/mod.rs
- [-] 役割の再整理

## 次に着手
中断せずやりきる
- [ ] issue: zip crateはBufferReadで8KBのキャッシュしか効いていないので、ZipCacheReaderをラップする　`zipreader.md` 参照
- [ ] thumbnail抑制オプション
- [ ] issue: LinuxとMacOS用がbuild出来ない問題
- [ ] examplesに実装単位のベンチマークテストを実装する どのタスクがボトルネックか発見出来る出来るようにする
- [+] `src/dependent/plugins/*` に実ランタイムを足して internal(内蔵Codec) /system(OS Codec, Windows/MAC) / ffmpeg / susie64(windows only) の優先順位解決を実装する
- プラグイン: 実装続き
  - [x] ffmpegプラグイン(動作:windows o avif  o jp2)
  - [x] susie64プラグイン(動作:x avif  o jp2)
  - [x] Windows Codecプラグイン(動作:windows o avif  x jp2)
  - [ ] MacOS Codecプラグイン(o heif)
    - jpeg2000/avifは ./samplesにサンプルあり susie64はjpeg2000だけ、ffmpegは両方可能のはず
    - test/plugins/susie64, test/plugins/ffmpeg の実ファイルに合わせた runtime 実装
    - ffmpegのリンクはcrate ffmpeg-sysを考慮(自力実装の方が安定する可能性あり)
    - systemにserach pathは不要 OS APIを叩くため
  - [ ] 設定を変えた時、再起動を促すポップアップを出す 
- [ ] プラグインでViewerに画像が表示出来る様にする
- [ ] `src/ui/viewer/mod.rs` の state 分離を進めて `ViewerApp` をさらに薄くする
- [ ] `src/ui/menu/fileviewer/worker.rs` の lazy load / incremental snapshot をさらに進めて大規模フォルダを高速化する
- [ ] wml2viewerのREADME.ja.mdとREADME.mdの更新
- [ ] todo.mdの更新


## レビュアーissue
- [*] zip 内ファイルソートの実機確認
- [+] 数字入りファイルのソート順の Explorer 差分調整(確認中)
- [+] ファイラー/サブファイラー/viewer のファイル表示順の実機確認(確認中)
- [*] ファイラー: OS name collation の最終調整(確認中)
- [ ] コードの整理 モジュール境界をハッキリさせる
  - [ ]未実装 action の no-op 整理
