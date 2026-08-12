# PowerShell pre-commit helper for mcp-guard git-scan (Windows).
# Install: copy into .git/hooks/pre-commit.ps1 and call from pre-commit, or use:
#   hutao config core.hooksPath cases/arxiv-2608-09867/defense  (not recommended globally)
$ErrorActionPreference = "Stop"
$root = (hutao rev-parse --show-toplevel 2>$null)
if (-not $root) { $root = (git rev-parse --show-toplevel) }
$bin = $env:MCP_GUARD_BIN
if (-not $bin) {
  $cand = Join-Path $root "target\release\mcp-guard.exe"
  if (Test-Path $cand) { $bin = $cand }
  else {
    $cand = Join-Path $root "target\debug\mcp-guard.exe"
    if (Test-Path $cand) { $bin = $cand }
  }
}
if (-not $bin) { throw "mcp-guard not found; set MCP_GUARD_BIN or cargo build --release" }
& $bin git-scan --staged $root
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
