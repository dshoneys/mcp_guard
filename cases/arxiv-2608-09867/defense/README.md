# Defense hooks for REQ-GIT-REASONING-LEAK

```bash
# Linux/macOS
chmod +x cases/arxiv-2608-09867/defense/pre-commit.sh
cp cases/arxiv-2608-09867/defense/pre-commit.sh .git/hooks/pre-commit
```

Windows（PowerShell）：

```powershell
# 在 .git/hooks/pre-commit 里调用：
pwsh -File cases/arxiv-2608-09867/defense/pre-commit.ps1
```

或手动：

```bash
mcp-guard git-scan --staged .
```
