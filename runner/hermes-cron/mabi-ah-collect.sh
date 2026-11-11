#!/bin/sh
# Hermes cron (--no-agent) job: run one mabi-ah collection.
# Success: empty stdout, exit 0. Failure: non-zero exit (Hermes --failure-deliver reports it).
# Binary: $MABI_AH_BIN if set, otherwise `mabi-ah` from PATH.
set -eu

BIN="${MABI_AH_BIN:-mabi-ah}"

if ! command -v "$BIN" >/dev/null 2>&1; then
  echo "mabi-ah-collect: binary not found: $BIN" >&2
  exit 127
fi

exec "$BIN" collect --quiet
