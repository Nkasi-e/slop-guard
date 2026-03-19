import * as vscode from "vscode";
import { resolveAnalysisSettings } from "./config";
import { analyzeSelection } from "./commands/analyzeSelection";
import { OUTPUT_CHANNEL_NAME } from "./output";

let idleTimeout: ReturnType<typeof setTimeout> | undefined;

export function activate(context: vscode.ExtensionContext) {
  const output = vscode.window.createOutputChannel(OUTPUT_CHANNEL_NAME);

  const command = vscode.commands.registerCommand("slopguard.analyzeSelection", async () =>
    analyzeSelection(output, { mode: "manual" })
  );

  const saveListener = vscode.workspace.onDidSaveTextDocument(async (document) => {
    const settings = resolveAnalysisSettings();
    if (!settings.autoAnalyzeOnSave) {
      return;
    }
    await analyzeSelection(output, { mode: "auto", document });
  });

  // Copilot-style: run analysis automatically after user stops typing.
  let lastDocumentUri: string | undefined;
  const changeListener = vscode.workspace.onDidChangeTextDocument((event) => {
    const settings = resolveAnalysisSettings();
    if (!settings.autoAnalyzeOnIdle || settings.autoAnalyzeOnIdleDelayMs < 500) {
      return;
    }
    const doc = event.document;
    if (doc.uri.scheme !== "file" || doc.languageId === "plaintext") {
      return;
    }
    lastDocumentUri = doc.uri.toString();
    if (idleTimeout) clearTimeout(idleTimeout);
    idleTimeout = setTimeout(() => {
      idleTimeout = undefined;
      const activeEditor = vscode.window.activeTextEditor;
      if (!activeEditor || activeEditor.document.uri.toString() !== lastDocumentUri) {
        return;
      }
      analyzeSelection(output, { mode: "auto", document: activeEditor.document }).catch(() => {});
    }, settings.autoAnalyzeOnIdleDelayMs);
  });

  const configListener = vscode.workspace.onDidChangeConfiguration((event) => {
    if (event.affectsConfiguration("slopguard")) {
      const settings = resolveAnalysisSettings();
      output.appendLine(`Auto analyze on save: ${settings.autoAnalyzeOnSave ? "enabled" : "disabled"}`);
      output.appendLine(`Auto analyze on idle: ${settings.autoAnalyzeOnIdle ? "enabled" : "disabled"}`);
    }
  });

  context.subscriptions.push(command, saveListener, changeListener, configListener, output);
}

export function deactivate() {
  if (idleTimeout) {
    clearTimeout(idleTimeout);
    idleTimeout = undefined;
  }
}
