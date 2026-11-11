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

저장소 데이터와 사용자 로컬 카탈로그를 합칠 때 같은 항목은 `last_seen`이 가장 최근인 쪽이 이깁니다. 날짜가 같으면 `user_import` > `local_observed` > `repo_observed` > `repo_fact` 순입니다. 출처는 항목마다 `source`로 표시됩니다.

## 알아둘 점

- 관측 데이터는 시장에 나온 물건만 반영하므로 빠진 이름이 있을 수 있습니다. `observed_count`와 `last_seen`을 함께 보세요.
- `세공 랭크`는 관측된 적이 없고 의미도 확인되지 않았습니다. 원문만 보존하며 해석하지 않습니다. 인챈트 착용 부위와 세공 공식 상한(한돌 여부)도 알 수 없어 넣지 않았습니다.
- 위키(나무위키, Mabinogi World Wiki)와 라바뉴 데이터는 복사하지 않았습니다.
- 넥슨 오픈 API 약관 제5조⑤(결과 데이터의 복제·저장·배포) 위험은 사용자가 감수하기로 결정했습니다. 삭제 요청이 오면 `data/observed/`를 한 커밋으로 제거합니다.
