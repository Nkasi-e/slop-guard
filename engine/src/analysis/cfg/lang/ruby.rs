use tree_sitter::Node;

pub fn blocking_call_tokens() -> &'static [&'static str] {
    &["sleep(", "Net::HTTP.get(", "Open3.capture3(", "File.read("]
}

pub fn blocking_suggestion() -> &'static str {
    "Prefer event-loop friendly async IO patterns and avoid direct blocking file/network calls in async/reactor flows."
}

pub fn is_async_context(node: Node, source: &str) -> bool {
    node.utf8_text(source.as_bytes())
        .ok()
        .map(|t| t.contains("async") || t.contains("Concurrent::Promise"))
        .unwrap_or(false)
}
