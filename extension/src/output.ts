import * as vscode from "vscode";
import { AlgorithmAnalysis, EngineIssue } from "./types";

export const OUTPUT_CHANNEL_NAME = "SlopGuard";

export type RenderContext = {
  sourceFile?: string;
  /** Absolute file URI for clickable snippet links. */
  documentUri?: vscode.Uri;
  scopeLabel?: string;
  engineLabel?: string;
  llmEnriched?: boolean;
  maxIssuesDetailed?: number;
};

function padCell(value: string, width: number): string {
  const trimmed = value.trim();
  if (trimmed.length <= width) {
    return trimmed.padEnd(width, " ");
  }
  return `${trimmed.slice(0, Math.max(0, width - 1))}…`;
}

function renderRunHeader(output: vscode.OutputChannel, context: RenderContext): void {
  output.appendLine("");
  output.appendLine("SlopGuard Results");
  output.appendLine("=".repeat(24));
  if (context.scopeLabel) {
    output.appendLine(`Run:    ${context.scopeLabel}`);
  }
  if (context.engineLabel) {
    output.appendLine(`Engine: ${context.engineLabel}`);
  }
  output.appendLine(`LLM:    ${context.llmEnriched ? "enriched" : "off"}`);
  if (context.sourceFile) {
    output.appendLine(`File:   ${context.sourceFile}`);
  }
  output.appendLine("-".repeat(24));
}

/** VS Code / Cursor often linkifies `path:line:col` in the output panel. */
function appendSnippetLine(
  output: vscode.OutputChannel,
  uri: vscode.Uri | undefined,
  lineNo: number | undefined,
  text: string
): void {
  if (typeof lineNo === "number" && uri?.scheme === "file") {
    const loc = `${uri.fsPath}:${lineNo}:1`;
    output.appendLine(`    ${lineNo}: ${text}  (${loc})`);
  } else if (typeof lineNo === "number") {
    output.appendLine(`    ${lineNo}: ${text}`);
  } else {
    output.appendLine(`    ${text}`);
  }
}

/** Side-by-side style scorecard for algorithmic issues (educational USP). */
function renderAlgorithmScorecard(output: vscode.OutputChannel, a: AlgorithmAnalysis): void {
  output.appendLine("  ─── Complexity scorecard (current → suggested) ───");
  const suggestedTime = a.suggestedTimeComplexity ?? "—";
  const suggestedSpace = a.suggestedSpaceComplexity ?? "—";
  const col = 26;
  output.appendLine(
    `  ${padCell("Dimension", 11)} │ ${padCell("Current (as written)", col)} │ Suggested direction`
  );
  output.appendLine(
    `  ${padCell("Time", 11)} │ ${padCell(a.timeComplexity, col)} │ ${suggestedTime}`
  );
  output.appendLine(
    `  ${padCell("Space", 11)} │ ${padCell(a.spaceComplexity, col)} │ ${suggestedSpace}`
  );
  if (a.tradeOffSummary) {
    output.appendLine(`  ▸ ${a.tradeOffSummary}`);
  }
  if (a.optimizationHint) {
    output.appendLine(`  How: ${a.optimizationHint}`);
  }
  const details = a.tradeOffs ?? [];
  if (details.length > 0) {
    output.appendLine("  Trade-offs (detail):");
    for (const line of details) {
      output.appendLine(`    • ${line}`);
    }
  }
}

/** Maintainability / readability issues: “why” without Big-O table. */
function renderApproachScorecard(output: vscode.OutputChannel, issue: EngineIssue): void {
  if (!issue.suggestion) {
    return;
  }
  output.appendLine("  ─── Approach scorecard (why / how) ───");
  const headline = issue.explanation[0] ?? issue.issue;
  output.appendLine(`  Current:   ${headline}`);
  output.appendLine(`  Suggested: ${issue.suggestion}`);
  if (issue.explanation.length > 1) {
    output.appendLine("  Why it matters:");
    for (let i = 1; i < issue.explanation.length; i++) {
      output.appendLine(`    • ${issue.explanation[i]}`);
    }
  }
}

function renderOneIssue(
  output: vscode.OutputChannel,
  issue: EngineIssue,
  context: RenderContext
): void {
  const uri = context.documentUri;

  output.appendLine("");
  output.appendLine(`- 💡 ${issue.issue}`);
  output.appendLine(`  Confidence: ${Math.round(issue.confidence * 100)}%`);
  if (issue.issueType) {
    output.appendLine(`  Type: ${issue.issueType}`);
  }

  if (issue.algorithmAnalysis) {
    renderAlgorithmScorecard(output, issue.algorithmAnalysis);
    output.appendLine("  Context:");
    for (const reason of issue.explanation) {
      output.appendLine(`    • ${reason}`);
    }
    if (issue.suggestion) {
      output.appendLine(`  Suggestion: ${issue.suggestion}`);
    }
  } else if (issue.suggestion) {
    renderApproachScorecard(output, issue);
  } else {
    for (const reason of issue.explanation) {
      output.appendLine(`  - ${reason}`);
    }
  }

  if (issue.snippet) {
    const start = issue.snippetStartLine;
    const end = issue.snippetEndLine;
    if (typeof start === "number" && typeof end === "number") {
      output.appendLine(`  Evidence snippet (lines ${start}-${end}):`);
    } else {
      output.appendLine(`  Evidence snippet:`);
    }

    const lines = issue.snippet.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const lineNo = typeof start === "number" ? start + i : undefined;
      appendSnippetLine(output, uri, lineNo, lines[i]);
    }
  }
}

export function renderIssues(
  output: vscode.OutputChannel,
  issues: EngineIssue[],
  context: RenderContext = {}
): void {
  renderRunHeader(output, context);

  if (issues.length === 0) {
    output.appendLine("No obvious slop patterns detected in selection.");
    return;
  }

  const maxDetailed = context.maxIssuesDetailed ?? 30;
  const detailed = issues.slice(0, maxDetailed);
  const rest = issues.slice(maxDetailed);

  for (const issue of detailed) {
    renderOneIssue(output, issue, context);
  }

  if (rest.length > 0) {
    output.appendLine("");
    output.appendLine(`… and ${rest.length} more issue(s) (summary):`);
    for (const issue of rest) {
      output.appendLine(
        `  - ${issue.issue} (${Math.round(issue.confidence * 100)}%)${issue.issueType ? ` — ${issue.issueType}` : ""}`
      );
    }
  }
}
