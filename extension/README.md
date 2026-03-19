# SlopGuard

**Ship cleaner code faster. SlopGuard is the in-editor quality layer for modern engineering teams.**

SlopGuard helps engineers and vibe coders move fast without creating long-term code debt.  
It reviews structure and algorithm choices while you code, so your team gets production-minded feedback before pull requests.

## Why teams choose SlopGuard

- **Higher engineering velocity**: catch low-quality patterns early and reduce back-and-forth in PRs.
- **Lower review overhead**: let reviewers focus on architecture and product decisions, not repetitive cleanup comments.
- **Consistent standards at scale**: deterministic checks enforce shared quality expectations across the team.
- **Faster onboarding**: junior and mid-level engineers get immediate senior-style guidance in the editor.
- **Safer rapid iteration**: ideal for high-output teams and vibe-coding workflows that need strong guardrails.

## Built for vibe coding, designed for production

When you are shipping quickly with AI-assisted coding, the biggest risk is not speed, it is hidden complexity.

SlopGuard gives you real-time algorithm and maintainability feedback such as:

- branching complexity and deep nesting hotspots
- repeated logic that should be extracted
- manual iteration patterns that can be refactored to cleaner transforms
- redundant variable patterns that reduce readability

This keeps fast experimentation aligned with production quality.

## Better algorithmic recommendations

SlopGuard does not just say "this looks bad."  
It explains where complexity risk appears and suggests practical optimization directions:

- when nested loops indicate likely `O(n^2)` behavior
- where indexing/maps can reduce repeated lookups
- where trade-offs improve runtime at acceptable memory cost

Your team gets recommendations that are actionable, not generic.

## Quick start (how to use the editor)

### 1) Select code (or rely on auto-detect)

- Manual: select a code section in the editor.
- Auto-detect: if nothing is selected and your scope is `auto`, SlopGuard tries to analyze the current function/block around your cursor.

### 2) Run SlopGuard

Open the Command Palette and run:
- `SlopGuard: Analyze Selection`

### 3) Read results

SlopGuard writes findings to the `SlopGuard` output panel.
Each issue includes a title, explanation, confidence, and (when available) algorithm/trade-off analysis.

Images:
![Cursor usage](./media/cursorshot.png)

![VS Code usage](./media/vscodeshot.png)

Optional demo video:
<video controls width="100%" src="./media/selectionrec.mov"></video>
<video controls width="100%" src="./media/file%20analysicrec.mov"></video>

## Auto-analysis on save (optional)

Auto-analysis is disabled by default.

To enable it:
1. Set `slopguard.autoAnalyzeOnSave` to `true`
2. Choose `slopguard.analysisScope`:
   - `auto`: selection -> current function/block -> file
   - `selection`: only selection
   - `function`: current function/block
   - `file`: whole file

Optional: if you want toast notifications, set `slopguard.showAutoNotifications` to `true`.

## Engine resolution (no extra steps for MVP)

SlopGuard locates the Rust engine in this order:

1. `slopguard.enginePath` (if configured)
2. `engine/target/debug|release/slopguard-engine` under your workspace (or one directory up)
3. if it finds `engine/Cargo.toml`, it runs `cargo run --quiet --manifest-path <engine/Cargo.toml>`

## Optional LLM narrative layer (disabled by default)

Core analysis is local and deterministic.
LLM enrichment refines explanations and algorithm commentary, but you must explicitly enable it.

Enable:
- `slopguard.llm.enabled = true`

LLM credentials are read from environment variables (not from editor settings).
Provide one:
- `OPENROUTER_API_KEY` (OpenRouter)
- `OPENAI_API_KEY` (OpenAI)
- `SLOP_GUARD_LLM_API_KEY` (+ optional `SLOP_GUARD_LLM_ENDPOINT`)

If the LLM call fails, SlopGuard falls back to raw Rust-engine results.

## Ideal for teams that care about outcomes

Use SlopGuard when you want to:

- improve code quality without slowing delivery
- reduce PR noise and reviewer fatigue
- scale engineering standards across fast-moving teams
- keep vibe coding creative while staying technically disciplined

Open any project and run **SlopGuard: Analyze Selection** to see immediate value.
