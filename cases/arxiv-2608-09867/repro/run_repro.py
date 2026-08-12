#!/usr/bin/env python3
"""
Case entrypoint — scripts live here; secrets only in config.toml.

  cp config.example.toml config.toml   # fill [llm] base_url / api_key
  python run_repro.py demo
  python run_repro.py critical
  python run_repro.py github
  python run_repro.py decode
  python run_repro.py all
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def run(script: str, args: list[str] | None = None) -> int:
    cmd = [sys.executable, str(ROOT / script), *(args or [])]
    print("+", " ".join(cmd))
    return subprocess.call(cmd, cwd=str(ROOT))


def step_demo() -> int:
    return run("scan_reasoning_blobs.py", ["demo"])


def step_critical() -> int:
    return run("repro_critical.py")


def step_github() -> int:
    from config_loader import load_config

    cfg = load_config()
    n = str(cfg.get("scan", {}).get("max_per_query") or 20)
    out_dir = Path(cfg["scan"].get("out_dir", "out"))
    out_dir.mkdir(parents=True, exist_ok=True)
    return run(
        "scan_github.py",
        ["--max-per-query", n, "--out", str(out_dir / "github_reasoning_report.json")],
    )


def step_decode() -> int:
    from config_loader import load_config

    cfg = load_config()
    out_dir = Path(cfg["scan"].get("out_dir", "out"))
    report = out_dir / "github_reasoning_report.json"
    if not report.is_file():
        print(f"missing {report}; run: python run_repro.py github", file=sys.stderr)
        return 2
    limit = str((cfg.get("decode") or {}).get("limit") or 6)
    return run(
        "decode_past_turn.py",
        [
            "--report",
            str(report),
            "--limit",
            limit,
            "--mode",
            "past",
            "--out",
            str(out_dir / "decode_report.json"),
        ],
    )


def step_all() -> int:
    for fn in (step_demo, step_critical):
        rc = fn()
        if rc:
            return rc
    from config_loader import load_config

    cfg = load_config()
    if not (cfg.get("github") or {}).get("token"):
        print("skip github/decode (no github.token in config.toml)")
        return 0
    rc = step_github()
    if rc:
        return rc
    return step_decode()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "step",
        choices=("demo", "critical", "github", "decode", "all"),
    )
    args = ap.parse_args()
    return {
        "demo": step_demo,
        "critical": step_critical,
        "github": step_github,
        "decode": step_decode,
        "all": step_all,
    }[args.step]()


if __name__ == "__main__":
    raise SystemExit(main())
