import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { AnalysisSettings, EngineCommand, LlmSettings } from "./types";

/** Set from `activate()` so bundled `runtime/` resolves to the real install dir (not only `out/`). */
let extensionInstallRoot: string | null = null;

export function setSlopguardExtensionInstallRoot(root: string): void {
  extensionInstallRoot = root;
}

function extensionRootForBundledRuntime(): string {
  return extensionInstallRoot ?? path.join(__dirname, "..");
}

/** Resolved native engine + short label for output / UX. */
export type NativeEngineInfo = {
  command: EngineCommand;
  label: string;
};

/**
 * Directory containing the `slopguard-engine` executable, for PATH in integrated terminals.
 * `null` when the resolved engine is `cargo run` (no single binary path) or nothing native exists.
 */
export function nativeEngineExecutableDirectory(): string | null {
  const info = resolveNativeEngine();
  if (!info) {
    return null;
  }
  const { command } = info.command;
  if (command === "cargo") {
    return null;
  }
  return path.dirname(command);
}

export function applySlopguardEnginePathToIntegratedTerminals(context: vscode.ExtensionContext): void {
  const coll = context.environmentVariableCollection;
  coll.description = "SlopGuard: add slopguard-engine to PATH (integrated terminals)";
  const pathKey = process.platform === "win32" ? "Path" : "PATH";
  coll.delete("PATH");
  coll.delete("Path");
  const dir = nativeEngineExecutableDirectory();
  if (!dir) {
    return;
  }
  // macOS login shells often run `path_helper` in /etc/zprofile and replace PATH, dropping
  // values set only at process creation. Shell-integration injection runs after that.
  const pathMutatorOptions: vscode.EnvironmentVariableMutatorOptions = {
    applyAtProcessCreation: true,
    applyAtShellIntegration: true,
  };
  coll.prepend(pathKey, dir, pathMutatorOptions);
}

export function resolveNativeEngine(): NativeEngineInfo | null {
  const configured = vscode.workspace.getConfiguration("slopguard").get<string>("enginePath");
  if (configured && fs.existsSync(configured)) {
    return {
      command: { command: configured, args: [] },
      label: "Custom binary (slopguard.enginePath)",
    };
  }

  const binaryName = process.platform === "win32" ? "slopguard-engine.exe" : "slopguard-engine";
  const extensionRoot = extensionRootForBundledRuntime();

  type PlatformKey = "darwin-arm64" | "darwin-x64" | "linux-x64" | "win32-x64" | "win32-arm64";
  const platformKey = `${process.platform}-${process.arch}` as PlatformKey;

  const runtimeFolder: string | undefined = (() => {
    switch (platformKey) {
      case "darwin-arm64":
        return "darwin-arm64";
      case "darwin-x64":
        return "darwin-x64";
      case "linux-x64":
        return "linux-x64";
      case "win32-x64":
        return "win32-x64";
      case "win32-arm64":
        return "win32-arm64";
      default:
        return undefined;
    }
  })();

  if (runtimeFolder) {
    const bundled = path.join(extensionRoot, "runtime", runtimeFolder, binaryName);
    if (fs.existsSync(bundled)) {
      return {
        command: { command: bundled, args: [] },
        label: `Bundled native engine (${runtimeFolder})`,
      };
    }
  }

  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceRoot) {
    return null;
  }

  const binaryCandidates = [
    path.join(workspaceRoot, "engine", "target", "debug", binaryName),
    path.join(workspaceRoot, "engine", "target", "release", binaryName),
    path.join(workspaceRoot, "..", "engine", "target", "debug", binaryName),
    path.join(workspaceRoot, "..", "engine", "target", "release", binaryName),
  ];

  for (const candidate of binaryCandidates) {
    if (fs.existsSync(candidate)) {
      return {
        command: { command: candidate, args: [] },
        label: "Workspace engine binary (target/debug or release)",
      };
    }
  }

  const cargoManifests = [
    path.join(workspaceRoot, "engine", "Cargo.toml"),
    path.join(workspaceRoot, "..", "engine", "Cargo.toml"),
  ];

  for (const manifest of cargoManifests) {
    if (fs.existsSync(manifest)) {
      return {
        command: {
          command: "cargo",
          args: ["run", "--quiet", "--manifest-path", manifest],
        },
        label: "cargo run (dev)",
      };
    }
  }

  return null;
}

/** How to invoke the engine from a user-level CLI wrapper (real binary vs cargo dev). */
export type NativeLaunchSpec =
  | { kind: "binary"; executable: string }
  | { kind: "cargo"; manifest: string };

export function resolveNativeLaunchSpec(): NativeLaunchSpec | null {
  const info = resolveNativeEngine();
  if (!info) {
    return null;
  }
  const { command, args } = info.command;
  if (command === "cargo") {
    const mpIdx = args.indexOf("--manifest-path");
    const manifest = mpIdx >= 0 && args[mpIdx + 1] ? String(args[mpIdx + 1]) : "";
    if (!manifest) {
      return null;
    }
    const resolved = path.resolve(manifest);
    if (!fs.existsSync(resolved)) {
      return null;
    }
    return { kind: "cargo", manifest: resolved };
  }
  if (!fs.existsSync(command)) {
    return null;
  }
  return { kind: "binary", executable: path.resolve(command) };
}

export function resolveEngineCommand(): EngineCommand | null {
  return resolveNativeEngine()?.command ?? null;
}

export function resolveLlmSettings(): LlmSettings {
  const config = vscode.workspace.getConfiguration("slopguard.llm");

  const enabled = config.get<boolean>("enabled", false);

  // Security note:
  // We intentionally do NOT require users to type an API key into extension settings.
  // Instead, we read from environment variables (builder can set these for their users/CI).
  const apiKey =
    process.env.SLOP_GUARD_LLM_API_KEY ??
    process.env.OPENROUTER_API_KEY ??
    process.env.OPENAI_API_KEY ??
    "";

  const usingOpenRouter = Boolean(process.env.OPENROUTER_API_KEY ?? "");
  const usingOpenAI = Boolean(process.env.OPENAI_API_KEY ?? "");

  const endpoint =
    process.env.SLOP_GUARD_LLM_ENDPOINT ??
    (usingOpenRouter
      ? "https://openrouter.ai/api/v1/chat/completions"
      : "https://api.openai.com/v1/chat/completions");

  // Default model is selected based on provider unless explicitly overridden.
  const model =
    process.env.SLOP_GUARD_LLM_MODEL ??
    config.get<string>(
      "model",
      usingOpenRouter ? "anthropic/claude-3.5-sonnet" : "gpt-4o-mini"
    ) ??
    (usingOpenRouter ? "anthropic/claude-3.5-sonnet" : "gpt-4o-mini");

  return {
    enabled,
    endpoint,
    apiKey,
    model,
    temperature: config.get<number>("temperature", 0.2),
    timeoutMs: config.get<number>("timeoutMs", 12000),
  };
}

export function resolveAnalysisSettings(): AnalysisSettings {
  const config = vscode.workspace.getConfiguration("slopguard");
  return {
    autoAnalyzeOnSave: config.get<boolean>("autoAnalyzeOnSave", false),
    autoAnalyzeOnIdle: config.get<boolean>("autoAnalyzeOnIdle", true),
    autoAnalyzeOnIdleDelayMs: config.get<number>("autoAnalyzeOnIdleDelayMs", 1500),
    scope: config.get<"auto" | "selection" | "function" | "file">("analysisScope", "auto"),
    showAutoNotifications: config.get<boolean>("showAutoNotifications", false),
    maxAnalyzeLines: config.get<number>("maxAnalyzeLines", 12000),
    showFirstRunHint: config.get<boolean>("showFirstRunHint", true),
    maxIssuesDetailed: config.get<number>("maxIssuesDetailed", 30),
    maxWorkspaceScanFiles: config.get<number>("maxWorkspaceScanFiles", 500),
  };
}
