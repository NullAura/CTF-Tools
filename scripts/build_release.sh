#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/scripts/make_macos_app.sh" "$ROOT/target/CTF Tools.app"
"$ROOT/scripts/make_macos_dmg.sh" "$ROOT/target/CTF Tools.app"
