# PROJECT_TERMS — hermes-mabi-ah

Single source of truth for the words this repository uses. Code identifiers, CLI commands, the skill
document and the docs use these names. Korean labels are display labels for users.

| Term | Korean | Definition | Distinct from |
|---|---|---|---|
| `Listing` | 매물 | One row of `auction/list` or `auction/keyword-search`: an item currently for sale. | Item (the kind of thing), Trade |
| `Trade` | 거래 | One row of `auction/history`: a completed sale. `trade_id` = API `auction_buy_id`. | query history, Listing |
| `ItemName` | 아이템 이름 | `raw` = API `item_name` as received; `base` = `raw` without a leading `@` (comparison and grouping key); `display` = API `item_display_name` (with enchant prefixes etc.). | — |
| `Category` | 카테고리 | API `auction_item_category`. The allowed set is a registry: spec enum plus values discovered in responses. | spec enum (reference only) |
| `ItemOption` | 옵션 | One element of `item_option`: `option_type`, `sub_type`, `value`, `value2`, `desc` (all strings). | `OptionValue` |
| `OptionValue` | 옵션 값 | Parsed meaning of an `ItemOption`, chosen by `option_type` through a parser registry; unknown shapes stay `Raw`. | `ItemOption` |
| `unit_price` | 개당 가격 | API `auction_price_per_unit`. Bundle price = `unit_price × count`. | total price |
| `Cursor` | 커서 | Opaque next-page token (`next_cursor`); `null` means last page. | page number |
| `CollectRun` | 수집 실행 | One `collect` invocation that pages `auction/history` into the store. | Runner |
| `Watermark` | 워터마크 | Newest trade already stored; collection stops when it reaches it. | — |
| `Gap` | 수집 공백 | A time span where collection could not cover the 1-hour history window. | Staleness |
| `Runner` | 실행 주체 | What runs CLI commands periodically: `hermes-cron` (default, `--no-agent`), `launchd`. | CLI commands |
| `Trend` | 추이 | Time series computed from stored Trades. | API data |
| `Rank` | 순위 | Ordering of items by stored trade volume or value. | — |
| `WatchRule` | 관심 규칙 | User-defined condition on listings (name, max price, option filter). | Alert |
| `Alert` | 알림 | The event of a WatchRule first becoming true for a listing. Fired once. | WatchRule |
| `Outbox` | 알림 대기열 | Alerts waiting to be delivered by a Runner or read by the agent. | — |
| `Budget` | 호출 예산 | Client-side count and cap of API calls per second and per day. | NEXON server limit |
| `Staleness` | 데이터 정체 | The newest trade is older than a threshold relative to now. | API error, Gap |
| `NpcShop` | NPC 상점 | `npcshop/list` result for one NPC, server and channel. | — |
| `HornMessage` | 뿔피리 메시지 | One `horn-bugle-world/history` row. Carries a character name: never stored. | — |

## Decisions

- D1 `Listing` over `AuctionItem`/`Offer`.
- D2 `Trade` over `Sale`/`AuctionHistory`.
- D3 CLI commands use domain names: `listings`, `trades`, `search`, `npc-shop`, `horn`, `collect`, `trend`,
  `rank`, `watch`, plus operations helpers `categories`, `budget`, `doctor`, `config`.
- D4 Keep `item_name` verbatim and add the normalized `base` key; the meaning of `@` is unknown, so it is
  never discarded.
