---
name: mabi-ah
description: Use when asked about Mabinogi auction prices. Runs mabi-ah.
version: 0.1.0
platforms: [macos]
metadata:
  hermes:
    tags: [mabinogi, auction, game, nexon, prices]
---

# mabi-ah — 마비노기 경매장 조회

`mabi-ah`는 NEXON Open API로 마비노기 경매장 매물·거래·시세를 조회하는 CLI다. 모든 출력은 JSON envelope(기본)이다.

## When to use
- 마비노기 아이템 시세, 현재 매물, 최근 거래, 랭킹, 가격 추이, NPC 상점, 확성기(뿔피리) 질문.
- 가격 감시(watch) 등록/확인.
- 마비노기와 무관한 요청에는 쓰지 않는다.

설치 확인: `mabi-ah --version`. 없으면 `sh scripts/install.sh` (이 스킬 폴더) 안내.

## Hard rules
- **API 키를 출력·붙여넣기·로그에 절대 노출하지 않는다.** 키는 `~/.config/nexon/openapi_key` 파일 또는 `NEXON_OPEN_API_KEY` 환경변수에서만 읽힌다. `--key-file <KEY_FILE>`로 경로만 바꿀 수 있다. `cat`으로 키 파일을 읽지 않는다.
- **출처 표기 필수**: 사용자에게 보여주는 모든 답변 끝에 `meta.attribution` 값("Data based on NEXON Open API")을 붙인다.
- **horn**: 기본으로 `--redact-names`를 붙인다(캐릭터명 `***`). 사용자가 명시적으로 요청할 때만 생략.
- 아이템명은 정확히 일치해야 한다(`--name`). 앞의 `@`는 API 원본 이름의 일부다.

## Global options (모든 서브커맨드)
`--format <json|table>` (기본 json), `--db <DB>`, `--key-file <KEY_FILE>`, `--tier <development|service>` (기본 development), `--force-budget`.

## Commands
조회 계열(실시간 API 호출, 예산 소모):

```sh
# 현재 매물: --name 또는 --category, -o 옵션 필터(반복 가능), --stats 통계
mabi-ah listings --name "<아이템명>" [--category <카테고리>] [-o '<필터>'] [--stats] [--all-pages] [--max-pages 10]
mabi-ah listings --category 검 -o '인챈트~불타는' --stats

# 완료된 거래 (listings와 같은 플래그)
mabi-ah trades --name "<아이템명>" --stats [--all-pages] [--max-pages 10]

# 키워드 검색 (1~10 단어, 위치 인자)
mabi-ah search [--category <카테고리>] [-o '<필터>'] <WORDS>...
mabi-ah search 드래곤 블레이드

# NPC 상점 (--npc, --server 필수; --channel 기본 1)
mabi-ah npc-shop --npc <NPC> --server <서버> [--channel 1]

# 확성기 (--server 필수)
mabi-ah horn --server <서버> --redact-names
```

로컬 저장소 계열(API 호출 없음, 단 collect는 호출):

```sh
# 거래 수집 (--max-calls 기본 20, --retain-days 기본 30, --quiet는 성공 시 무출력, cron용)
mabi-ah collect [--max-calls 20] [--retain-days 30] [--quiet]

# 추이: --window 기본 7d (예 24h, 7d), --bucket hour|day (기본 day), --name 생략 시 전체
mabi-ah trend --name "<아이템명>" [--window 7d] [--bucket day]

# 랭킹: --by volume|value (기본 volume), --window 기본 24h, --top 기본 10, --category
mabi-ah rank [--by volume] [--window 24h] [--top 10] [--category <카테고리>]

# 감시 규칙
mabi-ah watch add --name "<아이템명>" [--max-price <골드>] [--category <c>] [-o '<필터>']
mabi-ah watch list
mabi-ah watch rm <ID>
mabi-ah watch check [--emit json|text]   # 새 알림만 보고; text는 크론 전달용 한 줄 형식

# 진단/상태
mabi-ah categories     # 알려진 카테고리
mabi-ah budget         # 오늘의 호출 예산
mabi-ah doctor         # 환경·데이터 상태 점검
mabi-ah config         # 해석된 설정(키 값은 출력 안 됨)
```

`collect --quiet`와 `watch check --emit text`는 envelope 없이 텍스트만(없으면 무출력) 내보낸다.

## Option filter grammar
`-o '<type>[:<sub>](>=|<=|=|~)<value>'`, 반복하면 AND.
- `>=`, `<=`: 숫자 값 필요(`%`는 무시). `=`: 숫자면 숫자 비교, 아니면 문자열 일치. `~`: 포함 검색.
- `<type>`은 API의 `option_type` 문자열, `<sub>`은 `option_sub_type`. 필터 잘못되면 exit 2.
- 예: `-o '인챈트~불타는'`, `-o '내구도>=15'`.
- 쓸 수 있는 type 이름이 불확실하면 `--stats` 없이 한 번 조회해 결과의 options를 보고 정한다.

## Envelope fields to read
```json
{"schema":"mabi_ah/v1","ok":true,"command":"listings","data":{...},
 "meta":{"fetched_at":"...","calls_used":1,
   "budget":{"tier":"development","used_today":12,"limit_today":900},
   "staleness":{"newest_trade_at":"...","lag_seconds":120,"stale":false},
   "attribution":"Data based on NEXON Open API"},
 "error":null}
```
- 항상 `ok`부터 확인. `ok:false`면 `error.{code,nexon_code,message_ko,retryable,hint}`를 읽는다.
- 목록류 `data`: `items`, `count`, `pages`, `truncated`, `stats?`. `truncated:true`면 "일부만 조회됨"이라고 알린다.
- `meta.calls_used`, `meta.budget`: 필요 시 남은 예산을 알린다.
- `meta.staleness.stale:true`면 데이터가 오래됐다고 알린다(아래 참고).

## Exit codes → 사용자에게 할 말
| code | 의미 | 안내 |
|---|---|---|
| 0 | 성공 | 결과 요약 + 출처 |
| 1 | 내부 오류 | "도구 내부 오류가 났어요" + `error.message_ko` |
| 2 | 사용법 오류 | 인자/필터 수정해서 재시도(`error.hint`) |
| 3 | 없음 | 아이템명이 정확해야 함; `hint`의 비슷한 이름 제안 |
| 4 | 키/권한 | "API 키가 없거나 권한이 없어요" — 키 설정 방법만 안내(키 값 요구 금지) |
| 5 | 호출 한도/예산 초과 | "오늘 호출 예산을 다 썼어요(또는 너무 빠름)"; `--force-budget`은 사용자가 원할 때만 |
| 6 | NEXON 점검/장애 | "넥슨 API가 점검/장애 중이에요", 나중에 재시도 |
| 7 | 저장소 오류 | `mabi-ah doctor` 실행해 상태 확인 |

`error.retryable:true`면 잠시 후 한 번 재시도할 수 있다. 반복 재시도 금지.

## Budget rules
- 개발 티어: 초당 5회, **하루 1,000회**. CLI는 **900회로 상한**(90%)을 두고 넘으면 exit 5. `--force-budget`은 1,000까지 허용.
- 호출은 재시도 포함 HTTP 요청마다 센다. 모든 페이지가 1회(페이지당 500건).
- `--all-pages`는 `--max-pages`(기본 10)까지만. 넓은 질의 전에 `mabi-ah budget`을 확인하고, 가능하면 `--name`으로 좁힌다.
- `trend`/`rank`/`categories`/`budget`/`doctor`/`config`/`watch list|add|rm`은 API를 호출하지 않는다. 시세 질문은 수집된 데이터가 있으면 이쪽을 먼저 쓴다.

## Staleness
거래 데이터는 지연이 있다. `meta.staleness.stale`이 true면 답변에 "최근 거래 기준 시각: `newest_trade_at` (UTC; KST는 +9시간), 약 N분 지연"을 명시한다. 수집 데이터(trend/rank)는 `collect`가 돌지 않으면 오래된다 → Runner 설정 안내.

## Watch + Runner (자동 수집·알림)
주기 수집과 알림 전달은 Runner가 한다. 설정은 `runner/hermes-cron/README.md`(저장소) 참고. 요지: Hermes cron이 `mabi-ah collect --quiet` 후 `mabi-ah watch check --emit text`를 실행하고 출력이 있을 때만 전달한다.

## 답변 템플릿 (한국어)
- 시세: `"<아이템> 현재 최저가 <N>골드 (매물 <M>개). 최근 거래 중앙값 <P>골드.\n※ 기준 시각 <KST>\n출처: Data based on NEXON Open API"`
- 매물 없음: `"지금 '<아이템>' 매물이 없어요. 최근 거래 기준 시세는 <P>골드예요."`
- 오래된 데이터: `"⚠️ 데이터가 약 <N>분 지연돼 있어요(최근 거래 <KST>)."`
- 추이: `"<아이템> 최근 <window> 추이: <시작>→<끝>골드 (<±x%>). 거래 <n>건.\n출처: Data based on NEXON Open API"`
- 감시 알림: `"[감시 <id>] <아이템> <n>개, 개당 <price>골드 (목표 <max>골드 이하)\nData based on NEXON Open API"`
- 예산 소진(exit 5): `"오늘 API 호출 예산(<used>/<limit>)을 다 써서 더 조회할 수 없어요. 내일 다시 시도해 주세요."`
- 이름 오류(exit 3): `"'<입력>'과 정확히 일치하는 아이템이 없어요. 혹시 <제안>인가요?"`
