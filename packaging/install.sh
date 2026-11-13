#!/usr/bin/env bash
set -euo pipefail
SOURCE="$(cd "$(dirname "$0")" && pwd)"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/freepinpaste/app"
mkdir -p "$DEST" "$HOME/.local/bin"
if [[ "$SOURCE" != "$DEST" ]]; then cp -a "$SOURCE/." "$DEST/"; fi
ln -sfn "$DEST/freepinpaste" "$HOME/.local/bin/freepinpaste"
"$DEST/freepinpaste" --install-desktop
printf 'Installed: %s\nStart: %s/freepinpaste --settings\nAutostart is opt-in in Settings.\n' "$DEST" "$DEST"
