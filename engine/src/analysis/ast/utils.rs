use tree_sitter::Node;

use super::language::SupportedLanguage;

pub fn is_loop_kind(kind: &str) -> bool {
    kind.contains("for") || kind.contains("while") || kind == "loop_expression"
}

pub fn is_loop_node(node: Node, source: &str, language: SupportedLanguage) -> bool {
    if is_loop_kind(node.kind()) {
        return true;
    }
    if matches!(language, SupportedLanguage::Ruby) && node.kind() == "block" {
        return node_text(node, source).map(|t| t.contains(".each")).unwrap_or(false);
    }
    false
}

pub fn is_control_kind(kind: &str) -> bool {
    is_loop_kind(kind)
        || kind.contains("if")
        || kind.contains("switch")
        || kind.contains("match")
        || kind.contains("case")
        || kind.contains("try")
        || kind.contains("catch")
        || kind.contains("except")
}

pub fn is_block_kind(kind: &str) -> bool {
    kind.contains("block") || kind.contains("body")
}

pub fn is_assignment_like_statement(kind: &str) -> bool {
    kind.contains("declaration")
        || kind.contains("assignment")
        || kind == "assignment"
        || kind == "expression_statement"
}

pub fn contains_any(input: &str, tokens: &[&str]) -> bool {
    tokens.iter().any(|token| input.contains(token))
}

pub fn node_text(node: Node, source: &str) -> Option<String> {
    node.utf8_text(source.as_bytes()).ok().map(|v| v.to_string())
}

pub fn named_children<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}
