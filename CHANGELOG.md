## Changelog

All notable changes to **SlopGuard** will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project adheres to **semantic versioning** once it reaches `1.0.0`.

---

## [Unreleased]

### Added

### Changed

### Fixed

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
