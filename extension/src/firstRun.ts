import * as vscode from "vscode";

/**
 * One-time hint after install / first activation (toggle with slopguard.showFirstRunHint).
 */
export async function maybeShowFirstRunHint(
  context: vscode.ExtensionContext,
  openQuickActions: () => Promise<void>
): Promise<void> {
  const config = vscode.workspace.getConfiguration("slopguard");
  if (!config.get<boolean>("showFirstRunHint", true)) {
    return;
  }
  const key = "slopguard.firstRunHintShown";
  if (context.globalState.get(key)) {
    return;
  }
  await context.globalState.update(key, true);

  const choice = await vscode.window.showInformationMessage(
    "SlopGuard: Use the status bar, Quick Actions, or Cmd+Alt+A / Ctrl+Alt+A to analyze. Symbol impact uses your language server.",
    "Quick Actions",
    "Got it"
  );
  if (choice === "Quick Actions") {
    await openQuickActions();
  }
}
