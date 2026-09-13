## samples
- このフォルダはPluginが機能しているか確認するためのサンプルデータが置いてあります
- テスト専用の画像アセットは `test/images/` に分離します
- ライセンスや出所の確認が必要な外部 sample は `test/images/external/` を使います

## TIFF拡張用の外部テストスイート

TIFF拡張の確認には、ファイル単位の出所とライセンスが記録された [imazen/codec-corpus TIFF Conformance](https://github.com/imazen/codec-corpus/tree/main/tiff-conformance) を使用します。TIFF 6.0、BigTIFF、strip/tile、Deflate、LZW、PackBits、PlanarConfiguration、Predictor、CMYK、multi-page、異常入力を含む154ファイルです。

- 配置先: `D:\data\samples\images\tiff\codec-corpus\`
- 機能別コピー: `bigtiff`、`deflate`、`lzw`、`packbits`、`planar`、`tiles`、`predictor`、`cmyk`、`multipage`、`orientation-metadata`、`robustness`
- 取得元コミット: `8e10d4d765667c1c49d74413878fc4bfb46dcf8d`
- 出所・ライセンス: [SOURCES.md](https://github.com/imazen/codec-corpus/blob/main/tiff-conformance/SOURCES.md)（MIT、CC0、permissive libtiff license、unrestricted、Public Domain等のファイル単位管理）
- 検証用ハッシュ: `D:\data\samples\images\tiff\codec-corpus\SHA256SUMS.txt`

詳細な仕様調査、採用範囲、ImageMagick/Pillowによる確認結果は [docs/tiff-extend-research.md](../docs/tiff-extend-research.md) に記録しています。

## TIFFレビュー修正の再現画像

`D:\data\samples\images\tiff\review-a08bf058\` に独自生成のTIFF 37枚を追加しました。格納tileの展開上限、SampleFormat、通常ページ、用途未指定の追加チャンネル、FillOrder=2、RowsPerStrip最大値、InkSet、Gray/RGB/RGBA 16-bit、planar、Classic/BigTIFFとLE/BEを含みます。第三者の画像は使っていません。正常入力30枚と意図した異常・未対応入力7枚を `manifest.json` で区別しています。

- [生成器](../wml2-test/scripts/generate_tiff_review_samples.py): `--dest` で保存先を指定。外部Pythonパッケージは不要。
- [実行検証器](../wml2-test/scripts/verify_tiff_review_samples.py): converter・metadataの実行、PNG画素／ページ数、PillowでのTIFF再保存確認。`--magick` で独立した全ページ比較を追加。
- `encoder-roundtrip/` にClassic/BigTIFF × None/LZW/Deflateの再保存結果6枚を保持（合計43 TIFF）。
- `validation/` に実行ログとJSON比較結果を保持。

修正内容と再現コマンドは [レビュー修正記録](../docs/tiff-extend-review-fixes.md) に記載しています。

## ExtraSamplesとLZW形式の追加画像

2026-09-13の追加レビュー用に、第三者の画像を使わない再現用TIFFを追加しました。

- `D:\data\samples\images\tiff\alpha-lzw-20260913\`: CMYKの非乗算alphaと、明示的に未対応として拒否するCMYK乗算済みalpha・Palette alpha。LE/BE、BigTIFF、LZW、16-bit、planar、tileを含みます。
- `D:\data\samples\images\tiff\lzw-modes-20260913\`: 標準MSB、旧LibTIFF LSB late-change、WML2独自LSB early-change。9→10→11-bitのコード幅境界とFillOrderを分離して検証します。
- [alpha生成器](../wml2-test/scripts/generate_tiff_alpha_lzw_samples.py)、[LZW生成器](../wml2-test/scripts/generate_tiff_lzw_modes_samples.py)、[LZW検証器](../wml2-test/scripts/verify_tiff_lzw_modes.py)。生成器は標準Pythonのみ、検証にはPillowとImageMagick/LibTIFFを使用します。
- 各保存先の `manifest.json` に期待画素・正常／拒否・互換形式を記録し、`validation/` に実バイナリと外部oracleの比較結果を保持します。新しい `tiff-oracle` CIジョブでも生成・比較を実行します。

採用した未対応判定と検証の範囲は [追加レビュー対応](../docs/tiff-extend-alpha-lzw-review.md) に記載しています。
