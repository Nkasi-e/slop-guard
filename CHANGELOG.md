## Changelog

All notable changes to **SlopGuard** will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project adheres to **semantic versioning** once it reaches `1.0.0`.

---

## [Unreleased]

### Added

- **Complexity scorecard (educational USP)** in the output panel: side-by-side **current vs suggested** time/space complexity for algorithmic issues, plus a **trade-off headline** and detailed trade-off bullets.
- **Approach scorecard** for maintainability-style issues: **Current → Suggested** framing with “why it matters” context.

### Changed

- Engine `algorithmAnalysis` JSON now includes optional `suggestedTimeComplexity`, `suggestedSpaceComplexity`, and `tradeOffSummary` (populated for nested-loop / algorithmic hotspot findings when AST analysis is available).

---

## [0.0.1] - Initial public preview

### Added

- **VS Code-compatible extension** (VS Code, Cursor, Antigravity) with command `SlopGuard: Analyze Selection`.
- **Rust analysis engine** (`slopguard-engine`) that:
  - Accepts `{ code, languageId }` over stdin and returns structured JSON issues.
  - Supports TypeScript, JavaScript, Python, Go, Rust, Ruby, and Java.
  - Detects:
    - Manual iteration + accumulation instead of `map`/`filter`/comprehensions/iterators.
    - Redundant variable before return.
    - Deep nesting and high branching complexity.
    - Algorithmic complexity hotspots via nested loops (with time/space complexity).
    - Repeated logic lines (copy-paste detection).
    - Language-specific idioms (JS/TS, Python, Go, Rust, Ruby, Java).
- **Editor workflows**:
  - Manual command (`SlopGuard: Analyze Selection`) for selection/function/file.
  - `slopguard.analysisScope` with `auto | selection | function | file`.
  - `slopguard.autoAnalyzeOnSave` to analyze on each save.
  - **Copilot-style idle analysis** via `slopguard.autoAnalyzeOnIdle` and `slopguard.autoAnalyzeOnIdleDelayMs`.
- **Result surfacing**:
  - Dedicated `SlopGuard` output channel with human-readable issues.
  - Evidence snippets for each issue (including line ranges) to show the exact code region.
  - Optional notifications via `slopguard.showAutoNotifications`.
- **Optional LLM enrichment layer**:
  - Off by default; configured via `slopguard.llm.*` and environment variables for API keys.
  - Enriches explanations and algorithm analysis without affecting deterministic engine behavior.
- **Developer experience**:
  - Keybinding: `Ctrl+Alt+A` (Windows/Linux), `Cmd+Alt+A` (macOS) to run analysis.
  - Editor context menu entry: `SlopGuard: Analyze Selection`.
  - Engine auto-detection and explicit `slopguard.enginePath` override.
  - Placeholder extension icon path at `extension/media/icon.png`.

