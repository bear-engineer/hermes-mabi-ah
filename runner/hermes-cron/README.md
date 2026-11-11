# Hermes cron Runner (recommended)

Two `--no-agent` jobs: no model calls, the script's stdout is delivered as-is, and empty stdout sends nothing.

## Install

1. Copy the scripts to the Hermes scripts directory:

   ```sh
   mkdir -p ~/.hermes/scripts
   cp runner/hermes-cron/mabi-ah-collect.sh runner/hermes-cron/mabi-ah-alerts.sh ~/.hermes/scripts/
   chmod +x ~/.hermes/scripts/mabi-ah-collect.sh ~/.hermes/scripts/mabi-ah-alerts.sh
   ```

2. If `mabi-ah` is not on PATH, export the binary location for the Hermes process:

   ```sh
   export MABI_AH_BIN=/absolute/path/to/mabi-ah
   ```

3. Create the jobs. Replace `<channel>` with the Discord channel ID.

   Collection every 30 minutes. Failures go to Discord; success is silent:

   ```sh
   hermes cron create 30m --no-agent --script mabi-ah-collect.sh --name mabi-ah-collect --deliver local --failure-deliver discord:<channel>
   ```

   Alerts every 10 minutes. Output only when a watch rule fires:

   ```sh
   hermes cron create 10m --no-agent --script mabi-ah-alerts.sh --name mabi-ah-alerts --deliver discord:<channel>
   ```

## Check

```sh
hermes cron runs
hermes cron incidents
```

## Notes

- `mabi-ah-collect.sh` runs `mabi-ah collect --quiet` and exits non-zero on failure.
- `mabi-ah-alerts.sh` runs `mabi-ah watch check --emit text`. Empty output means no alerts.
- The scripts use `MABI_AH_BIN` when set, otherwise `mabi-ah` from PATH.
- The launchd alternative is in `runner/launchd/` (template only).
