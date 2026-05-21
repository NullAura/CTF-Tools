#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="${1:-$ROOT/target/CTF Tools.app}"
VERSION="0.1.0"
DIST="$ROOT/dist"
DMG="$DIST/CTF-Tools-${VERSION}.dmg"

if [[ ! -d "$APP" ]]; then
  "$ROOT/scripts/make_macos_app.sh" "$APP"
fi

mkdir -p "$DIST"
rm -f "$DMG"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/stage"
cp -R "$APP" "$STAGE/stage/"
ln -s /Applications "$STAGE/stage/Applications"

hdiutil create \
  -volname "CTF Tools ${VERSION}" \
  -srcfolder "$STAGE/stage" \
  -ov \
  -format UDZO \
  "$DMG"

echo "wrote: $DMG"
