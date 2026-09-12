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
