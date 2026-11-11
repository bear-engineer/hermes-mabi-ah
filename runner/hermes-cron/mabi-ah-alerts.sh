#!/bin/sh
# Hermes cron (--no-agent) job: check watch rules and print new alerts.
# No alerts: empty stdout (Hermes sends nothing). Alerts: text lines to stdout.
# Failure: non-zero exit.
# Binary: $MABI_AH_BIN if set, otherwise `mabi-ah` from PATH.
set -eu

BIN="${MABI_AH_BIN:-mabi-ah}"

if ! command -v "$BIN" >/dev/null 2>&1; then
  echo "mabi-ah-alerts: binary not found: $BIN" >&2
  exit 127
fi

exec "$BIN" watch check --emit text
