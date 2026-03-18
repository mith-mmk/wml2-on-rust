# wml2viewer TODO
ステータス
- [x] チェック済み（人力修正）
- [+] 実装済み（タスクを行ったら修正すること） 
- [-] 実装遅延
- [*] issues(実装されているがバグがある) 
- [ ] 未実装

## 0. 現在の到達点

### 基本表示
- [x] `wml2viewer <file>`
- [x] `wml2viewer <directory>`
- [x] 静止画表示
- [x] アニメーション表示
- [x] スクロール表示
- [x] ScreenFit / zoom 表示
- [x] ダブルクリックで `100% <-> Fit` トグル
- [x] ウィンドウタイトルを現在画像に追従
- [x] 起動時の表示サイズ
- [x] 起動時の表示位置ずれ修正
- [x] 前回の表示サイズ保存（設定で切り替え）
- [+] フルスクリーン時のワークアラウンド
- [*] フルスクリーンモードの復帰
- [ ] VSP の読み込み(DATの判別できない、ファイル構造まで見る必要あり)

### 入力
- [x] `+` / `-` で zoom in / out
- [x] `Shift+0` で 100%
- [x] `Enter` で fullscreen toggle
- [x] `Shift+R` で reload
- [x] `Space` / `Right` で次画像
- [x] `Shift+Space` / `Left` で前画像
- [x] `Home` で先頭画像
- [x] `End` で末尾画像
- [ ] `Shift+G` で grayscale toggle
- [x] `Shift+C` で comic mode toggle

### navigation / filesystem
- [x] 単一ファイル起動時に親ディレクトリの画像一覧を取得
- [x] `STOP`
- [x] `LOOP`
- [x] `NEXT`
- [x] `RECURSIVE`
- [x] `RECURSIVE`: 親へ登りながら次の枝を探す DFS 風探索
- [x] filesystem worker 分離
- [x] render worker 分離
- [x] ディレクトリ単位 cache
- [x] ネットワークフォルダでの待ち時間(～100)
- [x] ネットワークフォルダでの待ち時間(～1000)
- [ ] フォルダの直下に画像ファイルが無いとき終了する問題の修正（"RECURCIVE"の場合は探してください）

#### 優先度中
- [ ] ネットワークフォルダでの待ち時間をさらに短縮(1000～1万)

### viewer / render
- [x] `viewer.align`
- [x] `viewer.background.color`
- [x] `viewer.background.tile`
- [x] `viewer.animation`
- [x] `render.zoom`
- [x] `render.zoomMethod`
- [-] 縮小時 pixel minimize
- [ ] `render.orientation`
- [ ] `render.transpearent`
- [ ] 巨大な画像で落ちる問題（分割テクスチャなどで回避）

#### 優先度低
- [ ] `render.rotation`
- [ ] `render.flap`
- [ ] `render.flip`
- [ ] `render.monochrome`

## 1. 最優先

### 1-0. 最優先のIssue
- [+] 動作時のウィンドウ表示位置のズレ

### 1-1. ネットワークフォルダ高速化
- [ ] `DirectoryListing` を clone 前提ではなく参照前提にして無駄コピーを減らす
- [ ] `RECURSIVE` の subtree 探索で sibling ごとの深掘りを途中結果 cache できるようにする
- [ ] `SetCurrent` 後の同一フォルダ一覧ロードを段階化する
- [ ] `scan_directory_listing` の sort コストを再確認し、必要なら `os_name` 実装時に整理する
- [ ] 実測用の簡易 timing log を入れて、network share の律速点を切り分ける

### 1-2. キー入力の整理
- [x] `input.key_mapping` 用の内部 action を作る
- [x] デフォルトキー設定を定義する
- [x] egui input から action dispatch する
- [x] SPACEをプレスしたままの状態だと画像が表示されないので適度なWAITを入れる
- [+] メニュー表示中にビューアーイベントが透過しないようにする
- [-] シングルクリックで次の画面を表示（上が実装された後）
- [ ] 未実装 action を no-op として整理する
- [x] `PageUp` / `PageDown` のフォルダ移動
- [ ] `F1` help
- [x] `P` setting
- [*] 左クリックで簡単なメニュー
- [ ] タッチパネルUI（低優先度）

### 1-3. 設定画面の先行タスク
- [x] option menu の土台
- [x] viewer / render / window の編集 UI
- [ ] 適用とキャンセルボタン

### 1-4. config 永続化
- [x] `configs/config.rs` を実装に接続
- [x] config load
- [x] config save
- [x] 設定画面の現在項目を config に接続
- [x] keep window state
- [x] config import/export
- [x] config option --config [path] 
- [x] デフォルトパスの設定 macos, linux, windowsは、crate dirctoryで設定
- [-] デフォルトパス、それ以外

### 1-5. ファイラー
 - [*] OS非依存のファイラー（最小版）
 - [+] フォントファミリー(i18n対応)
 - [+] フォントサイズの選択
 - [ ] サムネイル表示機能、ソート機能
 - [ ] サムネイルの永続化 crate dirctoryで設定
 - [ ] レスポンシブ対応デザイン
 - [ ] Function Copy File
 - [ ] Function Move File
 - [ ] Function Trushed File
 - [ ] function convert format
### 1-5-1. サブファイラー
 - [ ] 画面の下側に表示されるサブファイラー（ページ移動用）

### 1-5-2. クラウドファイラー(Android, iOS専用)
 - [ ] ネットワークマウント

### 1-6. CI/CD
- [ ] Auto Builder
- [ ] Windows x64(Win10/11)
- [ ] MacOS Intel
- [ ] MacOS Arm
- [ ] Linux Win
- [ ] Linux Arm
- [ ] Android
- [ ] iOS
- [ ] iPad

### 1-7. サムネイル
 - [ ] 独自サムネイル index作成（Bloom filter方式を検討　サムネイルのローカル保存は保留）
 - [ ] OS Indexキャプチャ
 - [ ] サムネイルのクリア機能（設定画面）
 - [ ] --clear chache

## 2. viewer / render

### 2-1. viewer
- [ ] `viewer.fade`
- [ ] 背景描画と texture 表示の責務整理

### 2-2. render
- [ ] drawers に変換パイプライン入口を作る
- [ ] 再描画時に毎回 full resize しない cache 方針
- [ ] resize 品質と速度の切り替え方針
- [ ] メッセージUI

## 3. ファイル探索 / ListedFile

### 3-1. filesystem 基盤
- [x] `file` protocol
- [x] sort order を `os_name` / `name` / `date` / `size` で切り替えられるようにする
- [ ] filter 条件
- [-] archive option (`FOLDER` / `SKIP` / `ARCHIVER`)
- [x] directory scan を openable entry 前提にして `.wml` を拾う
- [x] 仮想化ファイルシステム(ListedFile, zip用)
- [x] `.zip` も同じ openable/archive mode に接続
- [ ] （検討中）キャッシュのシリアライズ

### 3-2. ListedFile
- [-] `.txt` / ListedFile parser
- [x] `.wml` + `#!WMLViewer2 ListedFile 1.0` を判定
- [x] フォルダ区切り `\\` / `/` を許可
- [x] 相対 path 行を file entry として読む
- [x] 相対 path の基準を ListedFile 親ディレクトリにする
- [x] コメント行 `#` を無視
- [-] `@command` / `@(...)` は予約語として parse

### 3-3. ZippedFile
- [-] feature で有効/無効を切り替えられるようにする
- [x] まず `zip` を読む
- [x] `navigation` / `filesystem` から folder 相当として扱えるようにする
- [ ] index読み込みとファイル読み込みの分離
- [ ] zip entry sort
- [ ] zip encoding option
- [ ] `gzip`
- [ ] `lzh`
- [ ] `7z`
- [ ] `rar`（最低限）

## 4. 非同期実装の整理
- [x] render worker
- [x] filesystem worker
- [ ] app 起動時の初回 decode も完全に worker 化する
- [ ] load / resize / filesystem request の state 管理を整理する
- [ ] preload queue と連携できる構造にする
- [ ] ライブラリの分割(coreとuiに分ける)

## 5. 画像表示とディレクトリ操作の分離

- [ ] 画像表示 state と file list state を別 struct に分ける
- [ ] loader と filesystem の依存方向を一方向にする
- [ ] `app.rs` を composition root にする

## 6. 先読みデコード

- [ ] 次画像 1 枚先読み
- [ ] preload queue struct
- [ ] UI thread と decode thread の受け渡し
- [ ] animation を含む preload サイズ制御

## 7. マンガモード

- [+] 横長時 2 ページ表示条件
- [+] `r2l` / `l2r`
- [ ] partition 描画
- [ ] サムネイル起点のページ移動
- [+] `Shift+C` toggle
- [ ] @ script

## 8. 設定画面

- [x] 1-3 / 1-4 が終わったら import/export をつなぐ
- [*] 左クリックメニュー
- [x] 設定画面表示時のメインペインへのイベント貫通防止
- [+] 閉じるボタン
- [ ] キーバインドUI


## 10. リソース

- [ ] 日本語/英語 resource のキー設計
- [ ] 外部 resource 読み込み

## 11. OSサポート
- [x] Windows Support
- [ ] Linux Support
- [ ] Mac OS Support
- [ ] Android Support
- [ ] iOS Support 
- [ ] インスーラー(windows)
- [ ] アンインストーラ(windows)

## 12. 以降

- [ ] OS dependent encoder/decoder(about avif, hfif, GPU enc/dec)
- [ ] plugin encoder/decoder
- [ ] offline cache
- [ ] filter function search similar images
- [ ] filter function datetime
- [ ] filter function filename
- [ ] filter function extentions
- [ ] metadata cache
- [ ] filter function metadata
- [ ] network protocol `http`
- [ ] `smb`
- [ ]  `cloud` cloud drives
- [ ] key remap UI
- [ ] OS dependent function
- [ ] command / external command
- [ ] WMLScripts

## 次に着手する候補
　途中で停止しないこと。一括で実装。
  デフォルトconfigパスは、create directoryで実装しなおしました dependent/thirdpartyのした

1. リソースの追加(システムfontの切り替え)
   1. system言語を検出
   2. locale fontを設定
   3. fallbackを設定(絵文字フォントも指定)
   4. 例：日本語の場合、Windows10は"Yu Gothic UI"、 Macは"ヒラギノ角ゴシック" -> NotoSansJP -> NotoSansCJK -> 英語デフォルトにフォールバック
   5. フォントサイズの設定 Auto(画面サイズとDPIから計算), S, M, L, LL (現在のフォントサイズはS相当)
2. Issueの修正, ファイラー：日本語が化けるバグ（Fontの問題）
3. viwerに含まれているファイラー部分をui/menu/fileviewerに、設定部分をui/menu/configに分離、入力関係はui/inputに分離
   画像表示部分はui/renderに分離
4. Issueの修正, ファイラー：表示が遅い問題
5. Issueの修正, viewer：遅くなっている問題（ファイラーに引きずられている可能性、ファイラーのUIもしくはバックエンドをワーカー分離）
6. Issueの修正, ZIP：二枚目以降が表示されない場合がある
7. Issueの修正, ZIP：FitScreenなどの指示が無視されるケースがある
8. Issueの修正, マンガモード:表示が中央にアラインされない（右端にアラインされる）
9.  Issueの修正, Viewer:ワークアラウンド：フルスクリーンモードで起動すると表示がおかしい。起動時は無効化
10. todo.mdの整理 todo.mdと実装を比較して終了しているタスクには[+]を付ける
11. ファイラー：一覧表示、サムネイル表示（大・中・小）、詳細表示、の追加と切り替えボタン
12. ファイラー：UIにメタデータも表示出来る様にして、ソートボタンにする
13. 設定：閉じるボタンの実装
14. CTRL+S でファイル保存（保存用ディレクトリを選べる様にする）
15. 外部プラグインの実装：susie plugin の実装とplugin conifigの実装
    1. プラグインの有効化
    2. プラグインの優先順位（内部より上、下）
