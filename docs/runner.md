# Runner 선택

수집(`mabi-ah collect`)과 알림(`mabi-ah watch check`)을 주기 실행하는 방식 네 가지를 비교합니다. 결정은 **Hermes cron `--no-agent`(B2)** 입니다.

| 방식 | 구성 | 알림 전달 | 모델 비용 | 장애 감지 | 판단 |
|---|---|---|---|---|---|
| A. launchd | `runner/launchd/` plist 템플릿이 `mabi-ah collect` 실행 | Outbox에 쌓이고, 에이전트가 읽을 때 보고. Discord 전달은 별도 연동 필요 | 없음 | launchd 로그를 직접 확인 | 전달 경로가 없음. Hermes가 꺼져 있어도 수집해야 할 때의 대안 |
| B1. Hermes cron 에이전트 모드 | 실행마다 에이전트를 깨워 CLI 실행 | 에이전트가 Discord로 보고 | 30분마다 모델 호출 | `hermes cron runs` | 비용 대비 이득 없음 |
| **B2. Hermes cron `--no-agent` (권장)** | `runner/hermes-cron/` 스크립트 2개 | 알림 스크립트 stdout을 `--deliver discord:<channel>`로 전달. 출력이 없으면 전송 안 함 | 없음 | `hermes cron runs` / `incidents`, `--failure-deliver` | 비용 0, 기존 전달 인프라 재사용, 실패 알림 내장 |
| C. CLI 데몬 | 상주 루프 | 웹훅 직접 호출 (웹훅 URL 비밀 필요) | 없음 | 감시자(supervisor) 별도 필요 | 기각 |

## 결정: B2 채택

- CLI는 특정 Runner에 묶이지 않는 1회 실행 명령만 제공합니다. Runner 차이는 설치 스크립트가 흡수합니다.
- 모델을 부르지 않으므로 비용이 없습니다. 알림은 새 알림이 있을 때만 출력되어 전달됩니다.
- 실패는 `--failure-deliver`로 같은 채널에 보고됩니다. 별도 감시 프로세스가 필요 없습니다.
- 설치 절차와 명령은 `runner/hermes-cron/README.md`를 봅니다.

## 기각 사유

- **A (launchd)**: 수집만 하고 알림 전달 경로가 없습니다. plist 템플릿만 제공해, Hermes가 꺼진 상태에서 수집이 필요한 경우에 씁니다.
- **B1 (에이전트 모드)**: 30분마다 모델 비용이 나가지만 B2보다 나은 점이 없습니다.
- **C (데몬)**: 상주 프로세스 감시와 웹훅 비밀 관리가 늘어납니다. Hermes가 이미 제공하는 실패 알림과 전달을 다시 만드는 셈입니다.

## 관련 제약

- 매물 전체 상시 스냅샷은 하지 않습니다. 1회 250회 이상 호출이 필요해 개발 키 일일 한도(1,000건)와 약관 과부하 조항에 걸립니다. 스냅샷은 관심 대상(WatchRule)으로 한정합니다.
- 스크립트와 템플릿에는 실제 경로·사용자명·채널 ID를 넣지 않습니다. 플레이스홀더만 씁니다.
