# data/ — 저장소 관리 메타데이터

인챈트·세공 카탈로그의 저장소 내장 데이터입니다. 바이너리에 내장되어 설치 직후 `catalog` 명령이 사용합니다.

## 구성

| 경로 | 내용 | 작성 방식 |
|---|---|---|
| `facts/enchant_slots.json` | 인챈트 슬롯(접두·접미) | 직접 작성 |
| `facts/enchant_ranks.json` | 랭크 순서 F<E<D<C<B<A<9<…<1 (코드의 `EnchantRank`와 테스트로 대조) | 직접 작성 |
| `facts/option_families.json` | 옵션군별 `option_sub_type`·`option_value`·`option_value2` 형식, 관측 여부 | 직접 작성 |
| `facts/equipment_types.json` | 인챈트·세공이 붙는 장비 유형(경매장 카테고리 enum의 부분집합) | 직접 작성 |
| `observed/*.json` | API 관측 카탈로그(인챈트, 세공 옵션, 옵션 값) | `catalog export`로 생성 (수동 편집 금지) |
| `reference/labanyu/*.json` | 라바뉴 참고 데이터(아래 절) | `scripts/labanyu/build.py`로 생성 (수동 편집 금지) |

## 공통 형식 (`schema_version: 1`)

```json
{
  "schema_version": 1,
  "kind": "observed_enchant",
  "attribution": "Data based on NEXON Open API",
  "observed_from": "YYYY-MM-DD",
  "observed_to": "YYYY-MM-DD",
  "expires_at": "YYYY-MM-DD",
  "entries": []
}
```

- `kind`: 관측 파일 `observed_enchant` | `observed_reforge` | `observed_option_value`, 사실 파일 `enchant_slot` | `enchant_rank` | `option_family` | `equipment_type`.
- 관측 파일은 세 날짜가 모두 필요합니다. `expires_at`은 `observed_to` 다음 날부터 30일 이내여야 합니다(NEXON Open API 30일 갱신 규칙). 사실 파일은 날짜를 생략할 수 있습니다.
- 항목에 정의되지 않은 필드가 있으면 로드가 실패합니다.

## 항목에 들어가는 필드 (화이트리스트)

- 인챈트: `slot`, `name`, `rank`, `effects`(효과문), `observed_count`, `first_seen`, `last_seen`
- 세공: `option`, `slot`(1~3), `category`(장비 유형), `min_level`, `max_level`, `observed_count`, `first_seen`, `last_seen`
- 옵션 값: `option_type`, `sub_type`, `value`, `category`, `min_level`, `max_level`, `observed_count`, `first_seen`, `last_seen`

가격, 판매자·구매자, 캐릭터 이름, 뿔피리, 거래 ID, API 키, 원본 응답은 넣지 않습니다. `find_forbidden_keys`(domain `repo_data`)가 이런 키 이름을 검사합니다.

## 병합 규칙

저장소 데이터와 사용자 로컬 카탈로그를 합칠 때 같은 항목은 `last_seen`이 가장 최근인 쪽이 이깁니다. 날짜가 같으면 `user_import` > `local_observed` > `repo_observed` > `labanyu_reference` > `repo_fact` 순입니다. 참고 데이터의 날짜는 `fetched_at`의 날짜입니다. 출처는 항목마다 `source`로 표시됩니다.

## 알아둘 점

- 관측 데이터는 시장에 나온 물건만 반영하므로 빠진 이름이 있을 수 있습니다. `observed_count`와 `last_seen`을 함께 보세요.
- `세공 랭크`는 관측된 적이 없고 의미도 확인되지 않았습니다. 원문만 보존하며 해석하지 않습니다. 인챈트 착용 부위와 세공 공식 상한(한돌 여부)도 알 수 없어 넣지 않았습니다.
- 위키(나무위키, Mabinogi World Wiki) 데이터는 복사하지 않았습니다. 라바뉴 데이터는 `reference/labanyu/`에만 있습니다.
- 넥슨 오픈 API 약관 제5조⑤(결과 데이터의 복제·저장·배포) 위험은 사용자가 감수하기로 결정했습니다. 삭제 요청이 오면 `data/observed/`를 한 커밋으로 제거합니다.

## 라바뉴 참고 데이터 (`reference/labanyu/`)

- 출처: [mabi.labanyu.com](https://mabi.labanyu.com) 공개 페이지(서버 렌더링 `__NEXT_DATA__`). 파일마다 `source: "labanyu_reference"`, `source_url`, `fetched_at`, `attribution: "mabi.labanyu.com"`, `notes`가 있습니다. 출력에도 `source`가 표시되므로 출처 표기는 "mabi.labanyu.com"으로 합니다.
- 파일(파일 이름 = `kind`): `auction_item_categories`, `reforge_equipment_types`, `reforge_option_names`(이름만), `reforge_level_ranges`(장비 유형별 세공 범위와 한돌 상한 `limit_break_max_level`), `totem_abilities`, `echostone_awakening_abilities`(`max_level`), `murias_relic_effects`, `set_bonus_names`, `enchant_scrolls`(이름·슬롯·랭크·효과문·대상, 던전 등급, 장비 부위 분류).
- 담는 것: 게임 사실(이름, 슬롯, 랭크, 효과문, 레벨 범위, 최대값)과 라바뉴 분류(던전 등급, 장비 부위 분류). 담지 않는 것: 가격, 최근 가격, 거래량·TOP 목록, 사용자·캐릭터 이름, 사이트 내부 id, robots.txt 금지 경로(`/in-game/auction/item/`, `/lab`)의 내용.
- 한 사실은 한 곳에만 둡니다: 세공 범위는 `reforge_level_ranges`에만, 인챈트 랭크 순서는 코드의 `EnchantRank`에만 있습니다.
- 큐레이션 주의: **던전 등급은 라바뉴가 임의 부여**한 값입니다(사이트 스스로 그 던전에서 나오지 않을 수 있다고 밝힘). 1185개 중 160개만 값이 있고 나머지는 `null`입니다. 장비 부위 분류는 사이트 분류 규칙을 옮긴 것이라 이름 붙은 아이템·세트 대상은 빈 목록일 수 있습니다(원래 대상은 `target_names`). 토템 `max_threshold`는 사이트가 추적하는 최고값일 뿐 확인된 게임 상한이 아닙니다.
- 신선도: 참고 데이터는 `fetched_at`만 있고 30일 만료가 없습니다(30일 규칙은 API 관측 데이터 `observed/`에만 적용). 같은 항목을 API에서 관측하면 관측 데이터가 우선합니다.
- 갱신(수동): 저장소 밖 캐시 디렉터리를 정하고 `LABANYU_CACHE=<캐시> scripts/labanyu/fetch.sh <URL> <파일>`로 페이지를 한 번에 하나씩 받습니다. 스크립트는 robots.txt의 Crawl-delay 10을 지켜 요청 사이에 11초 이상 쉬고, 금지 경로를 거부하며, 캐시당 80회에서 멈춥니다. 받은 페이지의 `__NEXT_DATA__` → `props.pageProps`를 `raw/<페이지>.next.json`으로 저장한 뒤 `python3 scripts/labanyu/build.py <캐시>`를 실행하면 이 디렉터리가 다시 만들어집니다(네트워크 없음, 가격 계열 키가 있으면 실패). 원본 페이지는 커밋하지 않습니다.
- 삭제 요청(takedown): 라바뉴 운영자가 요청하면 `data/reference/labanyu/`와 `crates/mabi-ah-cli/src/repo_data.rs`의 해당 `include_str!` 줄을 한 커밋으로 제거합니다. 참고 데이터가 없어도 CLI는 관측·사실 데이터만으로 동작합니다.
