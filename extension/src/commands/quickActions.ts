import * as vscode from "vscode";
import { resolveAnalysisSettings } from "../config";
import { analyzeSelection } from "./analyzeSelection";
import { analyzeWorkspace } from "./analyzeWorkspace";
import { installCliForAllTerminals } from "../cliUserInstall";
import { copyScanCliToClipboard, runScanInIntegratedTerminal } from "./engineCli";
import { showSymbolImpact } from "./symbolImpact";
import { WorkspaceContextIndexer } from "../workspaceContext";

type QuickPickItem = vscode.QuickPickItem & { id: string };

export async function runQuickActions(
  context: vscode.ExtensionContext,
  output: vscode.OutputChannel,
  indexer: WorkspaceContextIndexer,
  diagnostics: vscode.DiagnosticCollection
): Promise<void> {
  const settings = resolveAnalysisSettings();
  const items: QuickPickItem[] = [
    {
      id: "analyze",
      label: "$(search) Analyze code",
      description: "SlopGuard engine on selection / block / file",
      detail: "Same as SlopGuard: Analyze Selection",
    },
    {
      id: "analyzeWorkspace",
      label: "$(folder-opened) Scan workspace",
      description: "Analyze many files (including closed) — Problems panel",
      detail: "Uses slopguard.maxWorkspaceScanFiles cap",
    },
    {
      id: "clearWorkspaceDiagnostics",
      label: "$(clear-all) Clear workspace scan markers",
      description: "Remove red/yellow SlopGuard lines from Scan workspace",
      detail: "Does not change your code",
    },
    {
      id: "installUserCli",
      label: "$(cloud-download) Install CLI for all terminals",
      description: "Terminal.app, iTerm, SSH, PowerShell — one-time + PATH",
      detail: "Writes ~/.local/bin/slopguard-engine (synced when the extension runs)",
    },
    {
      id: "copyScanCli",
      label: "$(terminal) Copy CLI scan command (full path)",
      description: "Git hooks, CI, or when you prefer a one-liner",
      detail: "No install; uses absolute engine path",
    },
    {
      id: "runScanInTerminal",
      label: "$(play) Run CLI scan in integrated terminal",
      description: "slopguard-engine scan . (PATH set by the extension)",
      detail: "Exit code 1 if issues; cargo dev engine uses cargo run …",
    },
    {
      id: "symbolImpact",
      label: "$(references) Symbol impact (references)",
      description: "Where this symbol is used (language service)",
    },
    {
      id: "openOutput",
      label: "$(output) Open SlopGuard output",
      description: "Focus the results panel",
    },
    {
      id: "settings",
      label: "$(gear) Open SlopGuard settings",
      description: "Filter settings for this extension",
    },
    {
      id: "toggleIdle",
      label: settings.autoAnalyzeOnIdle
        ? "$(debug-pause) Turn off analyze-on-idle"
        : "$(debug-start) Turn on analyze-on-idle",
      description: `Currently: ${settings.autoAnalyzeOnIdle ? "on" : "off"}`,
    },
    {
      id: "walkthrough",
      label: "$(book) Open Get Started walkthrough",
      description: "Built-in VS Code walkthrough steps",
    },
  ];

  const picked = await vscode.window.showQuickPick(items, {
    title: "SlopGuard",
    placeHolder: "Choose an action",
  });
  if (!picked) {
    return;
  }

  switch (picked.id) {
    case "analyze":
      await analyzeSelection(output, { mode: "manual", indexer });
      break;
    case "analyzeWorkspace":
      await analyzeWorkspace(output, diagnostics, indexer);
      break;
    case "clearWorkspaceDiagnostics":
      diagnostics.clear();
      vscode.window.showInformationMessage("SlopGuard: Cleared workspace scan markers (Problems).");
      break;
    case "installUserCli":
      await installCliForAllTerminals();
      break;
    case "copyScanCli":
      await copyScanCliToClipboard();
      break;
    case "runScanInTerminal":
      await runScanInIntegratedTerminal();
      break;
    case "symbolImpact":
      await showSymbolImpact(output);
      break;
    case "openOutput":
      output.show(true);
      break;
    case "settings":
      await vscode.commands.executeCommand("workbench.action.openSettings", `@ext:${context.extension.id}`);
      break;
    case "toggleIdle": {
      const config = vscode.workspace.getConfiguration("slopguard");
      await config.update("autoAnalyzeOnIdle", !settings.autoAnalyzeOnIdle, vscode.ConfigurationTarget.Global);
      vscode.window.showInformationMessage(
        `SlopGuard: Auto analyze on idle is now ${!settings.autoAnalyzeOnIdle ? "on" : "off"}.`
      );
      break;
    }
    case "walkthrough":
      await vscode.commands.executeCommand(
        "workbench.action.openWalkthrough",
        `${context.extension.id}#slopguard-intro`
      );
      break;
    default:
      break;
  }
}
