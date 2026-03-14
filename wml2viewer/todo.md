# wml2viewer TODO

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
- [ ] 起動時の表示位置ずれ修正
- [ ] 動作時のウィンドウサイズのズレ
- [ ] VSP の読み込み

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
- [ ] `Shift+C` で comic mode toggle

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
- [ ] フォルダの直下に画像ファイルが無いとき終了する（"RECURCIVE"の場合は探してください）

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
- [-] シングルクリックで次の画面を表示
- [ ] 未実装 action を no-op として整理する
- [x] `PageUp` / `PageDown` のフォルダ移動
- [ ] `F1` help
- [x] `P` setting
- [ ] 左クリックで簡単なメニュー

### 1-3. 設定画面の先行タスク
- [x] option menu の土台
- [ ] viewer / render / window の編集 UI
- [ ] 適用とキャンセル

### 1-4. config 永続化
- [ ] `configs/config.rs` を実装に接続
- [ ] config load
- [ ] config save
- [ ] 設定画面と config を接続
- [ ] keep window state
- [ ] runtime current file snapshot

## 2. viewer / render

### 2-1. viewer
- [ ] `viewer.fade`
- [ ] 背景描画と texture 表示の責務整理

### 2-2. render
- [ ] drawers に変換パイプライン入口を作る
- [ ] 再描画時に毎回 full resize しない cache 方針
- [ ] resize 品質と速度の切り替え方針

## 3. ファイル探索 / ListedFile

### 3-1. filesystem 基盤
- [x] `file` protocol
- [ ] sort order を `os_name` / `name` / `date` / `size` で切り替えられるようにする
- [ ] filter 条件
- [ ] archive option (`FOLDER` / `SKIP` / `ARCHIVER`)

### 3-2. ListedFile
- [ ] `.txt` / ListedFile parser
- [ ] コメント行 `#` を無視
- [ ] path 行を file entry として読む
- [ ] `@command` / `@(...)` は予約語として parse

### 3-3. ZippedFile
- [ ] feature で有効/無効を切り替えられるようにする
- [ ] まず `zip` を読む
- [ ] `navigation` / `filesystem` から folder 相当として扱えるようにする
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

- [ ] 横長時 2 ページ表示条件
- [ ] `r2l` / `l2r`
- [ ] partition 描画
- [ ] サムネイル起点のページ移動
- [ ] `Shift+C` toggle

## 8. 設定画面

- [ ] 1-3 / 1-4 が終わったら import/export をつなぐ

## 9. 設定に付随する機能

- [ ] config import/export

## 10. リソース

- [ ] 日本語/英語 resource のキー設計
- [ ] 外部 resource 読み込み

## 11. OSサポート
- [x] Windows Support
- [ ] Linux Support
- [ ] Mac OS Support
- [ ] Android Support
- [ ] iOS Support(先にMAC買わないと行けない) 

## 12. 以降
- [ ] filer
- [ ] network protocol (`http`, `smb`, `cloud`)
- [ ] OS dependent
- [ ] plugin
- [ ] key remap UI
- [ ] command / external command

## 次に着手する候補

1. config load/save
2. Windowの表示位置の固定化
3. `navigation.sort`
4. ListedFile parser
5. ZippedFile の最小版（zip）
