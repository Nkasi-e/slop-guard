use crate::protocol::Issue;
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
}

pub trait SemanticRule {
    fn run(&self, context: &RuleContext<'_>) -> Option<Issue>;
}

pub struct BlockingInAsyncRule;

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

pub fn run_rules(context: &RuleContext<'_>) -> Vec<Issue> {
    let rules: Vec<Box<dyn SemanticRule>> = vec![Box::new(BlockingInAsyncRule)];
    rules
        .into_iter()
        .filter_map(|rule| rule.run(context))
        .collect()
}
