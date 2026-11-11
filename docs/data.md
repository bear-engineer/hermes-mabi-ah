# 저장소 데이터 (`data/`)

바이너리에 내장되어 설치 직후 `catalog` 명령이 쓰는 메타데이터입니다. 필드 정의는 `data/README.md`를 봅니다.

## 출처

| 경로 | `source` | 출처 | 만료 |
|---|---|---|---|
| `data/facts/` | `repo_fact` | 직접 작성한 게임 규칙(슬롯, 랭크 순서, 옵션군 형식, 장비 유형) | 없음 |
| `data/observed/` | `repo_observed` | **NEXON Open API 관측**: 경매장 목록 응답에서 뽑은 인챈트·세공·옵션 값 이름, 슬롯, 랭크, 효과문, 레벨 범위, 관측 건수, 관측일 | 있음: `expires_at` = `observed_to` + 30일 |
| `data/reference/labanyu/` | `labanyu_reference` | 라바뉴(mabi.labanyu.com) 참고 데이터: 게임 사실(세공 옵션명·레벨 범위, 토템·에코스톤 효과명 등)과 큐레이션(인챈트 던전 등급, 장비 슬롯 분류) | 없음, 파일마다 `fetched_at` |

우선순위(같은 관측일일 때): `user_import` > `local_observed` > `repo_observed` > `labanyu_reference` > `repo_fact`.

## 표기

- NEXON 데이터: 모든 파일의 `attribution`은 `Data based on NEXON Open API`입니다. 출력 envelope의 `meta.attribution`도 같습니다.
- 라바뉴 데이터: 파일마다 `attribution: mabi.labanyu.com`, `source_url`, `fetched_at`을 둡니다.
- 가격, 거래량, 판매자·구매자, 캐릭터 이름, 뿔피리, 원본 응답, 키는 저장소에 두지 않습니다.

## 갱신

NEXON Open API 약관상 데이터는 30일 안에 갱신해야 합니다.

```sh
# 로컬(개발 키): 카테고리별 1페이지씩, 호출 간 1.2초, 총 40회 상한
sh scripts/refresh-data.sh          # catalog learn → catalog export --to data/observed
sh scripts/check-data-freshness.sh  # data/observed 만료·관측일 검사
sh scripts/check-data-fields.sh     # data/ 전체 금지 키·키 모양 문자열 검사
```

- `catalog export --to <dir>`는 로컬 DB에서 `local_observed` 행만 내보냅니다. `labanyu_reference`, `repo_fact`, `repo_observed`, `user_import` 행은 다시 내보내지 않습니다. 정렬은 결정적이라 같은 입력이면 출력이 같습니다.
- 새 파일이 생기면 `crates/mabi-ah-cli/src/repo_data.rs`의 `OBSERVED_FILES`에 `include_str!` 한 줄을 더합니다(현재 `enchants.json`, `reforges.json`, `option_values.json`).
- CI(`ci.yml`의 `data` 작업)는 신선도·필드 검사를 돌립니다. 30일이 지나면 CI가 실패합니다.
- `.github/workflows/refresh-data.yml`은 매주 월요일(UTC 03:17)과 수동 실행(`workflow_dispatch`)으로 갱신 PR을 엽니다. 저장소 시크릿 `NEXON_OPEN_API_KEY`가 없으면 건너뜁니다. 키는 출력하지 않습니다. 기본 `GITHUB_TOKEN`으로 연 PR은 CI를 자동 실행하지 않으므로, 병합 전에 PR 브랜치에서 CI를 수동으로 돌리거나 같은 스크립트를 로컬에서 실행합니다.
- 라바뉴 참고 데이터는 수동 갱신입니다(`scripts/labanyu/`, 요청 간격 10초 이상, `data/README.md` 참고).

## 삭제 요청(테이크다운) 절차

1. 대상 파일을 확인합니다(NEXON 관측이면 `data/observed/*.json`, 라바뉴면 `data/reference/labanyu/*.json`).
2. `crates/mabi-ah-cli/src/repo_data.rs`의 `OBSERVED_FILES` 또는 `REFERENCE_FILES`에서 해당 `include_str!` 줄을 지웁니다. 이 줄이 없어도 CLI는 나머지 데이터와 로컬 DB로 동작합니다.
3. 파일을 `git rm`하고 PR로 병합합니다. 새 릴리스를 내보냅니다.
4. 이미 배포된 릴리스와 git 기록에 남은 데이터가 문제이면, 요청자와 협의해 릴리스 자산 교체나 기록 정리(별도 승인 필요)를 진행합니다.
5. 사유와 처리일을 PR 설명에 남깁니다.
