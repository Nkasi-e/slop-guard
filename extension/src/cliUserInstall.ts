import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";
import { resolveNativeLaunchSpec } from "./config";

const LAUNCH_SH = "launch";
const LAUNCH_CMD = "launch.cmd";

export function userSlopguardConfigDir(): string {
  return path.join(os.homedir(), ".config", "slopguard");
}

export function userLocalBinDir(): string {
  return path.join(os.homedir(), ".local", "bin");
}

function escapeBatchArg(s: string): string {
  return s.replace(/"/g, '""');
}

function writePosixLaunch(specPath: string, spec: NonNullable<ReturnType<typeof resolveNativeLaunchSpec>>): void {
  let body: string;
  if (spec.kind === "binary") {
    const q = JSON.stringify(spec.executable);
    body = `#!/usr/bin/env sh\nset -e\nexec ${q} "$@"\n`;
  } else {
    const q = JSON.stringify(spec.manifest);
    body = `#!/usr/bin/env sh\nset -e\nexec cargo run --quiet --manifest-path ${q} -- "$@"\n`;
  }
  fs.writeFileSync(specPath, body, "utf8");
  try {
    fs.chmodSync(specPath, 0o755);
  } catch {
    /* ignore */
  }
}

function writeWindowsCmd(specPath: string, spec: NonNullable<ReturnType<typeof resolveNativeLaunchSpec>>): void {
  let body: string;
  if (spec.kind === "binary") {
    body = `@echo off\r\n"${escapeBatchArg(spec.executable)}" %*\r\n`;
  } else {
    body = `@echo off\r\ncargo run --quiet --manifest-path "${escapeBatchArg(spec.manifest)}" -- %*\r\n`;
  }
  fs.writeFileSync(specPath, body, "utf8");
}

/**
 * Keeps ~/.config/slopguard/launch* in sync with the resolved engine so existing symlinks keep working
 * after extension updates or config changes.
 */
export function syncUserCliLaunchers(): void {
  const spec = resolveNativeLaunchSpec();
  const dir = userSlopguardConfigDir();
  fs.mkdirSync(dir, { recursive: true });
  const shPath = path.join(dir, LAUNCH_SH);
  const cmdPath = path.join(dir, LAUNCH_CMD);

  if (!spec) {
    for (const p of [shPath, cmdPath]) {
      try {
        fs.unlinkSync(p);
      } catch {
        /* ok */
      }
    }
    return;
  }

  writePosixLaunch(shPath, spec);
  writeWindowsCmd(cmdPath, spec);
}

function pathSegments(): string[] {
  const raw = process.env.PATH ?? process.env.Path ?? "";
  const sep = process.platform === "win32" ? ";" : ":";
  return raw.split(sep).filter(Boolean);
}

export function isDirOnPath(dir: string): boolean {
  const want = path.resolve(dir);
  const wantNorm = process.platform === "win32" ? want.toLowerCase() : want;
  return pathSegments().some((seg) => {
    const r = path.resolve(seg);
    const n = process.platform === "win32" ? r.toLowerCase() : r;
    return n === wantNorm;
  });
}

export function pathSetupSnippets(binDir: string): string {
  const b = binDir;
  const bUnix = b.replace(/\\/g, "/");
  if (process.platform === "win32") {
    const psPath = b.replace(/'/g, "''");
    return [
      "PowerShell (current session):",
      `$env:Path = '${psPath}' + ';' + $env:Path`,
      "",
      "cmd (current session):",
      `set "PATH=${escapeBatchArg(b)};%PATH%"`,
      "",
      "Persist: Windows Settings → search “environment variables” → edit User PATH → add:",
      b,
    ].join("\n");
  }
  return [
    "# bash / zsh — add to ~/.bashrc or ~/.zshrc",
    `export PATH="${bUnix}:$PATH"`,
    "",
    "# fish — add to ~/.config/fish/config.fish",
    `fish_add_path ${bUnix}`,
  ].join("\n");
}

export async function installCliForAllTerminals(): Promise<void> {
  syncUserCliLaunchers();
  const spec = resolveNativeLaunchSpec();
  if (!spec) {
    vscode.window.showErrorMessage(
      "SlopGuard: No native engine to expose as CLI. Install a build with the bundled binary, set slopguard.enginePath, or open a workspace with engine/target / Cargo.toml."
    );
    return;
  }

  const binDir = userLocalBinDir();
  fs.mkdirSync(binDir, { recursive: true });

  const configDir = userSlopguardConfigDir();
  const shLauncher = path.join(configDir, LAUNCH_SH);
  const cmdLauncher = path.join(configDir, LAUNCH_CMD);

  if (process.platform === "win32") {
    const dest = path.join(binDir, "slopguard-engine.cmd");
    try {
      fs.copyFileSync(cmdLauncher, dest);
    } catch (e) {
      vscode.window.showErrorMessage(`SlopGuard: Could not write ${dest}: ${String(e)}`);
      return;
    }
  } else {
    const dest = path.join(binDir, "slopguard-engine");
    try {
      try {
        fs.unlinkSync(dest);
      } catch {
        /* ok */
      }
      fs.symlinkSync(shLauncher, dest);
    } catch (e) {
      vscode.window.showErrorMessage(
        `SlopGuard: Could not link ${dest} → ${shLauncher}. ${String(e)}`
      );
      return;
    }
  }

  const onPath = isDirOnPath(binDir);
  if (onPath) {
    vscode.window.showInformationMessage(
      "SlopGuard: Installed slopguard-engine. Open a new terminal (any app) and run: slopguard-engine scan ."
    );
    return;
  }

  const snippets = pathSetupSnippets(binDir);
  const choice = await vscode.window.showWarningMessage(
    `SlopGuard: Installed to ${binDir}. That folder is not on your PATH yet — add it once for bash, zsh, PowerShell, or fish.`,
    "Copy PATH setup",
    "OK"
  );
  if (choice === "Copy PATH setup") {
    await vscode.env.clipboard.writeText(snippets);
    vscode.window.showInformationMessage("SlopGuard: PATH snippets copied — paste into your shell config, then open a new terminal.");
  }
}
