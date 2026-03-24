use tree_sitter::Node;

use super::ir::{BasicBlock, Edge, EdgeKind, FunctionCfg, SymbolTable};

pub fn build_cfg(root: Node) -> FunctionCfg {
    let mut blocks: Vec<BasicBlock> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut stack = vec![root];
    let mut prev_block_id: Option<usize> = None;

    while let Some(node) = stack.pop() {
        if is_cfg_node(node.kind()) {
            let id = blocks.len();
            let start = node.start_position().row as usize;
            let end = node.end_position().row as usize;
            blocks.push(BasicBlock {
                id,
                start_line: start,
                end_line: end.saturating_sub(1).max(start),
            });
            if let Some(prev) = prev_block_id {
                edges.push(Edge {
                    from: prev,
                    to: id,
                    kind: EdgeKind::Fallthrough,
                });
            }
            prev_block_id = Some(id);

            if is_branch_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::BranchTrue,
                });
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::BranchFalse,
                });
            }

            if is_loop_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::LoopBack,
                });
            }
            if is_try_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::TryEdge,
                });
            }
            if is_catch_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::CatchEdge,
                });
            }
            if is_finally_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::FinallyEdge,
                });
            }
            if is_return_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::ReturnEdge,
                });
            }
            if is_throw_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::ThrowEdge,
                });
            }
            if is_break_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::Break,
                });
            }
            if is_continue_node(node.kind()) {
                edges.push(Edge {
                    from: id,
                    to: id,
                    kind: EdgeKind::Continue,
                });
            }
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push(child);
        }
    }

    FunctionCfg { blocks, edges }
}

pub fn extract_symbol_table(root: Node, source: &str) -> SymbolTable {
    let mut symbols = SymbolTable::default();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if is_function_definition_node(kind) {
            if let Some(name) = best_effort_symbol_name(node, source) {
                symbols.function_defs.push(name);
            }
        } else if is_call_node(kind) {
            if let Some(name) = best_effort_symbol_name(node, source) {
                symbols.call_sites.push(name);
            }
        } else if is_identifier_node(kind) {
            if let Some(name) = best_effort_symbol_name(node, source) {
                symbols.identifiers.push(name);
            }
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push(child);
        }
    }
    symbols
}

fn is_cfg_node(kind: &str) -> bool {
    is_branch_node(kind)
        || kind.contains("statement")
        || kind.contains("expression")
        || kind.contains("declaration")
}

fn is_branch_node(kind: &str) -> bool {
    kind.contains("if")
        || kind.contains("for")
        || kind.contains("while")
        || kind.contains("switch")
        || kind.contains("match")
        || kind.contains("case")
        || kind.contains("catch")
}

fn is_loop_node(kind: &str) -> bool {
    kind.contains("for") || kind.contains("while") || kind == "loop_expression"
}

fn is_try_node(kind: &str) -> bool {
    kind.contains("try")
}

fn is_catch_node(kind: &str) -> bool {
    kind.contains("catch") || kind.contains("except")
}

fn is_finally_node(kind: &str) -> bool {
    kind.contains("finally")
}

fn is_return_node(kind: &str) -> bool {
    kind.contains("return")
}

fn is_throw_node(kind: &str) -> bool {
    kind.contains("throw") || kind.contains("raise")
}

fn is_break_node(kind: &str) -> bool {
    kind.contains("break")
}

fn is_continue_node(kind: &str) -> bool {
    kind.contains("continue")
}

fn is_function_definition_node(kind: &str) -> bool {
    kind.contains("function")
        || kind.contains("method")
        || kind == "function_item"
        || kind == "function_definition"
}

fn is_call_node(kind: &str) -> bool {
    kind.contains("call")
}

fn is_identifier_node(kind: &str) -> bool {
    kind == "identifier" || kind.contains("name")
}

fn best_effort_symbol_name(node: Node, source: &str) -> Option<String> {
    let text = node.utf8_text(source.as_bytes()).ok()?.trim();
    if text.is_empty() {
        return None;
    }
    let token = text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .find(|part| !part.is_empty())?;
    Some(token.to_string())
}
