#!/usr/bin/env python3
"""Serve ui/preview/* for Code-as-Design review (no Figma).

  python scripts/ui_preview.py
  python scripts/ui_preview.py --port 8765
"""

from __future__ import annotations

import argparse
import functools
import http.server
import os
import socketserver
import sys
import webbrowser
from pathlib import Path


class RootedHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, directory: str | None = None, **kwargs):
        super().__init__(*args, directory=directory, **kwargs)

    def log_message(self, fmt: str, *args) -> None:
        sys.stderr.write("[%s] %s\n" % (self.log_date_time_string(), fmt % args))


def main() -> int:
    ap = argparse.ArgumentParser(description="MCP Guard UI preview server")
    ap.add_argument("--port", type=int, default=8765)
    ap.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="mcp_guard repo root",
    )
    ap.add_argument("--no-open", action="store_true")
    args = ap.parse_args()

    preview = args.root / "ui" / "preview"
    preview.mkdir(parents=True, exist_ok=True)

    # Index of available previews
    index = preview / "index.html"
    if not index.is_file():
        links = []
        for child in sorted(preview.iterdir()):
            if child.is_dir() and (child / "index.html").is_file():
                links.append(f'<li><a href="{child.name}/">{child.name}</a></li>')
        index.write_text(
            "<!DOCTYPE html><meta charset=utf-8><title>MCP Guard UI previews</title>"
            "<h1>UI previews (Code as Design)</h1><ul>"
            + ("".join(links) or "<li>(none yet)</li>")
            + "</ul>",
            encoding="utf-8",
        )

    handler = functools.partial(RootedHandler, directory=str(preview))
    with socketserver.TCPServer(("127.0.0.1", args.port), handler) as httpd:
        url = f"http://127.0.0.1:{args.port}/"
        print(f"Serving {preview}")
        print(f"Open {url}")
        if not args.no_open:
            webbrowser.open(url)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nstopped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
