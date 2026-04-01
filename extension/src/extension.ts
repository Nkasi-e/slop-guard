import * as vscode from "vscode";
import { syncUserCliLaunchers, installCliForAllTerminals } from "./cliUserInstall";
import {
  applySlopguardEnginePathToIntegratedTerminals,
  resolveAnalysisSettings,
  setSlopguardExtensionInstallRoot,
} from "./config";
import { analyzeSelection } from "./commands/analyzeSelection";
import { analyzeWorkspace } from "./commands/analyzeWorkspace";
import { copyScanCliToClipboard, runScanInIntegratedTerminal } from "./commands/engineCli";
import { runQuickActions } from "./commands/quickActions";
import { showSymbolImpact } from "./commands/symbolImpact";
import { maybeShowFirstRunHint } from "./firstRun";
import { OUTPUT_CHANNEL_NAME } from "./output";
import { disposePersistentEngineClient } from "./engineClient";
import { WorkspaceContextIndexer } from "./workspaceContext";

let idleTimeout: ReturnType<typeof setTimeout> | undefined;

export function activate(context: vscode.ExtensionContext) {
  setSlopguardExtensionInstallRoot(context.extensionPath);
  syncUserCliLaunchers();
  applySlopguardEnginePathToIntegratedTerminals(context);

  const output = vscode.window.createOutputChannel(OUTPUT_CHANNEL_NAME);
  const diagnostics = vscode.languages.createDiagnosticCollection("slopguard");
  const indexer = new WorkspaceContextIndexer(context, output);
  void indexer.warmStart();

  const command = vscode.commands.registerCommand("slopguard.analyzeSelection", async () =>
    analyzeSelection(output, { mode: "manual", indexer })
  );

  const symbolImpactCommand = vscode.commands.registerCommand("slopguard.symbolImpact", async () =>
    showSymbolImpact(output)
  );

  const quickActionsCommand = vscode.commands.registerCommand("slopguard.quickActions", async () =>
    runQuickActions(context, output, indexer, diagnostics)
  );

  const openOutputCommand = vscode.commands.registerCommand("slopguard.openOutput", () => {
    output.show(true);
  });

  const analyzeWorkspaceCommand = vscode.commands.registerCommand("slopguard.analyzeWorkspace", async () =>
    analyzeWorkspace(output, diagnostics, indexer)
  );

  const clearDiagnosticsCommand = vscode.commands.registerCommand("slopguard.clearWorkspaceDiagnostics", () => {
    diagnostics.clear();
    vscode.window.showInformationMessage("SlopGuard: Cleared workspace scan markers (Problems).");
  });

  const copyScanCliCommand = vscode.commands.registerCommand("slopguard.copyScanCli", () =>
    copyScanCliToClipboard()
  );
  const runScanTerminalCommand = vscode.commands.registerCommand("slopguard.runScanInTerminal", () =>
    runScanInIntegratedTerminal()
  );
  const installUserCliCommand = vscode.commands.registerCommand("slopguard.installUserCli", () =>
    installCliForAllTerminals()
  );
  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBar.text = "$(shield) SlopGuard";
  statusBar.tooltip = "SlopGuard — click for Quick Actions";
  statusBar.command = "slopguard.quickActions";
  statusBar.show();

  const saveListener = vscode.workspace.onDidSaveTextDocument(async (document) => {
    await indexer.onDocumentSaved(document);
    const settings = resolveAnalysisSettings();
    if (!settings.autoAnalyzeOnSave) {
      return;
    }
    await analyzeSelection(output, { mode: "auto", document, indexer });
  });

  let lastDocumentUri: string | undefined;
  const changeListener = vscode.workspace.onDidChangeTextDocument((event) => {
    const settings = resolveAnalysisSettings();
    if (!settings.autoAnalyzeOnIdle || settings.autoAnalyzeOnIdleDelayMs < 500) {
      return;
    }
    const doc = event.document;
    void indexer.onDocumentChanged(doc);
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
      analyzeSelection(output, { mode: "auto", document: activeEditor.document, indexer }).catch(
        () => {}
      );
    }, settings.autoAnalyzeOnIdleDelayMs);
  });

  const deleteListener = vscode.workspace.onDidDeleteFiles((event) => {
    void indexer.onFilesDeleted(event.files);
  });

  const configListener = vscode.workspace.onDidChangeConfiguration((event) => {
    if (event.affectsConfiguration("slopguard")) {
      syncUserCliLaunchers();
      applySlopguardEnginePathToIntegratedTerminals(context);
      const settings = resolveAnalysisSettings();
      output.appendLine(`Auto analyze on save: ${settings.autoAnalyzeOnSave ? "enabled" : "disabled"}`);
      output.appendLine(`Auto analyze on idle: ${settings.autoAnalyzeOnIdle ? "enabled" : "disabled"}`);
    }
  });

  const workspaceFoldersListener = vscode.workspace.onDidChangeWorkspaceFolders(() => {
    syncUserCliLaunchers();
    applySlopguardEnginePathToIntegratedTerminals(context);
  });

  context.subscriptions.push(
    command,
    symbolImpactCommand,
    quickActionsCommand,
    openOutputCommand,
    analyzeWorkspaceCommand,
    clearDiagnosticsCommand,
    copyScanCliCommand,
    runScanTerminalCommand,
    installUserCliCommand,
    diagnostics,
    saveListener,
    changeListener,
    deleteListener,
    configListener,
    workspaceFoldersListener,
    output,
    statusBar
  );

  void maybeShowFirstRunHint(context, async () => runQuickActions(context, output, indexer, diagnostics));
}

export function deactivate() {
  if (idleTimeout) {
    clearTimeout(idleTimeout);
    idleTimeout = undefined;
  }
  disposePersistentEngineClient();
}
