import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { AnalysisSettings, EngineCommand, LlmSettings } from "./types";

export function resolveEngineCommand(): EngineCommand | null {
  const configured = vscode.workspace.getConfiguration("slopguard").get<string>("enginePath");
  if (configured && fs.existsSync(configured)) {
    return { command: configured, args: [] };
  }

  const binaryName = process.platform === "win32" ? "slopguard-engine.exe" : "slopguard-engine";
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
      return { command: candidate, args: [] };
    }
  }

  const cargoManifests = [
    path.join(workspaceRoot, "engine", "Cargo.toml"),
    path.join(workspaceRoot, "..", "engine", "Cargo.toml"),
  ];

  for (const manifest of cargoManifests) {
    if (fs.existsSync(manifest)) {
      return {
        command: "cargo",
        args: ["run", "--quiet", "--manifest-path", manifest],
      };
    }
  }

  return null;
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
    config.get<string>(
      "model",
      usingOpenRouter ? "anthropic/claude-3.5-sonnet" : "gpt-4o-mini"
    ) ?? (usingOpenRouter ? "anthropic/claude-3.5-sonnet" : "gpt-4o-mini");

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
  };
}
