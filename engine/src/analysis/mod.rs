#[cfg(feature = "ast")]
mod ast;
#[cfg(feature = "ast")]
mod cfg;
mod complexity;
mod idioms;
mod patterns;

use crate::protocol::{AnalyzeRequest, Issue};
use std::collections::HashMap;
#[cfg(feature = "ast")]
use ast::AstAnalyzer;
#[cfg(feature = "ast")]
use cfg::CfgAnalyzer;
use complexity::ComplexityAnalyzer;
use idioms::IdiomaticAnalyzer;
use patterns::PatternAnalyzer;

trait Analyzer {
    fn analyze(&self, request: &AnalyzeRequest) -> Vec<Issue>;
}

pub fn run_all_analyzers(request: &AnalyzeRequest) -> Vec<Issue> {
    #[cfg_attr(not(feature = "ast"), allow(unused_mut))]
    let mut analyzers: Vec<Box<dyn Analyzer>> = vec![
        Box::new(ComplexityAnalyzer),
        Box::new(PatternAnalyzer),
        Box::new(IdiomaticAnalyzer),
    ];

    #[cfg(feature = "ast")]
    {
        analyzers.insert(0, Box::new(CfgAnalyzer));
        analyzers.insert(0, Box::new(AstAnalyzer));
    }

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
