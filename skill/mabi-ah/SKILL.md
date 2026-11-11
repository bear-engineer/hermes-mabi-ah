---
name: mabi-ah
description: Use when asked about Mabinogi auction prices. Runs mabi-ah.
version: 0.2.0
platforms: [macos]
metadata:
  hermes:
    tags: [mabinogi, auction, game, nexon, prices]
---

# mabi-ah — 마비노기 경매장 조회

`mabi-ah`는 NEXON Open API로 마비노기 경매장 매물·거래·시세를 조회하는 CLI다. 모든 출력은 JSON envelope(기본)이다.

## When to use
- 마비노기 아이템 시세, 현재 매물, 최근 거래, 랭킹, 가격 추이, NPC 상점, 확성기(뿔피리) 질문.
- 옵션이 붙은 매물 질문: 인챈트(이름·랭크), 세공(옵션명·레벨·슬롯), 에르그, 개조, 피어싱, 토템·에코스톤 효과.
- 옵션 이름·인챈트 효과·세공 레벨 범위 같은 메타데이터 질문(카탈로그).
- 가격 감시(watch) 등록/확인.
- 마비노기와 무관한 요청에는 쓰지 않는다.

설치 확인: `mabi-ah --version`. 없으면 `sh scripts/install.sh` (이 스킬 폴더) 안내.
명령·플래그가 헷갈리면 `mabi-ah describe`(API 호출 없음)를 먼저 본다. 명령, 플래그, 필터 문법(`filter_grammar`), 옵션군(`option_families`), 인챈트 랭크 순서, 카탈로그 출처, 종료 코드, 예시가 JSON으로 나온다. 여기 없는 플래그를 지어내지 않는다.

## Hard rules
- **API 키를 출력·붙여넣기·로그에 절대 노출하지 않는다.** 키는 `~/.config/nexon/openapi_key` 파일 또는 `NEXON_OPEN_API_KEY` 환경변수에서만 읽힌다. `--key-file <KEY_FILE>`로 경로만 바꿀 수 있다. `cat`으로 키 파일을 읽지 않는다.
- **출처 표기 필수**: 사용자에게 보여주는 모든 답변 끝에 `meta.attribution` 값("Data based on NEXON Open API")을 붙인다. 카탈로그 행에 `source: labanyu_reference`이거나 라바뉴 필드(`stats`, `dungeon_grade`, `equipment_slot_classification`, `limit_break_max_level`)가 있다면 "mabi.labanyu.com"도 함께 적는다(`meta.attribution`에 이미 들어 있음).
- **horn**: 기본으로 `--redact-names`를 붙인다(캐릭터명 `***`). 사용자가 명시적으로 요청할 때만 생략.
- 아이템명은 정확히 일치해야 한다(`--name`). 앞의 `@`는 API 원본 이름의 일부다.
- 옵션 이름은 추측하지 않는다. 카탈로그에서 확인한 이름만 필터에 넣는다.

## Global options (모든 서브커맨드)
`--format <json|table>` (기본 json), `--db <DB>`, `--key-file <KEY_FILE>`, `--tier <development|service>` (기본 development), `--force-budget`.

## Commands
조회 계열(실시간 API 호출, 예산 소모):

```sh
# 현재 매물: --name 또는 --category, 옵션 필터(아래), --stats 통계
mabi-ah listings --name "<아이템명>" [--category <카테고리>] [--stats] [--all-pages] [--max-pages 10]
mabi-ah listings --name "@롱 소드" --stats

# 완료된 거래(최근 1시간, listings와 같은 플래그)
mabi-ah trades --name "@롱 소드" --stats --enchant-rank '>=E'

# 키워드 검색 (1~10 단어, 위치 인자; 옵션 필터 사용 가능, --stats 없음)
mabi-ah search 드래곤 블레이드 --exclude-blueprints

# NPC 상점 (--npc, --server 필수; --channel 기본 1)
mabi-ah npc-shop --npc 델 --server 류트

# 확성기 (--server 필수)
mabi-ah horn --server 류트 --redact-names

# 옵션군 값·레벨별 현재 최저가 표 (--category, --family 필수; --max-pages 기본 10, 페이지당 1회)
mabi-ah option-prices --category 토템 --family "토템 효과" --max-pages 2
```

로컬 저장소·카탈로그 계열(API 호출 없음, 단 collect·catalog learn·watch check는 호출):

```sh
# 거래 수집 (--max-calls 기본 20, --retain-days 기본 30, --quiet는 성공 시 무출력, cron용)
mabi-ah collect --quiet

# 추이: --window 기본 7d (예 24h, 7d), --bucket hour|day (기본 day), --name 생략 시 전체
mabi-ah trend --name "@롱 소드" --window 7d

# 랭킹: --by volume|value (기본 volume), --window 기본 24h, --top 기본 10, --category
mabi-ah rank --by volume --window 24h

# 감시 규칙 (watch add의 -o는 v0.1 단순 필터만: !, |, 역슬래시, 옵션군 플래그는 종료 코드 2)
mabi-ah watch add --name "@롱 소드" --max-price 100000
mabi-ah watch list
mabi-ah watch rm 1
mabi-ah watch check --emit text   # 새 알림만 보고; text는 크론 전달용 한 줄 형식

# 카탈로그 (--query 부분 일치, --limit 최대 행 수)
mabi-ah catalog enchant --query 날카                  # 인챈트 이름·슬롯·랭크·효과 (--slot 접두|접미)
mabi-ah catalog reforge --category "한손 검"          # 세공 옵션·레벨 범위 (--slot 1~3)
mabi-ah catalog option-types                          # 옵션 종류·값 형식·관측 값 수
mabi-ah catalog values --type "토템 효과"             # 한 옵션 종류의 관측 값 (--category)
mabi-ah catalog reference --kind totem_abilities --query 대미지   # 라바뉴 참고 이름 목록
mabi-ah catalog learn --category 장갑 --max-calls 3   # 현재 매물을 훑어 로컬 카탈로그 키움 (API 호출)
mabi-ah catalog import ./enchants.json                # data/observed 형식 파일 가져오기
mabi-ah catalog export --to data/observed             # 로컬 관측을 data/observed 형식으로 (유지보수용)

# 진단/상태
mabi-ah categories     # 알려진 카테고리
mabi-ah budget         # 오늘의 호출 예산
mabi-ah doctor         # 환경·데이터 상태 점검
mabi-ah config         # 해석된 설정(키 값은 출력 안 됨)
mabi-ah describe       # 명령·플래그·필터 문법 전체 (JSON)
```

`listings`/`trades`의 `--category`는 경매장 카테고리(`catalog reference --kind auction_item_categories`, 예: `검`, `장갑`)입니다. 세공 장비 유형(`한손 검` 등, `catalog reforge --category`에서 쓰는 값)과 다르며, 잘못 넣으면 API가 OPENAPI00004로 거부하고 호출 1회를 씁니다.

`catalog reference --kind` 값: `auction_item_categories`, `reforge_equipment_types`, `reforge_option_names`, `totem_abilities`, `echostone_awakening_abilities`, `murias_relic_effects`, `set_bonus_names`.

`collect --quiet`와 `watch check --emit text`는 envelope 없이 텍스트만(없으면 무출력) 내보낸다.

## 옵션 질문 흐름
옵션이 붙은 매물 질문("접두 날카로운 E 이상 장갑", "최대 공격력 세공 18 이상 한손검")은 이 순서로 처리한다.

1. `mabi-ah describe` — 필터 문법(`filter_grammar`)과 옵션군(`option_families`)을 확인한다. 이미 알면 건너뛴다.
2. 카탈로그로 정확한 이름을 찾는다(API 호출 없음).
   - 인챈트: `mabi-ah catalog enchant --query <일부>` → `name`, `slot`, `rank`.
   - 세공: `mabi-ah catalog reforge --category <장비 유형> --query <일부>` → `option`, `min_level`~`max_level`.
   - 토템·에코스톤·무리아스 유물·세트 효과: `mabi-ah catalog reference --kind <kind> --query <일부>`.
   - 그 밖의 옵션 종류: `mabi-ah catalog option-types`, 값은 `mabi-ah catalog values --type <종류>`.
3. 필터 플래그로 조회한다. 범위는 `--category`나 `--name`으로 좁힌다(옵션 검색은 받아 온 뒤 거르므로 페이지마다 호출 1회).

   ```sh
   mabi-ah listings --category 장갑 --enchant '접두:날카로운' --enchant-rank '>=E'
   mabi-ah listings --category 검 --reforge '최대 공격력>=18' --exclude-blueprints
   mabi-ah listings --category 검 --reforge '2:최대 공격력>=18' --erg 'S>=40'
   mabi-ah listings --category 검 -o '!인챈트 불가능=true' -o '색상~빨강|파랑'
   ```
4. 옵션별 시세 표가 필요하면 `mabi-ah option-prices --category <카테고리> --family <option_type>`. `--family`는 정확한 `option_type`(예: `세공 옵션`, `인챈트`, `에르그`, `토템 효과`)이다. 행: `value`, `level`, `min_price`, `min_price_ko`, `count`.

## Option filter grammar
`listings`, `trades`, `search`에서 쓴다. 모든 플래그는 반복 가능하고 AND로 묶인다.

| 플래그 | 문법 |
|---|---|
| `--enchant` | `[접두\|접미:]<이름>[(>=\|<=\|=)<랭크>]` (이름 정확히 일치) |
| `--enchant-rank` | `[접두\|접미](>=\|<=\|=)<랭크>[,...]` — 순서 F<E<D<C<B<A<9<8<…<1 |
| `--reforge` | `[<슬롯 1~3>:]<옵션명>[(>=\|<=\|=)<레벨>]` — 슬롯 생략 시 아무 슬롯; 끝 단어 일치 허용 |
| `--erg` | `<등급 S\|A\|B>[(>=\|<=\|=)<레벨>]` — 레벨은 현재 레벨 |
| `-o`, `--option` | `[!]<type>[:<sub>](>=\|<=\|=\|~)<value>[\|<value>...]` |
| `--exclude-blueprints` | 이름에 도면/옷본/심볼이 든 아이템 제외 |

- `-o`: `>=`, `<=`는 숫자(`%` 무시), `=`는 숫자면 숫자 비교·아니면 문자열 일치, `~`는 포함. 맨 앞 `!`는 부정, `|`는 OR(값만 쓰면 첫 조건의 type·연산자를 이어받음), 글자 `|`는 `\|`(역슬래시가 있는 `-o`는 모두 이 규칙으로 해석).
- 개조·피어싱은 `-o`로: `-o '특별 개조:R>=7'`, `-o '피어싱 레벨>=3'`.
- 셸 해석을 막기 위해 필터는 작은따옴표로 감싼다.
- 자세한 설명: 저장소 `docs/options.md`.

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
- 매물·거래 항목의 `price_ko`(예 "1억5000만")를 답변의 가격 표기로 쓴다.
- 카탈로그 `data`: `count`, `rows`. 각 행의 `source`, `observed_at`을 보고 출처와 기준일을 말한다.
- `meta.calls_used`, `meta.budget`: 필요 시 남은 예산을 알린다.
- `meta.staleness.stale:true`면 데이터가 오래됐다고 알린다(아래 참고). 카탈로그 명령은 거래를 보지 않으므로 이 값은 무시한다.

## Exit codes → 사용자에게 할 말
| code | 의미 | 안내 |
|---|---|---|
| 0 | 성공 | 결과 요약 + 출처 |
| 1 | 내부 오류 | "도구 내부 오류가 났어요" + `error.message_ko` |
| 2 | 사용법 오류 | 인자/필터 수정해서 재시도(`error.hint`). 모르는 옵션 종류도 여기(`hint`에 비슷한 이름) |
| 3 | 없음 | 아이템명·인챈트명·세공 옵션명이 정확해야 함; `hint`의 비슷한 이름 제안 |
| 4 | 키/권한 | "API 키가 없거나 권한이 없어요" — 키 설정 방법만 안내(키 값 요구 금지) |
| 5 | 호출 한도/예산 초과 | "오늘 호출 예산을 다 썼어요(또는 너무 빠름)"; `--force-budget`은 사용자가 원할 때만 |
| 6 | NEXON 점검/장애 | "넥슨 API가 점검/장애 중이에요", 나중에 재시도 |
| 7 | 저장소 오류 | `mabi-ah doctor` 실행해 상태 확인 |

`error.retryable:true`면 잠시 후 한 번 재시도할 수 있다. 반복 재시도 금지.

이름 검사(exit 3)는 API 호출 전에 끝난다(`meta.calls_used` 0). 예: `--reforge '최대공격력>=18'` → `"hint":"비슷한 이름: 최대 공격력, 최소 공격력, 최대생명력"`. 제안 중 하나가 분명하면 그 이름으로 다시 실행하고, 애매하면 사용자에게 묻는다. 제안이 없고 실제 게임 이름이라고 확신하면 `mabi-ah catalog learn --category <카테고리>`(API 호출)로 관측한 뒤 다시 시도한다.

## Budget rules
- 개발 티어: 초당 5회, **하루 1,000회**. CLI는 **900회로 상한**(90%)을 두고 넘으면 exit 5. `--force-budget`은 1,000까지 허용.
- 호출은 재시도 포함 HTTP 요청마다 센다. 모든 페이지가 1회(페이지당 500건).
- `--all-pages`는 `--max-pages`(기본 10)까지만. 넓은 질의 전에 `mabi-ah budget`을 확인하고, 가능하면 `--name`으로 좁힌다.
- `option-prices`는 `--max-pages`만큼, `catalog learn`은 `--max-calls`만큼 호출한다. 작게 시작한다.
- `trend`/`rank`/`categories`/`budget`/`doctor`/`config`/`describe`/`watch list|add|rm`과 `catalog`(learn 제외)는 API를 호출하지 않는다. 시세 질문은 수집된 데이터가 있으면 이쪽을 먼저 쓴다.

## Staleness
거래 데이터는 지연이 있다. `meta.staleness.stale`이 true면 답변에 "최근 거래 기준 시각: `newest_trade_at` (UTC; KST는 +9시간), 약 N분 지연"을 명시한다. 수집 데이터(trend/rank)는 `collect`가 돌지 않으면 오래된다 → Runner 설정 안내.

## 카탈로그 데이터 출처와 신선도
| `source` | 출처 | 신선도 |
|---|---|---|
| `repo_fact` | 저장소 `data/facts/` 직접 작성 규칙(슬롯, 랭크 순서, 옵션군 형식, 장비 유형) | 만료 없음 |
| `labanyu_reference` | 저장소 `data/reference/labanyu/`, mabi.labanyu.com 참고 데이터 | `fetched_at` 기준, 수동 갱신 |
| `repo_observed` | 저장소 `data/observed/`, NEXON Open API 관측 | 관측 후 30일에 만료(만료된 파일은 무시됨) |
| `local_observed` | 이 컴퓨터에서 조회·collect·catalog learn이 관측 | 30일 지나면 정리 |
| `user_import` | `catalog import`로 가져온 파일 | 30일 지나면 정리 |

- 같은 항목은 관측일이 최근인 쪽이 이긴다. 같은 날이면 `user_import` > `local_observed` > `repo_observed` > `labanyu_reference` > `repo_fact`.
- 관측 데이터는 시장에 나온 물건만 반영한다. `observed_count`가 작거나 `observed_at`이 오래됐으면 "관측 기준"이라고 말한다.
- **던전 등급(`dungeon_grade`)은 라바뉴가 임의 부여**한 값이다. "라바뉴 분류 기준"이라고 밝히고, 실제 드롭처로 단정하지 않는다. 대부분 `null`이다. 장비 부위 분류도 라바뉴 분류다.
- 세공 `limit_break_max_level`(한돌 상한)과 토템 `max_threshold`는 라바뉴 참고값이지 공식 상한이 아니다.
- **`세공 랭크`는 관측된 적이 없어 의미를 모른다.** 나오면 원문만 전하고 해석하지 않는다.

## 할 수 있는 것 / 없는 것
- 가능(API만): 이름·카테고리별 매물, 최근 1시간 거래, 키워드 검색, NPC 상점, 뿔피리, 옵션 필터(받아 온 뒤 거름), 옵션 값·레벨별 현재 최저가 표.
- 가능(로컬 저장): 추이, 순위, 감시 알림, 관측 카탈로그.
- 가능(저장소 메타데이터): 설치 직후 인챈트·세공 카탈로그, 이름 오타 제안.
- 불가: 서버 측 옵션 검색(전부 받아서 거름 → 예산 소모), 판매자·등록 시각, 1시간 넘는 거래 이력(collect로 모은 것만), 인챈트 착용 부위(API에 없음), 세공 공식 상한, `세공 랭크`의 의미. 불가한 질문에는 지어내지 말고 할 수 없다고 말한다.

## Watch + Runner (자동 수집·알림)
주기 수집과 알림 전달은 Runner가 한다. 설정은 `runner/hermes-cron/README.md`(저장소) 참고. 요지: Hermes cron이 `mabi-ah collect --quiet` 후 `mabi-ah watch check --emit text`를 실행하고 출력이 있을 때만 전달한다.

## 답변 템플릿 (한국어)
- 시세: `"<아이템> 현재 최저가 <N>골드 (매물 <M>개). 최근 거래 중앙값 <P>골드.\n※ 기준 시각 <KST>\n출처: Data based on NEXON Open API"`
- 옵션 매물: `"<조건> <아이템> 매물 <M>개, 최저 <price_ko>골드. (조회 <pages>페이지, 일부만 조회됨이면 명시)\n출처: Data based on NEXON Open API"`
- 카탈로그: `"<이름>(<slot>, 랭크 <rank>): <effects>. 기준 <observed_at>, 출처 <source>"` — `labanyu_reference`이거나 라바뉴 필드가 있는 행이면 끝에 "출처: mabi.labanyu.com".
- 매물 없음: `"지금 '<아이템>' 매물이 없어요. 최근 거래 기준 시세는 <P>골드예요."`
- 오래된 데이터: `"⚠️ 데이터가 약 <N>분 지연돼 있어요(최근 거래 <KST>)."`
- 추이: `"<아이템> 최근 <window> 추이: <시작>→<끝>골드 (<±x%>). 거래 <n>건.\n출처: Data based on NEXON Open API"`
- 감시 알림: `"[감시 <id>] <아이템> <n>개, 개당 <price>골드 (목표 <max>골드 이하)\nData based on NEXON Open API"`
- 예산 소진(exit 5): `"오늘 API 호출 예산(<used>/<limit>)을 다 써서 더 조회할 수 없어요. 내일 다시 시도해 주세요."`
- 이름 오류(exit 3): `"'<입력>'과 정확히 일치하는 이름이 없어요. 혹시 <제안>인가요?"`
