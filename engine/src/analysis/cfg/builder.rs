use tree_sitter::Node;

use super::ir::{BasicBlock, Edge, EdgeKind, FunctionCfg};

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
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push(child);
        }
    }

    FunctionCfg { blocks, edges }
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
