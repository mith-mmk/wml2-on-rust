# Test Images

テストで使う画像アセットはこの配下にまとめます。

- `bundled/`
  - Git に含める小さな回帰用画像です。
  - 原則として自前生成したダミー画像か、出所と再配布条件が明確なものだけを置きます。
- `external/`
  - Git に含めない外部サンプル置き場です。
  - ライセンスが曖昧な TIFF/BMP/レトロ形式サンプルやサイズの大きい画像はここに隔離します。

## 外部サンプルの使い方

1. 別管理の sample repository や手元の検証用データを `test/images/external/` 配下に clone または展開します。
2. そのままファイル名で見つからない場合は、`wml2/tests/test_samples.example.txt` を `wml2/tests/test_samples.txt` にコピーしてパスを調整します。

テスト側は次の順で画像を探します。

1. `test/images/bundled/`
2. 旧来の互換パス (`test/samples`, `_test`, `_test/animation_webp`)
3. `wml2/tests/test_samples.txt` の明示設定
4. `test/images/external/` 配下の再帰検索

## 方針

- CI で必須な回帰は、できるだけ `bundled/` のダミー画像で完結させます。
- 外部 sample が必要なテストは、画像が無いときに skip できるように保ちます。
- `samples/` は viewer や手動確認用のデモ資産として扱い、テスト専用資産とは分離します。
