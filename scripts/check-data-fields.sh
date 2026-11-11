#!/bin/sh
# Fail when any JSON under data/ has a forbidden KEY (prices, volume, seller/buyer,
# character, horn, ...) or a string value that looks like an API key / token / email /
# local path. Effect text and names are not inspected for words, only for key shapes.
# Env: DATA_DIR (default: data)
set -eu
cd "$(dirname "$0")/.."
exec python3 - "${DATA_DIR:-data}" <<'PY'
import glob, json, os, re, sys

root = sys.argv[1]
# key fragments (lowercase compare); Korean fragments compared as-is
FORBIDDEN = ["price", "volume", "seller", "buyer", "character", "horn", "token",
             "api_key", "apikey", "secret", "password", "email", "nickname",
             "trade_id", "auction_buy_id", "username", "user_id", "cost",
             "가격", "시세", "거래량", "판매자", "구매자", "캐릭터", "뿔피리", "닉네임"]
VALUE_PATTERNS = {
    "api key": re.compile(r"\b(?:live|test)_[0-9A-Za-z_-]{20,}"),
    "long token": re.compile(r"(?<![0-9A-Za-z_/.-])[0-9A-Za-z_-]{40,}(?![0-9A-Za-z_/.-])"),
    "email": re.compile(r"[\w.+-]+@[\w-]+\.[\w.-]+"),
    "local path": re.compile(r"(?:/Users/|/home/|[A-Za-z]:\\Users\\)"),
}
errors = []

def walk(v, path, where):
    if isinstance(v, dict):
        for k, x in v.items():
            lk = k.lower()
            hit = [f for f in FORBIDDEN if f in lk]
            if hit:
                errors.append(f"{where}: forbidden key '{k}' at {path}")
            walk(x, f"{path}.{k}", where)
    elif isinstance(v, list):
        for i, x in enumerate(v):
            walk(x, f"{path}[{i}]", where)
    elif isinstance(v, str):
        for name, rx in VALUE_PATTERNS.items():
            if rx.search(v):
                errors.append(f"{where}: {name}-like string at {path}")

files = sorted(glob.glob(os.path.join(root, "**", "*.json"), recursive=True))
for p in files:
    with open(p, encoding="utf-8") as f:
        walk(json.load(f), "$", p)

if errors:
    print("data field check FAILED:", file=sys.stderr)
    for e in errors[:50]:
        print("  " + e, file=sys.stderr)
    sys.exit(1)
print(f"data field check ok ({len(files)} files)")
PY
