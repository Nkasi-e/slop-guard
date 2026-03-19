use std::collections::HashMap;

use crate::analysis::Analyzer;
use crate::protocol::{AnalyzeRequest, Issue};

pub struct ComplexityAnalyzer;

impl Analyzer for ComplexityAnalyzer {
    fn analyze(&self, request: &AnalyzeRequest) -> Vec<Issue> {
        let mut issues = Vec::new();
        let code = request.code.as_str();

        let branching_points = estimate_branching_points(code);
        if branching_points >= 8 {
            let mut issue = Issue::new(
                "High branching complexity",
                vec![
                    format!(
                        "Estimated branching points: {branching_points} (threshold: 8)."
                    ),
                    "High branch count is harder to reason about and test thoroughly.".to_string(),
                ],
                0.78,
                Some("Split complex conditions into smaller functions with clear responsibilities.".to_string()),
                Some("maintainability".to_string()),
            );

            // Best-effort evidence: show the first line that looks like branching.
            if let Some((snippet, start_line, end_line)) = branching_evidence_snippet(code) {
                issue = issue.with_snippet_evidence(snippet, start_line, end_line);
            }

            issues.push(issue);
        }

        if let Some((line, count, line_no)) = find_repeated_logic(code) {
            issues.push(Issue::new(
                "Repeated logic detected",
                vec![
                    format!("A similar statement appears {count} times: `{line}`."),
                    "Copy-pasted logic tends to drift and causes inconsistent fixes.".to_string(),
                ],
                0.81,
                Some("Extract shared logic into a helper function.".to_string()),
                Some("maintainability".to_string()),
            ).with_snippet_evidence(line, line_no, line_no));
        }

        issues
    }
}

fn branching_evidence_snippet(code: &str) -> Option<(String, usize, usize)> {
    let keywords = [
        "if ",
        "for ",
        "while ",
        "match ",
        "case ",
        "catch ",
        "else if ",
        "&&",
        "||",
    ];
    let lines: Vec<&str> = code.lines().collect();
    for (i, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        if keywords.iter().any(|kw| t.contains(kw)) {
            // Provide a wider context window so developers can immediately
            // see the branching structure, not just a single line.
            let start = i.saturating_sub(3);
            let end = (i + 3).min(lines.len().saturating_sub(1));
            return Some(
                (
                lines[start..=end]
                    .iter()
                    .map(|l| l.trim_end())
                    .collect::<Vec<_>>()
                    .join("\n"),
                    start,
                    end,
                )
            );
        }
    }
    None
}

fn estimate_branching_points(code: &str) -> usize {
    let keywords = ["if ", "for ", "while ", "match ", "case ", "catch ", "else if "];
    let mut count = 1usize;

    for keyword in keywords {
        count += code.matches(keyword).count();
    }

    count += code.matches("&&").count();
    count += code.matches("||").count();
    count
}

fn find_repeated_logic(code: &str) -> Option<(String, usize, usize)> {
    // Keep one representative raw line per normalized key.
    let mut line_counts: HashMap<String, (usize, String, usize)> = HashMap::new();

    for (idx, raw_line) in code.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.len() < 18 {
            continue;
        }
        if line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        if line == "{" || line == "}" {
            continue;
        }
        let key = normalize_line(line);
        if key.is_empty() {
            continue;
        }
        line_counts
            .entry(key)
            .and_modify(|v| v.0 += 1)
            .or_insert((1, line.to_string(), idx));
    }

    line_counts
        .into_iter()
        .filter(|(_, (count, _, _))| *count >= 3)
        .max_by_key(|(_, (count, _, _))| *count)
        .map(|(_, (count, raw_line, first_idx))| (raw_line, count, first_idx))
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}
