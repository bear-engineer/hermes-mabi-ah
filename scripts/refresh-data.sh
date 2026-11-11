#!/bin/sh
# Refresh data/observed from the NEXON Open API.
#   catalog learn (one page per category, >=1s pause between calls, total call cap)
#   -> catalog export --to data/observed
# Needs NEXON_OPEN_API_KEY (env) or the key file known to the CLI. Never prints the key.
#
# Env: MABI_AH_BIN      CLI binary (default: build target/release/mabi-ah)
#      MAX_TOTAL_CALLS  hard cap on live calls (default 40)
#      PAUSE_SECONDS    pause between calls (default 1.2)
#      CATEGORIES       space-separated list; default carries enchants/reforges/options
#      OUT_DIR          export target (default data/observed)
#      LEARN_DB         scratch DB (default: temp file, removed afterwards)
set -eu
cd "$(dirname "$0")/.."

MAX_TOTAL_CALLS=${MAX_TOTAL_CALLS:-40}
PAUSE_SECONDS=${PAUSE_SECONDS:-1.2}
OUT_DIR=${OUT_DIR:-data/observed}
DEFAULT_CATEGORIES="검 도끼 둔기 랜스 활 석궁 듀얼건 수리검 너클 스태프 원드 마도서 오브 체인_블레이드 핸들 실린더 아틀라틀 양손_장비 한손_장비 경갑옷 중갑옷 천옷 로브 장갑 신발 모자/가발 방패 액세서리 얼굴_장식 날개 꼬리 악기 토템 에코스톤"
CATEGORIES=${CATEGORIES:-$DEFAULT_CATEGORIES}

if [ -z "${MABI_AH_BIN:-}" ]; then
  cargo build --release -q -p mabi-ah-cli
  MABI_AH_BIN=target/release/mabi-ah
fi
DB=${LEARN_DB:-$(mktemp "${TMPDIR:-/tmp}/mabi-learn.XXXXXX")}
[ -n "${LEARN_DB:-}" ] || trap 'rm -f "$DB"' EXIT

used=0
for raw in $CATEGORIES; do
  if [ "$used" -ge "$MAX_TOTAL_CALLS" ]; then
    echo "call cap $MAX_TOTAL_CALLS reached; stopping" >&2
    break
  fi
  cat_name=$(printf '%s' "$raw" | tr '_' ' ')
  [ "$used" -eq 0 ] || sleep "$PAUSE_SECONDS"
  out=$("$MABI_AH_BIN" --db "$DB" catalog learn --category "$cat_name" --max-calls 1) || {
    echo "learn failed for '$cat_name' (exit $?)" >&2
    exit 1
  }
  calls=$(printf '%s' "$out" | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["calls"])')
  used=$((used + calls))
  echo "learned '$cat_name' (calls so far: $used)"
done

mkdir -p "$OUT_DIR"
"$MABI_AH_BIN" --db "$DB" catalog export --to "$OUT_DIR"
echo "live calls used: $used"
