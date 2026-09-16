# TIFFレビュー修正記録

対象: `tiff-extend`、レビュー基準コミット `a08bf058`、2026-09-12。

レビュー文書のR1〜R6と追加確認事項を修正した。手元で取得できた添付はMarkdown文書のみで、文書中のZIP・Rustテスト・画像は使用していない。以下の再現用TIFFとRustテストを独自に作成し、WML2を実行して確認した。

2026-09-13の追加指摘（CMYK/Palette alpha、LZW形式、外部oracle CI、作業メモリ）は [追加レビュー対応](tiff-extend-alpha-lzw-review.md) を参照。以下の実行件数は前回時点の記録として保持する。

2026-09-15再レビューでは、ImageMagick 6.9の`ExtraSamples=0`解釈差をfixtureのexpected-only比較として分離し、Gray/RGBの16/32-bit associated alphaを元精度でunassociateしてからRGBA8へ量子化する回帰を追加した。main由来のGray4/FillOrder=2順序問題は別の既存制限として保持する。

2026-09-16再レビューでは、WhiteIsZeroのGray associated alphaについて、元精度で`M-S`を正規化してからalphaを解除する順序へ修正した。A=0のRGBを0に保ち、native U16の正規化値とlegacy RGBA8の結果をLE/BE、8/16/32-bitで確認した。

同じ再レビュー用fixtureとしてWhiteIsZeroの8/16/32-bit・LE/BE・部分透明/完全透明12枚をalpha corpusへ追加し、合計29枚をconverter/metadataで検証した。

## 修正内容

| 指摘 | 修正と確認 |
| --- | --- |
| R1 展開サイズ | None/LZW/PackBits/Deflate 8・32946で、格納blockの期待サイズを入力blockの取得前に`expanded_bytes`へ照合。RGBA8とnative U16で、通常上限なら成功、4-byte上限ならblock上限エラーになることを確認。 |
| R2 SampleFormat | 保存後の画素構成に依存する元SampleFormat・MinMax・TransferFunction・色度・Ink関連タグを除外。Classic/BigTIFFとNone/LZW/Deflate/JPEGで再読込。 |
| R3 ページ選択 | 旧SubfileType=1/3とNewSubfileTypeのページbitを保持。縮小画像とmaskを除外し、最初の通常ページをlegacy/native双方の先頭にする。実行中に見つけたconverter `--split`の先頭ページ欠落も修正。 |
| R4 extra channel | 明示的なExtraSamples=1/2のみalphaとして解釈。用途未指定の追加チャンネルはlegacyでは不透明、nativeでは`NoSupportFormat`。 |
| R5 FillOrder | Paletteのpacked sampleのビットの重みを修正。4-bitパレットのFillOrder=2をLE/BEで確認。Gray4の旧分岐にはmain由来の画素順序問題が残るため、今回の取り込み修正とは分離して記録する。 |
| R6 32-bit計算 | strip/tile数を加算オーバーフローのない切り上げ除算で求める。高さ2、RowsPerStrip=u32::MAXを32-bit Windowsで実行。 |
| InkSet | CMYK以外のInkSetおよび4色でないNumberOfInksを未対応として拒否。 |
| Tiffの複製 | generic block描画とCCITTでページ全体のcloneを廃止。格納寸法と処理済み状態を渡し、CCITTでは必要な項目だけを使用。 |
| ICC | 元ICCをTIFFヘッダーと`Source ICC Profile`へ保持。Gray/CMYKの元ICCを現在のRGB用`ICC Profile`へ転記しない。明示されたRGBプロファイルは保存できる。 |

32-bitの実行では、4GiB超の疎なBigTIFFを入力上限無制限で読んでも、readerの長さを`usize`へ狭めて拒否する別の問題を再現した。入力長検査を`u64`のまま実行し、有限の上限と`usize::MAX`の無制限指定を区別して修正した。

修正コミットはPhaseごとに分けた。ページ判定を共有するためPhase 3を先に確定し、Phase 2、4、5、6を続けた。

| Phase | コミット |
| --- | --- |
| 3: ページ・元ICC・converter | `faa33a3` |
| 2: block・sample | `dd192b6` |
| 4: 32-bit BigTIFF | `3bce792` |
| 5: native U16 | `2b31fd6` |
| 6: encoder | `6aaa9bc` |

## 実行結果

- Windows x64、default + high-bit-depth: ライブラリ／統合テスト186件成功、既存1件ignored。
- Windows i686、tiff + high-bit-depth: 単体テスト29件、TIFF拡張22件、レビュー回帰11件、計62件成功。4GiB超のfirst IFD・strip・tile・next IFDを含む。
- 最小feature 23件、EXIF単独28件、doc test 12件成功。
- converterの引数／frame契約テスト15件成功。converterとmetadataの実バイナリを追加画像すべてで実行。
- Clippy成功（既存コードを含む警告あり）。変更したRustファイルをrustfmtで整形し、`git diff --check`成功。
- Terraによる静的再レビューで、残る明確な問題の指摘なし。

CIは`tiff-extend`のpushでも起動するようにし、Windows/macOSのTIFF回帰とWindows 32-bitの実行を追加した。2026-09-13に[実行34673928648](https://github.com/mith-mmk/wml2-on-rust/actions/runs/34673928648)を確認し、対象コミット`798b735`のLinuxテスト、Windows/macOSのTIFF回帰、Windows i686テスト、Wasm・i686 Linuxのコンパイル確認はすべて成功していた。Wasmは実行テストではない。この結果は同コミットについての記録であり、後続の変更のCI成功を示すものではない。既存の特殊CCITT等の制限は[初期拡張文書](tiff-extend.md)に記載した範囲を維持する。

## 追加画像と外部比較

保存先: `D:\data\samples\images\tiff\review-a08bf058\`。

生成fixtureは37 TIFF（正常30、異常・未対応7）。正常30枚の全32ページで、converter出力PNGのRGBA画素がmanifest期待値とImageMagick/LibTIFFの結果に完全一致した。metadataの寸法・ページ数も確認した。負例はSampleFormat不整合6枚とInkSet=2の1枚で、converter/metadata双方が拒否した。

不透明RGBA + SampleFormat 4要素の正常入力をconverterで再保存し、Classic/BigTIFF × None/LZW/Deflateの6組をPillowで確認した。画素、ページ数、compression、SamplesPerPixel、SampleFormatの整合、TIFF variantがすべて一致し、WML2 metadataでも再読込に成功した。6枚の生成結果は`encoder-roundtrip/`に保存した（追加TIFFは合計43枚）。

Pillow 11.1.0、ImageMagick 7.1.2-21 Q16を使用した。Pillowが一部のBE BigTIFFやGray extra channelを開けないため、元画像の全件比較にはImageMagick/LibTIFFを使用した。native U16値と設定した小さい資源上限はRustテストで検証し、converterのRGBA8検証と区別する。

実行ログと比較JSONは保存先の`validation/`に保持する。今回の一時ビルド・変換出力は`C:\temp\wml2-tiff-review-a08bf058`で処理した。

## 再現方法

生成器は標準Pythonのみ、検証器はPillowを必要とする。`--output`には存在しない一時ディレクトリを指定する。converter／metadataのパスはビルド環境に合わせる。

```text
python wml2-test/scripts/generate_tiff_review_samples.py --dest <sample-directory>
python wml2-test/scripts/verify_tiff_review_samples.py --samples <sample-directory> --output <fresh-temporary-output> --converter <converter.exe> --metadata <metadata.exe> --magick <magick.exe>
cargo test -p wml2 --lib --tests --features high-bit-depth --locked
cargo test -p wml2 --target i686-pc-windows-msvc --lib --test tiff_extend --test tiff_review_regressions --no-default-features --features tiff,high-bit-depth --locked
cargo test -p wml2-test --example converter --locked
```

生成器は第三者の画像を含まない。既存の外部テストスイートの出所とライセンスURLは[samples/README.md](../samples/README.md)を参照。

仕様の照合先: [TIFF 6.0](https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf)、[LibTIFFのbit/byte order](https://libtiff.gitlab.io/libtiff/functions/TIFFReadEncodedStrip.html)。
