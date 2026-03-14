# wml2viewer TODO

## 0. 現在の到達点

### 基本機能(キーボード+マウス)
- [x] webp/jpeg/bmp/gif/png/tiff/MAG/MAKI/PI/PIC の読み込み
- [ ] VSP の読み込み
- [x] 静止画表示
- [x] アニメーション表示
- [x] ScreenFit リサイズ
- [x] ダブルクリックで 100%
- [x]  ダブルクリックで 100% <-> Fit のトグル
- [ ] 起動時の表示位置ずれ修正
- [x] スクロール
- [x] wml2viewer ファイル名
- [ ] wml2viewer フォルダ名

## 1. 最優先: キー操作

### 1-1. 最低限の固定キー
- [x] `+` / `-` で zoom in / out
- [ ] `0` または `Shift+0` で 100%
- [x] `Enter` で fullscreen toggle
- [ ] `Shift+R` で reload
- [ ] `Shift+G` で grayscale toggle
- [x] `Space` / `Right` で次画像
- [x] `Shift+Space` / `Left` で前画像
- [x] `Home` で先頭画像
- [x] `End` で末尾画像

### 1-2. キー入力の設計
- [ ] `input.key_mapping` 用の内部 enum を作る
- [ ] デフォルトキー設定を定義する
- [ ] egui input から action dispatch する層を追加する
- [ ] 未実装 action は no-op で扱う

## 2. viewer / render

### 2-1. viewer
- [x] `viewer.align`
- [ ] `viewer.background.color`
- [ ] `viewer.background.tile`
- [ ] `viewer.fade`
- [x] `viewer.animation`

### 2-2. render
- [x] `render.zoom`
- [x] `render.zoomMethod`
- [ ] 縮小時 pixel minimize
- [ ] `render.orientation`
- [ ] `render.rotation`
- [ ] `render.flap`
- [ ] `render.flip`
- [ ] `render.monochrome`
- [ ] `render.transpearent`

### 2-3. viewer と render の整理
- [ ] `ViewerApp` から表示状態と描画設定を分離する
- [ ] `drawers` に変換パイプライン入口を作る
- [ ] 再描画時に毎回 full resize しないキャッシュ方針を決める

## 3. ファイル探索 / ListedFile

### 3-1. filesystem 基盤
- [ ] `filesystem` モジュールに `file` protocol を実装
- [ ] 単一ファイル起動時に親ディレクトリの画像一覧を取得
- [ ] sort order を `os_name` / `name` で切り替えられる形にする
- [ ] async ロード基盤
- [ ] 待ち時間の最短化
- [ ] Waitキャッシュ

### 3-2. ListedFile
- [ ] `.txt` / listed file parser を作る
- [ ] コメント行 `#` を無視する
- [ ] path 行を file entry として読む
- [ ] `@command` / `@(...)` は予約語として parse だけ行う

### 3-3. ナビゲーション
- [ ] 次画像 / 前画像 API
- [ ] 先頭 / 末尾 API
- [ ] `navigation.end_of_folder` の `STOP` / `LOOP` 実装
- [ ] `RECURSIVE` は最小版の仕様を決めてから実装

## 4. 表示とディレクトリ操作の分離

- [ ] 画像表示 state と file list state を別 struct に分ける
- [ ] loader と filesystem の依存方向を一方向にする
- [ ] `app.rs` を composition root にする

## 5. 先読みデコード

- [ ] 次画像 1 枚先読み
- [ ] preload queue の struct を作る
- [ ] UI thread と decode thread の受け渡しを決める
- [ ] animation を含む場合の preload サイズ制御

## 6. マンガモード

- [ ] 横長時 2 ページ表示条件を定義
- [ ] `r2l` / `l2r` を切り替える
- [ ] partition 描画
- [ ] サムネイル起点のページ移動設計

## 7. 設定画面

- [ ] option menu の土台
- [ ] viewer/render/window の編集 UI
- [ ] 適用とキャンセル

## 8. 設定に付随する機能

- [ ] config import/export
- [ ] keep window state
- [ ] runtime current file snapshot

## 9. リソース

- [ ] 日本語/英語 resource のキー設計
- [ ] 外部 resource 読み込み

## 10. 以降

- [ ] filer
- [ ] network
- [ ] OS dependent
- [ ] plugin
- [ ] key remap
- [ ] command / external command

## 次に着手する候補

1. 固定キーの action 層を入れて `zoom` / `fullscreen` / `reload` を動かす
2. `filesystem` に単一フォルダ列挙を入れて next/prev image を有効化する
3. `viewer.background.color` と double click toggle を入れて表示体験を固める
