import * as vscode from "vscode";
import { resolveAnalysisSettings, resolveLlmSettings } from "../config";
import { runEngineHybrid } from "../engineClient";
import { enrichIssuesWithLlm } from "../llmClient";
import { renderIssues } from "../output";
import { resolveAnalysisTarget } from "../scope";

type AnalyzeOptions = {
  mode: "manual" | "auto";
  document?: vscode.TextDocument;
};

export async function analyzeSelection(
  output: vscode.OutputChannel,
  options: AnalyzeOptions = { mode: "manual" }
): Promise<void> {
  const editor = getTargetEditor(options.document);
  if (!editor) {
    if (options.mode === "manual") {
      vscode.window.showWarningMessage("SlopGuard: No active editor found.");
    }
    return;
  }

  const analysisSettings = resolveAnalysisSettings();
  const target = resolveAnalysisTarget(editor, analysisSettings.scope);
  if (!target) {
    if (options.mode === "manual") {
      vscode.window.showWarningMessage("SlopGuard: Could not detect code scope to analyze.");
    }
    return;
  }

  const maxLines = analysisSettings.maxAnalyzeLines;
  let code = target.code;
  const lineCount = code.split("\n").length;
  let truncatedNote: string | undefined;
  if (lineCount > maxLines) {
    code = code.split("\n").slice(0, maxLines).join("\n");
    truncatedNote = `Input truncated: file has ${lineCount} lines; analyzing first ${maxLines} (slopguard.maxAnalyzeLines).`;
    if (options.mode === "manual") {
      vscode.window.showWarningMessage(`SlopGuard: Large file — analyzing first ${maxLines} lines only.`);
    }
  }

  output.clear();
  output.appendLine(`Analyzing ${target.label}...`);
  const sourceFile = vscode.workspace.asRelativePath(editor.document.uri, false);
  output.appendLine(`Source file: ${sourceFile}`);
  if (truncatedNote) {
    output.appendLine(truncatedNote);
  }

  try {
    const { response, engineLabel } = await runEngineHybrid({
      code,
      languageId: editor.document.languageId,
      documentKey: `${editor.document.uri.toString()}::${target.label}`,
    });

    let issues = response.issues;
    const llmSettings = resolveLlmSettings();
    let llmEnriched = false;
    if (llmSettings.enabled) {
      if (!llmSettings.apiKey) {
        output.appendLine("LLM enrichment skipped: missing API key env vars.");
      } else {
        output.appendLine("Running LLM enrichment...");
        try {
          issues = await enrichIssuesWithLlm(
            code,
            editor.document.languageId,
            response.issues,
            llmSettings
          );
          llmEnriched = true;
          output.appendLine("LLM enrichment applied.");
        } catch (llmError) {
          const message = llmError instanceof Error ? llmError.message : String(llmError);
          output.appendLine(`LLM enrichment skipped: ${message}`);
        }
      }
    }

    renderIssues(output, issues, {
      sourceFile,
      scopeLabel: `${analysisSettings.scope} → ${target.label}`,
      engineLabel,
      llmEnriched,
      maxIssuesDetailed: analysisSettings.maxIssuesDetailed,
    });
    output.show(true);

    if (issues.length === 0) {
      if (options.mode === "manual" || analysisSettings.showAutoNotifications) {
        vscode.window.showInformationMessage("SlopGuard: No obvious slop patterns detected.");
      }
      return;
    }

    if (options.mode === "manual" || analysisSettings.showAutoNotifications) {
      vscode.window.showInformationMessage(`SlopGuard: Found ${issues.length} potential issue(s).`);
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown engine error.";
    vscode.window.showErrorMessage(`SlopGuard: Analysis failed - ${message}`);
    output.appendLine(`Analysis failed: ${message}`);
    output.show(true);
  }
}

function getTargetEditor(document?: vscode.TextDocument): vscode.TextEditor | undefined {
  if (!document) {
    return vscode.window.activeTextEditor;
  }

  const visible = vscode.window.visibleTextEditors.find(
    (editor) => editor.document.uri.toString() === document.uri.toString()
  );
  return visible ?? vscode.window.activeTextEditor;
}
