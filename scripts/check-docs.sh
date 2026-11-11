#!/bin/sh
# Compare the documented commands with the real CLI (`mabi-ah describe`).
#   1. every command from describe is mentioned as `mabi-ah <command>` in
#      README.md, README.en.md and skill/mabi-ah/SKILL.md;
#   2. the sets of `mabi-ah ...` example lines in README.md and README.en.md are equal;
#   3. every example line in the three files names a real command and uses only
#      flags that command (or the global flag list) defines.
# MABI_AH_BIN may point at a built binary; otherwise `cargo run` is used.
set -eu
cd "$(dirname "$0")/.."

TMP=${TMPDIR:-/tmp}/check-docs.$$
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

if [ -n "${MABI_AH_BIN:-}" ]; then
  "$MABI_AH_BIN" describe > "$TMP/describe.json"
else
  cargo run -q -p mabi-ah-cli -- describe > "$TMP/describe.json"
fi

python3 - "$TMP/describe.json" README.md README.en.md skill/mabi-ah/SKILL.md <<'PY'
import json, re, shlex, sys

describe = json.load(open(sys.argv[1], encoding="utf-8"))["data"]
docs = sys.argv[2:]
commands = {c["name"]: c for c in describe["commands"]}
global_flags = {f["long"] for f in describe["global_flags"]} | {"--help", "--version", "-h", "-V"}
errors = []


def flags_of(cmd):
    out = set()
    for f in cmd.get("flags", []):
        if f.get("long"):
            out.add(f["long"])
        if f.get("short"):
            out.add("-" + f["short"].lstrip("-"))
    return out


def examples(text):
    """`mabi-ah ...` lines inside fenced code blocks, trailing `# comment` removed."""
    out, fenced = set(), False
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("```"):
            fenced = not fenced
            continue
        if fenced and s.startswith("mabi-ah "):
            s = re.sub(r"\s+#\s.*$", "", s).strip()
            out.add(s)
    return out


def resolve(tokens):
    """Longest command name matching the leading non-flag tokens."""
    words, i = [], 0
    while i < len(tokens):
        t = tokens[i]
        if t.startswith("-"):
            # global flag before the command; skip its value if it takes one
            g = next((f for f in describe["global_flags"] if f["long"] == t), None)
            i += 2 if g and g.get("takes_value") else 1
            continue
        words.append(t)
        i += 1
        if len(words) == 2:
            break
    for n in (2, 1):
        name = " ".join(words[:n])
        if name in commands:
            return name
    return None


per_doc = {}
for path in docs:
    text = open(path, encoding="utf-8").read()
    for name in commands:
        if f"mabi-ah {name}" not in text:
            errors.append(f"{path}: command not mentioned: mabi-ah {name}")
    ex = examples(text)
    per_doc[path] = ex
    for line in sorted(ex):
        try:
            tokens = shlex.split(line)[1:]
        except ValueError as e:
            errors.append(f"{path}: cannot parse example: {line} ({e})")
            continue
        name = resolve(tokens)
        if name is None:
            errors.append(f"{path}: example names no known command: {line}")
            continue
        allowed = flags_of(commands[name]) | global_flags
        for t in tokens:
            if t.startswith("-") and not re.match(r"^-\d", t):
                flag = t.split("=", 1)[0]
                if flag not in allowed:
                    errors.append(f"{path}: unknown flag {flag} for '{name}': {line}")

ko, en = per_doc["README.md"], per_doc["README.en.md"]
for line in sorted(ko - en):
    errors.append(f"README.en.md: missing example present in README.md: {line}")
for line in sorted(en - ko):
    errors.append(f"README.md: missing example present in README.en.md: {line}")

if errors:
    print("check-docs: FAIL", file=sys.stderr)
    for e in errors:
        print("  " + e, file=sys.stderr)
    sys.exit(1)
print(f"check-docs: ok ({len(commands)} commands; examples README.md={len(ko)} README.en.md={len(en)} SKILL.md={len(per_doc['skill/mabi-ah/SKILL.md'])})")
PY
