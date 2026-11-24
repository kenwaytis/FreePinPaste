#!/usr/bin/env bash
# Current-machine x86_64 Linux package; not a cross-distribution AppImage.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname -m)" == x86_64 ]] || { echo 'Only current Linux x86_64 is supported.' >&2; exit 1; }
scripts/build-ocr.sh
cargo build --release --locked
OUT="$PWD/dist/FreePinPaste-linux-x86_64"
# This is a generated artifact directory, not the user's install/config directory.
rm -rf -- "$OUT"
mkdir -p "$OUT/ocr" "$OUT/licenses" "$OUT/share"
install -m755 target/release/freepinpaste "$OUT/freepinpaste"
cp -a runtime/ocria/. "$OUT/ocr/"
cp README.md LICENSE THIRD_PARTY.md "$OUT/"
cp -a docs "$OUT/"
cp packaging/install.sh "$OUT/install.sh"
chmod +x "$OUT/install.sh"
# Include licenses for Rust dependencies used by the shipped Linux executable.
python scripts/collect-licenses.py "$OUT/licenses"
cp vendor/xattr/LICENSE-* "$OUT/licenses/"
env -u FREEPINPASTE_OCR_DIR "$OUT/freepinpaste" --ocr-info
(cd "$OUT/ocr"; sha256sum -c "$OLDPWD/packaging/models.sha256")
tar -C dist -czf dist/FreePinPaste-linux-x86_64.tar.gz FreePinPaste-linux-x86_64
sha256sum dist/FreePinPaste-linux-x86_64.tar.gz > dist/FreePinPaste-linux-x86_64.tar.gz.sha256
printf '\nPackage: %s/dist/FreePinPaste-linux-x86_64.tar.gz\n' "$PWD"
