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

## Developer experience

- Command: `SlopGuard: Analyze Selection`
- Editor context menu integration
- Shortcut:
  - macOS: `Cmd+Alt+A`
  - Windows/Linux: `Ctrl+Alt+A`
- Auto-analyze on idle (enabled by default)
- Scopes: `auto`, `selection`, `function`, `file`

## Zero-setup runtime model

SlopGuard resolves engines in this order:

1. `slopguard.enginePath` (if configured)
2. bundled native runtime for your platform
3. workspace binary / cargo fallback (dev mode)
4. bundled WASM fallback

Result: most users can install and run with no Rust toolchain setup.

## Optional LLM narrative layer

Core analysis is local and deterministic.  
Optional LLM enrichment can improve wording and coaching tone, but it is not required for functionality.

If LLM is unavailable, SlopGuard still returns full static-analysis output from the engine.

## Ideal for teams that care about outcomes

Use SlopGuard when you want to:

- improve code quality without slowing delivery
- reduce PR noise and reviewer fatigue
- scale engineering standards across fast-moving teams
- keep vibe coding creative while staying technically disciplined

Open any project and run **SlopGuard: Analyze Selection** to see immediate value.
