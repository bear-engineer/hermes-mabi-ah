#!/bin/bash
# Polite single-request fetcher for mabi.labanyu.com reference pages.
#
# usage: LABANYU_CACHE=<dir> fetch.sh URL OUTFILE
#
# - Writes raw/OUTFILE and appends to requests.log under $LABANYU_CACHE (keep the
#   cache OUTSIDE the repository; raw pages are never committed).
# - robots.txt: Crawl-delay 10 -> at least 11 s between requests (enforced from the
#   log); disallowed paths (/in-game/auction/item/, /lab) are refused.
# - Request cap: 80 per cache directory.
# Then extract props.pageProps from <script id="__NEXT_DATA__"> into raw/<page>.next.json
# and run build.py "$LABANYU_CACHE".
set -euo pipefail
[ $# -eq 2 ] || { echo "usage: LABANYU_CACHE=<dir> $0 URL OUTFILE" >&2; exit 2; }
CACHE="${LABANYU_CACHE:?set LABANYU_CACHE to a cache directory outside the repo}"
URL="$1"; OUT="$2"
case "$URL" in
  https://mabi.labanyu.com/*) ;;
  *) echo "only https://mabi.labanyu.com/ URLs" >&2; exit 2 ;;
esac
case "$URL" in
  */in-game/auction/item/*|https://mabi.labanyu.com/lab|https://mabi.labanyu.com/lab/*)
    echo "disallowed by robots.txt: $URL" >&2; exit 2 ;;
esac
mkdir -p "$CACHE/raw"
cd "$CACHE"
LOG=requests.log
touch "$LOG"
n=$(( $(wc -l < "$LOG") + 1 ))
[ "$n" -gt 80 ] && { echo "request cap (80) reached" >&2; exit 1; }
last=$(tail -1 "$LOG" | cut -f1)
now=$(date +%s)
if [ -n "$last" ]; then
  gap=$((now - last))
  [ "$gap" -lt 11 ] && sleep $((11 - gap))
fi
code=$(curl -sS -L -A "hermes-ref-collector/0.2 (personal, respects crawl-delay)" \
  -H "Accept-Language: ko" -o "raw/$OUT" -w "%{http_code}" "$URL")
printf "%s\t%s\t%s\t%s\n" "$(date +%s)" "$code" "$URL" "$(date -u +%FT%TZ)" >> "$LOG"
echo "req#$n $code $(wc -c < "raw/$OUT") bytes -> raw/$OUT"
