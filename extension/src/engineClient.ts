import { ChildProcessWithoutNullStreams, spawn } from "child_process";
import { AnalyzeInput, EngineCommand, EngineResponse } from "./types";
import { runEngineViaWasm } from "./wasmEngineClient";
import { resolveNativeEngine } from "./config";

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

export type HybridEngineResult = {
  response: EngineResponse;
  /** Human-readable engine mode for the output header. */
  engineLabel: string;
};

class PersistentEngineClient {
  private child?: ChildProcessWithoutNullStreams;
  private readonly queue: Array<{
    resolve: (value: EngineResponse) => void;
    reject: (reason?: unknown) => void;
  }> = [];
  private buffer = "";

  constructor(private readonly command: EngineCommand) {}

  async analyze(input: AnalyzeInput): Promise<EngineResponse> {
    this.ensureStarted();
    return new Promise<EngineResponse>((resolve, reject) => {
      let request:
        | {
            resolve: (value: EngineResponse) => void;
            reject: (reason?: unknown) => void;
          }
        | undefined;
      const timeout = setTimeout(() => {
        const idx = request ? this.queue.indexOf(request) : -1;
        if (idx >= 0) {
          this.queue.splice(idx, 1);
        }
        reject(new Error("Persistent engine timed out waiting for response."));
      }, 10000);

      const wrappedResolve = (value: EngineResponse) => {
        clearTimeout(timeout);
        resolve(value);
      };

      const wrappedReject = (reason?: unknown) => {
        clearTimeout(timeout);
        reject(reason);
      };

      request = { resolve: wrappedResolve, reject: wrappedReject };
      this.queue.push(request);
      this.child!.stdin.write(`${JSON.stringify(input)}\n`);
    });
  }

  dispose(): void {
    if (!this.child) return;
    this.child.kill();
    this.child = undefined;
    this.rejectAll(new Error("Persistent engine disposed."));
    this.buffer = "";
  }

  private ensureStarted(): void {
    if (this.child && !this.child.killed) return;

    const child = spawn(this.command.command, buildServeArgs(this.command), {
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.child = child;

    child.stdout.on("data", (chunk: Buffer) => {
      this.buffer += chunk.toString("utf8");
      this.drainResponses();
    });

    child.stderr.on("data", () => {
      // No-op: keep stderr consumed to avoid backpressure.
    });

    child.on("error", (err) => {
      this.rejectAll(err);
      this.child = undefined;
    });

    child.on("close", () => {
      this.rejectAll(new Error("Persistent engine process exited."));
      this.child = undefined;
      this.buffer = "";
    });
  }

  private drainResponses(): void {
    while (true) {
      const nl = this.buffer.indexOf("\n");
      if (nl < 0) {
        return;
      }

      const line = this.buffer.slice(0, nl).trim();
      this.buffer = this.buffer.slice(nl + 1);
      if (!line) {
        continue;
      }

      const pending = this.queue.shift();
      if (!pending) {
        continue;
      }

      try {
        const parsed = JSON.parse(line) as EngineResponse;
        if (!parsed.issues || !Array.isArray(parsed.issues)) {
          pending.reject(new Error("Invalid engine response shape."));
          continue;
        }
        pending.resolve(parsed);
      } catch (err) {
        pending.reject(new Error(`Invalid JSON from engine: ${String(err)}`));
      }
    }
  }

  private rejectAll(reason: Error): void {
    while (this.queue.length > 0) {
      const pending = this.queue.shift();
      pending?.reject(reason);
    }
  }
}

function buildServeArgs(command: EngineCommand): string[] {
  const baseArgs = [...command.args];
  // For `cargo run`, binary args must come after `--`.
  if (command.command === "cargo" && baseArgs.includes("run")) {
    if (!baseArgs.includes("--")) {
      baseArgs.push("--");
    }
    baseArgs.push("--serve");
    return baseArgs;
  }
  return [...baseArgs, "--serve"];
}

let persistentNativeClient: PersistentEngineClient | undefined;

function getPersistentNativeClient(command: EngineCommand): PersistentEngineClient {
  if (!persistentNativeClient) {
    persistentNativeClient = new PersistentEngineClient(command);
  }
  return persistentNativeClient;
}

export function disposePersistentEngineClient(): void {
  persistentNativeClient?.dispose();
  persistentNativeClient = undefined;
}

// Hybrid helper: prefer native engine when available, otherwise fall back to WASM.
export async function runEngineHybrid(input: AnalyzeInput): Promise<HybridEngineResult> {
  const native = resolveNativeEngine();
  if (native) {
    try {
      const response = await getPersistentNativeClient(native.command).analyze(input);
      return { response, engineLabel: `${native.label} (persistent)` };
    } catch {
      // Fall back to one-shot process if daemon mode fails unexpectedly.
      const response = await runEngine(native.command, input);
      return { response, engineLabel: `${native.label} (one-shot fallback)` };
    }
  }

  const response = await runEngineViaWasm(input);
  return {
    response,
    engineLabel: "WASM fallback (pattern + complexity rules; no AST)",
  };
}

