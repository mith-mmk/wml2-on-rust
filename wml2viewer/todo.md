# wml2viewer TODO

ステータス
- [x] 確認済み / 安定実装
- [+] 実装済み / 今後の拡張余地あり
- [*] 実装済みだが要再確認 or 既知の不具合あり
- [-] 設計保留
- [ ] 未実装

最終整理日: 2026-03-19

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
- [ ] 外部 JSON resource 読み込み

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

## src/dependent/windows/mod.rs
- [x] Windows locale 取得
- [x] Windows 向け日本語/繁体字フォント候補
- [x] Windows emoji font 候補
- [x] Windows drive 列挙
- [x] フォルダ選択ダイアログ
- [x] PowerShell 依存 http ダウンロードからの離脱

## src/dependent/linux/mod.rs
- [x] locale 環境変数取得
- [x] Linux font fallback 候補
- [ ] フォルダ選択ダイアログ
- [ ] http/https ダウンロード実装

## src/dependent/darwin/mod.rs
- [x] locale 環境変数取得
- [x] macOS font fallback 候補
- [ ] フォルダ選択ダイアログ
- [ ] http/https ダウンロード実装

## src/dependent/android/mod.rs
- [ ] Android 依存実装

## src/dependent/ios/mod.rs
- [ ] iOS 依存実装

## src/dependent/other/mod.rs
- [x] その他 OS 向け最低限の fallback

## src/dependent/plugins/mod.rs
- [+] plugin config 構造体
- [+] provider 別 default 設定
- [+] plugin 設定 UI 向けの土台
- [+] search path からの module 走査
- [ ] plugin 優先順位の実行ロジック
- [ ] MIME / wildcard 判定
- [ ] decoder / encoder / filter の実行ロジック

## src/dependent/plugins/system.rs
- [+] system provider の既定値
- [ ] WIC / OS bundle codec 実装

## src/dependent/plugins/ffmpeg.rs
- [+] ffmpeg provider の既定値
- [ ] 動的ライブラリ探索と呼び出し

## src/dependent/plugins/susie64.rs
- [+] susie64 provider の既定値
- [ ] Windows 専用ロード
- [ ] image / archiver plugin 実行

## src/filesystem/mod.rs
- [x] 単一ファイル起動時に親ディレクトリの画像一覧を取得
- [x] `STOP` / `NEXT` / `LOOP` / `RECURSIVE`
- [x] filesystem worker 分離
- [x] directory 単位 cache
- [x] `.wml` / `.zip` を browser container として扱う
- [x] fileviewer から zip の中身を辿れる
- [x] sort order `os_name` / `name` / `date` / `size`
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
- [ ] zip entry sort option
- [ ] zip encoding option
- [ ] `7z` / `rar` / `lzh` / `gzip`

## src/ui/mod.rs
- [x] viewer / render / input / menu / i18n の分離

## src/ui/i18n/mod.rs
- [x] `UiTextKey` ベースの翻訳経路
- [x] settings menu のローカライズ
- [x] filer menu の主要文言ローカライズ
- [x] save dialog の主要文言ローカライズ
- [ ] status message / zoom option / detailed menu 文言の全面移行

## src/ui/input/dispatch.rs
- [x] key/pointer から action 解決
- [ ] 未実装 action の no-op 整理

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
- [x] viewer / plugins / resources / render / window / navigation タブ
- [x] 閉じるボタン
- [x] 即時適用
- [x] manga separator 設定
- [x] window theme 設定
- [+] plugin 設定画面の土台
- [+] plugin search path 編集
- [+] plugin module load test ボタン
- [x] save path 記録設定
- [+] 適用/undo/初期化ボタン
- [ ] キーバインド編集 UI

## src/ui/menu/fileviewer/state.rs
- [x] filer state の分離
- [x] root drive 管理
- [x] view mode / sort / filter / URL input state
- [x] `available_roots` の曖昧 import 解消

## src/ui/menu/fileviewer/worker.rs
- [x] `FilerCommand / FilerResult`
- [x] directory scan の worker 分離
- [x] metadata 収集
- [x] sort / filter / ext filter / dir separate
- [ ] lazy load の段階化
- [ ] OS 準拠 name collation の強化

## src/ui/menu/fileviewer/thumbnail.rs
- [x] サムネイル worker
- [x] virtual zip/listed file のサムネイル生成
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
- [x] サブファイラー下部表示
- [x] サブファイラー閉じるボタン
- [x] 詳細表示で更新日時とサイズを表示
- [x] ファイル選択時に filer を閉じる
- [*] filer 表示時のさらなる高速化
- [ ] SVG アイコン化
- [ ] Copy / Move / Trash / Convert


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
- [ ] preload queue 連携

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
- [*] 起動時の manga Fit 再計算の実機確認
- [*] filer 表示時の manga レイアウトは実機で継続確認
- [ ] app 起動時の初回 decode 完全 worker 化
- [ ] preload queue
- [ ] message UI 整理

## src/drawers/affine.rs
- [x] resize / interpolation 実装
- [ ] resize 品質と速度の細かな切り替え

## src/drawers/image.rs
- [x] image load
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
- issue:【優先】保存: キャンセルボタンが効かない
- issue:【優先】zip内で次の画像表示が止まる画像がある(大きめのファイルが挟まる止まる。Filerで選択すれば移動可能)
- issue:ファイラー:http/httpsからロード出来ない
- ファイラー: SVG表示 resources/icons/icons.mdにテンプレが置いてあるのでそれでsvg iconをまず生成してください
- ファイラー: 文字をsvgに置き換えてください
- ファイラー: サムネイル 「フォルダ/アーカイブ」が大きくてフォルダ名が読めない
- ファイラー: サムネイル 真ん中を不要なペインがサムネイルを隠ぺいしているので削除
- zip: 日本語が文字化けする(恐らくエンコードがsjisのケースなので自動判別して変換/それでも化けるなら諦める)
  - crate encoding_rsを利用
- ファイラー: サムネイル ペインが小さい。可変にするか全画面表示
- プラグイン: 実装続き
    - load moduleで止まっている jpeg2000/avifが開けない ./samplesにサンプルあり
    - search path ファイルダイアログが欲しい(保存と同じで良い)
    - test/plugins/susie64, test/plugins/ffmpeg の実ファイルに合わせた runtime 実装
    - ffmpegのリンクはcrate ffmpeg-sysを考慮(自力実装の方が安定する可能性あり)
    - systemにserach pathは不要 OS APIを叩くため
- 設定：OSに拡張子を登録できる機能
- 設定：適用/undo/初期化ボタンの本格化
  windows は crate winreg を使う
  ProgID / open command / OpenWithProgids / OpenWithList / `--clean system`
- `src/ui/viewer/mod.rs` の state 分離を進めて `ViewerApp` をさらに薄くする
- `src/ui/menu/fileviewer/worker.rs` に lazy load / incremental snapshot を入れて大規模フォルダを高速化する
- `src/ui/i18n/mod.rs` を JSON resource loader に拡張し、未ローカライズ文言を全面移行する
- `src/dependent/plugins/*` に実ランタイムを足して system / ffmpeg / susie64 の優先順位解決を実装する
- todo.mdの更新
