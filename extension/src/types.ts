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
};
