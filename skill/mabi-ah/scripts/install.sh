#!/bin/sh
# Install mabi-ah (macOS arm64) and its Hermes skill.
#   install.sh [VERSION]            download release (default: latest)
#   install.sh --from-local PATH    install a local binary build
#   install.sh --dry-run            print actions only
set -eu

REPO="bear-engineer/hermes-mabi-ah"
TARGET="aarch64-apple-darwin"
HERMES_HOME_DIR="${HERMES_HOME:-$HOME/.hermes}"
SKILL_DIR="$HERMES_HOME_DIR/skills/gaming/mabi-ah"
LINK="$HOME/.local/bin/mabi-ah"
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
LOCAL_SKILL_SRC=$(cd "$SCRIPT_DIR/.." && pwd)

DRY=0
FROM_LOCAL=""
VERSION=""

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY=1 ;;
    --from-local)
      [ $# -ge 2 ] || { echo "error: --from-local needs a path" >&2; exit 2; }
      FROM_LOCAL="$2"; shift ;;
    -h|--help)
      sed -n '2,6p' "$0"; exit 0 ;;
    -*) echo "error: unknown option $1" >&2; exit 2 ;;
    *) VERSION="$1" ;;
  esac
  shift
done

run() {
  if [ "$DRY" -eq 1 ]; then echo "[dry-run] $*"; else "$@"; fi
}

hint() {
  echo "hint: build from source instead: cargo install --git https://github.com/$REPO mabi-ah-cli --locked" >&2
}

STAGE=""
cleanup() { [ -n "$STAGE" ] && rm -rf "$STAGE"; return 0; }
trap cleanup EXIT INT TERM

if [ -n "$FROM_LOCAL" ]; then
  [ -f "$FROM_LOCAL" ] || { echo "error: $FROM_LOCAL not found" >&2; exit 1; }
  BIN_SRC="$FROM_LOCAL"
  SKILL_SRC="$LOCAL_SKILL_SRC"
  echo "source: local build $FROM_LOCAL"
else
  if [ "$(uname -s)" != "Darwin" ] || [ "$(uname -m)" != "arm64" ]; then
    echo "error: prebuilt binaries are macOS arm64 only" >&2; hint; exit 1
  fi
  if [ -z "$VERSION" ]; then
    if [ "$DRY" -eq 1 ]; then
      VERSION="latest"
      echo "[dry-run] resolve latest: curl -fsSL https://api.github.com/repos/$REPO/releases/latest"
    else
      VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1) \
        || { echo "error: cannot resolve latest release" >&2; hint; exit 1; }
      [ -n "$VERSION" ] || { echo "error: no release found" >&2; hint; exit 1; }
    fi
  fi
  VERSION="${VERSION#v}"
  NAME="mabi-ah-$VERSION-$TARGET"
  URL="https://github.com/$REPO/releases/download/v$VERSION/$NAME.tar.gz"
  STAGE=$(mktemp -d)
  echo "source: $URL"
  if [ "$DRY" -eq 1 ]; then
    echo "[dry-run] curl -fsSL -o $NAME.tar.gz $URL"
    echo "[dry-run] curl -fsSL -o $NAME.tar.gz.sha256 $URL.sha256"
    echo "[dry-run] verify sha256 with shasum -a 256 -c"
    echo "[dry-run] tar -xzf $NAME.tar.gz"
    BIN_SRC="$STAGE/$NAME/mabi-ah"
    SKILL_SRC="$STAGE/$NAME/skill/mabi-ah"
  else
    curl -fsSL -o "$STAGE/$NAME.tar.gz" "$URL" || { echo "error: download failed" >&2; hint; exit 1; }
    curl -fsSL -o "$STAGE/$NAME.tar.gz.sha256" "$URL.sha256" || { echo "error: checksum download failed" >&2; hint; exit 1; }
    EXPECT=$(awk '{print $1}' "$STAGE/$NAME.tar.gz.sha256")
    ACTUAL=$(shasum -a 256 "$STAGE/$NAME.tar.gz" | awk '{print $1}')
    [ -n "$EXPECT" ] && [ "$EXPECT" = "$ACTUAL" ] || { echo "error: sha256 mismatch" >&2; exit 1; }
    echo "sha256 ok"
    tar -xzf "$STAGE/$NAME.tar.gz" -C "$STAGE"
    BIN_SRC="$STAGE/$NAME/mabi-ah"
    SKILL_SRC="$STAGE/$NAME/skill/mabi-ah"
    [ -f "$BIN_SRC" ] || { echo "error: binary missing in archive" >&2; exit 1; }
  fi
fi

echo "skill dir: $SKILL_DIR"
run mkdir -p "$SKILL_DIR/bin" "$(dirname "$LINK")"
run cp "$SKILL_SRC/SKILL.md" "$SKILL_DIR/SKILL.md"
run mkdir -p "$SKILL_DIR/scripts"
run cp "$SKILL_SRC/scripts/install.sh" "$SKILL_DIR/scripts/install.sh"
run cp "$BIN_SRC" "$SKILL_DIR/bin/mabi-ah"
run chmod 755 "$SKILL_DIR/bin/mabi-ah" "$SKILL_DIR/scripts/install.sh"
if [ "$DRY" -eq 1 ]; then
  echo "[dry-run] xattr -d com.apple.quarantine $SKILL_DIR/bin/mabi-ah (ignore if absent)"
else
  xattr -d com.apple.quarantine "$SKILL_DIR/bin/mabi-ah" 2>/dev/null || true
fi
run ln -sf "$SKILL_DIR/bin/mabi-ah" "$LINK"

case ":$PATH:" in
  *":$HOME/.local/bin:"*) ;;
  *) echo "note: add \$HOME/.local/bin to PATH" ;;
esac
echo "done. Set the API key in ~/.config/nexon/openapi_key (never paste it in chat), then run: mabi-ah doctor"
