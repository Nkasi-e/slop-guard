import * as fs from "fs";
import * as path from "path";
import { AnalyzeInput, EngineResponse } from "./types";

// Avoid depending on TS's WebAssembly lib types (DOM-only); use loose typing.
let wasmInstance: any | null = null;

async function loadWasm(): Promise<any> {
  if (wasmInstance) {
    return wasmInstance;
  }

  // Filename uses underscore (Rust cdylib convention: crate name → lib name).
  const wasmPath = path.join(__dirname, "..", "runtime", "wasm", "slopguard_engine.wasm");
  if (!fs.existsSync(wasmPath)) {
    throw new Error(
      `SlopGuard: WASM engine not found at ${wasmPath}. ` +
      `Native binary was also unavailable for platform ${process.platform}-${process.arch}.`
    );
  }

  const buffer = await fs.promises.readFile(wasmPath);
  const WA = (globalThis as any).WebAssembly;

  // WASI imports required by wasm32-wasip1 runtime. We provide stubs
  // for I/O functions we don't need — analysis runs fully in memory.
  const wasiImports = {
    wasi_snapshot_preview1: {
      fd_write: () => 0,
      fd_read: () => 0,
      fd_close: () => 0,
      fd_seek: () => 0,
      proc_exit: (code: number) => { throw new Error(`WASI proc_exit(${code})`); },
      environ_get: () => 0,
      environ_sizes_get: () => 0,
      args_get: () => 0,
      args_sizes_get: () => 0,
      clock_time_get: () => 0,
      random_get: (_ptr: number, len: number) => {
        // Fill with pseudorandom bytes — only used for hash seeds, not crypto.
        const mem = new Uint8Array(wasmInstance.exports.memory.buffer, _ptr, len);
        for (let i = 0; i < len; i++) {
          mem[i] = (Math.random() * 256) | 0;
        }
        return 0;
      },
    },
  };

  const module = await WA.compile(buffer);
  const instance = await WA.instantiate(module, wasiImports);

  if (!instance.exports.memory) {
    throw new Error("SlopGuard: WASM engine is missing exported `memory`.");
  }
  if (!instance.exports.analyze || !instance.exports.get_output_ptr || !instance.exports.alloc || !instance.exports.dealloc) {
    throw new Error("SlopGuard: WASM engine missing required exports (analyze, get_output_ptr, alloc, dealloc).");
  }

  wasmInstance = instance;
  return instance;
}

export async function runEngineViaWasm(input: AnalyzeInput): Promise<EngineResponse> {
  const instance = await loadWasm();
  const { memory, alloc, dealloc, analyze, get_output_ptr } = instance.exports;

  const encoder = new TextEncoder();
  const decoder = new TextDecoder("utf-8");

  // Serialize input to UTF-8 bytes and write into WASM memory.
  const inputBytes = encoder.encode(JSON.stringify(input));
  const inputLen = inputBytes.length;
  const inputPtr: number = alloc(inputLen);

  const mem = new Uint8Array(memory.buffer);
  mem.set(inputBytes, inputPtr);

  // Run analysis: positive return = output length (ok), negative = error length.
  const result: number = analyze(inputPtr, inputLen);
  dealloc(inputPtr, inputLen);

  const outputPtr: number = get_output_ptr();
  const outputLen = Math.abs(result);

  // Re-read memory after the analyze call (WASM memory may have grown).
  const outMem = new Uint8Array(memory.buffer, outputPtr, outputLen);
  const outputJson = decoder.decode(outMem);

  if (result < 0) {
    throw new Error(`SlopGuard WASM engine error: ${outputJson}`);
  }

  let parsed: EngineResponse;
  try {
    parsed = JSON.parse(outputJson) as EngineResponse;
  } catch {
    throw new Error(`SlopGuard WASM engine returned invalid JSON: ${outputJson.slice(0, 200)}`);
  }

  if (!parsed.issues || !Array.isArray(parsed.issues)) {
    throw new Error("SlopGuard WASM engine returned unexpected response shape.");
  }

  return parsed;
}
