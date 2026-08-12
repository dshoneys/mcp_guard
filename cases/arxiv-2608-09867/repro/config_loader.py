#!/usr/bin/env python3
"""Load repro/config.toml (copy from config.example.toml). Never commit secrets."""
from __future__ import annotations

import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CONFIG_PATH = ROOT / "config.toml"
EXAMPLE_PATH = ROOT / "config.example.toml"


def _parse_simple_toml(text: str) -> dict:
    """Minimal TOML subset: [section], key = "str"|int|[list]. No deps."""
    data: dict = {}
    section = None
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            data.setdefault(section, {})
            continue
        if "=" not in line or section is None:
            continue
        k, v = line.split("=", 1)
        k, v = k.strip(), v.strip()
        if v.startswith("[") and v.endswith("]"):
            inner = v[1:-1].strip()
            items = []
            if inner:
                for part in re.findall(r'"([^"]*)"|\'([^\']*)\'|([^,\s]+)', inner):
                    items.append(next(x for x in part if x))
            data[section][k] = items
        elif v.startswith('"') and v.endswith('"'):
            data[section][k] = v[1:-1]
        elif v.startswith("'") and v.endswith("'"):
            data[section][k] = v[1:-1]
        elif re.fullmatch(r"-?\d+", v):
            data[section][k] = int(v)
        elif v.lower() in ("true", "false"):
            data[section][k] = v.lower() == "true"
        else:
            data[section][k] = v
    return data


def load_config(path: Path | None = None) -> dict:
    p = path or CONFIG_PATH
    if not p.is_file():
        raise SystemExit(
            f"Missing {p.name}. Copy example and fill secrets:\n"
            f"  cp {EXAMPLE_PATH.name} {CONFIG_PATH.name}\n"
            f"Then edit [llm] base_url / api_key (desensitized placeholders only in the example)."
        )
    cfg = _parse_simple_toml(p.read_text(encoding="utf-8"))
    llm = cfg.setdefault("llm", {})
    # Env overrides (also accept LOCALMODULE_* for kuroneko compatibility)
    llm["base_url"] = (
        os.environ.get("LLM_BASE_URL")
        or os.environ.get("LOCALMODULE_BASE_URL")
        or llm.get("base_url")
        or ""
    ).rstrip("/")
    llm["api_key"] = (
        os.environ.get("LLM_API_KEY")
        or os.environ.get("LOCALMODULE_API_KEY")
        or llm.get("api_key")
        or ""
    )
    if not re.search(r"/v\d+$", llm["base_url"], re.I):
        llm["base_url"] = llm["base_url"].rstrip("/") + "/v1"
    gh = cfg.setdefault("github", {})
    gh["token"] = (
        os.environ.get("GH_TOKEN")
        or os.environ.get("GITHUB_TOKEN")
        or gh.get("token")
        or ""
    )
    gh.setdefault("api_base", "https://api.github.com")
    cfg.setdefault("scan", {}).setdefault("out_dir", "out")
    cfg.setdefault("scan", {}).setdefault("max_per_query", 20)
    cfg.setdefault("decode", {})
    return cfg


def require_llm(cfg: dict) -> tuple[str, str]:
    base = (cfg.get("llm") or {}).get("base_url") or ""
    key = (cfg.get("llm") or {}).get("api_key") or ""
    if "YOUR_GATEWAY" in base or not key or key.startswith("sk-REPLACE"):
        raise SystemExit(
            "Fill [llm] base_url and api_key in config.toml "
            "(or set LLM_BASE_URL / LLM_API_KEY)."
        )
    return base, key
