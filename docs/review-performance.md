# 修正前後の性能・割当記録

基準: `d40fed2e3cd01c4af86eae01b09487c15c1a54e7`。変更後: `codex/review-plan-implementation` の作業ツリー。
Rust 1.95.0 (59807616e 2026-04-14)、Windows 10.0.26200、AMD Ryzen 7 7700。release、既定feature + color-management、JPEG並列のみmultithread追加。
Cargo.lock SHA-256: `21acbad87ff15f8219b9d612f077cdd81d06b88e223c1ec1225590b2fd45f598`。

`review_bench`を同一合成入力でABBA順に3巡、各プロセス5反復した。各プロセス内の中央値を観測単位とし、独立起動6回ずつを再標本化した比の95% bootstrap区間を示す。値はこのマシン・入力の観測であり、全画像での速度を保証しない。JPEGは読み込み、描画、出力ハッシュ計算を含み、初回drawも別計測した。

| ケース | 方式 | 修正前 ms | 修正後 ms | 後/前の95%区間 | 割当回数 前→後 | 生存ヒープ最大 MiB 前→後 |
| --- | --- | ---: | ---: | --- | --- | --- |
| draw-4k | single | 3.483 | 1.965 | 0.526–0.598 | 0→0 | 0.000→0.000 |
| draw-small-10000 | single | 1.563 | 0.810 | 0.472–0.562 | 0→0 | 0.000→0.000 |
| icc-u16-gray | single | 32.790 | 19.334 | 0.553–0.641 | 26→25 | 4.501→1.502 |
| jpeg-1024x768 | multi | 16.610 | 18.466 | 1.091–1.132 | 188438→73868 | 8.599→3.371 |
| jpeg-1024x768 | single | 25.983 | 22.907 | 0.787–0.984 | 184446→73857 | 3.364→3.364 |
| jpeg-16x16 | multi | 0.182 | 0.027 | 0.136–0.159 | 211→157 | 0.029→0.021 |
| jpeg-16x16 | single | 0.027 | 0.028 | 0.899–1.143 | 186→153 | 0.019→0.019 |
| pick-4k | single | 20.083 | 5.959 | 0.285–0.310 | 1→1 | 31.641→31.641 |
| webp-animation-24 | single | 288.619 | 288.775 | 0.976–1.025 | 276494→276053 | 4.728→1.694 |

生存ヒープはGlobalAllocで割当と解放を追った差分で、RSSではない。JPEG/WebPの全出力ハッシュは前後一致。draw/pickは測定中に同一画素を観測し、全矩形の画素一致・stride・パディングは回帰試験で別に確認した。ICC計測は恒等Gray16のため、正しさは別の8/10/12/16ビット非恒等試験で確認した。

## 処理量と総割当

| ケース | 方式 | 百万画素/秒 前→後 | 総割当 MiB 前→後 |
| --- | --- | ---: | ---: |
| draw-4k | single | 2381.05→4220.35 | 0.000→0.000 |
| draw-small-10000 | single | — | 0.000→0.000 |
| icc-u16-gray | single | 23.98→40.68 | 4.502→1.502 |
| jpeg-1024x768 | multi | 47.35→42.59 | 52.663→17.657 |
| jpeg-1024x768 | single | 30.27→34.33 | 45.779→17.655 |
| jpeg-16x16 | multi | 1.41→9.65 | 0.049→0.031 |
| jpeg-16x16 | single | 9.42→9.25 | 0.038→0.029 |
| pick-4k | single | 413.01→1392.02 | 31.641→31.641 |
| webp-animation-24 | single | 1.36→1.36 | 784.775→781.707 |

## 初回描画とプロセスメモリ

- jpeg-1024x768 (multi): 初回draw 10.654→0.563 ms。
- jpeg-1024x768 (single): 初回draw 0.061→0.443 ms。
- jpeg-16x16 (multi): 初回draw 0.174→0.014 ms。
- jpeg-16x16 (single): 初回draw 0.014→0.015 ms。

プロセスのPeakWorkingSet64を5 ms間隔で観測した別測定（各1起動）。入力生成・符号化・ランタイムも含むため、デコード中の生存ヒープと区別する。

- baseline single icc: 10.785 MiB。
- baseline single animation: 11.945 MiB。
- baseline multi jpeg: 17.074 MiB。
- new single icc: 7.734 MiB。
- new single animation: 8.824 MiB。
- new multi jpeg: 12.340 MiB。

## AVIFプレーン抽出とICC変換器再利用

AVIF抽出は奇数寸法257×127、各反復100回。ビルドを完了した実行ファイルをABBA順に3巡し、各起動内8反復の中央値から算出した。入力は固定値で全出力サンプルを検証する。単位は100回あたりms。

| 配置 | 修正前 ms | 修正後 ms | 後/前の95%区間 |
| --- | ---: | ---: | --- |
| u16-interleaved | 5.317 | 2.634 | 0.419–0.577 |
| u16-padded | 6.125 | 2.443 | 0.335–0.480 |
| u16-planar | 5.686 | 2.390 | 0.367–0.474 |
| u8-planar | 5.134 | 2.266 | 0.430–0.453 |

抽出の原データは[review-extraction-benchmark.csv](review-extraction-benchmark.csv)。

ICC小フレーム1000回、1起動内8反復の中央値: 毎回構築 1.746 ms、構築済み変換器の適用 0.736 ms。解析・コンパイルを含む入口と適用のみを比較しており、独立起動の信頼区間は算出していない。

## WebP集約の規模とビルド成果物

外部webp-rust 0.3.1との符号化バイト一致を確認後、既に依存していた同版のlegacy公開入口へ委譲した。削除した14ファイルは8,478行で、ALPH、bit writer、Huffman、VP8/VP8L予測・エントロピー処理の重複を除いた。ローカルにはWML2の設定・メタデータ・アニメーション合成を残した。

| 計測用review_bench | 修正前 bytes | 修正後 bytes |
| --- | ---: | ---: |
| Windows native | 1,940,992 | 1,958,400 |
| wasm32 | 1,299,742 | 1,348,054 |

既定feature + color-managementの診断用バイナリで、配布アプリのサイズではない。安全検査等も同時に追加しており、コード集約でバイナリが縮むという結果ではない。記録されたビルド時間はnative 9.85→7.18秒、Wasm 7.90→6.72秒。各1回、キャッシュ条件を完全には揃えていないため、ビルド高速化の根拠にはしない。

## 解釈と再現

draw/pickの行コピーとICCの作業配列縮小、アニメーションの画素複製削減は改善した。JPEGの1024×768並列処理は総時間が約11%増加した（比の95%区間1.091–1.132）。初回描画とメモリ使用量を改善した代わりに、この入力ではスループットが低下している。小画像のスレッド起動費を避けるため、最大8 MCUは呼出元で処理する。並列時は8件以下の未描画MCUと固定容量キューを用い、コールバックは呼出元で呼ぶ。

`cargo build -p wml2 --release --example review_bench --features color-management` でビルドし、`review_bench draw 5`、`icc 5`、`animation 5`、`jpeg 5`を両版でABBA順に実行する。JPEG並列は`multithread`を追加する。合成画素はソースに固定され、入力ファイル・ネットワークに依存しない。原データは[review-benchmark.csv](review-benchmark.csv)。

ICC変換器再利用は `cargo test -p wml2 --release --no-default-features --features color-management --test highres_icc benchmark_reusable_icc_transform -- --ignored --nocapture`。AVIF抽出は `cargo test -p wml2 --release --no-default-features --features avifenc,high-bit-depth --lib role_extraction_benchmark -- --ignored --nocapture`。基準版への追加は計測用ソースとmod宣言だけで、実装を変更しない。
