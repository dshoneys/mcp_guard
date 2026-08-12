# Lab scripts — run via run_repro.py after copying config.example.toml → config.toml

| File | Role |
|------|------|
| `config.example.toml` | Desensitized URL/key placeholders |
| `config.toml` | Local secrets (**gitignored**) |
| `run_repro.py` | Entrypoint: demo / critical / github / decode / all |
| `repro_critical.py` | Marker control experiment (real decrypt proof) |
| `scan_reasoning_blobs.py` | Local detectors |
| `scan_github.py` | GitHub Code Search sample |
| `decode_past_turn.py` | past_turn decode |
| `config_loader.py` | Shared config loader |

No GitCode account required. See `../REPRO-PLAN.md`.
