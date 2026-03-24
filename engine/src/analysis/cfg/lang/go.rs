use tree_sitter::Node;

pub fn blocking_call_tokens() -> &'static [&'static str] {
    &["time.Sleep(", "(*http.Client).Do(", "sql.Open(", "os.ReadFile("]
}

pub fn blocking_suggestion() -> &'static str {
    "Avoid long blocking operations in goroutines that serve latency-sensitive paths; use context/timeouts and async patterns."
}

pub fn is_async_context(node: Node, source: &str) -> bool {
    node.kind() == "go_statement"
        || node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|t| t.contains("go func("))
            .unwrap_or(false)
}
