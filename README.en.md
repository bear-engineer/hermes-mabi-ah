# hermes-mabi-ah

[한국어](README.md)

`mabi-ah` is a command-line client for the PC Mabinogi auction house data published through the NEXON Open API. Its primary caller is an AI agent (Hermes), and it ships with a Hermes skill (`skill/mabi-ah/`). People can use it directly too.

Data based on NEXON Open API.

## Introduction

- Queries all five Mabinogi endpoints: current listings, trades of the last hour, keyword search, NPC shop, horn bugle.
- Follows cursors across pages, parses item options and filters on them (enchant, reforge, erg and more).
- Collects trades into a local SQLite store, computes price trends and rankings, and turns watch rules into alerts.
- Embeds an enchant/reforge name catalog in the binary, so option names can be looked up right after install.
- Counts API calls against the daily budget and reports how stale the trade data is.

Output is a JSON envelope by default (`schema: mabi_ah/v1`); `--format table` prints a table for people. Listings and trades carry `price_ko`, the price written with Korean units (e.g. `1억5000만`).

## Install

The skill's install script installs the binary and the Hermes skill together. Prebuilt binaries exist for macOS arm64 (Apple Silicon) only.

```sh
sh skill/mabi-ah/scripts/install.sh            # download the latest release (sha256 verified)
sh skill/mabi-ah/scripts/install.sh --dry-run  # print the actions only
cargo build --release -p mabi-ah-cli
sh skill/mabi-ah/scripts/install.sh --from-local target/release/mabi-ah   # a binary you built
```

The skill goes to `~/.hermes/skills/gaming/mabi-ah/` (under `HERMES_HOME` if set) and the executable is linked at `~/.local/bin/mabi-ah`. On other platforms build from source: `cargo install --git https://github.com/bear-engineer/hermes-mabi-ah mabi-ah-cli --locked`.

## API key

Register a development application for the game Mabinogi at [NEXON Open API](https://openapi.nexon.com) and save the key outside this repository:

```sh
mkdir -p ~/.config/nexon && umask 077 && printf '%s' 'YOUR_KEY' > ~/.config/nexon/openapi_key
```

The environment variable `NEXON_OPEN_API_KEY` also works; `--key-file <file>` points at another file. The key is never printed or logged.

A development key allows 5 calls per second and 1,000 per day. `mabi-ah` stops at 90% of that (900) and exits with code 5; `--force-budget` allows the full 1,000. For a service key pass `--tier service`.

## Commands

Global options for every command: `--format <json|table>`, `--db <DB>`, `--key-file <KEY_FILE>`, `--tier <development|service>`, `--force-budget`.

Commands that call the API (they use budget):

| Command | What it does |
|---|---|
| `mabi-ah listings` | Current listings for an item name (`--name`, exact) or a category (`--category`) |
| `mabi-ah trades` | Trades of the last hour; same flags as `listings` |
| `mabi-ah search` | Keyword search (1 to 10 words) over current listings |
| `mabi-ah npc-shop` | NPC shop contents (`--npc` and `--server` required) |
| `mabi-ah horn` | Horn bugle shouts (`--server` required, `--redact-names` hides character names) |
| `mabi-ah collect` | Collect trades into the local store (for the Runner) |
| `mabi-ah option-prices` | Lowest current price per value and level of one option family |
| `mabi-ah catalog learn` | Scan one category's current listings to grow the local catalog |
| `mabi-ah watch check` | Check watch rules against current listings; reports only new alerts |

Commands that do not call the API:

| Command | What it does |
|---|---|
| `mabi-ah trend` | Price trend of collected trades (`--window`, `--bucket hour\|day`) |
| `mabi-ah rank` | Ranking of collected trades by count or traded value |
| `mabi-ah watch add` / `mabi-ah watch list` / `mabi-ah watch rm` | Add, list, remove watch rules |
| `mabi-ah catalog enchant` | Enchant name, slot, rank, effects |
| `mabi-ah catalog reforge` | Reforge options and level ranges per equipment type |
| `mabi-ah catalog option-types` | Option types, value formats, observed value counts |
| `mabi-ah catalog values` | Observed values of one option type (e.g. 토템 효과, 에르그) |
| `mabi-ah catalog reference` | Labanyu reference name lists (totems, echostones, Murias relics, set bonuses and more) |
| `mabi-ah catalog import` | Import a file in the `data/observed` format into the local catalog |
| `mabi-ah catalog export` | Write the local observed catalog as `data/observed` files |
| `mabi-ah categories` | Known auction categories |
| `mabi-ah budget` | Calls used today and the limit |
| `mabi-ah doctor` | Environment and data health check |
| `mabi-ah config` | Resolved configuration (the key value is never shown) |
| `mabi-ah describe` | Every command, flag, filter grammar, option family and example as JSON |

Examples:

```sh
mabi-ah listings --name "@롱 소드" --stats
mabi-ah trades --name "@롱 소드" --stats --enchant-rank '>=E'
mabi-ah search 드래곤 블레이드 --exclude-blueprints
mabi-ah npc-shop --npc 델 --server 류트
mabi-ah horn --server 류트 --redact-names
mabi-ah collect --quiet
mabi-ah trend --name "@롱 소드" --window 7d
mabi-ah rank --by volume --window 24h
mabi-ah watch add --name "@롱 소드" --max-price 100000
mabi-ah watch list
mabi-ah watch rm 1
mabi-ah watch check --emit text
mabi-ah categories
mabi-ah budget
mabi-ah doctor
mabi-ah config
mabi-ah describe
```

`mabi-ah describe` and `mabi-ah <command> --help` are the reference for flags.

## Option search

The auction API cannot search by option. `mabi-ah` fetches listings and filters them afterwards, so narrow the query with `--name` or `--category` first to save calls. Filter flags work on `listings`, `trades` and `search`; several flags are ANDed.

| Flag | Grammar | Example |
|---|---|---|
| `--enchant` | `[접두\|접미:]<name>[(>=\|<=\|=)<rank>]` | `--enchant '접두:날카로운'` |
| `--enchant-rank` | `[접두\|접미](>=\|<=\|=)<rank>[,...]` | `--enchant-rank '>=E'` |
| `--reforge` | `[<slot 1-3>:]<option>[(>=\|<=\|=)<level>]`, any slot unless given | `--reforge '최대 공격력>=18'` |
| `--erg` | `<grade S\|A\|B>[(>=\|<=\|=)<level>]` | `--erg 'S>=40'` |
| `-o`, `--option` | `[!]<type>[:<sub>](>=\|<=\|=\|~)<value>[\|<value>...]` | `-o '색상~빨강\|파랑'` |
| `--exclude-blueprints` | Drop items whose name contains 도면, 옷본 or 심볼 | |

- Enchant rank order is F < E < D < C < B < A < 9 < 8 < … < 1, so `>=E` means E up to 1.
- In `-o`, `>=` and `<=` compare numbers, `=` compares numbers when the value is numeric and exact text otherwise, `~` tests containment. A leading `!` negates; `|` lists alternative values (any may match). Write a literal `|` as `\|` (any `-o` with a backslash is parsed by the same rules). `watch add -o` takes plain filters only; `!` and `|` exit with 2.
- Names are checked before any call. An unknown option type exits with 2; an unknown enchant or reforge option name exits with 3, and no API call is made. `error.hint` lists similar names (also for `option-prices --family`).

```sh
mabi-ah listings --category 장갑 --enchant '접두:날카로운' --enchant-rank '>=E'
mabi-ah listings --category 검 --reforge '최대 공격력>=18' --exclude-blueprints
mabi-ah listings --category 검 --reforge '2:최대 공격력>=18' --erg 'S>=40'
mabi-ah listings --category 검 -o '!인챈트 불가능=true' -o '색상~빨강|파랑'
mabi-ah option-prices --category 토템 --family "토템 효과" --max-pages 2
```

A reforge option name written without its space (real output, 0 API calls, shortened):

```text
$ mabi-ah listings --category 검 --reforge '최대공격력>=18'
mabi-ah: 찾을 수 없습니다: reforge option '최대공격력' [not_found]
{"schema":"mabi_ah/v1","ok":false,"command":"listings","data":null,"meta":{...,"calls_used":0,...},
 "error":{"code":"not_found","nexon_code":null,"message_ko":"찾을 수 없습니다: reforge option '최대공격력'",
 "retryable":false,"hint":"비슷한 이름: 최대 공격력, 최소 공격력, 최대생명력"}}
```

Grammar, option families and escapes in detail: [docs/options.md](docs/options.md).

## Catalog and repository metadata

The catalog keeps option names from being guessed. It merges data shipped in the repository with the user's local observations.

| Path | `source` | Contents | Freshness |
|---|---|---|---|
| `data/facts/` | `repo_fact` | Hand-written rules: enchant slots, rank order, value formats per option family, equipment types | No expiry |
| `data/observed/` | `repo_observed` | Enchant, reforge and option value names, slots, ranks, effect text, level ranges, observed counts and dates taken from NEXON Open API auction responses | Until `expires_at` (30 days after observation); a weekly workflow refreshes it and CI fails on expiry |
| `data/reference/labanyu/` | `labanyu_reference` | Reference data from [mabi.labanyu.com](https://mabi.labanyu.com): reforge option names and level ranges (including the 한돌 cap), totem, echostone and Murias relic effects, set bonus names, enchant scroll details, Labanyu's classifications | `fetched_at` per file, refreshed manually |
| Local DB | `local_observed`, `user_import` | Values seen by queries, `collect` and `catalog learn`, and files brought in with `catalog import` | Purged after 30 days |

- When the same entry exists in several places, the most recent observation wins. On equal dates the order is `user_import` > `local_observed` > `repo_observed` > `labanyu_reference` > `repo_fact`. Every row carries `source` and `observed_at`.
- Repository data never holds prices, trade volumes, sellers or buyers, character names, horn messages, raw responses or keys.
- Read Labanyu data with its source and curation in mind. **The dungeon grade is assigned at Labanyu's discretion (던전 등급은 라바뉴가 임의 부여)**: the enchant may not actually drop there, and most entries are `null`. The equipment slot classification is also Labanyu's, and a totem's `max_threshold` is only the highest value the site tracks, not a confirmed game cap.
- `세공 랭크` (reforge rank) has never been observed and its meaning is unknown. If it appears, the raw text is kept and not interpreted.

```sh
mabi-ah catalog enchant --query 날카
mabi-ah catalog reforge --category "한손 검"
mabi-ah catalog option-types
mabi-ah catalog values --type "토템 효과"
mabi-ah catalog reference --kind totem_abilities --query 대미지
mabi-ah catalog reference --kind echostone_awakening_abilities --query 체력 --limit 5
mabi-ah catalog learn --category 장갑 --max-calls 3
mabi-ah catalog import ./enchants.json
mabi-ah catalog export --to data/observed
```

Real output (`data` of `catalog enchant --query 날카 --limit 1`):

```json
{"count":1,"rows":[{"dungeon_grade":null,"effects":["레벨이 12 이상일때 크리티컬 2 증가"],
 "equipment_slot_classification":[{"group":"장비 제한 없음","slot":"장비 제한 없음"}],
 "first_seen":"2026-11-11","last_seen":"2026-11-11","name":"날카로운","observed_at":"2026-11-11",
 "observed_count":5,"rank":"E","slot":"접두","source":"repo_observed",
 "stats":[{"name":"크리티컬","value":2}],"target_names":[]}]}
```

Real output (first row of `catalog reforge --category "한손 검" --limit 2`):

```json
{"category":"한손 검","limit_break_max_level":5,"max_level":5,"min_level":2,"observed_at":"2026-11-11",
 "option":"공격 속도 증가 세트 효과 수치","range_text":"2~5 (한돌: 5)","slot":null,"source":"labanyu_reference"}
```

File formats and the refresh procedure: [data/README.md](data/README.md) and [docs/data.md](docs/data.md).

## What it can and cannot do

Can

- With the API alone: listings and last-hour trades by name or category, keyword search, NPC shop, horn bugle, option filters (applied after fetching), a lowest-current-price table per option value and level.
- With local storage: price trends, rankings, watch alerts, the observed catalog.
- With repository metadata: enchant and reforge catalogs plus typo suggestions right after install (source and observation date shown).

Cannot

- Search by option on the server. Everything is fetched and then filtered, so broad queries use a lot of budget.
- Show sellers or listing times.
- Show trades older than one hour, except what `collect` has stored.
- Tell where an enchant can be worn (not visible in the API), the official reforge caps, or what `세공 랭크` means. The 한돌 cap is a Labanyu reference value only.
- Bundle public wiki data.

## Runner

A Runner handles periodic collection and alert delivery. The recommended setup is two Hermes cron `--no-agent` jobs: one runs `mabi-ah collect --quiet`, the other runs `mabi-ah watch check --emit text` and delivers to Discord only when there is output. No model is called, so it costs nothing. Setup: [runner/hermes-cron/README.md](runner/hermes-cron/README.md); comparison of options: [docs/runner.md](docs/runner.md). A launchd template is in `runner/launchd/`.

## Terms and attribution

- Show "Data based on NEXON Open API" wherever the data is displayed; it is in `meta.attribution` of every output.
- When showing Labanyu reference data (`source: labanyu_reference`, or rows with Labanyu fields such as `stats`, `dungeon_grade`, `limit_break_max_level`), also credit "mabi.labanyu.com". `meta.attribution` already lists it when such rows are present.
- Refresh stored data within 30 days; the local store keeps trades for 30 days by default.
- Do not redistribute fetched data or provide the API to third parties, and do not overload the API.
- Use your own key and do not share it.

Full terms: https://openapi.nexon.com/ko/support/terms/

## Development

```sh
git config core.hooksPath .githooks   # gitleaks pre-commit (brew install gitleaks)
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
sh scripts/check-docs.sh              # compare documented commands and flags with describe
sh scripts/check-data-freshness.sh    # 30-day expiry of data/observed
sh scripts/check-data-fields.sh       # forbidden fields in data/
```

Architecture: `domain` ← `app` ← (`nexon`, `store`) ← `cli`. Vocabulary follows `PROJECT_TERMS.md`.

## License

Licensed under either of MIT (`LICENSE-MIT`) or Apache-2.0 (`LICENSE-APACHE`) at your option.
