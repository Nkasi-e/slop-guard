import * as vscode from "vscode";
import { resolveAnalysisSettings, resolveEngineCommand, resolveLlmSettings } from "../config";
import { runEngine } from "../engineClient";
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

  const engineCommand = resolveEngineCommand();
  if (!engineCommand) {
    vscode.window.showErrorMessage(
      "SlopGuard: Engine not found. Build `engine` first or set slopguard.enginePath."
    );
    return;
  }

  output.clear();
  output.appendLine(`Analyzing ${target.label}...`);

  try {
    const response = await runEngine(engineCommand, {
      code: target.code,
      languageId: editor.document.languageId,
    });

    let issues = response.issues;
    const llmSettings = resolveLlmSettings();
    if (llmSettings.enabled) {
      if (!llmSettings.apiKey) {
        output.appendLine("LLM enrichment skipped: missing API key env vars.");
      } else {
        output.appendLine("Running LLM enrichment...");
        try {
          issues = await enrichIssuesWithLlm(
            target.code,
            editor.document.languageId,
            response.issues,
            llmSettings
          );
          output.appendLine("LLM enrichment applied.");
        } catch (llmError) {
          const message = llmError instanceof Error ? llmError.message : String(llmError);
          output.appendLine(`LLM enrichment skipped: ${message}`);
        }
      }
    }

    renderIssues(output, issues);
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
