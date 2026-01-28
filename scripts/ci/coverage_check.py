#!/usr/bin/env python3
import sys
from pathlib import Path


def parse_lcov(path: Path) -> float:
    total_lines = 0
    hit_lines = 0
    for line in path.read_text().splitlines():
        if line.startswith("LF:"):
            total_lines += int(line.split(":", 1)[1])
        elif line.startswith("LH:"):
            hit_lines += int(line.split(":", 1)[1])
    if total_lines == 0:
        return 0.0
    return (hit_lines / total_lines) * 100.0


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: coverage_check.py <lcov.info> <baseline.txt>")
        return 2
    lcov_path = Path(sys.argv[1])
    baseline_path = Path(sys.argv[2])
    if not lcov_path.exists():
        print(f"lcov file not found: {lcov_path}")
        return 2
    if not baseline_path.exists():
        print(f"baseline file not found: {baseline_path}")
        return 2
    baseline = float(baseline_path.read_text().strip())
    actual = parse_lcov(lcov_path)
    print(f"coverage: {actual:.2f}% (baseline {baseline:.2f}%)")
    if actual + 1e-6 < baseline:
        print("coverage regression detected")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
