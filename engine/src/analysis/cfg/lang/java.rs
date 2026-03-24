use tree_sitter::Node;

pub fn blocking_call_tokens() -> &'static [&'static str] {
    &["Thread.sleep(", ".executeQuery(", ".executeUpdate(", ".join("]
}

pub fn blocking_suggestion() -> &'static str {
    "Prefer non-blocking/reactive APIs in async flows and avoid blocking waits in CompletableFuture or executor tasks."
}

pub fn is_async_context(node: Node, source: &str) -> bool {
    node.utf8_text(source.as_bytes())
        .ok()
        .map(|t| t.contains("CompletableFuture") || t.contains("@Async"))
        .unwrap_or(false)
}
