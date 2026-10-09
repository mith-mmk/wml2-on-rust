# `ifxld4.spi` Q4 復号処理の解析

公開仕様がないXLD4/Q4の互換性確認のため、手元のSusie pluginを静的に逆アセンブルした。バイナリや逆アセンブル全文はrepositoryへ含めず、decoderの判断に必要な命令意味だけを記録する。

## 対象

- ファイル: `ifxld4.spi`
- SHA256: `09181dd2511d4c8692be22ab7f431058a1a85629321cc3c77cfc8f6a6214801f`
- 形式: PE32 / COFF i386、53,248 bytes
- timestamp: 2004-06-28 09:58:16 UTC
- export: `GetPicture` RVA `0x22d0`、`GetPictureInfo` RVA `0x1ee0`、`GetPluginInfo` RVA `0x1f90`、`GetPreview` RVA `0x1e00`、`IsSupported` RVA `0x1e10`
- `MAJYO` signatureはRVA `0x8140`、固定寸法640×400は`GetPictureInfo`で確認できる。

## RLE opcode

画像blockのRLE展開routineはRVA `0x16a0`（image base `0x10000000`ではVA `0x100016a0`）。呼出しごとに最初に`xor bl, bl`があり、保持色をpalette index 0へ初期化する。

復元した処理は次のとおり。

```text
repeat_color = 0
while input remains:
    token = read_u8()
    if token != 0x10:
        emit(token)
        # repeat_colorは変更しない
        continue

    high = read_u8()
    if high == 0:
        repeat_color = read_u8()
        high = read_u8()
    low = read_u8()
    emit(repeat_color, high * 17 + low times)
```

根拠となる主要命令は、`0x16a3 xor bl,bl`、`0x16b7 cmp dl,0x10`、`0x16c5 test si,si`、明示色を読む`0x16cd mov bl,[edx+edi]`、countを作る`0x16e4 shl edx,4`・`0x16e8 add edx,ebp`・`0x16ea add edx,esi`、保持色を書き出す`0x16fd mov [edx+ebp],bl`。literal側の`0x170b mov [esi+ebp],dl`には`BL`更新がない。

したがって`0x10, high!=0, low`は、palette index 0のrunでも前行コピーでもない。直前の`0x10,0,color,high,low`で指定した色を継続する短縮runである。routineはimage blockごとに呼ばれるため、保持色はblockを跨がず、行境界を特別扱いしない。block先頭で明示色がまだない場合だけ0を使う。

## WML2とconverterによる照合

修正前のWML2は短縮runを常にpalette index 0で埋めていた。RECOIL 6.4.5のPNGとの比較ではsample corpus 28件すべてが不一致で、最初の不一致は画像によって先頭行から発生した。

上記の保持色規則へ修正後、同じ28件・168 image blockをRECOIL 6.4.5の640×400 RGB出力と無変換で全画素比較し、28件すべて相違0となった。比較に使ったパッケージと展開後binaryは次のとおり。

- `recoil-2png_6.4.5-1_amd64.deb`: SHA256 `3248d027c1ceb894aba180c6b6fbd899197d4d4cf440e63ff2cbf877fc0901f3`
- `recoil2png`: SHA256 `4ade6d82068381dd1ea500324a564e8b3c764607e5b2de626f6122f5df7cd2a7`

corpusには短縮runが21,794件あり、うち2,039件は行を跨ぎ、345件は各blockの先頭640画素内で始まる。前行予測ではなく保持色runなら、これらを同じ規則で処理できる。

## 適用範囲

この解析は上記hashのpluginと28件のcorpusに対する互換性根拠であり、未公開仕様そのものではない。pluginの境界外readや黙ったrun超過などの安全でない挙動は再現せず、WML2側の入力予算、checked arithmetic、block/canvas上限、truncation検査を維持する。
