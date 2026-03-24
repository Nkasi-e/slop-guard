use crate::analysis::Analyzer;
use crate::protocol::{AnalyzeRequest, Issue};
use tree_sitter::Parser;

mod complexity;
mod detectors;
mod evidence;
mod language;
mod parse_cache;
mod utils;

use complexity::{analyze_algorithmic_complexity, compute_control_nesting_depth};
use detectors::{detect_manual_iteration_ast, detect_redundant_assign_then_return_ast};
use evidence::deep_nesting_evidence_snippet;
use language::SupportedLanguage;
use parse_cache::parse_with_incremental_cache;

pub struct AstAnalyzer;

impl Analyzer for AstAnalyzer {
    fn analyze(&self, request: &AnalyzeRequest) -> Vec<Issue> {
        let supported_language = match SupportedLanguage::from_language_id(request.language_id.as_deref()) {
            Some(language) => language,
            None => return Vec::new(),
        };
        let language = match supported_language.tree_sitter_language() {
            Some(language) => language,
            None => return Vec::new(),
        };

        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }

        let Some(tree) = parse_with_incremental_cache(&mut parser, request, supported_language) else {
            return Vec::new();
        };

        let root = tree.root_node();
        let mut issues = Vec::new();

        if let Some((snippet, start_line, end_line)) =
            detect_manual_iteration_ast(root, &request.code, supported_language)
        {
            issues.push(
                Issue::new(
                    "Manual iteration detected",
                    vec![
                        format!(
                            "AST detected loop-based collection building in {}.",
                            supported_language.display_name()
                        ),
                        "Declarative transforms are usually easier to review and maintain.".to_string(),
                    ],
                    0.96,
                    Some(supported_language.manual_iteration_suggestion().to_string()),
                    Some("ai-slop".to_string()),
                )
                .with_snippet_evidence(snippet, start_line, end_line),
            );
        }

        if let Some((snippet, start_line, end_line)) =
            detect_redundant_assign_then_return_ast(root, &request.code, supported_language)
        {
            issues.push(
                Issue::new(
                    "Redundant variable before return",
                    vec![
                        "AST found a local binding immediately returned.".to_string(),
                        "This is often a temporary variable that can be removed safely.".to_string(),
                    ],
                    0.95,
                    Some("Return the expression directly unless the binding improves clarity.".to_string()),
                    Some("readability".to_string()),
                )
                .with_snippet_evidence(snippet, start_line, end_line),
            );
        }

        let max_depth = compute_control_nesting_depth(root);
        if max_depth > 3 {
            let mut issue = Issue::new(
                "Deep nesting detected",
                vec![
                    format!("AST observed control-flow nesting depth of {max_depth}."),
                    "High nesting raises cognitive load and makes edge cases easier to miss.".to_string(),
                ],
                0.90,
                Some("Use guard clauses or helper functions to flatten nested branches.".to_string()),
                Some("maintainability".to_string()),
            );

            if let Some((snippet, start_line, end_line)) =
                deep_nesting_evidence_snippet(&request.code, supported_language)
            {
                issue = issue.with_snippet_evidence(snippet, start_line, end_line);
            }

            issues.push(issue);
        }

        if let Some(algorithm_issue) =
            analyze_algorithmic_complexity(root, &request.code, supported_language)
        {
            issues.push(algorithm_issue);
        }

        issues
    }
}
