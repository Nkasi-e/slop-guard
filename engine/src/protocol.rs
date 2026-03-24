use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeRequest {
    pub code: String,
    pub language_id: Option<String>,
    #[serde(default)]
    pub document_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub issues: Vec<Issue>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub issue: String,
    pub explanation: Vec<String>,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm_analysis: Option<AlgorithmAnalysis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet_start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet_end_line: Option<usize>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AlgorithmAnalysis {
    /// Complexity of the code as written (nested loops, repeated scans, etc.).
    pub time_complexity: String,
    pub space_complexity: String,
    /// Target complexity after a typical refactor (index/map, single pass, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_time_complexity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_space_complexity: Option<String>,
    /// One-line headline for the scorecard (memory vs speed, clarity vs perf, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_off_summary: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trade_offs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimization_hint: Option<String>,
}

impl Issue {
    pub fn new(
        issue: impl Into<String>,
        explanation: Vec<String>,
        confidence: f64,
        suggestion: Option<String>,
        issue_type: Option<String>,
    ) -> Self {
        Self {
            issue: issue.into(),
            explanation,
            confidence,
            suggestion,
            issue_type,
            algorithm_analysis: None,
            snippet: None,
            snippet_start_line: None,
            snippet_end_line: None,
        }
    }

    /// Only used by the AST analyzer; omitted in WASM builds (`--no-default-features`).
    #[cfg(feature = "ast")]
    pub fn with_algorithm_analysis(mut self, algorithm_analysis: AlgorithmAnalysis) -> Self {
        self.algorithm_analysis = Some(algorithm_analysis);
        self
    }

    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    pub fn with_snippet_evidence(
        mut self,
        snippet: impl Into<String>,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        self.snippet = Some(snippet.into());
        self.snippet_start_line = Some(start_line);
        self.snippet_end_line = Some(end_line);
        self
    }
}
