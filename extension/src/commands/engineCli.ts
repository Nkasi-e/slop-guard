import * as vscode from "vscode";
import { NativeEngineInfo, resolveNativeEngine } from "../config";

/** Invoked in integrated terminals after SlopGuard prepends the engine directory to PATH. */
const CLI_INVOCATION = process.platform === "win32" ? "slopguard-engine.exe" : "slopguard-engine";

function workspaceRoot(): string | undefined {
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

/**
 * @param scanDir `"."` when terminal cwd is workspace root; else absolute workspace path for clipboard.
 */
export function buildScanCommandLine(native: NativeEngineInfo, scanDir: string): string {
  const { command, args } = native.command;
  if (command === "cargo" && args.includes("run")) {
    const mpIdx = args.indexOf("--manifest-path");
    const manifest = mpIdx >= 0 && args[mpIdx + 1] ? args[mpIdx + 1] : "";
    if (manifest) {
      return `cargo run --quiet --manifest-path ${JSON.stringify(manifest)} -- scan ${scanDir}`;
    }
  }
  return `${JSON.stringify(command)} scan ${scanDir}`;
}

export async function copyScanCliToClipboard(): Promise<void> {
  const root = workspaceRoot();
  if (!root) {
    vscode.window.showWarningMessage("SlopGuard: Open a folder workspace to build the scan command.");
    return;
  }
  const native = resolveNativeEngine();
  if (!native) {
    vscode.window.showErrorMessage(
      "SlopGuard: No native engine for `scan`. Use a build with the bundled binary, set slopguard.enginePath, or open a workspace with engine/target — WASM-only installs cannot run scan."
    );
    return;
  }
  const line = buildScanCommandLine(native, JSON.stringify(root));
  await vscode.env.clipboard.writeText(line);
  vscode.window.showInformationMessage(
    "SlopGuard: Full-path scan command copied — for CI or one-off scripts. For everyday terminals, use “Install CLI for All Terminals” once."
  );
}

export async function runScanInIntegratedTerminal(): Promise<void> {
  const root = workspaceRoot();
  if (!root) {
    vscode.window.showWarningMessage("SlopGuard: Open a folder workspace to run scan.");
    return;
  }
  const native = resolveNativeEngine();
  if (!native) {
    vscode.window.showErrorMessage(
      "SlopGuard: No native engine found. Install a build with the native binary or set slopguard.enginePath."
    );
    return;
  }
  const { command, args } = native.command;
  const line =
    command === "cargo" && args.includes("run")
      ? buildScanCommandLine(native, ".")
      : `${CLI_INVOCATION} scan .`;
  const term = vscode.window.createTerminal({
    name: "SlopGuard scan",
    cwd: root,
  });
  term.show();
  term.sendText(line, true);
}
