#!/usr/bin/env bash
# Build release mcp-guard and pack MCP Guard.app (menu-bar agent).
# Docs: doc/structurizr/MACOS-APP.md
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

pick_sdkroot() {
  if [[ -n "${SDKROOT:-}" && -d "${SDKROOT}" ]]; then
    echo "${SDKROOT}"
    return
  fi
  local base="/Library/Developer/CommandLineTools/SDKs"
  local cand
  for cand in MacOSX15.4.sdk MacOSX15.sdk MacOSX14.sdk MacOSX13.sdk; do
    if [[ -d "${base}/${cand}" ]]; then
      echo "${base}/${cand}"
      return
    fi
  done
  # Fall back to default (may fail on MacOSX27 TBD parse errors).
  xcrun --show-sdk-path 2>/dev/null || true
}

SDK="$(pick_sdkroot)"
if [[ -n "${SDK}" ]]; then
  export SDKROOT="${SDK}"
  echo "SDKROOT=${SDKROOT}"
fi

echo "==> cargo build --release"
cargo build --release

BIN="${ROOT}/target/release/mcp-guard"
test -x "${BIN}"

VAULT_DIR="${MCP_GUARD_VAULT_DIR:-${ROOT}}"
STORE="${VAULT_DIR}/mcp-guard-vault.enc"
KEY="${VAULT_DIR}/mcp-guard-vault.key"
LOGO="${ROOT}/ui/brand/logo.png"
APP_NAME="MCP Guard.app"
STAGE="${ROOT}/target/macos/${APP_NAME}"

echo "==> staging ${STAGE}"
rm -rf "${STAGE}"
mkdir -p "${STAGE}/Contents/MacOS" "${STAGE}/Contents/Resources"

# CFBundleExecutable MUST be this binary (do not exec a differently named path).
cp -f "${BIN}" "${STAGE}/Contents/MacOS/MCPGuard"
chmod +x "${STAGE}/Contents/MacOS/MCPGuard"

cat > "${STAGE}/Contents/Resources/mcp-guard.toml" <<EOF
[vault]
store_path = "${STORE}"
key_path = "${KEY}"
ref_ttl_secs = 300
EOF

if [[ -f "${LOGO}" ]]; then
  ICONSET="$(mktemp -d)/AppIcon.iconset"
  mkdir -p "${ICONSET}"
  for s in 16 32 64 128 256 512; do
    sips -z "${s}" "${s}" "${LOGO}" --out "${ICONSET}/icon_${s}x${s}.png" >/dev/null
    sips -z "$((s * 2))" "$((s * 2))" "${LOGO}" --out "${ICONSET}/icon_${s}x${s}@2x.png" >/dev/null
  done
  iconutil -c icns "${ICONSET}" -o "${STAGE}/Contents/Resources/AppIcon.icns"
fi

cat > "${STAGE}/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>zh_CN</string>
	<key>CFBundleExecutable</key>
	<string>MCPGuard</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
	<key>CFBundleIdentifier</key>
	<string>center.sud.mcp-guard</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>MCP Guard</string>
	<key>CFBundleDisplayName</key>
	<string>MCP Guard</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>0.1.0</string>
	<key>CFBundleVersion</key>
	<string>3</string>
	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>LSUIElement</key>
	<true/>
	<key>NSHighResolutionCapable</key>
	<true/>
</dict>
</plist>
PLIST

plutil -lint "${STAGE}/Contents/Info.plist" >/dev/null

if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "${STAGE}" 2>/dev/null || true
fi

install_copy() {
  local dest="$1"
  mkdir -p "$(dirname "${dest}")"
  rm -rf "${dest}"
  cp -R "${STAGE}" "${dest}"
  xattr -dr com.apple.quarantine "${dest}" 2>/dev/null || true
  echo "installed ${dest}"
}

install_copy "${HOME}/Applications/${APP_NAME}"
install_copy "${HOME}/Desktop/${APP_NAME}"
if [[ -w /Applications ]]; then
  install_copy "/Applications/${APP_NAME}"
fi

# Keep CLI in sync for PATH users.
mkdir -p "${HOME}/.cargo/bin"
cp -f "${BIN}" "${HOME}/.cargo/bin/mcp-guard"
chmod +x "${HOME}/.cargo/bin/mcp-guard"

echo "==> done"
echo "    open -a \"MCP Guard\""
echo "    vault: ${STORE}"
