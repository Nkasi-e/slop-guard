use tree_sitter::Node;

pub fn blocking_call_tokens() -> &'static [&'static str] {
    &[
        "readFileSync(",
        "writeFileSync(",
        "execSync(",
        "spawnSync(",
        "pbkdf2Sync(",
        "bcrypt.hashSync(",
    ]
}

pub fn blocking_suggestion() -> &'static str {
    "Prefer non-blocking APIs (fs.promises, async child_process, async crypto) inside async paths."
}

pub fn is_async_context(node: Node, source: &str) -> bool {
    (node.kind().contains("function") || node.kind().contains("method"))
        && node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|t| t.trim_start().starts_with("async "))
            .unwrap_or(false)
}
