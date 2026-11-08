#!/bin/bash
# Build Film Darkroom for Windows (x64) as a portable zip.
#
#   tools/package-windows.sh          → dist/Film-Darkroom-<version>-x64.zip
#
# Runs on Windows (Git Bash, as on GitHub Actions: .github/workflows/windows.yml) with
# the MSVC toolchain. The zip holds a "Film Darkroom" folder with the app, its notes and
# licences; unzip it anywhere and run "Film Darkroom.exe". The exe is not code-signed, so
# SmartScreen asks once: More info → Run anyway.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
TARGET=x86_64-pc-windows-msvc
VERSION=$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version *= *"\(.*\)"/\1/p;}' Cargo.toml)
DIST="$ROOT/dist"
NAME="Film-Darkroom-$VERSION-x64"
STAGE="$DIST/Film Darkroom"

echo "==> Listing third-party licences for the About window ($TARGET)"
python -I tools/gen_licenses.py "$TARGET"

echo "==> Building Film Darkroom $VERSION ($TARGET)"
cargo build --profile dist -p fd-desktop --target "$TARGET"

echo "==> Assembling $NAME.zip"
rm -rf "$STAGE" "$DIST/$NAME.zip"
mkdir -p "$STAGE"
cp "target/$TARGET/dist/film-darkroom.exe" "$STAGE/Film Darkroom.exe"
cp README.md CHANGELOG.md LICENSE-APACHE LICENSE-MIT "$STAGE/"
(cd "$DIST" && powershell -NoProfile -Command \
  "Compress-Archive -Path 'Film Darkroom' -DestinationPath '$NAME.zip' -Force")

echo "==> Done"
ls -l "$DIST/$NAME.zip"
