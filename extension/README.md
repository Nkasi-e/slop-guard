# SlopGuard

**Ship cleaner code faster — without slowing the business down.**  
SlopGuard is an **in-editor quality layer** for teams that ship software under pressure: it catches structural and maintainability risk **while developers work**, so review cycles stay focused on product and architecture — not the same repetitive feedback on every pull request.

**Current extension version: 0.0.6**

---

## Why SlopGuard matters for the business

| Organizational need | What SlopGuard does |
|---------------------|---------------------|
| **Shorter PR cycles** | Surfaces many “review nits” and complexity hotspots **before** code is pushed, so merges need fewer round-trips. |
| **Better use of senior time** | Automates consistent, objective checks so staff engineers and tech leads spend reviews on **design and trade-offs**, not repeating the same style and structure comments. |
| **Predictable quality at scale** | Same rules for every developer and repo area you scan — important when the team grows or uses AI-assisted coding at high volume. |
| **Lower operational risk** | Highlights patterns tied to bugs and hard-to-test code (deep branching, risky async usage, cross-boundary call patterns) **early**, not only in production incidents. |
| **Trust and compliance** | Core analysis runs **locally** by default; no code leaves the machine unless you **opt in** to the LLM narrative layer with your own keys. |

SlopGuard is not a replacement for human judgment on product or security sign-off. It **reduces noise**, **standardizes bar-raising feedback**, and **makes “good enough to merge” more consistent** across the org.

---

## What’s new in 0.0.6 (high level)

- **Workspace-wide visibility** — **Scan Workspace** pushes findings into the **Problems** panel across many files (including files you never opened), so leads can spot hotspots across a service without manual spot-checks.
- **Codebase-aware analysis** — A **background index** of imports, calls, and cross-file hints feeds the engine so issues that depend on **neighbors and boundaries** (not a single buffer) can surface in normal editing and in workspace scan.
- **CLI that fits real workflows** — **Lint-style** `scan` output for humans, **`--format json`** for automation, plus **Install CLI for All Terminals** and integrated-terminal `PATH` so the same checks can run in **CI and pre-commit** as in the editor.
- **Engine** remains local-first; native bundle + optional WASM fallback; LLM enrichment still **off by default**.

*(Full technical notes: [`CHANGELOG.md` on GitHub](https://github.com/Nkasi-e/slop-guard/blob/main/CHANGELOG.md).)*

---

## Why engineering leaders standardize on SlopGuard

- **Higher effective velocity** — Less rework after review; fewer “I wish we had caught this earlier” moments.
- **Lower reviewer fatigue** — Pull requests become shorter conversations when baseline quality is enforced consistently.
- **Faster onboarding** — New hires get **immediate, in-editor** explanations of maintainability and complexity issues, scaled without pulling seniors into every line.
- **AI-assisted coding with guardrails** — When output volume goes up, **deterministic** checks help keep structural debt from compounding silently.

## Built for fast shipping, designed for production

When teams move quickly — especially with AI-assisted coding — the main risk is not raw output; it is **hidden complexity and inconsistency** that shows up as bugs, slow reviews, and hard refactors later.

SlopGuard gives developers **real-time** feedback on structure and algorithms, for example:

- branching complexity and deep nesting
- repeated logic that drifts across copies
- manual iteration that is harder to review than declarative alternatives
- redundant patterns that obscure intent

That keeps experimentation aligned with what your business actually has to run in production.

## Better algorithmic recommendations

**Business angle:** algorithmic debt is **latency, infra cost, and incident risk** dressed up as “works on my machine.” SlopGuard makes that trade-off visible before it ships.

SlopGuard does not just say "this looks bad."  
It explains where complexity risk appears and suggests practical optimization directions:

- when nested loops indicate likely `O(n^2)` behavior
- where indexing/maps can reduce repeated lookups
- where trade-offs improve runtime at acceptable memory cost

For algorithm-heavy findings, the **SlopGuard** output shows a **complexity scorecard**: **current vs suggested** time/space, a **trade-off headline** (memory vs speed, clarity vs performance), and deeper trade-off notes.

Your team gets recommendations that are actionable, not generic.

## Scan workspace (breadth, not just the open file)

**Business angle:** spot-checking one file at a time does not scale when services grow or when many authors touch the same area. **Scan Workspace** runs SlopGuard across a **capped set of project files** and publishes findings to the **Problems** panel — including files nobody has open — so tech leads get a **portfolio view** of hotspots without a manual audit.

- Command Palette → **`SlopGuard: Scan Workspace`** (also in **Quick Actions**).
- Cap file count with **`slopguard.maxWorkspaceScanFiles`** (default 500) so scans stay predictable in large repos.
- A **lightweight codebase index** (imports, calls, cross-file hints) is used so the same cross-boundary signals you get while editing can appear during workspace scans.

## Quick start (how to use the editor)

### 1) Select code (or rely on auto-detect)

- Manual: select a code section in the editor.
- Auto-detect: if nothing is selected and your scope is `auto`, SlopGuard tries to analyze the current function/block around your cursor.

### 2) Run SlopGuard

Fastest paths:

- **Status bar**: click **SlopGuard** (bottom right) → **Quick Actions** (analyze, symbol impact, output, settings, walkthrough).
- **Editor title bar**: **SlopGuard** icon (when a file tab is open).
- **Command Palette**: `SlopGuard: Analyze Selection` or `SlopGuard: Quick Actions`
- **Shortcut**: `Cmd+Alt+A` (macOS) / `Ctrl+Alt+A` (Windows/Linux)

After analysis, the output panel shows a short **run header** (scope, engine: native vs WASM, LLM on/off). Evidence lines include **clickable file paths** where the editor supports it.

**Get Started**: Command Palette → **Welcome: Open Walkthrough…** → *Get started with SlopGuard* (or use Quick Actions → *Open Get Started walkthrough*).

### Symbol impact (workspace references)

**Business angle:** refactors and API changes have **blast radius**. A quick **who calls this** map reduces surprise breakages and wasted review cycles.

Before you change a function or export, see **where it is used**:

1. Put the cursor on the **symbol name** (function, variable, class, etc.).
2. Run **`SlopGuard: Show Symbol Impact (References)`** (also in the editor right-click menu).

SlopGuard asks the **language service** (same data as “Find All References”) for reference locations, then lists **per-file counts** in the output panel. **Install the usual language extension** for your stack (e.g. built-in TS/JS, Pylance, rust-analyzer) — SlopGuard does not ship language servers. This is **not** a full proof of breakage; it is a fast **call-site map** before you commit.

### 3) Read results

SlopGuard writes findings to the `SlopGuard` output panel.
Each issue includes a title, explanation, confidence, and (when available) algorithm/trade-off analysis.

Images:
![Cursor usage](./media/cursorshot.png)

![VS Code usage](./media/vscodeshot.png)

<!-- Optional demo video:
<video controls width="100%" src="./media/selectionrec.mov"></video>
<video controls width="100%" src="./media/file%20analysicrec.mov"></video> -->

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

### More settings

| Setting | Default | Purpose |
|--------|---------|---------|
| `slopguard.maxWorkspaceScanFiles` | `500` | Max files for **Scan Workspace** (raise for broader audits; lower for speed). |
| `slopguard.maxAnalyzeLines` | `12000` | Cap lines sent to the engine for very large files. |
| `slopguard.maxIssuesDetailed` | `30` | Full detail for the first N issues; rest as one-line summaries. |
| `slopguard.showFirstRunHint` | `true` | One-time tip after install (Quick Actions + shortcuts). |

## Engine resolution (install and use)

**No Rust install required** for normal Marketplace use. SlopGuard picks an engine in this order:

1. **`slopguard.enginePath`** — if set and the file exists.
2. **Bundled native binary** — `runtime/<platform>/slopguard-engine` (or `.exe`) shipped with the extension for your OS/arch when present.
3. **Workspace dev builds** — `engine/target/debug|release/slopguard-engine` under the workspace (or one folder up).
4. **`cargo run`** — if `engine/Cargo.toml` exists in the workspace (developer workflow).
5. **WASM fallback** — `runtime/wasm/slopguard_engine.wasm` for platforms without a native binary. *Uses pattern + complexity analyzers; AST-heavy rules need the native engine.*

### CLI (`scan`) on any terminal (macOS, Linux, Windows)

**Business angle:** the same quality bar you see in the editor can sit in **CI and pre-commit** — so “merge criteria” are not only social pressure in the PR thread.

`scan` uses the **native** engine only (not WASM). After a one-time setup you can run the same command everywhere — **Terminal.app**, **iTerm**, **SSH**, **bash**, **zsh**, **PowerShell**, **cmd**, **fish** (after you add `~/.local/bin` the fish way).

1. **Install the launcher (once)**  
   Command Palette → **`SlopGuard: Install CLI for All Terminals`** (or Quick Actions → *Install CLI for all terminals*).  
   This creates:
   - `~/.config/slopguard/launch` and `launch.cmd` — kept **up to date** whenever SlopGuard runs (engine path / bundled binary / `slopguard.enginePath` / workspace `cargo` dev).
   - A link or copy at **`~/.local/bin/slopguard-engine`** (Windows: `slopguard-engine.cmd` in the same folder).

2. **Put `~/.local/bin` on your PATH** (once per machine) if the extension says it isn’t there yet. Use **Copy PATH setup** from the notification, or add the line it shows for bash, zsh, PowerShell, or fish.

3. Open a **new** terminal and from a project root:

```bash
slopguard-engine scan .
```

Default output is **lint-style** (one block per issue: `path:line:col: level (type): title`, then `note:` / `help:` lines). A short summary goes to **stderr**; exit code **`1`** if any issues were found (good for hooks).

For the previous **JSON** report: `slopguard-engine scan . --format json`. Use `slopguard-engine scan --help` for flags (`--max-files`, `--min-confidence`, `--no-fail`, etc.).

**Also available**

- **Integrated terminals** in Cursor/VS Code still get the engine directory on `PATH` automatically (including macOS zsh + `path_helper` via shell integration).
- **`SlopGuard: Run CLI Scan in Integrated Terminal`** — runs `slopguard-engine scan .` in the editor (cwd = workspace root).
- **`SlopGuard: Copy CLI Scan Command`** — full-path one-liner for CI or scripts without relying on `~/.local/bin`.

**Note:** Set **`slopguard.enginePath`** or use a build that ships the native binary for your OS if you hit WASM-only installs.

LLM enrichment is still **optional** and **off by default** (see below).

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

## Roll out SlopGuard across a team

1. **Pilot** on one service or squad — use **Scan Workspace** once to baseline hotspots.  
2. **Standardize** editor usage (Quick Actions + optional auto-analyze) so feedback is habitual, not optional.  
3. **Gate** risky branches with `slopguard-engine scan` in CI or hooks when you are ready (**exit code 1** when issues exist; **`--format json`** for parsers).  
4. **Keep LLM optional** until policy allows; core value is local and deterministic.

Open any project and run **SlopGuard: Analyze Selection** or **Scan Workspace** to see value in minutes.
