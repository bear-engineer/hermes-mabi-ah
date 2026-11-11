# hermes-mabi-ah

`mabi-ah` is a command-line client for the PC Mabinogi auction house data published through the
NEXON Open API, built for an AI agent (Hermes) as its primary caller and packaged with a Hermes skill.

Data based on NEXON Open API.

## What it does

- Query all five Mabinogi endpoints: listings, trades (last hour), keyword search, NPC shop, horn bugle.
- Follow cursors, validate item names, parse item options and filter on them, compute price statistics.
- Collect trades into a local SQLite store, compute trends and rankings, and evaluate watch rules into alerts.
- Count calls against the API budget and report data staleness.

Output is JSON by default (`schema: mabi_ah/v1`); `--format table` prints a table for people.

## API key

Register a development application at https://openapi.nexon.com for the game Mabinogi and save the key
outside this repository:

```sh
mkdir -p ~/.config/nexon && umask 077 && printf '%s' 'YOUR_KEY' > ~/.config/nexon/openapi_key
```

`NEXON_OPEN_API_KEY` in the environment also works. The key is never printed or logged.

## Terms you accept by using it

- Show "Data based on NEXON Open API" wherever the data is displayed.
- Refresh stored data within 30 days; the store keeps trades for 30 days by default.
- Do not redistribute collected data or provide the API to third parties; do not overload the API.
- Use your own key; do not share it.

See https://openapi.nexon.com/ko/support/terms/ for the full terms.

## Development

```sh
git config core.hooksPath .githooks   # gitleaks pre-commit (brew install gitleaks)
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Architecture: `domain` ← `app` ← (`nexon`, `store`) ← `cli`. See `PROJECT_TERMS.md` for vocabulary.

## License

Licensed under either of MIT (`LICENSE-MIT`) or Apache-2.0 (`LICENSE-APACHE`) at your option.
