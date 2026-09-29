# cli-usage-tray

Claude Code / Codex / Gemini CLI의 **로컬 세션 로그**를 읽어 토큰 사용량(input/output/cache read/write)을 집계하고, macOS 메뉴바에 상주하며 보여주는 유틸리티입니다.

- 원격 API 호출이 전혀 없습니다 — 세션 로그를 로컬에서만 읽고 집계합니다.
- 로그인/계정/인증 기능이 없습니다.
- 원격 자동 업데이트 기능이 없습니다.

## 스크린샷

메뉴바 아이콘 클릭 시, 오늘/7일/전체 기준 토큰 사용량과 도구별·모델별 분해, 최근 7일 스파크라인을 보여주는 패널이 뜹니다.

## 요구 사항

- macOS
- [Rust](https://www.rust-lang.org/tools/install) (`cargo`, `rustc`)
- [Node.js](https://nodejs.org/) 18+ 와 npm

## 실행 방법

```bash
npm install
npm run app:dev     # 개발 모드 (hot reload)
```

또는 디버그 빌드를 만들어 실제 `.app`으로 실행:

```bash
npm run app:build
open src-tauri/target/debug/bundle/macos/digimon-adventure.app
```

앱은 Dock에 뜨지 않고 메뉴바에만 상주합니다. 메뉴바 아이콘을 **좌클릭**하면 사용량 패널이 열리고, **우클릭**하면 새로고침/설정/종료 메뉴가 나옵니다.

앱 상태(수집 경로 설정, 파싱된 레코드 등)는 `~/.digimon/` 아래에 저장됩니다. 완전히 초기 상태로 되돌리려면:

```bash
rm -rf ~/.digimon
```

## 무엇을 스캔하나요

기본적으로 다음 경로를 재귀적으로 스캔합니다 (존재하지 않아도 무해합니다):

| 도구 | 기본 경로 |
|---|---|
| Claude Code | `~/.claude/projects`, `~/.claude/projects2`, `~/Library/Application Support/Claude/local-agent-mode-sessions` |
| Codex | `~/.codex/sessions`, `~/Library/Application Support/orca/codex-runtime-home` |
| Gemini CLI | `~/.gemini/tmp` |

설정 화면에서 사용자가 직접 추가 경로를 등록할 수 있습니다.

## 개발

```bash
npm run typecheck    # 프론트엔드 타입 체크만
cd src-tauri && cargo test    # 백엔드 테스트
```

## 기술 스택

- 백엔드: Rust + [Tauri 2](https://tauri.app/)
- 프론트엔드: React + TypeScript + Vite

## 알려진 한계

- 로그인 시 자동 시작 기능은 아직 미구현입니다. macOS 시스템 설정의 "로그인 항목"에 이 앱을 직접 추가해 대체할 수 있습니다.
- Gemini CLI 파서는 실제 로그 스키마 확보 전 가정 기반으로 작성되어 있습니다.
- 창 배율 설정이 재시작 시 초기화됩니다.
- 코드 서명/공증이 되어 있지 않아, 배포된 바이너리를 받은 경우 macOS Gatekeeper 경고가 뜰 수 있습니다.

## 라이선스

TBD
