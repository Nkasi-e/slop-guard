use crate::protocol::{AlgorithmAnalysis, Issue};
use tree_sitter::Node;

use super::evidence::nested_loop_evidence_snippet;
use super::language::SupportedLanguage;
use super::utils::{contains_any, is_control_kind, is_loop_node};

pub fn compute_control_nesting_depth(root: Node) -> usize {
    let mut max_depth = 0usize;
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        let next_depth = if is_control_kind(node.kind()) {
            depth + 1
        } else {
            depth
        };

        max_depth = max_depth.max(next_depth);

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push((child, next_depth));
        }
    }

    max_depth
}

fn compute_loop_nesting_depth_for_language(
    root: Node,
    source: &str,
    language: SupportedLanguage,
) -> usize {
    let mut max_depth = 0usize;
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        let next_depth = if is_loop_node(node, source, language) {
            depth + 1
        } else {
            depth
        };
        max_depth = max_depth.max(next_depth);

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push((child, next_depth));
        }
    }

    max_depth
}

pub fn analyze_algorithmic_complexity(
    root: Node,
    source: &str,
    language: SupportedLanguage,
) -> Option<Issue> {
    let mut loop_depth = compute_loop_nesting_depth_for_language(root, source, language);
    if matches!(language, SupportedLanguage::Ruby) && source.matches(".each").count() >= 2 {
        loop_depth = loop_depth.max(2);
    }
    if loop_depth < 2 {
        return None;
    }

    let loop_depth_u32 = loop_depth.min(10) as u32;
    let time_complexity = format!("O(n^{loop_depth_u32})");
    let space_complexity = if contains_any(source, language.loop_accumulator_tokens()) {
        "O(n)".to_string()
    } else {
        "O(1)".to_string()
    };

    let mut trade_offs = vec![
        "Using a hash-based index can reduce repeated lookups but increases memory usage."
            .to_string(),
        "Pre-computing maps can improve runtime while making code less linear to read."
            .to_string(),
    ];
    if matches!(language, SupportedLanguage::Rust) {
        trade_offs.push(
            "In Rust, map-based optimizations may require ownership/borrowing refactors."
                .to_string(),
        );
    }

    let trade_off_summary = format!(
        "Trade-off: spend O(n) memory on a hash map or sorted index to avoid repeated O(n) inner scans — net often O(n) vs {} as data grows.",
        time_complexity
    );

    let mut issue = Issue::new(
        "Algorithmic complexity hotspot",
        vec![
            format!("AST detected nested loop depth of {loop_depth}."),
            "Nested iteration often becomes the dominant runtime cost on large inputs.".to_string(),
        ],
        0.86,
        Some("Where lookups are repeated, build an index/hash map to approach O(n) overall.".to_string()),
        Some("performance".to_string()),
    )
    .with_algorithm_analysis(AlgorithmAnalysis {
        time_complexity,
        space_complexity,
        suggested_time_complexity: Some(
            "O(n) typical (single pass + O(1) lookups per element via map/set/index)".to_string(),
        ),
        suggested_space_complexity: Some(
            "O(n) auxiliary if you store an index/map (often still better than O(n^k) time)".to_string(),
        ),
        trade_off_summary: Some(trade_off_summary),
        trade_offs,
        optimization_hint: Some(
            "True O(1) for whole-transform workloads is usually not possible; target O(1) lookups via indexing and O(n) total passes."
                .to_string(),
        ),
    });

    if let Some((snippet, start_line, end_line)) = nested_loop_evidence_snippet(source, language) {
        issue = issue.with_snippet_evidence(snippet, start_line, end_line);
    }

    Some(issue)
}
