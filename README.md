# aidot

> Claude Code, Cursor, GitHub Copilot 등 AI 코딩 도구 설정을 하나의 프리셋으로 관리하고 동기화하는 CLI

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[English](README.en.md)

---

## 소개

**aidot** (AI dotfiles)는 여러 LLM 코딩 도구(Claude Code, Cursor, GitHub Copilot 등)의 설정을 하나의 프리셋으로 통합 관리하는 CLI 도구입니다.

하나의 프리셋 저장소에서 설정을 관리하고, `aidot pull` 한 번으로 감지된 모든 LLM 도구에 자동 변환하여 적용합니다.

### 핵심 기능

- **도구 중립적 설정 관리** - 언어, IDE에 구애받지 않는 통합 프리셋
- **자동 감지 및 변환** - 설치된 LLM 도구를 자동 감지하고 각 도구 형식으로 변환
- **Git 기반 프리셋 공유** - 팀/개인 설정을 Git 저장소로 버전 관리
- **환경 동기화** - 여러 PC에서 동일한 설정 유지, 팀원 간 설정 공유에 유용

---

## 빠른 시작

```bash
# 설치 (macOS/Linux)
curl -fsSL https://raw.githubusercontent.com/USER/aidot/main/scripts/install.sh | bash

# 설치된 LLM 도구 확인
aidot detect

# 팀 프리셋 저장소 등록
aidot repo add team https://github.com/myteam/llm-config

# 현재 프로젝트에 설정 적용
aidot pull team
```

---

## 설치

### 스크립트로 설치 (권장)

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/USER/aidot/main/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/USER/aidot/main/scripts/install.ps1 | iex
```

### GitHub Releases에서 수동 설치

[Releases 페이지](https://github.com/Jooss287/aidot/releases)에서 플랫폼에 맞는 바이너리를 다운로드하세요:

| 플랫폼 | 파일명 |
|--------|--------|
| Linux (x64) | `aidot-vX.X.X-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `aidot-vX.X.X-aarch64-unknown-linux-gnu.tar.gz` |
| macOS (Intel) | `aidot-vX.X.X-x86_64-apple-darwin.tar.gz` |
| macOS (Apple Silicon) | `aidot-vX.X.X-aarch64-apple-darwin.tar.gz` |
| Windows (x64) | `aidot-vX.X.X-x86_64-pc-windows-msvc.zip` |

다운로드 후 PATH에 추가하세요.

### 소스에서 빌드

```bash
git clone https://github.com/Jooss287/aidot
cd aidot
cargo build --release

# 바이너리 위치: target/release/aidot
```

---

## 사용법

### 명령어 개요

| 명령어 | 설명 |
|--------|------|
| `aidot init` | 새 프리셋 저장소 초기화 |
| `aidot init --from-existing` | 기존 LLM 설정에서 프리셋 추출 |
| `aidot repo add <name> <url>` | 프리셋 저장소 등록 |
| `aidot repo list` | 등록된 저장소 목록 |
| `aidot repo remove <name>` | 저장소 제거 |
| `aidot pull <name>` | 프리셋 적용 |
| `aidot pull --dry-run` | 변경 사항 미리보기 |
| `aidot detect` | 설치된 LLM 도구 감지 |
| `aidot status` | 현재 설정 상태 확인 |
| `aidot diff <name>` | 프리셋과 현재 설정 비교 |
| `aidot cache update` | 캐시된 저장소 업데이트 |

### 저장소 관리

```bash
# Git 저장소 등록
aidot repo add team https://github.com/myteam/llm-config

# 로컬 폴더를 프리셋으로 등록
aidot repo add local-dev ./presets/dev-config --local

# default 플래그와 함께 등록 (pull 시 자동 적용)
aidot repo add team https://github.com/myteam/llm-config --default

# 등록된 저장소 확인
aidot repo list

# 저장소 제거
aidot repo remove team
```

### 설정 적용

```bash
# 특정 프리셋 적용
aidot pull team

# 모든 default 저장소 적용
aidot pull

# 특정 도구에만 적용
aidot pull team --tools claude,cursor

# 변경 사항 미리보기
aidot pull team --dry-run

# 기존 설정 덮어쓰기
aidot pull team --force
```

### 프리셋 생성

```bash
# 빈 프리셋 구조 생성
aidot init

# 기존 LLM 설정에서 프리셋 추출
aidot init --from-existing
```

---

## 프리셋 구조

aidot 프리셋은 다음과 같은 구조를 가집니다:

```
llm-preset/
├── .aidot-config.toml       # 프리셋 설정 파일
├── rules/                   # 규칙/인스트럭션 (LLM 행동 규칙)
│   ├── code-style.md
│   └── security.md
├── memory/                  # 프로젝트 메모리 (아키텍처, 워크플로)
│   └── architecture.md
├── commands/                # 사용자 정의 슬래시 명령어
│   └── review.md
├── mcp/                     # MCP 서버 설정
│   └── filesystem.json
├── hooks/                   # 이벤트 기반 자동화 훅
│   └── pre-commit.json
├── agents/                  # AI 에이전트 정의
│   └── code-reviewer.md
├── skills/                  # 에이전트 유틸리티
│   └── api-client.ts
└── settings/                # 도구별 일반 설정
    └── preferences.json
```

### .aidot-config.toml 예시

```toml
[metadata]
name = "Team LLM Config"
version = "1.0.0"
description = "팀 공용 LLM 설정"

[rules]
files = ["rules/*.md"]
merge_strategy = "concat"  # concat: 연결, replace: 대체

[memory]
directory = "memory/"
merge_strategy = "concat"

[commands]
directory = "commands/"

[mcp]
directory = "mcp/"

[hooks]
directory = "hooks/"

[agents]
directory = "agents/"

[skills]
directory = "skills/"

[settings]
directory = "settings/"
```

---

## 지원 도구

### Claude Code

| 프리셋 | 변환 결과 |
|--------|-----------|
| `rules/*.md` | `.claude/rules/` |
| `memory/*.md` | `.claude/CLAUDE.md` |
| `commands/*.md` | `.claude/commands/` |
| `mcp/*.json` | `.claude/settings.local.json` (mcpServers) |
| `hooks/*.json` | `.claude/hooks.json` |
| `agents/*.md` | `.claude/agents/` |
| `skills/*.ts` | `.claude/skills/` |
| `settings/*.json` | `.claude/settings.local.json` |

### Cursor

| 프리셋 | 변환 결과 |
|--------|-----------|
| `rules/*.md` | `.cursorrules` |
| `memory/*.md` | `.cursorrules` (Always Apply 섹션) |
| `commands/*.md` | `.cursor/commands/` |
| `mcp/*.json` | `.cursor/mcp.json` |
| `hooks/*.json` | `.cursor/hooks.json` |
| `agents/*.md` | `.cursor/agents/` |
| `skills/*.ts` | `.cursor/skills/` |

### GitHub Copilot

| 프리셋 | 변환 결과 |
|--------|-----------|
| `rules/*.md` | `.github/copilot-instructions.md` |
| `memory/*.md` | `.github/copilot-instructions.md` (Project Context) |
| `commands/*.md` | `.github/prompts/*.prompt.md` |
| `mcp/*.json` | `.vscode/mcp.json` |
| `agents/*.md` | `.github/agents/*.agent.md` |
| `skills/*.ts` | `.github/skills/` |

---

## 사용 예시

### 1. 신규 프로젝트 설정

```bash
# 프로젝트 생성
cargo new my-app && cd my-app

# LLM 도구 확인
aidot detect
# ✓ Cursor (IDE)
# ✓ Claude Code (CLI)

# 팀 설정 적용
aidot pull team
# ✓ Cursor → .cursorrules
# ✓ Claude Code → .claude/rules/
```

### 2. 기존 설정에서 프리셋 생성

```bash
# 이미 LLM 설정이 있는 프로젝트에서
cd my-existing-project

# 기존 설정을 프리셋으로 추출
aidot init --from-existing
# Found:
#   ✓ .cursorrules
#   ✓ .claude/rules/
# Converting to aidot preset...

# Git에 커밋하고 공유
git init && git add . && git commit -m "Add LLM preset"
git remote add origin https://github.com/myteam/llm-config
git push -u origin main
```

### 3. 팀 설정 공유

```bash
# 팀원 A: 프리셋 저장소 등록
aidot repo add team https://github.com/myteam/llm-config --default

# 팀원 B: 새 프로젝트에서
cd new-project
aidot pull  # default 저장소 자동 적용
```

---

## 기여하기

프로젝트에 기여하고 싶으시다면 [CONTRIBUTING.md](CONTRIBUTING.md)를 참고해주세요.

---

## 참고 자료

### Claude Code
- [Skills](https://code.claude.com/docs/ko/skills)
- [Plugins](https://code.claude.com/docs/ko/plugins)
- [Settings](https://code.claude.com/docs/ko/settings)

### GitHub Copilot
- [Repository Instructions](https://docs.github.com/ko/copilot/how-tos/configure-custom-instructions/add-repository-instructions)
- [Agent Skills](https://docs.github.com/ko/copilot/concepts/agents/about-agent-skills)
- [MCP](https://docs.github.com/ko/copilot/concepts/context/mcp)
- [Extend Copilot with MCP](https://docs.github.com/ko/copilot/how-tos/provide-context/use-mcp/extend-copilot-chat-with-mcp)
- [Custom Agents](https://docs.github.com/ko/copilot/how-tos/use-copilot-agents/coding-agent/create-custom-agents)

### Cursor
- [Rules](https://cursor.com/ko/docs/context/rules)
- [MCP](https://cursor.com/ko/docs/context/mcp)
- [Skills](https://cursor.com/ko/docs/context/skills)
- [Hooks](https://cursor.com/ko/docs/agent/hooks)
- [Commands](https://cursor.com/ko/docs/agent/chat/commands)
- [Subagents](https://cursor.com/ko/docs/agent/subagents)

---

## 라이선스

MIT License