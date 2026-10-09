# Q4 境界処理と検証状況

対象は `codex/p2-q4-boundaries`。baseはmainの `187ef7cb72cdaf8d98550ecc0e5daf58920a7646` で、TGA/PCX/DDS PR用branchとEX branchのcommitを含めません。

## 今回の修正

- RLE runがblockの宣言画素数を超えるとき、黙って切り詰めずエラーにします。`[0x10, 1, 0]` は17画素のrunで、16画素のblockに対する成功扱いをregression testで再現しました。
- 固定640×400 RGBA出力のpixel/byte予算を、header検証後・palette/dictionary解凍前に検査します。小さすぎる予算で約65KBの辞書scratchを確保する順序を修正しました。
- Q4が使う共通retro入力helperで、入力予算をu64のまま検査してからusize変換・入力確保を行います。共通helperを直接呼ぶ他retro形式にも適用されます。故意にreadを失敗させるreaderで、予算違反がreadより先に拒否されることを検証します。
- ローカル実画像testは既定で明示的にignoredです。明示実行時に環境変数や画像がなければ失敗し、未配置の早期returnを成功に見せません。`noretoro`ではQ4 integration test自体を無効にします。

## 実装の契約と未確認の仕様

現在のdecoderは固定640×400、16色、22byte header、`MAJYO` signature、4bit palette値×17のRGBA8（alpha255）を扱います。Q4 encoderやQ4D説明text処理はありません。

この受理範囲と圧縮方式は既存実装・解析に基づき、独立converterによる全画素一致は未確認です。declared file sizeのzero指定・末尾追加データ、辞書の`next > current`による終了など、一般仕様が未確認の挙動は今回変更していません。

公開28画像の168 image blockを観察した結果、全blockに11byteの未消費RLE末尾があり、bit decoderは末尾に4〜17bitのsynthetic zeroを利用します。既存のzero paddingと未消費末尾を禁止するとcorpusを壊すため、維持します。この観察scriptはWML2の処理を再現したもので、独立oracleではありません。今回のrun超過拒否は受理を厳格化しますが、28画像には超過runがありませんでした。

辞書storageと解凍blockは最大`0xfdf0` bytes、code数とbit幅にも既存上限があります。初期辞書参照・storage枯渇・code数・bit幅・palette値・短いheader/auxiliary payload・空block・短いRLE・出力予算の境界testを追加しました。予算は個別bufferの制限で、全bufferを合算したpeak heap/RSS上限ではありません。

## 検証

- 修正前に2件のregression testが失敗し、修正後は成功しました。
- Q4 unit/integrationは実サンプルを配置して31件成功。sample testは28画像を実際に復号しました。
- retro feature集合: 66件成功、sample test1件ignored。
- Q4＋high-bit-depth: 50件成功、sample test1件ignored。
- Q4＋noretoro: 15件成功。sample未配置での明示実行は期待どおり失敗しました。
- Clippy、format、diff check、wasm32 compile/checkに成功。wasm実行は未確認です。
- 28画像の修正前後のRGBA全画素が一致し、表示も確認しました。自己比較と復号成功は互換性の独立証明とは扱いません。

入力出所: [RECOIL ticket #14](https://sourceforge.net/p/recoil/bugs/14/attachment/XLD4_images.zip)。取得archive SHA256は `c4c9c8f2f4258d72648c2ec9f30c37b4f62568835530e1bde3d322f45b56e396`。画像、復号画素、GPL実装はrepoに同梱せず、ローカルの`.test-q4-*`検証物に保持します。

RECOIL 6.4.5による独立PNG比較は未実行です。入力/出力reportは28件とも`oracle_missing`、`compatibility_pass=false`です。ネットワーク・support対応は今回再試行していません。

2026-10-09追補: 親環境が正式なSediment `download_file`でユーザーの直接添付 `file_000000003e008209a293b213afcf1868`（`recoil-2png_6.4.5-1_amd64.deb`）を取得し、サイズ179580byte、SHA256 `3248d027c1ceb894aba180c6b6fbd899197d4d4cf440e63ff2cbf877fc0901f3` と公式掲載値の一致を報告しました。このハッシュは親環境での確認結果であり、WML2作業環境への配置・展開・実行は行っていません。28画像の独立比較は引き続き未完了です。

## 性能測定

Rust1.91 release、CPU固定、base/head順序を交互にした12 paired run/画像/CPU。各processで10回warmup後、100回decodeを計測しました。baselineは変更前Q4を含む`8d00e2c`で、共通入力予算検査はfinalと同じです。これは比較用binaryの出所であり、このbranchの祖先ではありません。全pairで出力hash/sizeは一致しました。

| 入力 | CPU0 median差 | CPU1 median差 |
|---|---:|---:|
| 30000.Q4 | +1.79% | +1.22% |
| ALIPHA_M.Q4 | +0.51% | -0.00% |
| MA3.Q4 | +0.33% | +2.20% |

負数は高速化、正数は遅延です。共有hostで小さい差を測定しており、ゼロ回帰とは主張しません。実時間・四分位・入力/source/binary SHA256をローカル`.test-q4-validation/benchmark.json`に保持します。
