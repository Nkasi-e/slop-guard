use crate::protocol::Issue;
use crate::protocol::{AnalysisContext, BlockingWrapperHint, CallGraphEdge, NPlusOneHint, RetryPolicyHint};
use tree_sitter::Node;

use super::ir::{EdgeKind, FunctionCfg, SymbolTable};
use super::lang::{is_async_context, SupportedLanguage};
use super::util::{first_match, line_for_byte, ranges_overlap, snippet_around_line};

pub struct RuleContext<'a> {
    pub root: Node<'a>,
    pub source: &'a str,
    pub language: SupportedLanguage,
    pub cfg: &'a FunctionCfg,
    pub symbols: &'a SymbolTable,
    pub analysis_context: Option<&'a AnalysisContext>,
}

pub trait SemanticRule {
    fn run(&self, context: &RuleContext<'_>) -> Option<Issue>;
}

pub struct BlockingInAsyncRule;
pub struct BlockingWrapperPropagationRule;
pub struct NPlusOneCrossBoundaryRule;
pub struct RetryPolicyConsistencyRule;

impl SemanticRule for BlockingInAsyncRule {
    fn run(&self, context: &RuleContext<'_>) -> Option<Issue> {
        let mut stack = vec![context.root];
        while let Some(node) = stack.pop() {
            if is_async_context(node, context.source, context.language) {
                let Some(text) = node.utf8_text(context.source.as_bytes()).ok() else {
                    continue;
                };
                if let Some((token, rel_byte)) = first_match(text, context.language.blocking_call_tokens()) {
                    let abs_byte = node.start_byte().saturating_add(rel_byte);
                    let line = line_for_byte(context.source, abs_byte);
                    let (snippet, start_line, end_line) = snippet_around_line(context.source, line, 4);
                    let cfg_block_count = context
                        .cfg
                        .blocks
                        .iter()
                        .filter(|b| ranges_overlap(start_line, end_line, b.start_line, b.end_line))
                        .count();
                    let cfg_edge_count = context
                        .cfg
                        .edges
                        .iter()
                        .filter(|e| {
                            e.from <= e.to
                                || matches!(
                                    e.kind,
                                    EdgeKind::BranchTrue
                                        | EdgeKind::BranchFalse
                                        | EdgeKind::LoopBack
                                        | EdgeKind::TryEdge
                                        | EdgeKind::CatchEdge
                                        | EdgeKind::FinallyEdge
                                )
                        })
                        .count();
                    let cfg_block_id_sum: usize = context.cfg.blocks.iter().map(|b| b.id).sum();
                    let symbol_count = context.symbols.identifiers.len();
                    let function_count = context.symbols.function_defs.len();
                    let call_count = context.symbols.call_sites.len();

                    return Some(
                        Issue::new(
                            "Blocking call in async context",
                            vec![
                                format!(
                                    "Potential blocking call `{}` detected inside {} async execution context.",
                                    token,
                                    context.language.display_name()
                                ),
                                format!(
                                    "CFG observed {} overlapping block(s), {} relevant edge(s), id-sum {}; symbols: {} identifiers, {} function defs, {} call sites.",
                                    cfg_block_count, cfg_edge_count, cfg_block_id_sum, symbol_count, function_count, call_count
                                ),
                                "Blocking operations can stall event loops/executors and degrade tail latency."
                                    .to_string(),
                            ],
                            0.72,
                            Some(context.language.blocking_suggestion().to_string()),
                            Some("async-blocking".to_string()),
                        )
                        .with_snippet_evidence(snippet, start_line, end_line),
                    );
                }
            }

            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                stack.push(child);
            }
        }
        None
    }
}

impl SemanticRule for BlockingWrapperPropagationRule {
    fn run(&self, context: &RuleContext<'_>) -> Option<Issue> {
        let analysis_context = context.analysis_context?;
        if analysis_context.blocking_wrapper_hints.is_empty() {
            return None;
        }
        let mut stack = vec![context.root];
        while let Some(node) = stack.pop() {
            if is_async_context(node, context.source, context.language) {
                let Some(text) = node.utf8_text(context.source.as_bytes()).ok() else {
                    continue;
                };
                if let Some((hint, rel_byte)) =
                    first_blocking_wrapper_match(text, &analysis_context.blocking_wrapper_hints)
                {
                    let abs_byte = node.start_byte().saturating_add(rel_byte);
                    let line = line_for_byte(context.source, abs_byte);
                    let (snippet, start_line, end_line) = snippet_around_line(context.source, line, 4);
                    let (tier, base_confidence) = confidence_for_tier(hint.confidence_tier.as_deref());
                    let unresolved_penalty = dynamic_uncertainty_penalty(analysis_context);
                    let confidence = (base_confidence - unresolved_penalty).clamp(0.35, 0.88);
                    let edge = first_call_edge_for_symbol(analysis_context, &hint.symbol);
                    return Some(
                        Issue::new(
                            "Blocking wrapper call in async context",
                            vec![
                                format!(
                                    "Async code appears to call wrapper/helper `{}` that maps to a blocking operation in workspace context.",
                                    hint.symbol
                                ),
                                format!(
                                    "Cross-file context tier={} (base {:.2}); unresolved dynamic imports/calls: {}/{}.",
                                    tier,
                                    base_confidence,
                                    analysis_context.unresolved_dynamic_imports,
                                    analysis_context.unresolved_dynamic_calls
                                ),
                                format!(
                                    "Context scope: file={} with {} dependency neighbor(s); hint source={}.",
                                    analysis_context
                                        .current_file
                                        .as_deref()
                                        .unwrap_or("unknown"),
                                    analysis_context.dependency_neighbors.len(),
                                    hint.source_file.as_deref().unwrap_or("unknown")
                                ),
                                format!(
                                    "Call graph edge boundary={}.",
                                    edge.and_then(|e| e.boundary.as_deref()).unwrap_or("unknown")
                                ),
                                format!(
                                    "Call graph edge detail={}.",
                                    edge.map(describe_edge).unwrap_or_else(|| "unavailable".to_string())
                                ),
                                "Verify wrapper internals or provide an async-safe variant for event-loop paths."
                                    .to_string(),
                            ],
                            confidence,
                            Some(context.language.blocking_suggestion().to_string()),
                            Some("async-blocking-propagated".to_string()),
                        )
                        .with_snippet_evidence(snippet, start_line, end_line),
                    );
                }
            }

            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                stack.push(child);
            }
        }
        None
    }
}

impl SemanticRule for NPlusOneCrossBoundaryRule {
    fn run(&self, context: &RuleContext<'_>) -> Option<Issue> {
        let analysis_context = context.analysis_context?;
        if analysis_context.n_plus_one_hints.is_empty() {
            return None;
        }

        let mut stack = vec![context.root];
        while let Some(node) = stack.pop() {
            if is_loop_like(node.kind()) {
                let Some(text) = node.utf8_text(context.source.as_bytes()).ok() else {
                    continue;
                };
                if let Some((hint, rel_byte)) = first_n_plus_one_match(text, &analysis_context.n_plus_one_hints) {
                    let abs_byte = node.start_byte().saturating_add(rel_byte);
                    let line = line_for_byte(context.source, abs_byte);
                    let (snippet, start_line, end_line) = snippet_around_line(context.source, line, 4);
                    let base = match hint.confidence_tier.as_deref().unwrap_or("medium") {
                        "high" => 0.86,
                        "low" => 0.6,
                        _ => 0.74,
                    };
                    let confidence =
                        (base - dynamic_uncertainty_penalty(analysis_context) * 0.7).clamp(0.4, 0.9);
                    let edge = first_call_edge_for_symbol(analysis_context, &hint.symbol);
                    let edge_boundary = edge
                        .and_then(|e| e.boundary.as_deref())
                        .or(hint.boundary.as_deref())
                        .unwrap_or("cross-module");
                    return Some(
                        Issue::new(
                            "Potential N+1 across service/repository boundary",
                            vec![
                                format!(
                                    "Loop appears to call `{}` with {} access pattern from workspace context.",
                                    hint.symbol, edge_boundary
                                ),
                                format!(
                                    "Cross-file context includes {} dependency neighbor(s); unresolved dynamic imports/calls: {}/{}.",
                                    analysis_context.dependency_neighbors.len(),
                                    analysis_context.unresolved_dynamic_imports,
                                    analysis_context.unresolved_dynamic_calls
                                ),
                                format!(
                                    "Candidate symbol source={}.",
                                    hint.source_file.as_deref().unwrap_or("unknown")
                                ),
                                format!(
                                    "Call graph edge detail={}.",
                                    edge.map(describe_edge).unwrap_or_else(|| "unavailable".to_string())
                                ),
                                "Consider batch loading, prefetching, or moving lookup outside the loop."
                                    .to_string(),
                            ],
                            confidence,
                            Some("Use bulk fetch or join-style query to avoid per-iteration datastore roundtrips."
                                .to_string()),
                            Some("n-plus-one-cross-boundary".to_string()),
                        )
                        .with_snippet_evidence(snippet, start_line, end_line),
                    );
                }
            }

            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                stack.push(child);
            }
        }
        None
    }
}

impl SemanticRule for RetryPolicyConsistencyRule {
    fn run(&self, context: &RuleContext<'_>) -> Option<Issue> {
        let analysis_context = context.analysis_context?;
        if analysis_context.retry_policy_hints.is_empty() {
            return None;
        }
        let mut stack = vec![context.root];
        while let Some(node) = stack.pop() {
            let Some(text) = node.utf8_text(context.source.as_bytes()).ok() else {
                continue;
            };
            if let Some((hint, rel_byte)) = first_retry_match(text, &analysis_context.retry_policy_hints) {
                let missing = missing_retry_guards(hint);
                if missing.is_empty() {
                    continue;
                }
                let abs_byte = node.start_byte().saturating_add(rel_byte);
                let line = line_for_byte(context.source, abs_byte);
                let (snippet, start_line, end_line) = snippet_around_line(context.source, line, 4);
                let (_, base_confidence) = confidence_for_tier(hint.confidence_tier.as_deref());
                let edge = first_call_edge_for_symbol(analysis_context, &hint.symbol);
                let boundary = edge
                    .and_then(|e| e.boundary.as_deref())
                    .unwrap_or("cross-module");
                let stale_penalty = if analysis_context.index_stale { 0.08 } else { 0.0 };
                let confidence = (base_confidence - dynamic_uncertainty_penalty(analysis_context) - stale_penalty)
                    .clamp(0.35, 0.87);
                return Some(
                    Issue::new(
                        "Retry policy inconsistency across call chain",
                        vec![
                            format!(
                                "Call chain includes retry-like symbol `{}` across {} boundary.",
                                hint.symbol, boundary
                            ),
                            format!("Missing retry safeguards: {}.", missing.join(", ")),
                            format!(
                                "Context source={} index_stale={} unresolved dynamic imports/calls: {}/{}.",
                                hint.source_file.as_deref().unwrap_or("unknown"),
                                analysis_context.index_stale,
                                analysis_context.unresolved_dynamic_imports,
                                analysis_context.unresolved_dynamic_calls
                            ),
                            format!(
                                "Call graph edge detail={}.",
                                edge.map(describe_edge).unwrap_or_else(|| "unavailable".to_string())
                            ),
                        ],
                        confidence,
                        Some(
                            "Standardize retry wrapper to include bounded backoff, jitter, cancellation propagation, and transient error filtering."
                                .to_string(),
                        ),
                        Some("retry-policy-cross-chain".to_string()),
                    )
                    .with_snippet_evidence(snippet, start_line, end_line),
                );
            }
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                stack.push(child);
            }
        }
        None
    }
}

fn first_blocking_wrapper_match<'a>(
    text: &'a str,
    hints: &'a [BlockingWrapperHint],
) -> Option<(&'a BlockingWrapperHint, usize)> {
    let mut best: Option<(&BlockingWrapperHint, usize)> = None;
    for hint in hints {
        let symbol = hint.symbol.as_str();
        if symbol.len() < 2 {
            continue;
        }
        for pattern in [format!("{symbol}("), format!(".{symbol}(")] {
            if let Some(idx) = text.find(&pattern) {
                let candidate = (hint, idx);
                match best {
                    Some((_, best_idx)) if best_idx <= idx => {}
                    _ => best = Some(candidate),
                }
            }
        }
    }
    best
}

fn confidence_for_tier(tier: Option<&str>) -> (&'static str, f64) {
    match tier.unwrap_or("medium").to_ascii_lowercase().as_str() {
        "high" => ("high", 0.82),
        "low" => ("low", 0.58),
        _ => ("medium", 0.72),
    }
}

fn dynamic_uncertainty_penalty(analysis_context: &AnalysisContext) -> f64 {
    let unresolved = analysis_context.unresolved_dynamic_calls + analysis_context.unresolved_dynamic_imports;
    if unresolved >= 8 {
        0.18
    } else if unresolved >= 4 {
        0.1
    } else if unresolved > 0 {
        0.05
    } else {
        0.0
    }
}

fn first_n_plus_one_match<'a>(
    text: &'a str,
    hints: &'a [NPlusOneHint],
) -> Option<(&'a NPlusOneHint, usize)> {
    let mut best: Option<(&NPlusOneHint, usize)> = None;
    for hint in hints {
        if hint.symbol.len() < 2 {
            continue;
        }
        for pattern in [format!("{}(", hint.symbol), format!(".{}(", hint.symbol)] {
            if let Some(idx) = text.find(&pattern) {
                let candidate = (hint, idx);
                match best {
                    Some((_, best_idx)) if best_idx <= idx => {}
                    _ => best = Some(candidate),
                }
            }
        }
    }
    best
}

fn is_loop_like(kind: &str) -> bool {
    kind.contains("for") || kind.contains("while") || kind.contains("loop")
}

fn first_retry_match<'a>(
    text: &'a str,
    hints: &'a [RetryPolicyHint],
) -> Option<(&'a RetryPolicyHint, usize)> {
    let mut best: Option<(&RetryPolicyHint, usize)> = None;
    for hint in hints {
        if hint.symbol.len() < 2 {
            continue;
        }
        for pattern in [format!("{}(", hint.symbol), format!(".{}(", hint.symbol)] {
            if let Some(idx) = text.find(&pattern) {
                match best {
                    Some((_, best_idx)) if best_idx <= idx => {}
                    _ => best = Some((hint, idx)),
                }
            }
        }
    }
    best
}

fn missing_retry_guards(hint: &RetryPolicyHint) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !hint.has_backoff {
        missing.push("backoff");
    }
    if !hint.has_jitter {
        missing.push("jitter");
    }
    if !hint.has_cap {
        missing.push("cap");
    }
    if !hint.propagates_cancellation {
        missing.push("cancellation");
    }
    if !hint.filters_transient_errors {
        missing.push("transient-error-filtering");
    }
    missing
}

fn first_call_edge_for_symbol<'a>(
    analysis_context: &'a AnalysisContext,
    symbol: &str,
) -> Option<&'a CallGraphEdge> {
    analysis_context.call_graph_edges.iter().find(|edge| {
        edge.callee == symbol
            || edge.callee.ends_with(symbol)
            || format!("{}()", edge.callee) == symbol
    })
}

fn describe_edge(edge: &CallGraphEdge) -> String {
    format!(
        "{} -> {} ({} -> {}, tier={})",
        edge.caller,
        edge.callee,
        edge.source_file,
        edge.target_file.as_deref().unwrap_or("unknown"),
        edge.confidence_tier.as_deref().unwrap_or("medium")
    )
}

pub fn run_rules(context: &RuleContext<'_>) -> Vec<Issue> {
    let rules: Vec<Box<dyn SemanticRule>> = vec![
        Box::new(BlockingInAsyncRule),
        Box::new(BlockingWrapperPropagationRule),
        Box::new(NPlusOneCrossBoundaryRule),
        Box::new(RetryPolicyConsistencyRule),
    ];
    rules
        .into_iter()
        .filter_map(|rule| rule.run(context))
        .collect()
}
