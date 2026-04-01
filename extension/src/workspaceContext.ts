import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { AnalysisContext, BlockingWrapperHint, CallGraphEdge, NPlusOneHint, RetryPolicyHint } from "./types";

type FileEntry = {
  languageId: string;
  imports: string[];
  exports: string[];
  calls: string[];
  blockingWrappers: BlockingWrapperHint[];
  nPlusOneHints: NPlusOneHint[];
  retryPolicyHints: RetryPolicyHint[];
  callGraphEdges: CallGraphEdge[];
  unresolvedDynamicCalls: number;
  unresolvedDynamicImports: number;
  mtimeMs: number;
};

type IndexSnapshot = {
  files: Record<string, FileEntry>;
  savedAt: number;
};

const INDEX_VERSION = 1;
const CACHE_FILE = `workspace-index-v${INDEX_VERSION}.json`;
const SUPPORTED_GLOB = "**/*.{ts,tsx,js,jsx,py,go,rs,java,rb}";
const MAX_INDEX_FILES = 800;
const INDEX_TIMEOUT_MS = 2500;
const STALE_INDEX_MS = 5 * 60 * 1000;

export class WorkspaceContextIndexer {
  private readonly files = new Map<string, FileEntry>();
  private indexingPromise: Promise<void> | undefined;
  private lastFullReindexAt = 0;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly output: vscode.OutputChannel
  ) {}

  async warmStart(): Promise<void> {
    await this.loadFromDisk();
    this.indexingPromise = this.reindexWorkspace();
    await this.indexingPromise;
  }

  async onDocumentChanged(document: vscode.TextDocument): Promise<void> {
    if (document.uri.scheme !== "file") return;
    const key = normalizeFileKey(document.uri.fsPath);
    this.files.set(key, parseFileEntry(document.getText(), document.languageId, Date.now(), key));
  }

  async onDocumentSaved(document: vscode.TextDocument): Promise<void> {
    if (document.uri.scheme !== "file") return;
    const key = normalizeFileKey(document.uri.fsPath);
    this.files.set(key, parseFileEntry(document.getText(), document.languageId, Date.now(), key));
    void this.persistToDisk();
  }

  async onFilesDeleted(files: readonly vscode.Uri[]): Promise<void> {
    let changed = false;
    for (const uri of files) {
      if (uri.scheme !== "file") continue;
      changed = this.files.delete(normalizeFileKey(uri.fsPath)) || changed;
    }
    if (changed) {
      void this.persistToDisk();
    }
  }

  async getAnalysisContext(document: vscode.TextDocument, budgetMs = 20): Promise<AnalysisContext> {
    if (document.uri.scheme !== "file") {
      return emptyContext();
    }
    const currentFile = normalizeFileKey(document.uri.fsPath);
    return this.buildAnalysisContextForFile(
      currentFile,
      document.getText(),
      document.languageId,
      budgetMs
    );
  }

  /** Context for a file path without an open editor (workspace scan, CLI parity). */
  async getAnalysisContextForFilePath(
    fsPath: string,
    code: string,
    languageId: string,
    budgetMs = 10
  ): Promise<AnalysisContext> {
    return this.buildAnalysisContextForFile(normalizeFileKey(fsPath), code, languageId, budgetMs);
  }

  private async buildAnalysisContextForFile(
    currentFile: string,
    code: string,
    languageId: string,
    budgetMs: number
  ): Promise<AnalysisContext> {
    if (this.indexingPromise) {
      await promiseWithTimeout(this.indexingPromise, budgetMs);
    }

    const own =
      this.files.get(currentFile) ?? parseFileEntry(code, languageId, Date.now(), currentFile);
    this.files.set(currentFile, own);

    const neighbors = new Set<string>();
    const wrappers = new Map<string, BlockingWrapperHint>();
    const nPlusHints = new Map<string, NPlusOneHint>();
    const retryHints = new Map<string, RetryPolicyHint>();
    const callGraphEdges = new Map<string, CallGraphEdge>();
    let unresolvedDynamicCalls = own.unresolvedDynamicCalls;
    let unresolvedDynamicImports = own.unresolvedDynamicImports;

    for (const dep of own.imports) {
      const depNorm = normalizeImportPath(dep, currentFile);
      if (!depNorm) continue;
      const entry = this.files.get(depNorm);
      if (!entry) continue;
      neighbors.add(depNorm);
      unresolvedDynamicCalls += entry.unresolvedDynamicCalls;
      unresolvedDynamicImports += entry.unresolvedDynamicImports;
      for (const hint of entry.blockingWrappers) {
        wrappers.set(hint.symbol, hint);
      }
      for (const hint of entry.nPlusOneHints) {
        nPlusHints.set(hint.symbol, hint);
      }
      for (const hint of entry.retryPolicyHints) {
        retryHints.set(hint.symbol, hint);
      }
      for (const edge of entry.callGraphEdges) {
        callGraphEdges.set(edgeKey(edge), edge);
      }
    }

    for (const hint of own.blockingWrappers) {
      wrappers.set(hint.symbol, hint);
    }
    for (const hint of own.nPlusOneHints) {
      nPlusHints.set(hint.symbol, hint);
    }
    for (const hint of own.retryPolicyHints) {
      retryHints.set(hint.symbol, hint);
    }
    for (const edge of own.callGraphEdges) {
      callGraphEdges.set(edgeKey(edge), edge);
    }

    const indexStale = Date.now() - this.lastFullReindexAt > STALE_INDEX_MS;
    if (indexStale && !this.indexingPromise) {
      this.indexingPromise = this.reindexWorkspace().finally(() => {
        this.indexingPromise = undefined;
      });
    }

    return {
      currentFile,
      dependencyNeighbors: [...neighbors].slice(0, 32),
      blockingWrapperHints: [...wrappers.values()].slice(0, 64),
      nPlusOneHints: [...nPlusHints.values()].slice(0, 64),
      retryPolicyHints: [...retryHints.values()].slice(0, 64),
      callGraphEdges: [...callGraphEdges.values()].slice(0, 200),
      indexStale,
      unresolvedDynamicCalls,
      unresolvedDynamicImports,
    };
  }

  private async reindexWorkspace(): Promise<void> {
    const folders = vscode.workspace.workspaceFolders ?? [];
    if (folders.length === 0) return;
    const start = Date.now();
    const uris = await vscode.workspace.findFiles(SUPPORTED_GLOB, "**/{node_modules,target,dist,out,.git}/**", MAX_INDEX_FILES);
    for (const uri of uris) {
      if (Date.now() - start > INDEX_TIMEOUT_MS) break;
      try {
        const stat = await fs.promises.stat(uri.fsPath);
        const key = normalizeFileKey(uri.fsPath);
        const existing = this.files.get(key);
        if (existing && Math.abs(existing.mtimeMs - stat.mtimeMs) < 1) {
          continue;
        }
        const raw = await fs.promises.readFile(uri.fsPath, "utf8");
        const languageId = inferLanguageId(uri.fsPath);
        this.files.set(key, parseFileEntry(raw, languageId, stat.mtimeMs, key));
      } catch {
        // Ignore unreadable files.
      }
    }
    void this.persistToDisk();
    this.lastFullReindexAt = Date.now();
    this.output.appendLine(`SlopGuard context index: ${this.files.size} file(s) ready.`);
  }

  private async loadFromDisk(): Promise<void> {
    try {
      const dir = this.context.globalStorageUri.fsPath;
      await fs.promises.mkdir(dir, { recursive: true });
      const file = path.join(dir, CACHE_FILE);
      const raw = await fs.promises.readFile(file, "utf8");
      const parsed = JSON.parse(raw) as IndexSnapshot;
      for (const [k, v] of Object.entries(parsed.files ?? {})) {
        this.files.set(k, v);
      }
      this.lastFullReindexAt = parsed.savedAt ?? 0;
    } catch {
      // Ignore cache load failures.
    }
  }

  private async persistToDisk(): Promise<void> {
    try {
      const dir = this.context.globalStorageUri.fsPath;
      await fs.promises.mkdir(dir, { recursive: true });
      const file = path.join(dir, CACHE_FILE);
      const payload: IndexSnapshot = { files: Object.fromEntries(this.files.entries()), savedAt: Date.now() };
      await fs.promises.writeFile(file, JSON.stringify(payload), "utf8");
    } catch {
      // Ignore cache write failures.
    }
  }
}

function parseFileEntry(code: string, languageId: string, mtimeMs: number, filePath: string): FileEntry {
  const imports = extractImports(code);
  const exports = extractExports(code);
  const calls = extractCalls(code);
  const blockingWrappers = inferBlockingWrappers(code, imports, calls);
  const nPlusOneHints = inferNPlusOneHints(code, exports, calls, filePath);
  const retryPolicyHints = inferRetryPolicyHints(code, exports, calls);
  const callGraphEdges = inferCallGraphEdges(filePath, imports, exports, calls);
  return {
    languageId,
    imports,
    exports,
    calls,
    blockingWrappers,
    nPlusOneHints,
    retryPolicyHints,
    callGraphEdges,
    unresolvedDynamicCalls: countDynamicCalls(code),
    unresolvedDynamicImports: countDynamicImports(code),
    mtimeMs,
  };
}

function extractImports(code: string): string[] {
  const patterns = [
    /\bimport\s+[^'"]*?from\s+["']([^"']+)["']/g,
    /\brequire\s*\(\s*["']([^"']+)["']\s*\)/g,
    /\bfrom\s+["']([^"']+)["']/g,
  ];
  return uniqueMatches(code, patterns);
}

function extractExports(code: string): string[] {
  const patterns = [
    /\bexport\s+(?:async\s+)?function\s+([A-Za-z_]\w*)/g,
    /\bdef\s+([A-Za-z_]\w*)\s*\(/g,
    /\bfn\s+([A-Za-z_]\w*)\s*\(/g,
    /\bfunc\s+([A-Za-z_]\w*)\s*\(/g,
  ];
  return uniqueMatches(code, patterns);
}

function extractCalls(code: string): string[] {
  const patterns = [/\b([A-Za-z_]\w*)\s*\(/g, /\.([A-Za-z_]\w*)\s*\(/g];
  return uniqueMatches(code, patterns);
}

function inferBlockingWrappers(code: string, _imports: string[], calls: string[]): BlockingWrapperHint[] {
  const blockingTokens = [
    "readFileSync",
    "writeFileSync",
    "sleep(",
    "time.sleep",
    "std::thread::sleep",
    "requests.get",
    "execSync",
  ];
  const isBlockingBody = blockingTokens.some((t) => code.includes(t));
  if (!isBlockingBody) return [];

  const out: BlockingWrapperHint[] = [];
  for (const symbol of calls.slice(0, 20)) {
    if (symbol.length < 3 || symbol === "if" || symbol === "for") continue;
    out.push({ symbol, confidenceTier: "medium" });
  }
  return dedupeHints(out);
}

function inferNPlusOneHints(
  code: string,
  exports: string[],
  calls: string[],
  filePath: string
): NPlusOneHint[] {
  const lowered = code.toLowerCase();
  const datastoreSignals = [
    "select ",
    "insert ",
    "update ",
    "delete ",
    ".find(",
    ".findone(",
    ".query(",
    ".execute(",
    "repository",
    "dao",
  ];
  const hasDatastoreSignal = datastoreSignals.some((s) => lowered.includes(s));
  if (!hasDatastoreSignal) return [];

  const out: NPlusOneHint[] = [];
  const boundary = lowered.includes("repository") || lowered.includes("dao") ? "repository" : "service";
  for (const symbol of exports.slice(0, 20)) {
    if (symbol.length < 3) continue;
    const confidenceTier = symbol.toLowerCase().includes("batch") ? "low" : "high";
    out.push({ symbol, sourceFile: filePath, boundary, confidenceTier });
  }
  for (const call of calls.slice(0, 20)) {
    if (!/(find|get|fetch|query|load)/i.test(call)) continue;
    out.push({ symbol: call, sourceFile: filePath, boundary: "cross-module", confidenceTier: "medium" });
  }
  return dedupeNPlusHints(out);
}

function inferRetryPolicyHints(code: string, exports: string[], calls: string[]): RetryPolicyHint[] {
  const lowered = code.toLowerCase();
  const retrySignal = /retry|backoff|attempt|maxattempts|transient|timeout/.test(lowered);
  if (!retrySignal) return [];
  const hasBackoff = /backoff|sleep|delay/.test(lowered);
  const hasJitter = /jitter|random/.test(lowered);
  const hasCap = /max(?:[_\s-])?(?:delay|backoff|wait)|cap/.test(lowered);
  const propagatesCancellation = /abortsignal|context\.done|cancel(?:led|ation)?/.test(lowered);
  const filtersTransientErrors = /5\d\d|econnreset|etimedout|temporar/.test(lowered);

  const out: RetryPolicyHint[] = [];
  for (const symbol of [...exports, ...calls].slice(0, 30)) {
    if (symbol.length < 3) continue;
    if (!/retry|fetch|request|call|load|sync|attempt/i.test(symbol) && exports.includes(symbol) === false) {
      continue;
    }
    out.push({
      symbol,
      confidenceTier: hasBackoff && hasCap ? "high" : "medium",
      hasBackoff,
      hasJitter,
      hasCap,
      propagatesCancellation,
      filtersTransientErrors,
    });
  }
  return dedupeRetryHints(out);
}

function inferCallGraphEdges(
  filePath: string,
  imports: string[],
  exports: string[],
  calls: string[]
): CallGraphEdge[] {
  const edges: CallGraphEdge[] = [];
  const primaryCaller = exports[0] ?? "module";
  const hasPackagePath = filePath.includes(`${path.sep}packages${path.sep}`);
  for (const callee of calls.slice(0, 30)) {
    if (callee.length < 2 || callee === primaryCaller) continue;
    const importTarget = imports.find((i) => i.includes(callee) || !i.startsWith("."));
    edges.push({
      caller: primaryCaller,
      callee,
      sourceFile: filePath,
      targetFile: importTarget,
      boundary: hasPackagePath && importTarget?.includes("packages/") ? "package-boundary" : "cross-module",
      confidenceTier: "medium",
    });
  }
  return dedupeCallGraphEdges(edges);
}

function countDynamicCalls(code: string): number {
  return (code.match(/\beval\s*\(|\bFunction\s*\(/g) ?? []).length;
}

function countDynamicImports(code: string): number {
  return (code.match(/\bimport\s*\(/g) ?? []).length;
}

function uniqueMatches(code: string, patterns: RegExp[]): string[] {
  const out = new Set<string>();
  for (const pattern of patterns) {
    let match: RegExpExecArray | null = null;
    while ((match = pattern.exec(code))) {
      const value = (match[1] ?? "").trim();
      if (value) out.add(value);
    }
  }
  return [...out];
}

function normalizeImportPath(specifier: string, fromFile: string): string | undefined {
  if (!specifier.startsWith(".") && !specifier.startsWith("/")) return undefined;
  const base = path.dirname(fromFile);
  const raw = path.resolve(base, specifier);
  for (const ext of ["", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".rs", ".java", ".rb"]) {
    const candidate = normalizeFileKey(raw + ext);
    if (fs.existsSync(candidate)) return candidate;
  }
  return normalizeFileKey(raw);
}

function normalizeFileKey(file: string): string {
  return path.normalize(file);
}

function inferLanguageId(file: string): string {
  const ext = path.extname(file).toLowerCase();
  switch (ext) {
    case ".ts":
    case ".tsx":
      return "typescript";
    case ".js":
    case ".jsx":
      return "javascript";
    case ".py":
      return "python";
    case ".go":
      return "go";
    case ".rs":
      return "rust";
    case ".rb":
      return "ruby";
    case ".java":
      return "java";
    default:
      return "plaintext";
  }
}

function dedupeHints(hints: BlockingWrapperHint[]): BlockingWrapperHint[] {
  const bySymbol = new Map<string, BlockingWrapperHint>();
  for (const hint of hints) {
    if (!bySymbol.has(hint.symbol)) {
      bySymbol.set(hint.symbol, hint);
    }
  }
  return [...bySymbol.values()];
}

function emptyContext(): AnalysisContext {
  return {
    dependencyNeighbors: [],
    blockingWrapperHints: [],
    nPlusOneHints: [],
    retryPolicyHints: [],
    callGraphEdges: [],
    indexStale: false,
    unresolvedDynamicCalls: 0,
    unresolvedDynamicImports: 0,
  };
}

function dedupeNPlusHints(hints: NPlusOneHint[]): NPlusOneHint[] {
  const bySymbol = new Map<string, NPlusOneHint>();
  for (const hint of hints) {
    if (!bySymbol.has(hint.symbol)) {
      bySymbol.set(hint.symbol, hint);
    }
  }
  return [...bySymbol.values()];
}

function dedupeRetryHints(hints: RetryPolicyHint[]): RetryPolicyHint[] {
  const bySymbol = new Map<string, RetryPolicyHint>();
  for (const hint of hints) {
    if (!bySymbol.has(hint.symbol)) {
      bySymbol.set(hint.symbol, hint);
    }
  }
  return [...bySymbol.values()];
}

function dedupeCallGraphEdges(edges: CallGraphEdge[]): CallGraphEdge[] {
  const byKey = new Map<string, CallGraphEdge>();
  for (const edge of edges) {
    byKey.set(edgeKey(edge), edge);
  }
  return [...byKey.values()];
}

function edgeKey(edge: CallGraphEdge): string {
  return `${edge.caller}->${edge.callee}@${edge.sourceFile}->${edge.targetFile ?? ""}`;
}

async function promiseWithTimeout<T>(promise: Promise<T>, ms: number): Promise<T | undefined> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(undefined), ms);
    void promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      () => {
        clearTimeout(timer);
        resolve(undefined);
      }
    );
  });
}
