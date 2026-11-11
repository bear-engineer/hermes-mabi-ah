# hermes-mabi-ah

[English](README.en.md)

`mabi-ah`는 NEXON Open API로 PC 마비노기 경매장 데이터를 조회하는 명령줄 도구입니다. 주 사용자는 AI 에이전트(Hermes)이고, Hermes 스킬(`skill/mabi-ah/`)이 함께 들어 있습니다. 사람도 그대로 쓸 수 있습니다.

Data based on NEXON Open API.

## 소개

- 마비노기 API 다섯 가지를 모두 조회합니다: 현재 매물, 최근 1시간 거래, 키워드 검색, NPC 상점, 뿔피리.
- 커서를 따라가며 여러 페이지를 받고, 아이템 옵션을 해석해 인챈트·세공·에르그 같은 옵션으로 거릅니다.
- 거래를 로컬 SQLite에 모아 가격 추이와 순위를 계산하고, 감시 규칙에 걸린 매물을 알림으로 만듭니다.
- 인챈트·세공 이름 카탈로그를 바이너리에 내장해, 설치 직후부터 옵션 이름을 찾아볼 수 있습니다.
- API 호출 수를 세어 하루 예산을 지키고, 거래 데이터가 얼마나 늦었는지 알려 줍니다.

출력은 기본이 JSON envelope(`schema: mabi_ah/v1`)이고, 사람이 볼 때는 `--format table`을 씁니다. 매물·거래 결과의 `price_ko`는 가격을 한국어 단위로 적은 값입니다(예: `1억5000만`).

## 설치

스킬 폴더의 설치 스크립트가 바이너리와 Hermes 스킬을 함께 설치합니다. 미리 빌드된 바이너리는 macOS arm64(Apple Silicon)용만 있습니다.

```sh
sh skill/mabi-ah/scripts/install.sh            # 최신 릴리스 내려받기(sha256 검증)
sh skill/mabi-ah/scripts/install.sh --dry-run  # 할 일만 출력
cargo build --release -p mabi-ah-cli
sh skill/mabi-ah/scripts/install.sh --from-local target/release/mabi-ah   # 직접 빌드한 바이너리
```

스킬은 `~/.hermes/skills/gaming/mabi-ah/`(`HERMES_HOME`을 정했다면 그 아래)에, 실행 파일 링크는 `~/.local/bin/mabi-ah`에 생깁니다. 다른 플랫폼에서는 소스에서 빌드하세요: `cargo install --git https://github.com/bear-engineer/hermes-mabi-ah mabi-ah-cli --locked`.

## API 키

[NEXON Open API](https://openapi.nexon.com)에서 게임을 마비노기로 골라 개발 단계 애플리케이션을 등록하고, 받은 키를 저장소 밖에 저장합니다.

```sh
mkdir -p ~/.config/nexon && umask 077 && printf '%s' 'YOUR_KEY' > ~/.config/nexon/openapi_key
```

환경 변수 `NEXON_OPEN_API_KEY`도 됩니다. 다른 파일을 쓰려면 `--key-file <파일>`을 줍니다. 키는 출력이나 로그에 남지 않습니다.

개발 단계 키는 초당 5회, 하루 1,000회까지 부를 수 있습니다. `mabi-ah`는 그 90%인 900회에서 멈추고 종료 코드 5를 돌려줍니다. `--force-budget`을 주면 1,000회까지 씁니다. 서비스 단계 키라면 `--tier service`를 줍니다.

## 명령 요약

모든 명령에 쓸 수 있는 전역 옵션: `--format <json|table>`, `--db <DB>`, `--key-file <KEY_FILE>`, `--tier <development|service>`, `--force-budget`.

API를 부르는 명령(예산 소모):

| 명령 | 하는 일 |
|---|---|
| `mabi-ah listings` | 아이템 이름(`--name`, 정확히 일치) 또는 카테고리(`--category`)의 현재 매물 |
| `mabi-ah trades` | 최근 1시간 거래. 플래그는 `listings`와 같음 |
| `mabi-ah search` | 키워드(1~10개)로 현재 매물 검색 |
| `mabi-ah npc-shop` | NPC 상점 목록(`--npc`, `--server` 필수) |
| `mabi-ah horn` | 뿔피리(`--server` 필수, `--redact-names`로 캐릭터명 가림) |
| `mabi-ah collect` | 거래를 로컬 저장소에 수집(Runner용) |
| `mabi-ah option-prices` | 한 옵션군의 값·레벨별 현재 최저가 표 |
| `mabi-ah catalog learn` | 한 카테고리의 현재 매물을 훑어 로컬 카탈로그를 키움 |
| `mabi-ah watch check` | 감시 규칙을 현재 매물과 대조해 새 알림만 보고 |

API를 부르지 않는 명령:

| 명령 | 하는 일 |
|---|---|
| `mabi-ah trend` | 수집한 거래의 가격 추이(`--window`, `--bucket hour\|day`) |
| `mabi-ah rank` | 수집한 거래의 거래량·거래액 순위 |
| `mabi-ah watch add` / `mabi-ah watch list` / `mabi-ah watch rm` | 감시 규칙 추가·목록·삭제 |
| `mabi-ah catalog enchant` | 인챈트 이름·슬롯·랭크·효과 |
| `mabi-ah catalog reforge` | 장비 유형별 세공 옵션과 레벨 범위 |
| `mabi-ah catalog option-types` | 옵션 종류와 값 형식, 관측된 값 수 |
| `mabi-ah catalog values` | 한 옵션 종류의 관측 값(예: 토템 효과, 에르그) |
| `mabi-ah catalog reference` | 라바뉴 참고 이름 목록(토템, 에코스톤, 무리아스 유물, 세트 효과 등) |
| `mabi-ah catalog import` | `data/observed` 형식 파일을 로컬 카탈로그로 가져오기 |
| `mabi-ah catalog export` | 로컬 관측 카탈로그를 `data/observed` 형식으로 내보내기 |
| `mabi-ah categories` | 알려진 경매장 카테고리 |
| `mabi-ah budget` | 오늘 쓴 호출 수와 한도 |
| `mabi-ah doctor` | 환경·데이터 상태 점검 |
| `mabi-ah config` | 적용된 설정(키 값은 출력하지 않음) |
| `mabi-ah describe` | 모든 명령·플래그·필터 문법·옵션군·예시를 JSON으로 |

예시:

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

플래그의 기준은 `mabi-ah describe`와 `mabi-ah <명령> --help`입니다.

## 옵션 검색

경매장 API는 옵션으로 검색하지 못합니다. `mabi-ah`는 매물을 받아 온 뒤 옵션으로 거릅니다. 그러니 `--name`이나 `--category`로 범위를 먼저 좁혀야 호출을 아낄 수 있습니다. 필터 플래그는 `listings`, `trades`, `search`에서 쓰고, 여러 개를 주면 모두 만족하는 매물만 남습니다(AND).

| 플래그 | 문법 | 예 |
|---|---|---|
| `--enchant` | `[접두\|접미:]<이름>[(>=\|<=\|=)<랭크>]` | `--enchant '접두:날카로운'` |
| `--enchant-rank` | `[접두\|접미](>=\|<=\|=)<랭크>[,...]` | `--enchant-rank '>=E'` |
| `--reforge` | `[<슬롯 1~3>:]<옵션명>[(>=\|<=\|=)<레벨>]`, 슬롯을 안 주면 어느 슬롯이든 | `--reforge '최대 공격력>=18'` |
| `--erg` | `<등급 S\|A\|B>[(>=\|<=\|=)<레벨>]` | `--erg 'S>=40'` |
| `-o`, `--option` | `[!]<종류>[:<하위>](>=\|<=\|=\|~)<값>[\|<값>...]` | `-o '색상~빨강\|파랑'` |
| `--exclude-blueprints` | 이름에 도면·옷본·심볼이 들어간 아이템 제외 | |

- 인챈트 랭크는 F < E < D < C < B < A < 9 < 8 < … < 1 순입니다. `>=E`는 E부터 1까지입니다.
- `-o`에서 `>=`, `<=`는 숫자 비교, `=`는 숫자면 숫자로·아니면 글자 그대로 비교, `~`는 포함 여부입니다. 맨 앞 `!`는 조건을 뒤집고, `|`로 값을 나열하면 그중 하나만 맞아도 됩니다. 글자 `|` 자체는 `\|`로 씁니다.
- 실행 전에 이름부터 검사합니다. 모르는 옵션 종류는 종료 코드 2, 모르는 인챈트·세공 옵션 이름은 종료 코드 3이고, 이때 API는 한 번도 부르지 않습니다. 오류의 `hint`에 비슷한 이름이 나옵니다.

```sh
mabi-ah listings --category 장갑 --enchant '접두:날카로운' --enchant-rank '>=E'
mabi-ah listings --category "한손 검" --reforge '최대 공격력>=18' --exclude-blueprints
mabi-ah listings --category "한손 검" --reforge '2:최대 공격력>=18' --erg 'S>=40'
mabi-ah listings --category 검 -o '!인챈트 불가능=true' -o '색상~빨강|파랑'
mabi-ah option-prices --category 토템 --family "토템 효과" --max-pages 2
```

세공 옵션 이름을 붙여 쓰면 이렇게 나옵니다(실제 출력, API 호출 0회, 일부 생략).

```text
$ mabi-ah listings --category "한손 검" --reforge '최대공격력>=18'
mabi-ah: 찾을 수 없습니다: reforge option '최대공격력' [not_found]
{"schema":"mabi_ah/v1","ok":false,"command":"listings","data":null,"meta":{...,"calls_used":0,...},
 "error":{"code":"not_found","nexon_code":null,"message_ko":"찾을 수 없습니다: reforge option '최대공격력'",
 "retryable":false,"hint":"비슷한 이름: 최대 공격력, 최소 공격력, 최대생명력"}}
```

문법·옵션군·이스케이프는 [docs/options.md](docs/options.md)에 자세히 정리했습니다.

## 카탈로그와 저장소 메타데이터

옵션 이름을 짐작하지 않도록 카탈로그를 둡니다. 카탈로그는 저장소에 든 데이터와 사용자 로컬 관측을 합친 것입니다.

| 경로 | `source` | 내용 | 신선도 |
|---|---|---|---|
| `data/facts/` | `repo_fact` | 직접 작성한 규칙: 인챈트 슬롯, 랭크 순서, 옵션군별 값 형식, 장비 유형 | 만료 없음 |
| `data/observed/` | `repo_observed` | NEXON Open API 경매장 응답에서 뽑은 인챈트·세공·옵션 값 이름, 슬롯, 랭크, 효과문, 레벨 범위, 관측 건수, 관측일 | `expires_at`까지(관측 후 30일). 주간 워크플로가 갱신하고 CI가 만료를 검사 |
| `data/reference/labanyu/` | `labanyu_reference` | [mabi.labanyu.com](https://mabi.labanyu.com) 참고 데이터: 세공 옵션명·레벨 범위(한돌 상한 포함), 토템·에코스톤·무리아스 유물 효과, 세트 효과 이름, 인챈트 스크롤 정보, 라바뉴의 분류 | 파일마다 `fetched_at`, 수동 갱신 |
| 로컬 DB | `local_observed`, `user_import` | 조회·`collect`·`catalog learn`이 관측한 값, `catalog import`로 가져온 값 | 30일 지나면 정리 |

- 같은 항목이 여러 곳에 있으면 관측일이 더 최근인 쪽을 씁니다. 날짜가 같으면 `user_import` > `local_observed` > `repo_observed` > `labanyu_reference` > `repo_fact` 순입니다. 결과의 각 행에 `source`와 `observed_at`이 붙습니다.
- 저장소 데이터에는 가격, 거래량, 판매자·구매자, 캐릭터 이름, 뿔피리 내용, 원본 응답, 키를 넣지 않습니다.
- 라바뉴 데이터는 출처와 큐레이션을 구분해서 읽으세요. **던전 등급은 라바뉴가 임의 부여**한 값이라 그 던전에서 실제로 나오지 않을 수 있고, 대부분은 `null`입니다. 장비 부위 분류도 라바뉴의 분류이고, 토템 `max_threshold`는 사이트가 추적하는 최고값일 뿐 확인된 게임 상한이 아닙니다.
- `세공 랭크`는 아직 관측된 적이 없어 뜻을 모릅니다. 나오면 원문만 보존하고 해석하지 않습니다.

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

실제 출력(`catalog enchant --query 날카 --limit 1`의 `data`):

```json
{"count":1,"rows":[{"dungeon_grade":null,"effects":["레벨이 12 이상일때 크리티컬 2 증가"],
 "equipment_slot_classification":[{"group":"장비 제한 없음","slot":"장비 제한 없음"}],
 "first_seen":"2026-11-11","last_seen":"2026-11-11","name":"날카로운","observed_at":"2026-11-11",
 "observed_count":5,"rank":"E","slot":"접두","source":"repo_observed",
 "stats":[{"name":"크리티컬","value":2}],"target_names":[]}]}
```

실제 출력(`catalog reforge --category "한손 검" --limit 2`의 첫 행):

```json
{"category":"한손 검","limit_break_max_level":5,"max_level":5,"min_level":2,"observed_at":"2026-11-11",
 "option":"공격 속도 증가 세트 효과 수치","range_text":"2~5 (한돌: 5)","slot":null,"source":"labanyu_reference"}
```

데이터 파일 형식과 갱신 절차는 [data/README.md](data/README.md)와 [docs/data.md](docs/data.md)에 있습니다.

## 할 수 있는 것·없는 것

할 수 있는 것

- API만으로: 이름·카테고리별 매물과 최근 1시간 거래, 키워드 검색, NPC 상점, 뿔피리, 옵션 필터(받아 온 뒤 거름), 옵션 값·레벨별 현재 최저가 표.
- 로컬 저장으로: 가격 추이, 순위, 감시 알림, 관측 카탈로그.
- 저장소 메타데이터로: 설치 직후부터 인챈트·세공 카탈로그와 이름 오타 제안(출처·관측일 표시).

할 수 없는 것

- 옵션으로 서버에서 검색하기. 매물을 다 받아서 거르므로 넓은 질의는 예산을 많이 씁니다.
- 판매자나 등록 시각 보기.
- 1시간보다 오래된 거래 이력 보기. `collect`로 모아 둔 범위만 볼 수 있습니다.
- 인챈트 착용 부위(API로는 안 보임), 세공 공식 상한, `세공 랭크`의 뜻 알기. 한돌 상한은 라바뉴 참고값일 뿐입니다.
- 공개 위키 데이터 동봉.

## Runner

주기 수집과 알림 전달은 Runner가 맡습니다. 권장 방식은 Hermes cron `--no-agent` 작업 두 개입니다. 하나는 `mabi-ah collect --quiet`로 거래를 모으고, 다른 하나는 `mabi-ah watch check --emit text`를 돌려 출력이 있을 때만 Discord로 보냅니다. 모델을 부르지 않으니 비용이 들지 않습니다. 설치는 [runner/hermes-cron/README.md](runner/hermes-cron/README.md), 방식 비교는 [docs/runner.md](docs/runner.md)를 보세요. launchd 템플릿은 `runner/launchd/`에 있습니다.

## 약관·출처 표기

- 데이터를 보여 주는 곳마다 "Data based on NEXON Open API"를 표시하세요. 출력의 `meta.attribution`에 들어 있습니다.
- 라바뉴 참고 데이터(`source: labanyu_reference`)를 보여 줄 때는 "mabi.labanyu.com"도 함께 표시하세요.
- 저장한 데이터는 30일 안에 갱신해야 합니다. 로컬 저장소는 거래를 기본 30일만 보관합니다.
- 받은 데이터를 재배포하거나 API를 제3자에게 제공하지 말고, API에 과한 부하를 주지 마세요.
- 키는 본인 것을 쓰고 남과 나누지 마세요.

전체 약관: https://openapi.nexon.com/ko/support/terms/

## 개발

```sh
git config core.hooksPath .githooks   # gitleaks pre-commit (brew install gitleaks)
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
sh scripts/check-docs.sh              # 문서의 명령·플래그를 describe와 대조
sh scripts/check-data-freshness.sh    # data/observed 30일 만료 검사
sh scripts/check-data-fields.sh       # data/ 금지 필드 검사
```

구조: `domain` ← `app` ← (`nexon`, `store`) ← `cli`. 용어는 `PROJECT_TERMS.md`를 따릅니다.

## 라이선스

MIT(`LICENSE-MIT`)와 Apache-2.0(`LICENSE-APACHE`) 중 원하는 쪽을 골라 쓸 수 있습니다.
