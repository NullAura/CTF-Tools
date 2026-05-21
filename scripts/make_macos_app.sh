#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="CTF Tools"
BUNDLE_ID="dev.codex.ctftools"
VERSION="0.1.0"
DEST="${1:-$ROOT/target/${APP_NAME}.app}"
PROFILE="${PROFILE:-release}"

if [[ "$PROFILE" == "release" ]]; then
  cargo build --release -p ctf-app --manifest-path "$ROOT/Cargo.toml"
  BIN="$ROOT/target/release/ctf-app"
else
  cargo build -p ctf-app --manifest-path "$ROOT/Cargo.toml"
  BIN="$ROOT/target/debug/ctf-app"
fi

rm -rf "$DEST"
mkdir -p "$DEST/Contents/MacOS" "$DEST/Contents/Resources"

cat > "$DEST/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>${BUNDLE_ID}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleExecutable</key>
  <string>ctf-app</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

cp "$BIN" "$DEST/Contents/MacOS/ctf-app"
chmod +x "$DEST/Contents/MacOS/ctf-app"

echo "installed: $DEST"
