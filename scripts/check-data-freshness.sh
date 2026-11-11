#!/bin/sh
# Fail when data/observed is stale: any expires_at < today, or observed_to older than
# 30 days (NEXON Open API 30-day rule). data/reference/* only needs fetched_at (no expiry).
# Env: DATA_DIR (default: data)   TODAY (YYYY-MM-DD, for tests)
set -eu
cd "$(dirname "$0")/.."
exec python3 - "${DATA_DIR:-data}" "${TODAY:-}" <<'PY'
import datetime, glob, json, os, re, sys

root, today_s = sys.argv[1], sys.argv[2]
today = datetime.date.fromisoformat(today_s) if today_s else datetime.date.today()
errors = []

def date(v, where, field):
    try:
        return datetime.date.fromisoformat(v)
    except (TypeError, ValueError):
        errors.append(f"{where}: {field} missing or not YYYY-MM-DD ({v!r})")

for path in sorted(glob.glob(os.path.join(root, "observed", "*.json"))):
    with open(path, encoding="utf-8") as f:
        d = json.load(f)
    exp = date(d.get("expires_at"), path, "expires_at")
    to = date(d.get("observed_to"), path, "observed_to")
    if exp and exp < today:
        errors.append(f"{path}: expired (expires_at {exp} < {today})")
    if exp and to and (exp - to).days > 30:
        errors.append(f"{path}: expires_at {exp} is {(exp - to).days} days after observed_to {to} (> 30)")
    if to and (today - to).days > 30:
        errors.append(f"{path}: observed_to {to} is {(today - to).days} days old (> 30)")

for path in sorted(glob.glob(os.path.join(root, "reference", "**", "*.json"), recursive=True)):
    with open(path, encoding="utf-8") as f:
        d = json.load(f)
    date((d.get("fetched_at") or "")[:10], path, "fetched_at")

if errors:
    print("data freshness check FAILED:", file=sys.stderr)
    for e in errors:
        print("  " + e, file=sys.stderr)
    sys.exit(1)
print(f"data freshness ok (today {today})")
PY
