export type AlgorithmAnalysis = {
  timeComplexity: string;
  spaceComplexity: string;
  /** Target time complexity after a typical refactor (indexing, single pass, etc.). */
  suggestedTimeComplexity?: string;
  suggestedSpaceComplexity?: string;
  /** One-line headline: memory vs speed, clarity vs performance, etc. */
  tradeOffSummary?: string;
  tradeOffs?: string[];
  optimizationHint?: string;
};

export type EngineIssue = {
  issue: string;
  explanation: string[];
  confidence: number;
  suggestion?: string;
  issueType?: string;
  algorithmAnalysis?: AlgorithmAnalysis;
  snippet?: string;
  snippetStartLine?: number;
  snippetEndLine?: number;
};

export type EngineResponse = {
  issues: EngineIssue[];
};

export type AnalyzeInput = {
  code: string;
  languageId: string;
  /** Stable key used by the engine for incremental AST parsing cache. */
  documentKey?: string;
  /** Optional workspace/project signals for cross-file context rules. */
  analysisContext?: AnalysisContext;
};

export type BlockingWrapperHint = {
  symbol: string;
  sourceFile?: string;
  confidenceTier?: "high" | "medium" | "low";
};

export type AnalysisContext = {
  currentFile?: string;
  dependencyNeighbors: string[];
  blockingWrapperHints: BlockingWrapperHint[];
  nPlusOneHints: NPlusOneHint[];
  retryPolicyHints: RetryPolicyHint[];
  callGraphEdges: CallGraphEdge[];
  indexStale: boolean;
  unresolvedDynamicCalls: number;
  unresolvedDynamicImports: number;
};

export type NPlusOneHint = {
  symbol: string;
  sourceFile?: string;
  boundary?: "repository" | "service" | "cross-module" | "package-boundary";
  confidenceTier?: "high" | "medium" | "low";
};

export type RetryPolicyHint = {
  symbol: string;
  sourceFile?: string;
  confidenceTier?: "high" | "medium" | "low";
  hasBackoff: boolean;
  hasJitter: boolean;
  hasCap: boolean;
  propagatesCancellation: boolean;
  filtersTransientErrors: boolean;
};

export type CallGraphEdge = {
  caller: string;
  callee: string;
  sourceFile: string;
  targetFile?: string;
  boundary?: "same-package" | "package-boundary" | "cross-module";
  confidenceTier?: "high" | "medium" | "low";
};

export type EngineCommand = {
  command: string;
  args: string[];
};

export type LlmSettings = {
  enabled: boolean;
  endpoint: string;
  apiKey: string;
  model: string;
  temperature: number;
  timeoutMs: number;
};

export type AnalysisScope = "auto" | "selection" | "function" | "file";

export type AnalysisSettings = {
  autoAnalyzeOnSave: boolean;
  autoAnalyzeOnIdle: boolean;
  autoAnalyzeOnIdleDelayMs: number;
  scope: AnalysisScope;
  showAutoNotifications: boolean;
  /** Cap lines sent to the engine for large files (scope file / fallback). */
  maxAnalyzeLines: number;
  /** One-time hint on first activation (can disable in settings). */
  showFirstRunHint: boolean;
  /** Full detail for first N issues; rest get one-line summaries. */
  maxIssuesDetailed: number;
  /** Max source files to analyze in “Scan workspace” (closed files use Problems panel). */
  maxWorkspaceScanFiles: number;
};
