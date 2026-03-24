use tree_sitter::Node;

pub fn blocking_call_tokens() -> &'static [&'static str] {
    &[
        "time.sleep(",
        "requests.",
        "subprocess.run(",
        "urllib.request.urlopen(",
    ]
}

pub fn blocking_suggestion() -> &'static str {
    "Use async-native libraries (aiohttp/httpx asyncio) and avoid blocking sleep/network APIs in async code."
}

pub fn is_async_context(node: Node, source: &str) -> bool {
    if node.kind() == "async_function_definition" {
        return true;
    }
    node.kind() == "function_definition"
        && node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|t| t.trim_start().starts_with("async def "))
            .unwrap_or(false)
}
