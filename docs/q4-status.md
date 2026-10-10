# Q4 境界処理と検証状況

対象は `codex/p2-q4-boundaries`。baseはmainの `187ef7cb72cdaf8d98550ecc0e5daf58920a7646` で、TGA/PCX/DDS PR用branchとEX branchのcommitを含めません。

**横帯の原因は`ifxld4.spi`の逆アセンブルで確定し、decoderを修正済みです。** `0x10, high!=0, low`はpalette index 0でも前行コピーでもなく、同一image block内で直前に明示されたrun色の継続です。修正後はRECOIL 6.4.5の28画像とRGB全画素が一致し、代表画像の表示確認にも合格しました。以下には修正前の切り分け履歴も残します。

## 2026-10-10 横帯修正

- SHA256 `09181dd2511d4c8692be22ab7f431058a1a85629321cc3c77cfc8f6a6214801f` の`ifxld4.spi`（PE32 i386）を静的解析しました。詳細は[`ifxld4.spi`解析](q4-ifxld4-analysis.md)を参照してください。
- RLE routine RVA `0x16a0`はblock開始時に保持色を0へ初期化し、`0x10,0,color,high,low`でだけ保持色を更新します。literalは保持色を変えず、`0x10,high,low`は保持色を`high * 17 + low`画素出力します。前行・行境界・前blockの参照はありません。
- 修正前に新しい回帰testが失敗し、修正後はQ4 unit 35件、integration 4件成功・local sample 1件ignoredとなりました。
- RECOIL 6.4.5で28件の独立PNGを生成しました。deb SHA256は`3248d027c1ceb894aba180c6b6fbd899197d4d4cf440e63ff2cbf877fc0901f3`、展開した`recoil2png`は`4ade6d82068381dd1ea500324a564e8b3c764607e5b2de626f6122f5df7cd2a7`です。
- 修正前は28/28画像が不一致でした。修正後は640×400 RGBをresize・補間・ICC変換なしで比較し、28/28画像が全画素一致しました。
- corpusの168 blockには短縮runが21,794件あり、2,039件は行を跨ぎ、345件はblock先頭640画素内で始まります。保持色runとして一貫して処理でき、前行予測の初期化問題は発生しません。

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

## decoder追補検証

手計算ベクトルでMSB読み取り・3→4→5bit幅遷移、辞書の等値参照・既存entry再利用・最終entry出力を確認しました。既存の入力切れ時flushと`next > current`終了も挙動として明記し、一般仕様の証明とは区別しています。

literal-only streamと行別のpalette色から期待RGBAを別々に構成し、全640×400画素を比較しました。280＋280＋80画素で一行を作り、最初の280画素でblockを分割することで、行の途中のblock境界をまたいでも行配置が維持されることを確認します。短いblock header/payload、canvas不足・超過も確認しました。このベクトルはdecoder出力を期待値に使いませんが、現在の解析モデルの検証であり、独立converterの代替ではありません。

28実画像のheader/blockを別途走査すると、168blockの宣言画素数は48000または16000で、すべて行境界で終わり、全画像が256000画素を埋めます。auxiliary blockはありません。このためauxiliaryの正常系、一般的な辞書終了・padding、予約header/palette byteの意味はcorpusから確定できません。行途中のblock分割も一般仕様としての受理を証明するものではありません。

この追補はunit 35件＋integration 5件成功（ignored sample testを明示実行し28画像を復号）。新規5件のテストは未関与の担当による独立レビューを実施しました。再現する実装不具合は見つからず、production decoderの変更はありません。表示・benchmarkは前回のproduction変更時の結果を保持し、テストのみの追補では再実行していません。

## 横帯の原因調査（修正前の記録）

> この節の保持色修正前の記述は、`ifxld4.spi`を入手して逆アセンブルする前の調査履歴です。前行コピー・初行補完・block履歴の仮説は、上記の保持色run解析と全画素比較によって棄却されました。

同じ環境で保存済みPNGを確認し、非露骨な8画像を対象にしました。修正前の処理を別scriptで再構成したRGBは保存済み出力と一致し、観察を再現できました。非zero-repeat形式`[0x10, high, low]`（high != 0）は、当時`high * 17 + low`画素をpalette index0で埋めていました。位置を変えず、このrunだけを直前色で埋める仮説と、640画素前から順次コピーする仮説を比較しました。前行コピーではCACAO4の顔・髪、AEK_IKOIの輪郭、TOM093の背景などの帯が大幅に減りました。これは当時の診断画像であり、検証済みの修正結果ではありません。

| 画像 | 非zero-repeat数 | 修正前にindex0 runがある行数 | 前行コピー仮説との相違画素数 | 相違がある行数 |
|---|---:|---:|---:|---:|
| CACAO4 | 607 | 285 | 19438 | 263 |
| AEK_IKOI | 927 | 375 | 43899 | 377 |
| TOM093 | 177 | 127 | 8358 | 101 |

runは各画像の全6blockに分布しており、block継ぎ目だけの問題ではありません。CACAO4は先頭行x300から109画素、x437から26画素、次行x300から107画素のindex0 runがあり、位置や長さは変動します。単一の固定周期は確認していません。相違はRGBの複数成分に及び、alphaは255のままです。paletteを変更せずに帯が減るため、palette成分よりrunの画素源が優先調査対象です。

別構造の通常LZW（literal0..16、最初の新規entry17）で28 palette＋168 image streamを検査し、辞書展開byteが196件すべて一致しました。同値参照12282回を含みます。ただしstage1のbit解釈は共有しているため、独立形式oracleではありません。辞書破損の可能性を絞る内部整合性の証拠として扱います。

28画像全体には非zero-repeatが21794回あり、最大288画素、行跨ぎ2039回、最初の行内で開始54回、各block最初の640画素内で開始345回でした。前行コピー診断は「存在しない前行をindex0扱い」「blockを跨いでcanvas履歴を保持」を仮定していました。この段階では初行・block状態が未確定で、行跨ぎrunには順次コピーも必要でした。前行コピー説は未関与の担当も有力と判断しましたが、仕様または独立oracleの裏付けがなく、production修正を見送りました。当時の最小挙動は`[0x10,1,0]`が17画素をindex0で埋めることで、unit testにも記録していました。

診断script・相違位置・行統計・比較PNGはローカル`.test-q4-stripes/`、別LZW検査とrun境界の全件記録は`.test-q4-stage-audit/`に保持します。元の復号出力は上書きしていません。外部取得、Library再試行、Drive保存は行っていません。

### 初行・block履歴の追加切り分け

ローカルの元Susie plugin・逆解析資料・独立した形式仕様を探索しましたが見つかりませんでした。取得済みarchiveのQ4Dは作品説明で、命令仕様ではありません。header byte4は28件すべて1、byte2は27件が18・1件が17であり、corpusのmode多様性も乏しいため、headerに予測modeがある可能性を確定・排除できません。

全28画像について、前行コピーを仮定したうえで初行のzero補完／直前画素補完とblock履歴の持越し／リセットを比較しました。次の数は元decoderとの相違ではなく、「履歴持越し＋初行zero補完」モデルとの相違です。

| 代替の仮定 | 差がある画像 | 相違するpalette index数 |
|---|---:|---:|
| 履歴持越し＋初行を直前画素で補完 | 21/28 | 110729 |
| blockごとにzero補完から開始 | 28/28 | 114810 |
| blockごとに直前画素で補完から開始 | 28/28 | 221156 |

CACAO4では初行の135画素の選択が後続行へ伝播し、1662画素の差になります。AEK_IKOIでは264→11334、TOM093では410→2556画素です。block履歴をzeroリセットする差はそれぞれ1156、4539、370画素です。数値から仮定の重要性は示せますが、どちらが正しいかは判定できません。

`.test-q4-predictor-audit/predictor.py`に小ベクトルを分離しました。既存解析モデルのliteral/明示色/count計算は、`[3,16,0,5,1,2]`がliteral3＋色5の19画素run、`[16,1,2]`が19画素の非明示色runになることを確認します。未確定の予測モデルは、幅4で`3,copy2,2`が初行zeroなら`[3,0,0,2]`・直前色なら`[3,3,3,2]`、前行`[1,2,3,4]`から6画素copyなら`[1,2,3,4,1,2]`、次blockの4画素copyなら持越しとリセットで`[1,2,3,4]`と`[0,0,0,0]`に分かれることを確認します。4件のmodel unittestが成功しました。これは形式互換の合格testではなく、仮定を混同しないための数学的な切り分けです。全件結果は同directoryの`results.json`です。

この段階では、命令意味、初行予測値、block間履歴の3点に独立な根拠が不足しており、正式なRLE修正は行っていませんでした。

ユーザーの不合格判定を受け、実画像testの30000.Q4の4画素固定期待値を外しました。これらは旧decoder由来であり、不正確な出力を正解として固定する恐れがありました。実画像testはサイズ・opaque alpha・復号のsmoke testに限定しました。手計算のliteral-only期待値は実画像出力に依存しませんが、この段階では予測命令の正しさを検証していませんでした。非zero-repeatのzero埋めunit testも当時の挙動の記録で、仕様oracleではありませんでした。

非露骨な代表3画像について初期化variantを別々に表示しました。CACAO4の初行zeroモデルにはx300から109画素・x437から26画素の黒い区間が残り、直前画素補完では変わりました。block resetモデルにはrow75のx441から25画素などで差が再発し、後続行に伝播しました。元のblock境界はrow75/150/225/300/375です。これらの対応位置は追跡できましたが、見た目の改善だけで初期化規則を採用しませんでした。比較PNGは`.test-q4-stripes/*-fallback-variants.png`で、すべて診断用と明記しました。実画像smoke 5件は成功しましたが、この段階では視覚検品の不合格は解消していませんでした。

### opcode・位置・符号・countの切り分けと引き継ぎ

CACAO4のrow1 x300の帯は、RLE byte範囲`[96,99)`の`[16,6,5]`に対応します。countは107、出力範囲は`[940,1047)`です。これを出力した圧縮code50〜52はpayload内bit範囲`[295,313)`（幅6）で、synthetic paddingを使用していません。前行コピー仮説の参照範囲は`[300,407)`です。payloadのファイル開始位置は67で、opcodeそのもののファイルbyte位置という意味ではありません。traceは辞書entryを出力したcodeの位置を記録し、entry構築の全依存関係は追跡しません。

28画像の168 blockでtoken/high/lowは最大16、colorは最大15でした。signed8の符号拡張との差はなく、このcorpusの帯を説明しません。宣言画素量に一致した既存token列を固定し、全runを一律−1すると、末尾zeroもliteralとして使ってなお167/168 blockで不足します。＋1や形式別±1も一致を支持しません。これは別解釈で圧縮入力を再解析した互換性検証ではありません。色付きrunも含めた行跨ぎは7024回あり、行境界で禁止する変更にも根拠がありません。palette index順の変更は色を変えますが、index0 runの位置・長さは変えず、帯の画素源を解決しません。

この段階では論理的な位置ずれが未解決でした。panicなし／有効な配列範囲への書き込みでも、誤った画素源を選べば不正確な画像になります。既存の予算・run超過検査を保ち、予測命令の意味・初行とblock履歴を優先しました。数値診断、トレース、RGBA保存例、独立PNG比較の可搬な手順を[ローカル引き継ぎ](q4-local-handoff.md)に用意しました。外部画像・バイナリは同梱せず、保存・画像提示も停止していました。

可搬auditは4件のmodel testと28画像の実行に成功しました。RGBA保存例はrelease build成功、28入力の保存とoracle未配置のfalse判定を確認しました。比較helperは合成fixtureの完全一致・1画素差（座標／最大channel差）・oracle欠落・寸法エラー時の旧成功report排除を検査しました。auditのpalette/auxiliary範囲と累積canvas上限も検査し、未関与の担当が独立レビューしました。これらはローカル調査補助toolの検証であり、この段階ではQ4の独立比較をまだ実施しておらず、decoder本体も変更していませんでした。

## 性能測定

Rust1.91 release、CPU固定、base/head順序を交互にした12 paired run/画像/CPU。各processで10回warmup後、100回decodeを計測しました。baselineは変更前Q4を含む`8d00e2c`で、共通入力予算検査はfinalと同じです。これは比較用binaryの出所であり、このbranchの祖先ではありません。全pairで出力hash/sizeは一致しました。

| 入力 | CPU0 median差 | CPU1 median差 |
|---|---:|---:|
| 30000.Q4 | +1.79% | +1.22% |
| ALIPHA_M.Q4 | +0.51% | -0.00% |
| MA3.Q4 | +0.33% | +2.20% |

負数は高速化、正数は遅延です。共有hostで小さい差を測定しており、ゼロ回帰とは主張しません。実時間・四分位・入力/source/binary SHA256をローカル`.test-q4-validation/benchmark.json`に保持します。

### 保持色修正後の再測定

base `66e2ba0`と保持色修正版をrelease buildし、各入力10回warmup後、base/head順を交互に80 processずつ実行しました。数値はprocess起動と`NUL`へのRGBA書き込みを含むwall-clock中央値なのでdecoder単体の絶対性能ではありませんが、同一条件の回帰検出に使えます。

| 入力 | base median | 修正版 median | 差 |
|---|---:|---:|---:|
| 30000.Q4 | 9.8400 ms | 9.8200 ms | -0.27% |
| CACAO4.Q4 | 9.5400 ms | 9.4100 ms | -1.38% |
| MA3.Q4 | 9.4700 ms | 9.5000 ms | +0.37% |

3入力とも測定揺らぎの範囲で、性能低下は確認されませんでした。
