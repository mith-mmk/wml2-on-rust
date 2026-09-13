# TIFF追加レビュー対応（2026-09-13）

対象ブランチは `tiff-extend`。`798b735` に対する追加レビューのExtraSamples、LZW、外部oracleのCI化、描画時の複製、CI記録を扱う。

## ExtraSamples

CMYKのunassociated alpha（ExtraSamples=2）は8/16-bit、LE/BE、strip/tile、chunky/planarでRGBAの値を検証した。CMYKのassociated alpha（1）とPaletteのalpha（1/2）は、意味を正しく保持する実装がないため `NoSupportFormat` として明示的に拒否する。ページ検証と公開の直接描画経路の双方で拒否し、透明度を捨てた成功結果を返さない。

PaletteのExtraSamples=0および用途未指定の追加サンプルは従来どおり不透明として扱う。ColorMapの必要サイズは追加チャンネルを含む総bit数ではなく、indexサンプルのbit数から求める。Gray/RGBの既存alpha対応は維持する。

## LZWの形式

| 形式 | コードのbit順 | 幅変更 | 出力オプション |
| --- | --- | --- | --- |
| 標準TIFF | MSB first | early change | `lzw`（FillOrder=1） |
| 旧LibTIFF | LSB first | late change | 読み込み互換 |
| 旧WML2拡張 | LSB first | early change | `lzw_lsb`（既存のFillOrder=2を保持） |

FillOrderはLZW形式を選ぶフラグではない。展開後のpacked sampleのbit順と圧縮コード列のbit順を別に扱う。`lzw_lsb`は標準TIFF相互運用を保証する出力ではなく、既存WML2ファイルとの互換オプションとして残す。

LSBの2形式は先頭Clearのパターンが同じなので、両候補のEOIと完全なblock長を検証する。一つだけ成立する場合はその候補、両方が同じ画素ならその結果を採用し、異なる画素に復号できる場合は曖昧な入力として拒否する。比較には2候補分の展開量を事前に上限へ照合する。標準MSB形式も初期Clear、EOI、期待block長を要求する。

TIFF 6.0のLZW記載と[LibTIFFの互換実装](https://gitlab.com/libtiff/libtiff/-/blob/master/libtiff/tif_lzw.c)を照合した。仕様の参照先は [TIFF 6.0](https://techheap.packetizer.com/compression/graphics/tiff6.pdf)。

## 作業メモリと既存制限

汎用block描画では展開済みサンプルと既存ColorMapを借用する。公開の未処理データ描画ではPredictor/planar変換が必要な場合だけ所有バッファを作る。この直接描画の回帰確認で、planar Predictorが先頭planeにしか適用されない問題も再現し、全planeの行を復元するよう修正した。

`expanded_bytes`は展開量の制限であり、圧縮入力、LZW辞書、最終画像等を合算したプロセスのピークRSSの上限ではない。今回の変更で不要な全block複製を除去するが、総メモリを同値以内に保証するという意味には変更しない。

Orientation 2〜8を自動回転しない従来設計を維持する。TIFFからRGBAを経由して再保存するとOrientation=1になる既存問題は今回の修正対象に含めておらず、解消済みとは扱わない。

## CIと外部比較

前回コミット `798b735` の [CI実行34673928648](https://github.com/mith-mmk/wml2-on-rust/actions/runs/34673928648) はLinux/Windows/macOS成功、Wasmはコンパイル成功を確認した。旧レビュー記録の未確認という記載は更新した。

今回 `tiff-oracle` ジョブを追加し、独立生成TIFFをconverter/metadata実バイナリで読み、ImageMagick/LibTIFFで元画像の画素、Pillowで出力PNGと再保存TIFFを照合する。標準LZW・旧LibTIFF・WML2独自LSBはコード幅の9→10→11-bit境界を跨ぐ別fixtureとして扱う。この追加CIジョブ自体のGitHub上での実行は、ローカル検証とは区別する。

## 最終検証

- Windows x64: default + high-bit-depthのライブラリ・統合テスト199件成功、既存1件ignored。追加した「誤候補がEOIへ到達するが932バイトしかなく、正候補1024バイトを選ぶ」回帰1件も成功（計200件）。
- Windows i686: tiff + high-bit-depthの単体・TIFF統合74件と同じ追加回帰1件、計75件成功。4GiB超のBigTIFFテストを含む。
- 最小feature 23件、doc test 12件、converterの引数・frameテスト15件成功。Clippy成功（既存警告あり）、`cargo fmt -p wml2 --check`、Python構文、CI YAML、`git diff --check`成功。
- 公開エンコーダの `lzw` / `lzw_lsb` × Classic / BigTIFF × Predictorなし / horizontalの8組で、4096画素のRGBA・寸法・FillOrderタグを確認。
- 旧37枚と新15枚、計52ケースを最終converter/metadataで確認し、正常39枚・明示的拒否13枚が期待どおりになった。外部照合を要求する37ページはImageMagick/LibTIFFと完全一致。Classic/BigTIFF × None/LZW/Deflateの再保存6組もPillowで一致。
- LZW6枚のうち標準MSB/FillOrder=1と旧LibTIFF/FillOrder=1は外部画素一致。WML2独自LSBの2枚と、それ以外のFillOrder=2変種2枚はWML2の期待画素に一致し、今回のImageMagick/LibTIFFは拒否した。後者を標準相互運用成功には数えない。
- Terraによる最終静的レビューで明確な不具合の残指摘なし。今回の変更はローカルコミットまでで、追加CIジョブのGitHub上での実行は未確認。

追加画像は `D:\data\samples\images\tiff\alpha-lzw-20260913`（9枚）と `lzw-modes-20260913`（6枚）。独自生成のCC0画像で、各保存先にmanifest、ライセンス、ハッシュ、validationを保持する。使用したツールはPillow 11.1.0、ImageMagick 7.1.2-21 Q16。旧 `review-a08bf058` のデータは保持した。

## コミットと再現

| Phase | コミット |
| --- | --- |
| 3: 未対応alphaのページ検証 | `de0a9d5` |
| 2: LZW形式・借用描画・回帰 | `c18515d` |
| 6: 標準／旧WML2エンコーダ契約 | `3e0ba22` |

新しい画像の生成・検証コマンドは `.github/workflows/ci.yml` の `tiff-oracle` に記載した。Windowsでは `/usr/bin/python3` をPythonのパスに、`convert` をImageMagickの `magick.exe` に置き換え、converter/metadataと一時出力の場所を指定する。`--output` は存在しないディレクトリとし、今回の一時作業には `C:\temp\wml2-tiff-followup-20260913` を使用した。
