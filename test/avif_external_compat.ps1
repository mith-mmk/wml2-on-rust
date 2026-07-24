[CmdletBinding()]
param(
    [switch]$DownloadMissing,
    [switch]$KeepWork,
    [string]$WorkRoot
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$externalRoot = Join-Path $repoRoot 'test/images/external'
$workRoot = if ($WorkRoot) {
    [IO.Path]::GetFullPath($WorkRoot)
} else {
    Join-Path $repoRoot '.test-avif-script'
}
if ([IO.Path]::GetFileName($workRoot) -notlike '.test*') {
    throw "WorkRoot must be a .test* directory: $workRoot"
}
$downloadRoot = Join-Path $workRoot 'download'
$outputRoot = Join-Path $workRoot 'outputs'
$sourceCommit = 'c666a368b73006246694919b5dbcc078317af6cc'
$sourceBase = "https://raw.githubusercontent.com/link-u/avif-sample-images/$sourceCommit"

$manifest = @(
    [pscustomobject]@{ Name = 'fox.profile1.8bpc.yuv444.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile1.8bpc.yuv444.avif'; Sha256 = 'a0cdc981a6b056c8af2d177a1438c332d630040dacbfd1c89bb5e3e381ba5822'; Width = 1204; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile1.8bpc.yuv444.odd-height.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile1.8bpc.yuv444.odd-height.avif'; Sha256 = '77e0510def73213c00ebcf051cf45fa63cf27d7c69340cc145ab6d44ec77bb07'; Width = 1204; Height = 799 }
    [pscustomobject]@{ Name = 'fox.profile1.8bpc.yuv444.odd-width.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile1.8bpc.yuv444.odd-width.avif'; Sha256 = '12787042364bd13be01830f988cb714220bde340a3329baa808df27a269b83f8'; Width = 1203; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile1.8bpc.yuv444.odd-width.odd-height.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile1.8bpc.yuv444.odd-width.odd-height.avif'; Sha256 = 'fad3b6dd9cb99e6925858f69aafae3f68c861845f2c3d4a6d1c51c6161490134'; Width = 1203; Height = 799 }
    [pscustomobject]@{ Name = 'fox.profile0.8bpc.yuv420.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/fox.profile0.8bpc.yuv420.avif'; Sha256 = 'cb884c82ac7b6d4fa03b1f687e9e20abc346107095473e9c1d422aaf0de14eaf'; Width = 1204; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile0.8bpc.yuv420.monochrome.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/fox.profile0.8bpc.yuv420.monochrome.avif'; Sha256 = '15d84077066c47fdbe2a7b8ed583a17017d09a033144ac1b31486d6c8f6f5c82'; Width = 1204; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile0.8bpc.yuv420.odd-width.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile0.8bpc.yuv420.odd-width.avif'; Sha256 = 'f91b6f455412adabc5094011362eaaa1f6a9d5740de0b8a1be42a96c16e7617f'; Width = 1203; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile0.8bpc.yuv420.odd-height.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile0.8bpc.yuv420.odd-height.avif'; Sha256 = '75628450288ace3386651725411c8f0ffff7eb95f82c5307b0faa3350f09f50e'; Width = 1204; Height = 799 }
    [pscustomobject]@{ Name = 'fox.profile0.8bpc.yuv420.monochrome.odd-width.odd-height.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile0.8bpc.yuv420.monochrome.odd-width.odd-height.avif'; Sha256 = 'dd069f3c3c4f7589f5f7ef1d7b6a91b8cb975d32663a4a92b6d75219edd72954'; Width = 1203; Height = 799 }
    [pscustomobject]@{ Name = 'fox.profile1.10bpc.yuv444.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/fox.profile1.10bpc.yuv444.avif'; Sha256 = 'a10de8204aee73ba1786daca6390546bd7aa6b069aaa644012219a1c11246a43'; Width = 1204; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile2.8bpc.yuv422.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/fox.profile2.8bpc.yuv422.avif'; Sha256 = '2cb363d30f83bff58ee049874b1808b37cb1d35342edf16b3ce25cb243c9ea55'; Width = 1204; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile2.8bpc.yuv422.odd-width.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile2.8bpc.yuv422.odd-width.avif'; Sha256 = 'c07575c88ef400c1725c9260a19439e0e784da41c7db3867059019ddbdb3bebe'; Width = 1203; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile2.8bpc.yuv422.odd-height.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile2.8bpc.yuv422.odd-height.avif'; Sha256 = 'efc70882aacbb533c0e833a4401949d152dceb364846442cdccca5048ad17a60'; Width = 1204; Height = 799 }
    [pscustomobject]@{ Name = 'fox.profile2.8bpc.yuv422.odd-width.odd-height.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile2.8bpc.yuv422.odd-width.odd-height.avif'; Sha256 = '86aee64fd7b11b9834537ea14b2eff234c062c98d32fab51ff14aba262d5b106'; Width = 1203; Height = 799 }
    [pscustomobject]@{ Name = 'plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif'; Sha256 = '245a3dad6371dc702f29eb7e9735f843b63c525da871859728bedbe5bb274985'; Width = 128; Height = 128 }
    [pscustomobject]@{ Name = 'kimono.crop.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/kimono.crop.avif'; Sha256 = 'f175dcd9c64813b759da185fa67076fb772b76059845b2aad3ddcfab257f75ad'; Width = 385; Height = 330 }
    [pscustomobject]@{ Name = 'kimono.mirror-horizontal.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/kimono.mirror-horizontal.avif'; Sha256 = '2bbc004d91145488610158a5acdb4d706495a2b15511db20ff57bb9efd80885c'; Width = 722; Height = 1024 }
    [pscustomobject]@{ Name = 'kimono.rotate270.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/kimono.rotate270.avif'; Sha256 = '79a99a0415276cc11f2e871d070a9df84df3385888a2f2fa3534320f6bed98ed'; Width = 722; Height = 1024 }
    [pscustomobject]@{ Name = 'kimono.mirror-vertical.rotate270.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/kimono.mirror-vertical.rotate270.avif'; Sha256 = '33c36ec2274b00ac6f81c9f61e55c20cbfce1649ad27520afe635310f516ead1'; Width = 722; Height = 1024 }
    [pscustomobject]@{ Name = 'abc_color_irot_alpha_irot.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/abc_color_irot_alpha_irot.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/abc_color_irot_alpha_irot.avif'; Sha256 = 'b371cc88244a873131e4d10ff9363d71ce4f41cf333bd4a491b38d970d9abd3b'; Width = 256; Height = 512 }
    [pscustomobject]@{ Name = 'abc_color_irot_alpha_NOirot.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/abc_color_irot_alpha_NOirot.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/abc_color_irot_alpha_NOirot.avif'; Sha256 = 'f2c8cd6ded641c68d13b3363417a62288a5eb335870de8d0b9da5093865ffb9a'; Width = 256; Height = 512 }
    [pscustomobject]@{ Name = 'kimono.rotate90.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/kimono.rotate90.avif'; Sha256 = 'bd1157d8c840713c82b907b9d3ae80bc3817849e11c323d875f8016e035bd3cc'; Width = 722; Height = 1024 }
    [pscustomobject]@{ Name = 'red-at-12-oclock-with-color-profile-8bpc.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/red-at-12-oclock-with-color-profile-8bpc.avif'; Sha256 = '3e6f2f4016e66e3c94707eaa8373e6f582321e005964cd35b64bc183e1bf10ea'; Width = 800; Height = 800 }
    [pscustomobject]@{ Name = 'star-8bpc.avifs'; Kind = 'supported'; RelativePath = 'avif/unsupported/star-8bpc.avifs'; Sha256 = 'ae35b161de67a5afeb195ee401f369c34990f0ff8662f70ab4065bc6931f0a66'; Width = 159; Height = 159 }
    [pscustomobject]@{ Name = 'sofa_grid1x5_420.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/sofa_grid1x5_420.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/sofa_grid1x5_420.avif'; Sha256 = 'c9e04ff9d90d7093454750fa33b7543ee5479e0cfb151e2c3d2ce6a16c1651c1'; Width = 1024; Height = 770 }
    [pscustomobject]@{ Name = 'sofa_grid1x5_420_dimg_repeat.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/sofa_grid1x5_420_dimg_repeat.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/sofa_grid1x5_420_dimg_repeat.avif'; Sha256 = '0a2abbe8b388df51e51b47cc4f1a932fed8c64d340490604fe6b510d77025514'; Width = 1024; Height = 770 }
    [pscustomobject]@{ Name = 'sofa_grid1x5_420_reversed_dimg_order.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/sofa_grid1x5_420_reversed_dimg_order.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/sofa_grid1x5_420_reversed_dimg_order.avif'; Sha256 = '8a77888b3d8b4876636666e4f8ebfaf6248361700b78e34b12dfc458919bd2b7'; Width = 1024; Height = 770 }
    [pscustomobject]@{ Name = 'draw_points_idat.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/draw_points_idat.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/draw_points_idat.avif'; Sha256 = 'ce2fd627efae49391ea82584e9beae05959b867ba429e688a2b95a015b38d3db'; Width = 33; Height = 11 }
    [pscustomobject]@{ Name = 'draw_points_idat_progressive.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/draw_points_idat_progressive.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/draw_points_idat_progressive.avif'; Sha256 = '077ab2ad1e46dd912a973e4f024cb1eb242a08298be2dbf1a52a058e88c48a4a'; Width = 33; Height = 11 }
    [pscustomobject]@{ Name = 'draw_points_idat_progressive_metasize0.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/draw_points_idat_progressive_metasize0.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/draw_points_idat_progressive_metasize0.avif'; Sha256 = '1921cbbf0002c1fba64072298a1a232d106b46ea1b427574723db66d90c3443e'; Width = 33; Height = 11 }
    [pscustomobject]@{ Name = 'draw_points_idat_metasize0.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/draw_points_idat_metasize0.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/draw_points_idat_metasize0.avif'; Sha256 = 'a5f429bef6d2ef2f6022be4848d7266145bfaa060c0fe684150411fa4bf562a1'; Width = 33; Height = 11 }
    [pscustomobject]@{ Name = 'extended_pixi.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/extended_pixi.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/extended_pixi.avif'; Sha256 = '7de53620b571aa61f54df2fc00cfa32955cd4e474a6a4b723a513b51ef21e946'; Width = 4; Height = 4 }
    [pscustomobject]@{ Name = 'clap_irot_imir_non_essential.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/clap_irot_imir_non_essential.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/clap_irot_imir_non_essential.avif'; Sha256 = '33f869fcf2a879913eb394982b8fc03e9a60c25831aa37622ddefa656fd39fc1'; Width = 10; Height = 8 }
    [pscustomobject]@{ Name = 'clop_irot_imor.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/clop_irot_imor.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/clop_irot_imor.avif'; Sha256 = '28e96ad4c913d75a32d66bce116f2963e29d93c01952698c7b33dd893f8bd541'; Width = 34; Height = 12 }
    [pscustomobject]@{ Name = 'colors-animated-12bpc-keyframes-0-2-3.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/colors-animated-12bpc-keyframes-0-2-3.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/colors-animated-12bpc-keyframes-0-2-3.avif'; Sha256 = '3bf9f91da471749e7df639ba7945d4d94c1c3e3968c26f3619fbbcfc92790576'; Width = 64; Height = 64 }
    [pscustomobject]@{ Name = 'fox.profile2.12bpc.yuv444.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/fox.profile2.12bpc.yuv444.avif'; DownloadUrl = 'https://raw.githubusercontent.com/link-u/avif-sample-images/c666a368b73006246694919b5dbcc078317af6cc/fox.profile2.12bpc.yuv444.avif'; Sha256 = 'ed96eca6ed79863eaf91e4d666e4e220b5fa4e5a6cb1696477ba901ac12f5dde'; Width = 1204; Height = 800 }
    [pscustomobject]@{ Name = 'alpha_noispe.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/alpha_noispe.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/alpha_noispe.avif'; Sha256 = '8deb96e78c3e5d608a157b2de4c98eb1a30e0c85736b4230758400509c88d47e'; Width = 80; Height = 80 }
    [pscustomobject]@{ Name = 'color_grid_alpha_nogrid.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/color_grid_alpha_nogrid.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/color_grid_alpha_nogrid.avif'; Sha256 = 'bae56368b348b1d847e2bfb662522599f0c63dfe62fb68826c9e42a300ff405d'; Width = 80; Height = 80 }
    [pscustomobject]@{ Name = 'color_grid_alpha_grid_tile_shared_in_dimg.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/color_grid_alpha_grid_tile_shared_in_dimg.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/color_grid_alpha_grid_tile_shared_in_dimg.avif'; Sha256 = '1924ad27fa74aff5278367245d56e14804f6f5a6ac9fbc3da19b39033e167235'; Width = 80; Height = 80 }
    [pscustomobject]@{ Name = 'circle_custom_properties.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/circle_custom_properties.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/circle_custom_properties.avif'; Sha256 = '6c57595c1b814392c6a0d0e1f60e34c6f7f09a8ce7e46885d85059b7205e82fb'; Width = 100; Height = 60 }
    [pscustomobject]@{ Name = 'colors-animated-8bpc-alpha-exif-xmp.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/colors-animated-8bpc-alpha-exif-xmp.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/colors-animated-8bpc-alpha-exif-xmp.avif'; Sha256 = 'c2e38681057c15009c4b76ea08cea68cdde80806abd41d42a646f697bf5aabb2'; Width = 150; Height = 150 }
    [pscustomobject]@{ Name = 'colors-animated-8bpc-audio.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/colors-animated-8bpc-audio.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/colors-animated-8bpc-audio.avif'; Sha256 = '624f3bfe78b6bd75e9e12fe9b36c6132e3effaf82aa2a443f1b2a207a7d3561b'; Width = 150; Height = 150 }
    [pscustomobject]@{ Name = 'colors-animated-8bpc-depth-exif-xmp.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/colors-animated-8bpc-depth-exif-xmp.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/colors-animated-8bpc-depth-exif-xmp.avif'; Sha256 = '93177031f6177cff1e14c9065eb9dc97dbd7bbfa3e32a8b99af222398be1daac'; Width = 150; Height = 150 }
    [pscustomobject]@{ Name = 'arc_triomphe_extent1000_nullbyte_extent1310.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/arc_triomphe_extent1000_nullbyte_extent1310.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/arc_triomphe_extent1000_nullbyte_extent1310.avif'; Sha256 = '709b48ce4481a70efe725ae5e5da4d0e5482d8fd126cee8bd42da31c0b67823c'; Width = 64; Height = 64 }
    [pscustomobject]@{ Name = 'colors_hdr_p3.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/colors_hdr_p3.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/colors_hdr_p3.avif'; Sha256 = 'ec4b67fa129360f4b44768bdd1027fb32834d1a1f7e49ae53bed44c819def9c4'; Width = 200; Height = 200 }
    [pscustomobject]@{ Name = 'colors_hdr_rec2020.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/colors_hdr_rec2020.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/colors_hdr_rec2020.avif'; Sha256 = '9980e58ddf718a923f1738c34aad1c72f8e5795ec07e68f1a5f9bd216ca19740'; Width = 200; Height = 200 }
    [pscustomobject]@{ Name = 'colors_hdr_srgb.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/colors_hdr_srgb.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/colors_hdr_srgb.avif'; Sha256 = '1aecb78d6d363caae95a5dd198f347ce4b200073ced638647364a2ad04d9707d'; Width = 200; Height = 200 }
    [pscustomobject]@{ Name = 'colors_sdr_srgb.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/colors_sdr_srgb.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/colors_sdr_srgb.avif'; Sha256 = '24463b9b79e4624d06e247079efddf624f9062c516d43ddf6fe5e150343629d6'; Width = 200; Height = 200 }
    [pscustomobject]@{ Name = 'weld_sato_12B_8B_q0.avif'; Kind = 'supported'; RelativePath = 'avif/unsupported/weld_sato_12B_8B_q0.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/weld_sato_12B_8B_q0.avif'; Sha256 = 'fa41d615d244d50fc99d71c1fea14561e0814382a748d1b8b672c3fd5a595dbe'; Width = 1024; Height = 684 }
    [pscustomobject]@{ Name = 'poc_b_506387278.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/poc_b_506387278.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/poc_b_506387278.avif'; Sha256 = 'd477bfd1160faf5aa53c5b58df127f5e882547a696c16c7b12a5e2f85112fd7c'; ErrorPattern = 'nclx range does not match' }
    [pscustomobject]@{ Name = 'seine_hdr_gainmap_srgb.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/seine_hdr_gainmap_srgb.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/seine_hdr_gainmap_srgb.avif'; Sha256 = '9bf9c6a7606951de07e4079cd63c2cfe379d95139cd99ab9142d8a6ee22d28c7'; Width = 400; Height = 300 }
    [pscustomobject]@{ Name = 'seine_hdr_gainmap_small_srgb.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/seine_hdr_gainmap_small_srgb.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/seine_hdr_gainmap_small_srgb.avif'; Sha256 = '573a67fdd581f6e634da198a819cc92a071539dfb720c5d7dfdf02e4e87a0346'; Width = 400; Height = 300 }
    [pscustomobject]@{ Name = 'seine_sdr_gainmap_big_srgb.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/seine_sdr_gainmap_big_srgb.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/seine_sdr_gainmap_big_srgb.avif'; Sha256 = 'b672cfd5ac792ae0a70f82b68ef7bab06c117e5d297624ec8afdf56af847c6b7'; Width = 400; Height = 300 }
    [pscustomobject]@{ Name = 'seine_sdr_gainmap_srgb.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/seine_sdr_gainmap_srgb.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/seine_sdr_gainmap_srgb.avif'; Sha256 = 'e0ebdb2f1f44c7d901e6b5f817eb2520eace344624cab0d57aaa929d05d6d971'; Width = 400; Height = 300 }
    [pscustomobject]@{ Name = 'seine_sdr_gainmap_notmapbrand.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/seine_sdr_gainmap_notmapbrand.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/seine_sdr_gainmap_notmapbrand.avif'; Sha256 = '63da24747562724235e3fc803d2230d25bf5a4f91112aa87a3b804a333f12d7f'; Width = 400; Height = 300 }
    [pscustomobject]@{ Name = 'unsupported_gainmap_version.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/unsupported_gainmap_version.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/unsupported_gainmap_version.avif'; Sha256 = 'f67ef979ee9df50c7893eafea591030b3897dba1285ea8331dc0687e063dda9b'; Width = 100; Height = 100 }
    [pscustomobject]@{ Name = 'unsupported_gainmap_minimum_version.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/unsupported_gainmap_minimum_version.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/unsupported_gainmap_minimum_version.avif'; Sha256 = 'd675f46519029ce3da98fac587cb25fa2eb33c7b77d7ae5c04903b2825367331'; Width = 100; Height = 100 }
    [pscustomobject]@{ Name = 'unsupported_gainmap_writer_version_with_extra_bytes.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/unsupported_gainmap_writer_version_with_extra_bytes.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/unsupported_gainmap_writer_version_with_extra_bytes.avif'; Sha256 = '1ede5af67433062cffd620833d2b132e1572348596c5659ad2ed4c4d708edd83'; Width = 100; Height = 100 }
    [pscustomobject]@{ Name = 'supported_gainmap_writer_version_with_extra_bytes.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/supported_gainmap_writer_version_with_extra_bytes.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/supported_gainmap_writer_version_with_extra_bytes.avif'; Sha256 = '189398ea72d391c75c8679942c539ae7d4324152fa2f948d742b73f3bd6cc8f1'; Width = 100; Height = 100 }
    [pscustomobject]@{ Name = 'color_nogrid_alpha_nogrid_gainmap_grid.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/color_nogrid_alpha_nogrid_gainmap_grid.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/color_nogrid_alpha_nogrid_gainmap_grid.avif'; Sha256 = 'd783e0d9ce778f972e88586b6b1b9eb062f54d38f28521721a8b9cbbda3b7fb0'; Width = 128; Height = 200 }
    [pscustomobject]@{ Name = 'color_grid_alpha_grid_gainmap_nogrid.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/color_grid_alpha_grid_gainmap_nogrid.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/color_grid_alpha_grid_gainmap_nogrid.avif'; Sha256 = 'c424c43fe4bab3b8ef37b86c0bab3851b850b94e5d46b9fae979586dae45de0a'; Width = 512; Height = 600 }
    [pscustomobject]@{ Name = 'color_grid_gainmap_different_grid.avif'; Kind = 'supported'; RelativePath = 'avif/gainmap/color_grid_gainmap_different_grid.avif'; DownloadUrl = 'https://raw.githubusercontent.com/AOMediaCodec/libavif/main/tests/data/color_grid_gainmap_different_grid.avif'; Sha256 = '73a68c3d6daad7b8298db975a00f02bca46b6c3f292eac09d3c1443d2006fab2'; Width = 512; Height = 600 }
)

function Read-BigEndianUInt32([byte[]]$bytes, [int]$offset) {
    return ([uint32]$bytes[$offset] -shl 24) -bor
        ([uint32]$bytes[$offset + 1] -shl 16) -bor
        ([uint32]$bytes[$offset + 2] -shl 8) -bor
        [uint32]$bytes[$offset + 3]
}

function Test-PngDimensions([string]$path, [int]$width, [int]$height) {
    $bytes = [IO.File]::ReadAllBytes($path)
    if ($bytes.Length -lt 24) { return $false }
    $signature = @(137, 80, 78, 71, 13, 10, 26, 10)
    for ($index = 0; $index -lt $signature.Count; $index++) {
        if ($bytes[$index] -ne $signature[$index]) { return $false }
    }
    $chunkLength = Read-BigEndianUInt32 $bytes 8
    $chunkType = [Text.Encoding]::ASCII.GetString($bytes, 12, 4)
    return $chunkLength -eq 13 -and $chunkType -eq 'IHDR' -and
        (Read-BigEndianUInt32 $bytes 16) -eq $width -and
        (Read-BigEndianUInt32 $bytes 20) -eq $height
}

$oldLocation = Get-Location
$oldTemp = $env:TEMP
$oldTmp = $env:TMP
$failures = [System.Collections.Generic.List[string]]::new()
$successCount = 0
$expectedFailureCount = 0
$unexpectedCount = 0
$partialPngCount = 0

try {
    Set-Location $repoRoot
    New-Item -ItemType Directory -Force -Path $downloadRoot, $outputRoot | Out-Null
    $env:TEMP = Join-Path $workRoot 'temp'
    $env:TMP = $env:TEMP
    New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null

    foreach ($entry in $manifest) {
        $target = Join-Path $externalRoot $entry.RelativePath
        try {
            if (-not (Test-Path -LiteralPath $target -PathType Leaf)) {
                if (-not $DownloadMissing) {
                    throw "missing sample; rerun with -DownloadMissing: $target"
                }

                $download = Join-Path $downloadRoot $entry.Name
                $downloadUrl = if ($entry.DownloadUrl) { $entry.DownloadUrl } else { "$sourceBase/$($entry.Name)" }
                Invoke-WebRequest -UseBasicParsing -Uri $downloadUrl -OutFile $download
                New-Item -ItemType Directory -Force -Path (Split-Path $target) | Out-Null
                Move-Item -LiteralPath $download -Destination $target
            }

            $bytes = [System.IO.File]::ReadAllBytes($target)
            if ($bytes.Length -lt 12) {
                throw 'file is shorter than an AVIF ftyp header'
            }
            $brand = [Text.Encoding]::ASCII.GetString($bytes, 4, 8)
            if ($brand -notmatch '^ftypavi[fs]$') {
                throw "unexpected AVIF brand: $brand"
            }

            $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash.ToLowerInvariant()
            if ($hash -ne $entry.Sha256) {
                throw "SHA-256 mismatch: expected $($entry.Sha256), got $hash"
            }

            $probeRoot = Join-Path $outputRoot $entry.Name
            New-Item -ItemType Directory -Force -Path $probeRoot | Out-Null
            $converterOutput = @(& cargo run -q -p wml2-test --example converter --features avif -- $target -o $probeRoot -f png 2>&1)
            $converterExit = $LASTEXITCODE
            $converterText = $converterOutput -join "`n"
            $partialOutputs = @()
            for ($attempt = 0; $attempt -lt 10; $attempt++) {
                $partialOutputs = if (Test-Path -LiteralPath $probeRoot -PathType Container) {
                    @(Get-ChildItem -File -Recurse $probeRoot)
                } else {
                    @()
                }
                if ($partialOutputs.Count -gt 0 -or $converterExit -ne 0) {
                    break
                }
                # OneDrive can publish a newly written PNG slightly after the
                # converter process exits; give the filesystem a short window
                # before classifying a successful conversion as partial.
                Start-Sleep -Milliseconds 200
            }

            if ($entry.Kind -eq 'supported') {
                $pngOutputs = @($partialOutputs | Where-Object { $_.Extension -ieq '.png' })
                if ($converterExit -ne 0 -or $pngOutputs.Count -ne 1 -or
                    -not (Test-PngDimensions $pngOutputs[0].FullName $entry.Width $entry.Height)) {
                    $unexpectedCount++
                    $failures.Add("$($entry.Name): expected success, exit=$converterExit, pngs=$($pngOutputs.Count), output=$converterText")
                    Write-Host "[FAIL] $($entry.Name) expected success"
                } else {
                    $finalOutput = Join-Path $externalRoot "converted/avif/$($entry.Name).png"
                    New-Item -ItemType Directory -Force -Path (Split-Path $finalOutput) | Out-Null
                    $generatedHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $pngOutputs[0].FullName).Hash
                    if (Test-Path -LiteralPath $finalOutput) {
                        $finalHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $finalOutput).Hash
                        if ($finalHash -ne $generatedHash) {
                            Remove-Item -LiteralPath $finalOutput -Force
                            Copy-Item -LiteralPath $pngOutputs[0].FullName -Destination $finalOutput -Force
                        }
                    } else {
                        Copy-Item -LiteralPath $pngOutputs[0].FullName -Destination $finalOutput -Force
                    }
                    $successCount++
                    Write-Host "[PASS] $($entry.Name) converted $($entry.Width)x$($entry.Height)"
                }
            } else {
                if ($converterExit -eq 0 -or $converterText -notmatch $entry.ErrorPattern -or $partialOutputs.Count -ne 0) {
                    $unexpectedCount++
                    if ($partialOutputs.Count -ne 0) { $partialPngCount += $partialOutputs.Count }
                    $failures.Add("$($entry.Name): exit=$converterExit, expected error '$($entry.ErrorPattern)', partial_outputs=$($partialOutputs.Count), output=$converterText")
                    Write-Host "[FAIL] $($entry.Name) expected failure"
                } else {
                    $expectedFailureCount++
                    Write-Host "[PASS] $($entry.Name) expected failure"
                }
            }
        } catch {
            $failures.Add("$($entry.Name): $($_.Exception.Message)")
            Write-Host "[FAIL] $($entry.Name)"
        }
    }
} finally {
    Set-Location $oldLocation
    $env:TEMP = $oldTemp
    $env:TMP = $oldTmp
    if (-not $KeepWork -and (Test-Path -LiteralPath $workRoot)) {
        Remove-Item -LiteralPath $workRoot -Recurse -Force
    }
}

if ($failures.Count -gt 0) {
    Write-Error ("AVIF compatibility check failed ({0}):`n{1}" -f $failures.Count, ($failures -join "`n"))
    exit 1
}

Write-Host "AVIF compatibility check passed: $successCount successes, $expectedFailureCount expected failures, unexpected $unexpectedCount, partial PNGs $partialPngCount."
exit 0
