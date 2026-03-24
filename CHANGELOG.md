## Changelog

All notable changes to **SlopGuard** will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project adheres to **semantic versioning** once it reaches `1.0.0`.

---

## [Unreleased]

### Added

### Changed

### Fixed

---

## [0.0.5] - 2026-03-24

### Added

- **Persistent native engine mode** (`--serve`) in `slopguard-engine`:
  - Extension can keep one long-lived engine process instead of spawning per analysis.
  - Enables stateful analysis behavior in native mode (foundation for incremental recomputation).
  - One-shot mode remains supported for compatibility.
- **Incremental AST parsing cache** in engine:
  - Per-document/scope cache key (`documentKey`) support added to request payload.
  - Engine reuses previous tree-sitter `Tree` and applies `InputEdit` before reparse.
  - Keeps deterministic results while reducing repeated parse work.
- **New CFG semantic analyzer scaffold** (AST feature path):
  - CFG IR now includes block/edge model and extended edge markers:
    - `Fallthrough`, `BranchTrue/False`, `LoopBack`, `Break`, `Continue`,
      `TryEdge`, `CatchEdge`, `FinallyEdge`, `ReturnEdge`, `ThrowEdge`.
  - Added semantic rule framework with `RuleContext` + `SemanticRule` trait.
  - Added symbol extraction pass (function defs, call sites, identifiers) for semantic rules.
- **First CFG-backed detector**:
  - `Blocking call in async context` (`issueType: async-blocking`) with evidence snippets.
  - Implemented across supported languages using language adapters:
    - TypeScript, JavaScript, Python, Go, Rust, Ruby, Java.
- **Automated integration tests** for CFG async-blocking behavior:
  - New test suite at `engine/tests/cfg_async_blocking.rs`.
  - Includes TS positive/negative and Python positive coverage.
- **Engine architecture modularization**:
  - CFG split into dedicated modules:
    - `analysis/cfg/{mod,ir,builder,rules,util}`
    - `analysis/cfg/lang/{mod,javascript,python,go,rust,ruby,java}`
  - AST analyzer split from monolith to modules:
    - `analysis/ast/{mod,language,parse_cache,detectors,complexity,evidence,utils}`

### Changed

- Extension engine client now prefers persistent native mode and degrades gracefully:
  - If persistent mode fails, falls back to one-shot native execution.
  - Session-level disablement prevents repeated timeout penalties after first failure.
  - Short timeout guard added to avoid silent hangs in persistent path.
- Output readability improvements:
  - Removed per-line absolute path noise from evidence rendering.
  - Kept concise, structured issue formatting while preserving full snippet visibility.

### Fixed

- Resolved regression where persistent mode could appear to “hang” on unsupported binaries:
  - Added safer argument handling for `cargo run` daemon mode (`-- --serve`).
  - Added timeout + fallback behavior to keep analysis responsive.
- Fixed Python async-context detection edge case revealed by automated tests:
  - Async detection now handles both `async_function_definition` and `async def ...` text-shape fallback.
---

## [0.0.4] - 2026-03-19

### Added

- **Complexity scorecard (educational USP)** in the output panel: side-by-side **current vs suggested** time/space complexity for algorithmic issues, plus a **trade-off headline** and detailed trade-off bullets.
- **Approach scorecard** for maintainability-style issues: **Current → Suggested** framing with “why it matters” context.
- **Symbol impact (LSP references)**: command `SlopGuard: Show Symbol Impact (References)` uses the editor’s reference provider to list how many times a symbol appears and **which files** are affected — a lightweight change-impact preview (works best with TS/JS and other language servers).
- **UX polish (low friction)**:
  - **Status bar** entry (`SlopGuard`) opening **Quick Actions** (analyze, symbol impact, open output, settings, toggle idle, walkthrough).
  - **Quick Actions** command and **editor title bar** button.
  - **Run header** in output: scope, engine mode (native label vs WASM), LLM on/off.
  - **Clickable paths** in output (`path:line:col`) for evidence snippets and symbol-impact file list (editor-dependent).
  - **Peek references** button after symbol impact (uses `editor.action.showReferences` when available).
  - **Large-file guard**: `slopguard.maxAnalyzeLines` truncates huge inputs.
  - **Issue cap**: `slopguard.maxIssuesDetailed` summarizes extra issues in one line each.
  - **Get Started walkthrough** (3 steps) + optional **first-run hint** (`slopguard.showFirstRunHint`).
- **Commands**: `SlopGuard: Quick Actions`, `SlopGuard: Open Output`.

### Changed

- Engine `algorithmAnalysis` JSON now includes optional `suggestedTimeComplexity`, `suggestedSpaceComplexity`, and `tradeOffSummary` (populated for nested-loop / algorithmic hotspot findings when AST analysis is available).
- WASM output buffer uses `Mutex` instead of `static mut` (Rust 2024 compatibility); `with_algorithm_analysis` is gated behind the `ast` feature for clean WASM builds.

---

## [0.0.3] - 2026-03-19

### Added

- Extension `README.md` and `LICENSE` for Marketplace listing quality.
- `Makefile` helpers for packaging and version bumps (`release-patch`, etc.).

### Changed

- Marketplace-focused description and documentation.

---

## [0.0.2] - 2026-03-19

### Changed

- Publisher identifier and packaging metadata for Marketplace uploads.

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
