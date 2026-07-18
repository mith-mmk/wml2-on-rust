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
- 8-bit YUV444 の4件、YUV420・monochrome・YUV422の3件、奇数寸法6件、alpha auxiliaryの1件、clap/imir の2件、irot+alphaの1件、grid compositionの1件、irotを含む回転・反転の2件、ICC matrix-shaperの1件はconverterで指定寸法のPNGへ変換できます。
- さらにYUV420/YUV422の奇数幅・奇数高サンプルを6件追加し、端部のplane切り詰めを検証します。
- 全25件がconverterでPNG化できます。12-bit sampleもFFmpegのRGB oracleを通過し、最終RGB差分は平均約0.075、最大6です。`avis` の primary item は先頭フレームを静止画として変換できます。
- 変換結果は `converted/avif/` に保持し、PNG signature/IHDR寸法を検証します。

### 取得ファイルと実測結果

リンクはすべて上記固定 commit の raw file を指します。

| 分類 | sample | bytes | SHA-256 | ライセンス / 作者 | 期待した確認 | converter の実結果 |
|---|---|---:|---|---|---|---|
| supported | [fox.profile1.8bpc.yuv444.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.8bpc.yuv444.avif) | 95375 | `a0cdc981a6b056c8af2d177a1438c332d630040dacbfd1c89bb5e3e381ba5822` | CC-BY-SA 4.0 / Kaede Fujisaki | 8-bit YUV444、1204x800 | 成功: PNG IHDR `1204x800` |
| supported | [fox.profile1.8bpc.yuv444.odd-height.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.8bpc.yuv444.odd-height.avif) | 91421 | `77e0510def73213c00ebcf051cf45fa63cf27d7c69340cc145ab6d44ec77bb07` | CC-BY-SA 4.0 / Kaede Fujisaki | 8-bit YUV444、1204x799 | 成功: PNG IHDR `1204x799` |
| supported | [fox.profile1.8bpc.yuv444.odd-width.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.8bpc.yuv444.odd-width.avif) | 91492 | `12787042364bd13be01830f988cb714220bde340a3329baa808df27a269b83f8` | CC-BY-SA 4.0 / Kaede Fujisaki | 8-bit YUV444、1203x800 | 成功: PNG IHDR `1203x800` |
| supported | [fox.profile1.8bpc.yuv444.odd-width.odd-height.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.8bpc.yuv444.odd-width.odd-height.avif) | 89658 | `fad3b6dd9cb99e6925858f69aafae3f68c861845f2c3d4a6d1c51c6161490134` | CC-BY-SA 4.0 / Kaede Fujisaki | 8-bit YUV444、1203x799 | 成功: PNG IHDR `1203x799` |
| supported | [fox.profile0.8bpc.yuv420.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile0.8bpc.yuv420.avif) | 80743 | `cb884c82ac7b6d4fa03b1f687e9e20abc346107095473e9c1d422aaf0de14eaf` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV420、1204x800 | 成功: PNG IHDR `1204x800` |
| supported | [fox.profile0.8bpc.yuv420.monochrome.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile0.8bpc.yuv420.monochrome.avif) | 69856 | `15d84077066c47fdbe2a7b8ed583a17017d09a033144ac1b31486d6c8f6f5c82` | CC-BY-SA 4.0 / Kaede Fujisaki | monochrome、1204x800 | 成功: PNG IHDR `1204x800` |
| supported | [fox.profile1.10bpc.yuv444.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile1.10bpc.yuv444.avif) | 97436 | `a10de8204aee73ba1786daca6390546bd7aa6b069aaa644012219a1c11246a43` | CC-BY-SA 4.0 / Kaede Fujisaki | 10-bit YUV444、1204x800 | 成功: PNG IHDR `1204x800` |
| supported | [fox.profile2.8bpc.yuv422.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile2.8bpc.yuv422.avif) | 86782 | `2cb363d30f83bff58ee049874b1808b37cb1d35342edf16b3ce25cb243c9ea55` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV422、1204x800 | 成功: PNG IHDR `1204x800` |
| supported | [fox.profile0.8bpc.yuv420.odd-width.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile0.8bpc.yuv420.odd-width.avif) | 78348 | `f91b6f455412adabc5094011362eaaa1f6a9d5740de0b8a1be42a96c16e7617f` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV420、1203x800 | 成功: PNG IHDR `1203x800` |
| supported | [fox.profile0.8bpc.yuv420.odd-height.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile0.8bpc.yuv420.odd-height.avif) | 78504 | `75628450288ace3386651725411c8f0ffff7eb95f82c5307b0faa3350f09f50e` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV420、1204x799 | 成功: PNG IHDR `1204x799` |
| supported | [fox.profile0.8bpc.yuv420.monochrome.odd-width.odd-height.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile0.8bpc.yuv420.monochrome.odd-width.odd-height.avif) | 66487 | `dd069f3c3c4f7589f5f7ef1d7b6a91b8cb975d32663a4a92b6d75219edd72954` | CC-BY-SA 4.0 / Kaede Fujisaki | monochrome、1203x799 | 成功: PNG IHDR `1203x799` |
| supported | [fox.profile2.8bpc.yuv422.odd-width.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile2.8bpc.yuv422.odd-width.avif) | 84128 | `c07575c88ef400c1725c9260a19439e0e784da41c7db3867059019ddbdb3bebe` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV422、1203x800 | 成功: PNG IHDR `1203x800` |
| supported | [fox.profile2.8bpc.yuv422.odd-height.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile2.8bpc.yuv422.odd-height.avif) | 83919 | `efc70882aacbb533c0e833a4401949d152dceb364846442cdccca5048ad17a60` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV422、1204x799 | 成功: PNG IHDR `1204x799` |
| supported | [fox.profile2.8bpc.yuv422.odd-width.odd-height.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile2.8bpc.yuv422.odd-width.odd-height.avif) | 82819 | `86aee64fd7b11b9834537ea14b2eff234c062c98d32fab51ff14aba262d5b106` | CC-BY-SA 4.0 / Kaede Fujisaki | YUV422、1203x799 | 成功: PNG IHDR `1203x799` |
| supported | [plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif) | 3463 | `245a3dad6371dc702f29eb7e9735f843b63c525da871859728bedbe5bb274985` | CC-BY 4.0 / Ryo Hirafuji | alpha auxiliary、128x128 | 成功: PNG IHDR `128x128` |
| supported | [kimono.crop.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/kimono.crop.avif) | 85486 | `f175dcd9c64813b759da185fa67076fb772b76059845b2aad3ddcfab257f75ad` | CC-BY-SA 4.0 / Momiji Jinzamomi, Kaede Fujisaki | `clap`、385x330 | 成功: PNG IHDR `385x330` |
| supported | [kimono.mirror-horizontal.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/kimono.mirror-horizontal.avif) | 84996 | `2bbc004d91145488610158a5acdb4d706495a2b15511db20ff57bb9efd80885c` | CC-BY-SA 4.0 / Momiji Jinzamomi, Kaede Fujisaki | `imir`、722x1024 | 成功: PNG IHDR `722x1024` |
| supported | [kimono.rotate270.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/kimono.rotate270.avif) | 84886 | `79a99a0415276cc11f2e871d070a9df84df3385888a2f2fa3534320f6bed98ed` | CC-BY-SA 4.0 / Momiji Jinzamomi, Kaede Fujisaki | `irot`、出力722x1024 | 成功: PNG IHDR `722x1024` |
| supported | [kimono.mirror-vertical.rotate270.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/kimono.mirror-vertical.rotate270.avif) | 85529 | `33c36ec2274b00ac6f81c9f61e55c20cbfce1649ad27520afe635310f516ead1` | CC-BY-SA 4.0 / Momiji Jinzamomi, Kaede Fujisaki | `imir`+`irot`、出力722x1024 | 成功: PNG IHDR `722x1024` |
| supported | [kimono.rotate90.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/kimono.rotate90.avif) | 84837 | `bd1157d8c840713c82b907b9d3ae80bc3817849e11c323d875f8016e035bd3cc` | CC-BY-SA 4.0 / Momiji Jinzamomi, Kaede Fujisaki | `irot`、722x1024 | 成功: PNG IHDR `722x1024` |
| supported | [red-at-12-oclock-with-color-profile-8bpc.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/red-at-12-oclock-with-color-profile-8bpc.avif) | 106983 | `3e6f2f4016e66e3c94707eaa8373e6f582321e005964cd35b64bc183e1bf10ea` | GNU LGPL v2.1 or BSD 2-Clause / Tony Payne | ICC matrix-shaper、800x800 | 成功: PNG IHDR `800x800`、プロファイル適用を検証 |
| supported | [star-8bpc.avifs](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/star-8bpc.avifs) | 15679 | `ae35b161de67a5afeb195ee401f369c34990f0ff8662f70ab4065bc6931f0a66` | CC-BY 4.0 / Ryo Hirafuji | `avis` primary item の先頭フレーム、159x159 | 成功: PNG IHDR `159x159`、FFmpeg RGBA oracle通過 |
| supported | [sofa_grid1x5_420.avif](https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/sofa_grid1x5_420.avif) | 取得時固定 | `c9e04ff9d90d7093454750fa33b7543ee5479e0cfb151e2c3d2ce6a16c1651c1` | BSD-2-Clause / AOMediaCodec | grid 1x5、1024x770 | 成功: PNG IHDR `1024x770`、FFmpeg平均誤差0.75 |
| supported | [abc_color_irot_alpha_irot.avif](https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/abc_color_irot_alpha_irot.avif) | 取得時固定 | `b371cc88244a873131e4d10ff9363d71ce4f41cf333bd4a491b38d970d9abd3b` | BSD-2-Clause / AOMediaCodec | `irot` + alpha、出力256x512 | 成功: PNG IHDR `256x512`、RGBA oracle通過 |
| supported | [fox.profile2.12bpc.yuv444.avif](https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile2.12bpc.yuv444.avif) | 取得時固定 | `ed96eca6ed79863eaf91e4d666e4e220b5fa4e5a6cb1696477ba901ac12f5dde` | CC-BY-SA 4.0 / Kaede Fujisaki | profile-2 12-bit YUV444、1204x800 | 成功: PNG IHDR `1204x800`、RGB平均差0.075、最大6 |

`supported` は decode／PNG出力と strict pixel oracle を確認済みです。

### 再実行

親 workspace root から実行します。

25件のhash、AVIFヘッダー、成功PNGのsignature/IHDR、converterの期待エラー、部分PNGなしを一括確認するスクリプトは
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

成功25件は `supported/` または既存の外部sample置き場と `converted/avif/` にあります。全件を再確認するときは、上記スクリプトを実行します。
