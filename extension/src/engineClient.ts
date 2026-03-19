import { spawn } from "child_process";
import { AnalyzeInput, EngineCommand, EngineResponse } from "./types";
import { runEngineViaWasm } from "./wasmEngineClient";
import { resolveEngineCommand } from "./config";

export function runEngine(engine: EngineCommand, input: AnalyzeInput): Promise<EngineResponse> {
  return new Promise((resolve, reject) => {
    const child = spawn(engine.command, engine.args, { stdio: ["pipe", "pipe", "pipe"] });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk: Buffer) => {
      stdout += chunk.toString("utf8");
    });

    child.stderr.on("data", (chunk: Buffer) => {
      stderr += chunk.toString("utf8");
    });

    child.on("error", (err) => reject(err));

    child.on("close", (code) => {
      if (code !== 0) {
        const details = stderr.trim() || `exit code ${code}`;
        reject(new Error(details));
        return;
      }

      try {
        const parsed = JSON.parse(stdout) as EngineResponse;
        if (!parsed.issues || !Array.isArray(parsed.issues)) {
          reject(new Error("Invalid engine response shape."));
          return;
        }
        resolve(parsed);
      } catch (err) {
        reject(new Error(`Invalid JSON from engine: ${String(err)}`));
      }
    });

    child.stdin.write(JSON.stringify(input));
    child.stdin.end();
  });
}

// Hybrid helper: prefer native engine when available, otherwise fall back to WASM.
export async function runEngineHybrid(input: AnalyzeInput): Promise<EngineResponse> {
  const engine = resolveEngineCommand();
  if (engine) {
    return runEngine(engine, input);
  }

  // No native engine found (no enginePath, no bundled binary, no workspace engine).
  // Try the WASM backend instead.
  return runEngineViaWasm(input);
}

