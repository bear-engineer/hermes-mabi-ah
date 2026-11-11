#!/bin/sh
# Hermes cron (--no-agent) job: run one mabi-ah collection.
# Success: empty stdout, exit 0. Failure: non-zero exit (Hermes --failure-deliver reports it).
# Binary: $MABI_AH_BIN if set, else `mabi-ah` from PATH, else ~/.local/bin/mabi-ah
# (where the skill installer links it; cron jobs often run with a minimal PATH).
set -eu

BIN="${MABI_AH_BIN:-mabi-ah}"
if [ -z "${MABI_AH_BIN:-}" ] && ! command -v "$BIN" >/dev/null 2>&1 && [ -x "$HOME/.local/bin/mabi-ah" ]; then
  BIN="$HOME/.local/bin/mabi-ah"
fi

if ! command -v "$BIN" >/dev/null 2>&1; then
  echo "mabi-ah-collect: binary not found: $BIN" >&2
  exit 127
fi

exec "$BIN" collect --quiet
