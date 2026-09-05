"""Summarize review benchmark CSVs, resampling independent process runs.

Usage: python test/summarize_review_bench.py RESULTS_DIRECTORY OUTPUT_DIRECTORY
"""
import csv
import json
import random
import re
import statistics as stats
import sys
from collections import defaultdict
from pathlib import Path

root, output = map(Path, sys.argv[1:3])
groups = defaultdict(lambda: defaultdict(list))
records = []
for file in sorted(root.glob("bench-*.csv")) + sorted(root.glob("jpeg-final-*.csv")):
    match = re.fullmatch(r"bench-(\d+)-(baseline|new)-(single|multi)-(.+)\.csv", file.name)
    if match:
        run, variant, mode, _ = match.groups()
        if mode == "multi":
            continue  # superseded by the final small-image threshold measurements
    else:
        match = re.fullmatch(r"jpeg-final-(\d+)-(baseline|new)\.csv", file.name)
        if not match:
            continue
        run, variant = match.groups()
        mode = "multi"
    for row in csv.DictReader(file.open(encoding="utf-8-sig")):
        groups[(row["case"], mode, variant)][run].append(row)
        records.append(dict(run=run, variant=variant, mode=mode, **row))

output.mkdir(parents=True, exist_ok=True)
with (output / "review-benchmark.csv").open("w", newline="", encoding="utf-8") as file:
    writer = csv.DictWriter(file, fieldnames=list(records[0]))
    writer.writeheader()
    writer.writerows(records)

rng = random.Random(20260905)
def interval(a, b):
    samples = sorted(
        stats.mean(rng.choices(b, k=len(b))) / stats.mean(rng.choices(a, k=len(a)))
        for _ in range(10000)
    )
    return samples[249], samples[9749]

lines = ["# 修正前後の性能・割当記録", "",
    "基準: `d40fed2e3cd01c4af86eae01b09487c15c1a54e7`。変更後: `codex/review-plan-implementation` の作業ツリー。",
    "Rust 1.95.0 (59807616e 2026-04-14)、Windows 10.0.26200、AMD Ryzen 7 7700。release、既定feature + color-management、JPEG並列のみmultithread追加。",
    "Cargo.lock SHA-256: `21acbad87ff15f8219b9d612f077cdd81d06b88e223c1ec1225590b2fd45f598`。",
    "",
    "`review_bench`を同一合成入力でABBA順に3巡、各プロセス5反復した。各プロセス内の中央値を観測単位とし、独立起動6回ずつを再標本化した比の95% bootstrap区間を示す。値はこのマシン・入力の観測であり、全画像での速度を保証しない。JPEGは読み込み、描画、出力ハッシュ計算を含み、初回drawも別計測した。",
    "",
    "| ケース | 方式 | 修正前 ms | 修正後 ms | 後/前の95%区間 | 割当回数 前→後 | 生存ヒープ最大 MiB 前→後 |",
    "| --- | --- | ---: | ---: | --- | --- | --- |"]
summary = []
for case, mode in sorted({(key[0], key[1]) for key in groups}):
    metrics = []
    for variant in ["baseline", "new"]:
        runs = groups[(case, mode, variant)]
        times = [stats.median(int(row["ns"]) for row in rows) for rows in runs.values()]
        rows = [row for values in runs.values() for row in values]
        metrics.append(dict(times=times, ns=stats.mean(times),
            allocations=stats.median(int(row["allocations"]) for row in rows),
            allocated_bytes=stats.median(int(row["allocated_bytes"]) for row in rows),
            peak=stats.median(int(row["peak_live_bytes"]) for row in rows),
            first=stats.median(int(row["first_draw_ns"]) for row in rows),
            hashes=sorted({row["hash"] for row in rows})))
    a, b = metrics
    assert a["hashes"] == b["hashes"], (case, mode, "output differs")
    lo, hi = interval(a["times"], b["times"])
    lines.append(f'| {case} | {mode} | {a["ns"]/1e6:.3f} | {b["ns"]/1e6:.3f} | {lo:.3f}–{hi:.3f} | {a["allocations"]:g}→{b["allocations"]:g} | {a["peak"]/2**20:.3f}→{b["peak"]/2**20:.3f} |')
    summary.append(dict(case=case, mode=mode, before=a, after=b, ratio_interval=[lo, hi]))
lines += ["", "生存ヒープはGlobalAllocで割当と解放を追った差分で、RSSではない。JPEG/WebPの全出力ハッシュは前後一致。draw/pickは測定中に同一画素を観測し、全矩形の画素一致・stride・パディングは回帰試験で別に確認した。ICC計測は恒等Gray16のため、正しさは別の8/10/12/16ビット非恒等試験で確認した。", "",
    "## 処理量と総割当", "",
    "| ケース | 方式 | 百万画素/秒 前→後 | 総割当 MiB 前→後 |",
    "| --- | --- | ---: | ---: |"]
pixel_counts = {"draw-4k": 3840*2160, "pick-4k": 3840*2160,
    "icc-u16-gray": 1024*768, "jpeg-1024x768": 1024*768,
    "jpeg-16x16": 16*16, "webp-animation-24": 24*128*128}
for item in summary:
    a, b = item["before"], item["after"]
    pixels = pixel_counts.get(item["case"])
    rate = f'{pixels*1000/a["ns"]:.2f}→{pixels*1000/b["ns"]:.2f}' if pixels else "—"
    lines.append(f'| {item["case"]} | {item["mode"]} | {rate} | {a["allocated_bytes"]/2**20:.3f}→{b["allocated_bytes"]/2**20:.3f} |')
lines += ["",
    "## 初回描画とプロセスメモリ", ""]
for item in summary:
    if item["case"].startswith("jpeg"):
        lines.append(f'- {item["case"]} ({item["mode"]}): 初回draw {item["before"]["first"]/1e6:.3f}→{item["after"]["first"]/1e6:.3f} ms。')
lines += ["", "プロセスのPeakWorkingSet64を5 ms間隔で観測した別測定（各1起動）。入力生成・符号化・ランタイムも含むため、デコード中の生存ヒープと区別する。", ""]
for entry in json.loads((root/"rss.json").read_text(encoding="utf-8-sig")):
    lines.append(f'- {entry["variant"]} {entry["kind"]} {entry["mode"]}: {entry["peak_working_set"]/2**20:.3f} MiB。')
lines += ["", "## AVIFプレーン抽出とICC変換器再利用", "",
    "AVIF抽出は奇数寸法257×127、各反復100回。ビルドを完了した実行ファイルをABBA順に3巡し、各起動内8反復の中央値から算出した。入力は固定値で全出力サンプルを検証する。単位は100回あたりms。", "",
    "| 配置 | 修正前 ms | 修正後 ms | 後/前の95%区間 |",
    "| --- | ---: | ---: | --- |"]
extraction = defaultdict(lambda: defaultdict(list))
extraction_records = []
for file in sorted(root.glob("extract-final-*.log")):
    run, variant = re.fullmatch(r"extract-final-(\d+)-(baseline|new)\.log", file.name).groups()
    for line in file.read_text(encoding="utf-8-sig").splitlines():
        match = re.fullmatch(r"(u\d+-[a-z]+),(\d+),(\d+),(\d+)", line)
        if match:
            layout, iteration, ns, samples = match.groups()
            extraction[(layout, variant)][run].append(int(ns))
            extraction_records.append(dict(run=run, variant=variant, layout=layout,
                iteration=iteration, ns=ns, samples=samples))
for layout in sorted({key[0] for key in extraction}):
    a, b = [[stats.median(values) for values in extraction[(layout, variant)].values()]
        for variant in ["baseline", "new"]]
    lo, hi = interval(a, b)
    lines.append(f'| {layout} | {stats.mean(a)/1e6:.3f} | {stats.mean(b)/1e6:.3f} | {lo:.3f}–{hi:.3f} |')
if extraction_records:
    with (output/"review-extraction-benchmark.csv").open("w", newline="", encoding="utf-8") as file:
        writer = csv.DictWriter(file, fieldnames=list(extraction_records[0]))
        writer.writeheader()
        writer.writerows(extraction_records)
reuse = defaultdict(list)
for flag, ns in re.findall(r"reuse=(true|false),run=\d+,ns=(\d+)",
        (root/"icc-reuse-bench.log").read_text(encoding="utf-8-sig")):
    reuse[flag].append(int(ns))
lines += ["", "抽出の原データは[review-extraction-benchmark.csv](review-extraction-benchmark.csv)。", "",
    f'ICC小フレーム1000回、1起動内8反復の中央値: 毎回構築 {stats.median(reuse["false"])/1e6:.3f} ms、構築済み変換器の適用 {stats.median(reuse["true"])/1e6:.3f} ms。解析・コンパイルを含む入口と適用のみを比較しており、独立起動の信頼区間は算出していない。', "",
    "## WebP集約の規模とビルド成果物", "",
    "外部webp-rust 0.3.1との符号化バイト一致を確認後、既に依存していた同版のlegacy公開入口へ委譲した。削除した14ファイルは8,478行で、ALPH、bit writer、Huffman、VP8/VP8L予測・エントロピー処理の重複を除いた。ローカルにはWML2の設定・メタデータ・アニメーション合成を残した。", "",
    "| 計測用review_bench | 修正前 bytes | 修正後 bytes |",
    "| --- | ---: | ---: |",
    "| Windows native | 1,940,992 | 1,958,400 |",
    "| wasm32 | 1,299,742 | 1,348,054 |", "",
    "既定feature + color-managementの診断用バイナリで、配布アプリのサイズではない。安全検査等も同時に追加しており、コード集約でバイナリが縮むという結果ではない。記録されたビルド時間はnative 9.85→7.18秒、Wasm 7.90→6.72秒。各1回、キャッシュ条件を完全には揃えていないため、ビルド高速化の根拠にはしない。", "",
    "## 解釈と再現", "",
    "draw/pickの行コピーとICCの作業配列縮小、アニメーションの画素複製削減は改善した。JPEGの1024×768並列処理は総時間が約11%増加した（比の95%区間1.091–1.132）。初回描画とメモリ使用量を改善した代わりに、この入力ではスループットが低下している。小画像のスレッド起動費を避けるため、最大8 MCUは呼出元で処理する。並列時は8件以下の未描画MCUと固定容量キューを用い、コールバックは呼出元で呼ぶ。", "",
    "`cargo build -p wml2 --release --example review_bench --features color-management` でビルドし、`review_bench draw 5`、`icc 5`、`animation 5`、`jpeg 5`を両版でABBA順に実行する。JPEG並列は`multithread`を追加する。合成画素はソースに固定され、入力ファイル・ネットワークに依存しない。原データは[review-benchmark.csv](review-benchmark.csv)。", "",
    "ICC変換器再利用は `cargo test -p wml2 --release --no-default-features --features color-management --test highres_icc benchmark_reusable_icc_transform -- --ignored --nocapture`。AVIF抽出は `cargo test -p wml2 --release --no-default-features --features avifenc,high-bit-depth --lib role_extraction_benchmark -- --ignored --nocapture`。基準版への追加は計測用ソースとmod宣言だけで、実装を変更しない。", ""]
(output/"review-performance.md").write_text("\n".join(lines), encoding="utf-8")
(output/"review-benchmark-summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
print("\n".join(lines[:23]))
