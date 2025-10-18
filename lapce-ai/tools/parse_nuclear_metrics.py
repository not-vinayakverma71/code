#!/usr/bin/env python3
import sys
import re
import json
import argparse

PATTERNS = [
    (re.compile(r"Throughput:\s*([0-9.]+)M\s*msg/sec", re.I), lambda m, acc: acc.update({"throughput_msgs_per_sec": float(m.group(1)) * 1_000_000})),
    (re.compile(r"Bandwidth:\s*([0-9.]+)\s*MB/sec", re.I), lambda m, acc: acc.update({"bandwidth_mb_per_sec": float(m.group(1))})),
    (re.compile(r"Latency Percentiles:\s*$", re.I), lambda m, acc: None),
    (re.compile(r"p50:\s*([0-9.]+)\s*μs", re.I), lambda m, acc: acc.setdefault("latency_p50_us", float(m.group(1)))),
    (re.compile(r"p99:\s*([0-9.]+)\s*μs", re.I), lambda m, acc: acc.setdefault("latency_p99_us", float(m.group(1)))),
    (re.compile(r"p99\.9:\s*([0-9.]+)\s*μs", re.I), lambda m, acc: acc.setdefault("latency_p999_us", float(m.group(1)))),
    (re.compile(r"Max latency:\s*([0-9.]+)μs", re.I), lambda m, acc: acc.update({"max_latency_us": float(m.group(1))})),
    (re.compile(r"Peak memory:\s*([0-9.]+)\s*MB", re.I), lambda m, acc: acc.update({"peak_memory_mb": float(m.group(1))})),
    (re.compile(r"Memory growth\s*([0-9.]+)KB", re.I), lambda m, acc: acc.update({"memory_growth_kb": float(m.group(1))})),
    (re.compile(r"Final memory growth:\s*([0-9]+)\s*KB", re.I), lambda m, acc: acc.update({"final_memory_growth_kb": float(m.group(1))})),
    # Generic RESULT lines: RESULT key=value
    (re.compile(r"RESULT\s+([A-Za-z0-9_]+)=([0-9.]+)"), lambda m, acc: acc.update({m.group(1): float(m.group(2))})),
]


def parse_metrics(stream):
    acc = {}
    for raw in stream:
        line = raw.strip()
        for rx, fn in PATTERNS:
            m = rx.search(line)
            if m:
                try:
                    fn(m, acc)
                except Exception:
                    pass
    return acc


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--input", required=False, help="input log file; if omitted, reads stdin")
    ap.add_argument("--output", required=False, help="output json file; if omitted, prints to stdout")
    args = ap.parse_args()

    if args.input:
        with open(args.input, "r", encoding="utf-8", errors="ignore") as f:
            metrics = parse_metrics(f)
    else:
        metrics = parse_metrics(sys.stdin)

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            json.dump(metrics, f, indent=2, sort_keys=True)
    else:
        print(json.dumps(metrics, indent=2, sort_keys=True))

if __name__ == "__main__":
    main()
