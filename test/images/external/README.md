このディレクトリは Git 管理外の外部 sample 置き場です。

- TIFF/BMP の出所が曖昧な sample
- ローカル検証専用の画像
- 別 repository から `git clone` して持ってくる sample

必要に応じて、sample repository をこの配下に clone してください。
ファイル名だけで解決できない場合は、`wml2/tests/test_samples.example.txt` を
`wml2/tests/test_samples.txt` にコピーしてパスを明示します。

# テスト用データ
- bmp https://github.com/jdelauney/BMP-ImageTestSuite
- tiff https://github.com/tlnagy/exampletiffs
- tiff/png https://people.math.sc.edu/Burkardt/data/tif/tif.html

## AVIF 外部表示テスト

2026-07-14 に
[link-u/avif-sample-images](https://github.com/link-u/avif-sample-images) の
commit `c666a368b73006246694919b5dbcc078317af6cc` を使用して、
`wml2-test` の `converter` による PNG 変換を確認しました。

- 入力: `test/images/external/avif/`
- 変換成功時の出力: `test/images/external/converted/avif/`
- 外部 sample と変換結果は `.gitignore` 対象です。この README だけを Git 管理します。
- 8-bit YUV444 の4件は `avif/supported/` に移動し、converterで指定寸法のPNGへ変換できました。
- 残る10件は未対応機能の期待エラーで失敗し、部分PNGは生成されません。
- 4枚の変換結果は `converted/avif/` に保持し、PNG signature/IHDR寸法を検証済みです。

### 取得ファイルと実測結果

リンクはすべて上記固定 commit の raw file を指します。

| 分類 | sample | bytes | SHA-256 | ライセンス / 作者 | 期待した確認 | converter の実結果 |
|---|---|---:|---|---|---|---|
| supported | [fox.profile1.8bpc.yuv444.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.8bpc.yuv444.avif) | 95375 | `a0cdc981a6b056c8af2d177a1438c332d630040dacbfd1c89bb5e3e381ba5822` | CC-BY-SA 4.0 / Kaede Fujisaki | 8-bit YUV444、1204x800 | 成功: PNG IHDR `1204x800` |
| supported | [fox.profile1.8bpc.yuv444.odd-height.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.8bpc.yuv444.odd-height.avif) | 91421 | `77e0510def73213c00ebcf051cf45fa63cf27d7c69340cc145ab6d44ec77bb07` | CC-BY-SA 4.0 / Kaede Fujisaki | 8-bit YUV444、1204x799 | 成功: PNG IHDR `1204x799` |
| supported | [fox.profile1.8bpc.yuv444.odd-width.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.8bpc.yuv444.odd-width.avif) | 91492 | `12787042364bd13be01830f988cb714220bde340a3329baa808df27a269b83f8` | CC-BY-SA 4.0 / Kaede Fujisaki | 8-bit YUV444、1203x800 | 成功: PNG IHDR `1203x800` |
| supported | [fox.profile1.8bpc.yuv444.odd-width.odd-height.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.8bpc.yuv444.odd-width.odd-height.avif) | 89658 | `fad3b6dd9cb99e6925858f69aafae3f68c861845f2c3d4a6d1c51c6161490134` | CC-BY-SA 4.0 / Kaede Fujisaki | 8-bit YUV444、1203x799 | 成功: PNG IHDR `1203x799` |
| unsupported | [fox.profile0.8bpc.yuv420.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile0.8bpc.yuv420.avif) | 80743 | `cb884c82ac7b6d4fa03b1f687e9e20abc346107095473e9c1d422aaf0de14eaf` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV420 の fail-closed | 失敗: public decode は 4:4:4 color のみ対応 |
| unsupported | [fox.profile0.8bpc.yuv420.monochrome.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile0.8bpc.yuv420.monochrome.avif) | 69856 | `15d84077066c47fdbe2a7b8ed583a17017d09a033144ac1b31486d6c8f6f5c82` | CC-BY-SA 4.0 / Kaede Fujisaki | monochrome の fail-closed | 失敗: public decode は 4:4:4 color のみ対応 |
| unsupported | [fox.profile1.10bpc.yuv444.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.10bpc.yuv444.avif) | 97436 | `a10de8204aee73ba1786daca6390546bd7aa6b069aaa644012219a1c11246a43` | CC-BY-SA 4.0 / Kaede Fujisaki | 10-bit の fail-closed | 失敗: 10-bit quantization は未対応 |
| unsupported | [fox.profile2.8bpc.yuv422.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile2.8bpc.yuv422.avif) | 86782 | `2cb363d30f83bff58ee049874b1808b37cb1d35342edf16b3ce25cb243c9ea55` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV422 の fail-closed | 失敗: `av1C chroma subsampling` と sequence header の不一致。公開ガード到達前 |
| unsupported | [plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif) | 3463 | `245a3dad6371dc702f29eb7e9735f843b63c525da871859728bedbe5bb274985` | CC-BY 4.0 / Ryo Hirafuji | alpha の fail-closed | 失敗: `av1C seq_profile` と sequence header の不一致。alpha ガード到達前 |
| unsupported | [kimono.crop.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/kimono.crop.avif) | 85486 | `f175dcd9c64813b759da185fa67076fb772b76059845b2aad3ddcfab257f75ad` | CC-BY-SA 4.0 / Momiji Jinzamomi, Kaede Fujisaki | `clap` の fail-closed | 失敗: `loop_filter_level[3] is truncated`。transform ガード到達前 |
| unsupported | [kimono.mirror-horizontal.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/kimono.mirror-horizontal.avif) | 84996 | `2bbc004d91145488610158a5acdb4d706495a2b15511db20ff57bb9efd80885c` | CC-BY-SA 4.0 / Momiji Jinzamomi, Kaede Fujisaki | `imir` の fail-closed | 失敗: `loop_filter_level[3] is truncated`。transform ガード到達前 |
| unsupported | [kimono.rotate90.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/kimono.rotate90.avif) | 84837 | `bd1157d8c840713c82b907b9d3ae80bc3817849e11c323d875f8016e035bd3cc` | CC-BY-SA 4.0 / Momiji Jinzamomi, Kaede Fujisaki | `irot` の fail-closed | 失敗: `loop_filter_level[3] is truncated`。transform ガード到達前 |
| unsupported | [red-at-12-oclock-with-color-profile-8bpc.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/red-at-12-oclock-with-color-profile-8bpc.avif) | 106983 | `3e6f2f4016e66e3c94707eaa8373e6f582321e005964cd35b64bc183e1bf10ea` | GNU LGPL v2.1 or BSD 2-Clause / Tony Payne | ICC の fail-closed | 失敗: public decode は 4:4:4 color のみ対応。ICC ガード到達前 |
| unsupported | [star-8bpc.avifs](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/star-8bpc.avifs) | 15679 | `ae35b161de67a5afeb195ee401f369c34990f0ff8662f70ab4065bc6931f0a66` | CC-BY 4.0 / Ryo Hirafuji | AVIF sequence の fail-closed | 失敗: AVIF sequences は未対応 |

### 再実行

親 workspace root から実行します。

14件のhash、AVIFヘッダー、成功PNGのsignature/IHDR、converterの期待エラー、部分PNGなしを一括確認するスクリプトは
`test/avif_external_compat.ps1` です。入力が不足している環境では
`-DownloadMissing` を付けると、上記固定commitから不足分だけ取得します。

```powershell
pwsh -File test/avif_external_compat.ps1
pwsh -File test/avif_external_compat.ps1 -DownloadMissing
```

```powershell
git ls-remote https://github.com/link-u/avif-sample-images.git refs/heads/master

Get-ChildItem -File -Recurse test/images/external/avif |
    Get-FileHash -Algorithm SHA256

cargo run -p wml2-test --example converter --features avif -- `
    "test/images/external/avif/supported/*.avif" `
    -o test/images/external/converted/avif -f png

Get-ChildItem -File test/images/external/avif/unsupported | ForEach-Object {
    cargo run -q -p wml2-test --example converter --features avif -- $_.FullName `
        -o test/images/external/converted/avif -f png
}
```

成功4件は `supported/` と `converted/avif/` にあります。未対応10件を含む全件を再確認するときは、
上記スクリプトを実行します。
