# 기여 가이드

aidot 프로젝트에 기여해 주셔서 감사합니다!

---

## 개발 환경 설정

### 요구 사항

- Rust 1.70 이상
- Git

### 빌드 및 테스트

```bash
# 저장소 클론
git clone https://github.com/Jooss287/aidot
cd aidot

# 빌드
cargo build

# 테스트 실행
cargo test

# 코드 검사
cargo clippy

# 포맷팅
cargo fmt
```

---

## 프로젝트 구조

```
src/
├── main.rs              # 엔트리 포인트
├── cli.rs               # CLI 정의 (clap)
├── commands/            # 명령어 구현
│   ├── init.rs          # 프리셋 초기화
│   ├── pull.rs          # 프리셋 적용
│   ├── repo.rs          # 저장소 관리
│   ├── detect.rs        # LLM 도구 감지
│   ├── status.rs        # 상태 확인
│   ├── cache.rs         # 캐시 관리
│   └── diff.rs          # 설정 비교
├── adapters/            # 도구별 어댑터
│   ├── traits.rs        # ToolAdapter trait 정의
│   ├── detector.rs      # 도구 자동 감지 로직
│   ├── claude_code.rs   # Claude Code 어댑터
│   ├── cursor.rs        # Cursor 어댑터
│   └── copilot.rs       # GitHub Copilot 어댑터
├── preset/              # 프리셋 처리
│   ├── config.rs        # .aidot-config.toml 파싱
│   └── parser.rs        # 프리셋 파일 읽기
├── repository.rs        # 저장소 관리
├── cache.rs             # 캐시 시스템 (~/.aidot/cache/)
├── git.rs               # Git 작업 (clone, pull)
├── config.rs            # 글로벌 설정 (~/.aidot/config.toml)
├── merge.rs             # 파일 병합 전략
└── error.rs             # 에러 타입 정의
```

---

## 아키텍처 개요

aidot은 **어댑터 패턴**을 사용하여 여러 LLM 도구를 지원합니다.

### 핵심 흐름

```
1. 사용자가 `aidot pull team` 실행
                ↓
2. Git에서 프리셋 저장소 가져오기
                ↓
3. .aidot-config.toml 파싱
                ↓
4. 설치된 LLM 도구 자동 감지 (detector.rs)
                ↓
5. 각 도구의 어댑터가 프리셋을 도구별 형식으로 변환
                ↓
6. 변환된 설정 파일을 프로젝트에 적용
```

### 주요 컴포넌트

| 컴포넌트 | 역할 |
|----------|------|
| `ToolAdapter` | 각 LLM 도구가 구현해야 하는 trait |
| `PresetFiles` | 섹션별로 정리된 프리셋 파일 컬렉션 |
| `MergeStrategy` | 파일 병합 방식 (concat, replace) |
| `ConflictMode` | 파일 충돌 처리 방식 (force, skip, ask) |

---

## 새 LLM 도구 어댑터 추가하기

새로운 LLM 도구(예: Aider, Continue 등)를 지원하려면 `ToolAdapter` trait을 구현합니다.

### ToolAdapter trait

```rust
pub trait ToolAdapter {
    fn name(&self) -> &str;       // 도구 이름 (예: "Claude Code")
    fn detect(&self) -> bool;     // 도구 설치 여부 감지
    fn apply(...) -> Result<ApplyResult>;   // 프리셋 적용
    fn preview(...) -> PreviewResult;       // dry-run 미리보기
}
```

| 메서드 | 역할 |
|--------|------|
| `name()` | CLI 출력에 표시할 도구 이름 |
| `detect()` | CLI 존재 여부, 설정 파일 존재 여부 등으로 감지 |
| `apply()` | 프리셋 파일을 도구별 형식으로 변환하여 저장 |
| `preview()` | 실제 쓰기 없이 변경될 파일 목록 반환 |

### 공용 헬퍼 함수

`src/adapters/traits.rs`에서 제공하는 헬퍼 함수를 활용하세요:

| 함수 | 용도 | 예시 |
|------|------|------|
| `strip_section_prefix()` | 섹션 경로 접두사 제거 | `"rules/code.md"` → `"code.md"` |
| `add_suffix_before_ext()` | `.md` 앞에 접미사 삽입 | `"build.md"` + `"prompt"` → `"build.prompt.md"` |
| `convert_frontmatter_key()` | 프론트매터 키 변환 | `globs:` → `applyTo:` |
| `has_frontmatter()` | YAML 프론트매터 존재 확인 | `---\n...\n---` 감지 |
| `normalize_content()` | 내용 정규화 비교 | 줄 끝 공백/줄바꿈 정규화 |
| `write_with_conflict()` | 충돌 처리 포함 파일 쓰기 | 자동 스킵, 덮어쓰기, diff 표시 |

```rust
use super::traits::{
    strip_section_prefix, add_suffix_before_ext, convert_frontmatter_key,
    has_frontmatter, write_with_conflict, // ...
};

// 예: 프리셋 rules/code-style.md → .mytool/instructions/code-style.instructions.md
let name = strip_section_prefix(&file.relative_path, "rules");
let filename = add_suffix_before_ext(&name, "instructions");

// 예: 프론트매터 globs → paths 키 변환
let content = convert_frontmatter_key(&file.content, "globs", "paths");
```

### 구현 방법

기존 어댑터를 참고하여 구현하세요:
- `src/adapters/cursor.rs` - Cursor 어댑터
- `src/adapters/claude_code.rs` - Claude Code 어댑터
- `src/adapters/copilot.rs` - GitHub Copilot 어댑터

새 어댑터 구현 후 `src/adapters/mod.rs`에 등록하고, `detector.rs`의 감지 목록에 추가합니다.

---

## 기여 프로세스

1. 이슈 생성 또는 기존 이슈 확인
2. 저장소 포크
3. 기능 브랜치 생성 (`git checkout -b feature/my-feature`)
4. 변경 사항 커밋
5. 테스트 실행 (`cargo test`)
6. 린트 확인 (`cargo clippy`)
7. Pull Request 생성

---

## 코드 스타일

- `cargo fmt`로 포맷팅
- `cargo clippy`의 경고 해결
- 테스트 코드 작성

---

## 라이선스

이 프로젝트에 기여하시면 MIT 라이선스에 동의하는 것으로 간주됩니다.
