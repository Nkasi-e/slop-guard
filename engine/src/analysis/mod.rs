mod ast;
mod complexity;
mod idioms;
mod patterns;

use crate::protocol::{AnalyzeRequest, Issue};
use std::collections::HashMap;
use ast::AstAnalyzer;
use complexity::ComplexityAnalyzer;
use idioms::IdiomaticAnalyzer;
use patterns::PatternAnalyzer;

trait Analyzer {
    fn analyze(&self, request: &AnalyzeRequest) -> Vec<Issue>;
}

pub fn run_all_analyzers(request: &AnalyzeRequest) -> Vec<Issue> {
    let analyzers: Vec<Box<dyn Analyzer>> = vec![
        // AST analyzer runs first and provides language-aware structural checks.
        Box::new(AstAnalyzer),
        // Complexity analyzer adds heuristic complexity + repeated logic signals.
        Box::new(ComplexityAnalyzer),
        Box::new(PatternAnalyzer),
        Box::new(IdiomaticAnalyzer),
    ];

    let all_issues: Vec<Issue> = analyzers
        .into_iter()
        .flat_map(|analyzer| analyzer.analyze(request))
        .collect();

    dedupe_issues(all_issues)
}

fn dedupe_issues(issues: Vec<Issue>) -> Vec<Issue> {
    let mut by_title: HashMap<String, Issue> = HashMap::new();

    for issue in issues {
        match by_title.get(&issue.issue) {
            Some(existing) if existing.confidence >= issue.confidence => {}
            _ => {
                by_title.insert(issue.issue.clone(), issue);
            }
        }
    }

    by_title.into_values().collect()
}
