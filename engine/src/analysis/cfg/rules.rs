use crate::protocol::Issue;
use tree_sitter::Node;

use super::ir::FunctionCfg;
use super::lang::{is_async_context, SupportedLanguage};
use super::util::{first_match, line_for_byte, ranges_overlap, snippet_around_line};

pub fn detect_blocking_in_async(
    root: Node,
    source: &str,
    language: SupportedLanguage,
    cfg: &FunctionCfg,
) -> Option<Issue> {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if is_async_context(node, source, language) {
            let Some(text) = node.utf8_text(source.as_bytes()).ok() else {
                continue;
            };
            if let Some((token, rel_byte)) = first_match(text, language.blocking_call_tokens()) {
                let abs_byte = node.start_byte().saturating_add(rel_byte);
                let line = line_for_byte(source, abs_byte);
                let (snippet, start_line, end_line) = snippet_around_line(source, line, 4);
                let cfg_block_count = cfg
                    .blocks
                    .iter()
                    .filter(|b| ranges_overlap(start_line, end_line, b.start_line, b.end_line))
                    .count();
                let cfg_edge_count = cfg
                    .edges
                    .iter()
                    .filter(|e| {
                        e.from <= e.to
                            || matches!(
                                e.kind,
                                super::ir::EdgeKind::BranchTrue | super::ir::EdgeKind::BranchFalse
                            )
                    })
                    .count();
                let cfg_block_id_sum: usize = cfg.blocks.iter().map(|b| b.id).sum();

                return Some(
                    Issue::new(
                        "Blocking call in async context",
                        vec![
                            format!(
                                "Potential blocking call `{}` detected inside {} async execution context.",
                                token,
                                language.display_name()
                            ),
                            format!(
                                "Aggressive CFG pass marked {} overlapping block(s), {} edge(s), id-sum {} in this region.",
                                cfg_block_count, cfg_edge_count, cfg_block_id_sum
                            ),
                            "Blocking operations can stall event loops/executors and degrade tail latency."
                                .to_string(),
                        ],
                        0.72,
                        Some(language.blocking_suggestion().to_string()),
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
