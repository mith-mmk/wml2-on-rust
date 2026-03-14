優先順位
1. キー操作の機能を最優先
2. viewerとrender(background, zoom)
3. ファイル探索機能、リステッドファイル
4. 画像表示とディレクトリ操作は分離する
5. 画像先読みデコードを実装にする
6. マンガモード（サムネイルをみてページ移動する機能）
7. 設定画面
8. 設定に付随する機能
9. リソース
10. ファイラー
11. ネットワーク機能
12. OS依存機能
13. プラグイン
14. キー操作の変更
15. コマンド（file nameとかfile copyとか）, external commandとか

```jsonc
{
  "viewer": {
    "align": "center", // 画像の配置 "center", "right-up", "left-up", "right-down", "left-down", "left", "right", "up", "down"
    "background":  {"color": "#000000" }, // or , {"tile": {"color1": "", "color1": "", "size":"16x16"}}, 
    "fade": false, // bool フェードイン・アウト
    "animation": true, // bool アニメーションを有効にするか
  },
  "window": {
    "fullscreen": false, // bool モバイルでは無効
    // 起動時のwindowの位置 モバイルでは無効
    "start_position": "center", // || { "start_x": 0, "start_y": 0}  
    // 起動時のwindowのサイズ
    "size": "80%",  // {"width": int,"hight": int}
    "keep": false, // 終了時の状態を維持する
  },
  "render": {
    "zoom": "FitScreen",　// ZoomOption
    "zoomMethpod": "bilinear", // "bicubic", "bilinear", "nearest neighbor", "lancos3"
    // 縮小は、基本pixel minimize
    "orientation": true, // メタデータの回転情報を反映させるか
    "roatation": 0, // 画像を回転(degree)
    "flap": false, // 上下反転
    "flip": false, // 左右反転
    "monochrome": false, // モノクロモード
    "transpearent": "background", // "override" , "ignore", "background" // 透過色の扱い、 override 前の画像に上書き、 ignore 無視、 background （背景色に置き換え）
  },
  "reading": {
    "manga": {
      "enable": false, //コミックモード 横長の場合、最大2枚表示
      "r2l": true, // コミックモードの時、右から左か左から右か
      "partition_size": 0, // px
      "partition_color": "#000000",
      "partition_effect": "shadow" // "shadow"  , "solid",...
    },
    "slideshow": {
      "enable": false,
      "wait_time": 1.0, // sec
      "next_foloder":  "STOP" // フォルダの最後の時の挙動、 "STOP", "NEXT", "LOOP", "RECURSIVE"    
    }
  },
  "navigation": {
    "sort": {
      "sort_by": "os_name", // "date", "size", "name", "win_name", "linux_name", "os_name", "none", "random" nameは通常の名前sort、os_nameはosによる名前ソート
      "order": "asc", // "asc", "desc"
      "filter_by": ""　// filter condition
    },
    "end_of_folder": "RECURSIVE", //FolderOption, // フォルダの最後の時の挙動、 "STOP", "NEXT", "LOOP", "RECURSIVE"
    "archive": "FOLDER",// ArchiverOption "FOLDER"（フォルダの用に扱う）, "SKIP"（読まない）, "ARCHIVER"（複数画面の画像フォーマットの用に扱う）
  },
  "thumbnail": { // サムネイル
      "enable": false,
      "os_thumbnail": false, // OSのサムネイルがあれば横取りする
      "cache": { // サムネイルキャッシュ
        "enable": false, // 有効化
        "path": "default", //サムネイルの場所 defaultはOSデフォルト フォルダ内にサムネイルキャッシュは置かない
        // windowsの場合は %APPDATA%\Local\wml2viewer\cache\
      },
      "size": 64, //サムネイルのサイズ size x size (png)
  },
  "loader": {
    "max_size": 0, // 使用する最大メモリ（これを越える画像はリサイズロードする 0はOSの最大メモリの1/4）
    "split_file": true, // 複数画面が入って居る画像を順番に読み込むか true, 最初だけ表示するか false, アニメーション"animation":falseの場合にチェック
    "preload": true, // 先読みするか
    // ここから先は後でインプリ
    "os_decoder": false, // OSのデコーダを優先するか（後でインプリ）
    "plugin": false, // 外部プラグイン ffiで読み込む(winはsusie, macとlinuxはこれから考える)を有効にするか
    "ffmpeg": false, // プラグインにffmpegを使うか(ffmpeg dll/soを使う)
  },
  "storage": {　// 保存オプション
    "os_encoder": false, // OSのエンコーダを優先するか
    "plugin": false, // 外部プラグイン ffiで読み込む(winはsusie, macとlinuxはこれから考える)を有効にするか
    "ffmpeg": false, // 保存のプラグインにffmpegを使うか(ffmpeg dll/soを使う)
    "default": false, // 保存時デフォルトで使うフォーマット 省略時 png, png, bmp, tiff,jpeg, webp
    "png_option": null, // (default optimize=6), optimize = 0-9, exif = copy, none
    "jpg_option": null, // default quality=80, quality = 0 - 100 , exif = copy, none
    "bmp_option": null,　// default compless = none,　now constructions
    "tiff_option": null, // default compless=none,  compless = none, LZW, JPEG, exif = copy, none
    "webp_option": null, // default compless = lossy, quality=80, optimize=6,  compless lossless, quality = 0 - 100(lossy only), optimize = 0-9(default 6), exif = "copy", "none"
  },
  "FileSystem": { // FileSystemは画像ローダと分離して動くため、messageでやりとりする（PerfectViwer遅い理由はファイルクロールなので）
    "protocol": ["file", "zip", "ListedFile"], // 有効にするプロトコル "file"(must), "http", "smb"(モバイルのみ), "cloud"(モバイルのみ), "zip", "7z","ListedFile"
    "zip_encoding": "AUTO" // SJIS, Unicodeを自動判別 他 "CP932", "UTF-8",....
  },
  "input": {
    "key_mapping": {}, // {"key": "fanction"} ... 未指定はデフォルト KEYはJava Script準拠
    /*
      o ... Open (FileDialog)
      Shift+R ... Reload
      + .. Zoom Up
      - .. Zoom Down
      Space ... next image
      right allow ... next image
      Shift + right allow ... only 1page next (manga mode only)
      Shitf + Space ... prev image
      left allow .. prev image
      Shift + left allow ... only 1page prev (manga mode only)
      page up ... next folder
      page down ... prev folder
      home .. 1st image
      end .. last image
      Shift+G .. Glaysacle toggle
      Shift+C .. Comic mode toggle
      enter .. full screen
      F1 .. help
      P .. setting

      // no assinged function
      file delete
      file move
      file copy
      run exec (use external_cmd)
      exit (os default)
      filter() 
      crop
      resize

    
     */
    "mouse_setting": {}, // {"key": "function"} ... 右、中、左クリック、ホイール、 4button以上に対応（できる？）
    "touch_setting": {} // {"key": "function"} ... タッチの場所でメニュー（perfect viewerを参考）
  },
  "runtime": {
    "resource_path": null, // リソースファイルの場所　デフォルトは実行ファイルの中 指定がある場合、外部リソースを優先（基本は言語リソース？）
    "current_file": "", // path like string, 現在のファイルの場所（スナップショット）
    "external ": { // 【要注意】 外部コマンドを利用する（pngを経由する） // 通常は無効
        "external_tmp": null, // 受け渡しに使うtmpフォルダ 無い場合は、環境変数 TMP -> TEMP の順で探す
        "external_cmd": [] // 外部コマンドのコマンドライン %i(入力名) %O(フォルダ) ...
    },
    "os_depend": { // OS依存の設定
    }
  }
  ```

ListedFile 以下の様なファイル
```txt
https://example.org/test.webp
\\pi4\data\images\sample.png
d:\data\images\sample.jpg
/home/user/images/sample.bmp
# コメント
@command
# @で始まるのはコマンド 複数行は@() で括る 実装予定だがまだ何も決まってない　取りあえず予約語
@(
 command1
 command2
 command3
 command4
)
```