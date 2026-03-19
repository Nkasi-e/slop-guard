import * as vscode from "vscode";
import { EngineIssue } from "./types";

export const OUTPUT_CHANNEL_NAME = "SlopGuard";

export function renderIssues(output: vscode.OutputChannel, issues: EngineIssue[]): void {
  output.appendLine("");
  output.appendLine("SlopGuard Results");
  output.appendLine("=".repeat(24));

  if (issues.length === 0) {
    output.appendLine("No obvious slop patterns detected in selection.");
    return;
  }

  for (const issue of issues) {
    output.appendLine("");
    output.appendLine(`- 💡 ${issue.issue}`);
    output.appendLine(`  Confidence: ${Math.round(issue.confidence * 100)}%`);
    if (issue.issueType) {
      output.appendLine(`  Type: ${issue.issueType}`);
    }
    for (const reason of issue.explanation) {
      output.appendLine(`  - ${reason}`);
    }
    if (issue.suggestion) {
      output.appendLine(`  Suggestion: ${issue.suggestion}`);
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
        const text = lines[i];
        if (typeof lineNo === "number") {
          output.appendLine(`    ${lineNo}: ${text}`);
        } else {
          output.appendLine(`    ${text}`);
        }
      }
    }
    if (issue.algorithmAnalysis) {
      output.appendLine(`  Time: ${issue.algorithmAnalysis.timeComplexity}`);
      output.appendLine(`  Space: ${issue.algorithmAnalysis.spaceComplexity}`);
      if (issue.algorithmAnalysis.optimizationHint) {
        output.appendLine(`  Optimization: ${issue.algorithmAnalysis.optimizationHint}`);
      }
      for (const tradeOff of issue.algorithmAnalysis.tradeOffs ?? []) {
        output.appendLine(`  Trade-off: ${tradeOff}`);
      }
    }
  }
}
