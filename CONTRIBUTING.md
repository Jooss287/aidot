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
├── main.rs              # 엔트리 포인트 (CLI 라우팅)
├── cli.rs               # CLI 정의 (clap)
├── commands/            # 명령어 구현
│   ├── init.rs          # 프리셋 초기화
│   ├── pull.rs          # 프리셋 적용
│   ├── repo.rs          # 저장소 관리 (add/list/remove/set-default)
│   ├── detect.rs        # LLM 도구 감지
│   ├── status.rs        # 상태 확인
│   ├── cache.rs         # 캐시 관리
│   ├── diff.rs          # 설정 비교 (scan 결과 기반)
│   └── update.rs        # 업데이트 확인
├── adapters/            # 도구별 어댑터
│   ├── traits.rs        # ToolAdapter trait, PresetFiles, ScanResult, ApplyResult 정의
│   ├── common.rs        # 공용 어댑터 로직 (apply_one_to_one, apply_json_merge 등)
│   ├── helpers.rs       # 유틸리티 함수 (strip_section_prefix, has_frontmatter 등)
│   ├── conflict.rs      # 충돌 처리 (ConflictMode, write_with_conflict 등)
│   ├── detector.rs      # 도구 자동 감지 로직
│   ├── claude_code.rs   # Claude Code 어댑터
│   ├── cursor.rs        # Cursor 어댑터
│   └── copilot.rs       # GitHub Copilot 어댑터
├── preset/              # 프리셋 처리
│   ├── config.rs        # .aidot-config.toml 파싱
│   └── parser.rs        # 프리셋 파일 읽기
├── repository.rs        # 저장소 소스 해석 (이름/URL/로컬 경로)
├── cache.rs             # 캐시 시스템 (~/.aidot/cache/)
├── git.rs               # Git 작업 (clone, pull)
├── config.rs            # 글로벌 설정 (~/.aidot/config.toml)
└── error.rs             # 에러 타입 정의
```

---

## 아키텍처 개요

aidot은 **어댑터 패턴**을 사용하여 여러 LLM 도구를 지원합니다.

```
aidot pull → 프리셋 가져오기 → 파싱 → 도구 감지 → 어댑터 변환 → 파일 적용
```

핵심 타입은 `adapters/traits.rs`에 정의되어 있으며, 공용 로직은 `common.rs`, `helpers.rs`, `conflict.rs`에 분리되어 있습니다.

---

## 새 LLM 도구 어댑터 추가하기

`ToolAdapter` trait (`adapters/traits.rs`)을 구현하면 새로운 LLM 도구를 지원할 수 있습니다.

```rust
pub trait ToolAdapter {
    fn name(&self) -> &str;                          // 도구 이름
    fn detect(&self) -> bool;                        // 설치 여부 감지
    fn scan(&self, ...) -> ScanResult;               // 변경 사항 스캔
    fn apply(&self, ...) -> Result<ApplyResult>;     // 프리셋 적용
}
```

기존 어댑터(`cursor.rs`, `claude_code.rs`, `copilot.rs`)를 참고하세요. `common.rs`의 `apply_one_to_one()`, `apply_json_merge()` 등 공용 함수를 활용하면 중복 없이 구현할 수 있습니다.

새 어댑터 구현 후 `adapters/mod.rs`에 등록하고, `detector.rs`의 감지 목록에 추가합니다.

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
