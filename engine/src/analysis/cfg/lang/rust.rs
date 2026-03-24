use tree_sitter::Node;

pub fn blocking_call_tokens() -> &'static [&'static str] {
    &[
        "std::thread::sleep(",
        "std::fs::read(",
        "std::fs::write(",
        "reqwest::blocking::",
    ]
}

pub fn blocking_suggestion() -> &'static str {
    "Use async-compatible crates (tokio::time::sleep, tokio::fs, async reqwest) instead of blocking std APIs in async tasks."
}

pub fn is_async_context(node: Node, source: &str) -> bool {
    (node.kind() == "function_item" || node.kind() == "closure_expression")
        && node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|t| t.trim_start().starts_with("async "))
            .unwrap_or(false)
}
