import * as vscode from "vscode";
import { resolveAnalysisSettings } from "../config";
import { analyzeSelection } from "./analyzeSelection";
import { showSymbolImpact } from "./symbolImpact";

type QuickPickItem = vscode.QuickPickItem & { id: string };

export async function runQuickActions(
  context: vscode.ExtensionContext,
  output: vscode.OutputChannel
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
      await analyzeSelection(output, { mode: "manual" });
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
