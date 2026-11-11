#!/bin/sh
# Self-test for check-data-freshness.sh and check-data-fields.sh using temp fixtures.
set -eu
cd "$(dirname "$0")/.."
T=$(mktemp -d "${TMPDIR:-/tmp}/datachecks.XXXXXX")
trap 'rm -rf "$T"' EXIT
fail=0
expect() { # expect <0|1> <label> <cmd...>
  want=$1; label=$2; shift 2
  if "$@" >/dev/null 2>&1; then got=0; else got=1; fi
  if [ "$got" -ne "$want" ]; then echo "FAIL: $label (want exit $want, got $got)"; fail=1; else echo "ok: $label"; fi
}
obs() { # obs <dir> <observed_to> <expires_at> [extra-entry-json]
  mkdir -p "$1/observed"
  printf '{"schema_version":1,"kind":"observed_enchant","attribution":"Data based on NEXON Open API","observed_from":"%s","observed_to":"%s","expires_at":"%s","entries":[%s]}\n' \
    "$2" "$2" "$3" "${4:-}" > "$1/observed/enchants.json"
}
# observed_to 2026-01-01, expires 2026-01-31
obs "$T/a" 2026-01-01 2026-01-31
expect 0 "29 days old passes" env DATA_DIR="$T/a" TODAY=2026-01-30 sh scripts/check-data-freshness.sh
expect 0 "30 days old passes" env DATA_DIR="$T/a" TODAY=2026-01-31 sh scripts/check-data-freshness.sh
expect 1 "31 days old fails" env DATA_DIR="$T/a" TODAY=2026-02-01 sh scripts/check-data-freshness.sh
obs "$T/b" 2026-01-29 2026-02-28
expect 1 "observed_to > 30 days fails even if expires_at is later" env DATA_DIR="$T/b" TODAY=2026-03-01 sh scripts/check-data-freshness.sh
mkdir -p "$T/c/reference/labanyu"
printf '{"fetched_at":"2020-01-01T00:00:00Z","entries":[]}\n' > "$T/c/reference/labanyu/x.json"
expect 0 "reference file without expiry passes" env DATA_DIR="$T/c" TODAY=2026-11-11 sh scripts/check-data-freshness.sh
printf '{"entries":[]}\n' > "$T/c/reference/labanyu/x.json"
expect 1 "reference file without fetched_at fails" env DATA_DIR="$T/c" TODAY=2026-11-11 sh scripts/check-data-freshness.sh

obs "$T/d" 2026-01-01 2026-01-31 '{"slot":"접두","name":"가격 인하 시세 효과","rank":"E","effects":["price of power +5","거래량 증가"],"observed_count":1,"first_seen":"2026-01-01","last_seen":"2026-01-01"}'
expect 0 "ordinary effect text with price words passes" env DATA_DIR="$T/d" sh scripts/check-data-fields.sh
obs "$T/e" 2026-01-01 2026-01-31 '{"slot":"접두","name":"x","rank":"E","price":5}'
expect 1 "key price fails" env DATA_DIR="$T/e" sh scripts/check-data-fields.sh
obs "$T/f" 2026-01-01 2026-01-31 '{"가격":5}'
expect 1 "key 가격 fails" env DATA_DIR="$T/f" sh scripts/check-data-fields.sh
obs "$T/g" 2026-01-01 2026-01-31 '{"nested":[{"Seller":"x"}]}'
expect 1 "nested key seller fails" env DATA_DIR="$T/g" sh scripts/check-data-fields.sh
obs "$T/h" 2026-01-01 2026-01-31 '{"name":"live_0123456789abcdef0123456789abcdef"}'
expect 1 "key-like string fails" env DATA_DIR="$T/h" sh scripts/check-data-fields.sh
obs "$T/i" 2026-01-01 2026-01-31 '{"user":"x"}'
expect 1 "key user fails" env DATA_DIR="$T/i" sh scripts/check-data-fields.sh
obs "$T/j" 2026-01-01 2026-01-31 '{"최저가":5}'
expect 1 "key 최저가 fails" env DATA_DIR="$T/j" sh scripts/check-data-fields.sh
obs "$T/k" 2026-01-01 2026-01-31 '{"amount":5}'
expect 1 "key amount fails" env DATA_DIR="$T/k" sh scripts/check-data-fields.sh
obs "$T/l" 2026-01-01 2027-06-01
expect 1 "expires_at > observed_to + 30d fails" env DATA_DIR="$T/l" TODAY=2026-01-02 sh scripts/check-data-freshness.sh
expect 0 "real data passes" sh scripts/check-data-fields.sh
exit "$fail"
