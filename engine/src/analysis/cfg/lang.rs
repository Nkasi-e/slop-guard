use tree_sitter::{Language, Node};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SupportedLanguage {
    JavaScript,
    TypeScript,
    Python,
    Go,
    Rust,
    Ruby,
    Java,
}

impl SupportedLanguage {
    pub fn from_language_id(language_id: Option<&str>) -> Option<Self> {
        let id = language_id.unwrap_or_default().to_ascii_lowercase();
        match id.as_str() {
            "javascript" | "javascriptreact" => Some(Self::JavaScript),
            "typescript" | "typescriptreact" => Some(Self::TypeScript),
            "python" => Some(Self::Python),
            "go" => Some(Self::Go),
            "rust" => Some(Self::Rust),
            "ruby" => Some(Self::Ruby),
            "java" => Some(Self::Java),
            _ => None,
        }
    }

    pub fn tree_sitter_language(&self) -> Option<Language> {
        match self {
            Self::JavaScript => Some(tree_sitter_javascript::language()),
            Self::TypeScript => Some(tree_sitter_typescript::language_typescript()),
            Self::Python => Some(tree_sitter_python::language()),
            Self::Go => Some(tree_sitter_go::language()),
            Self::Rust => Some(tree_sitter_rust::language()),
            Self::Ruby => Some(tree_sitter_ruby::language()),
            Self::Java => Some(tree_sitter_java::language()),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Rust => "Rust",
            Self::Ruby => "Ruby",
            Self::Java => "Java",
        }
    }

    pub fn blocking_call_tokens(&self) -> &'static [&'static str] {
        match self {
            Self::JavaScript | Self::TypeScript => &[
                "readFileSync(",
                "writeFileSync(",
                "execSync(",
                "spawnSync(",
                "pbkdf2Sync(",
                "bcrypt.hashSync(",
            ],
            Self::Python => &[
                "time.sleep(",
                "requests.",
                "subprocess.run(",
                "urllib.request.urlopen(",
            ],
            Self::Go => &["time.Sleep(", "(*http.Client).Do(", "sql.Open(", "os.ReadFile("],
            Self::Rust => &[
                "std::thread::sleep(",
                "std::fs::read(",
                "std::fs::write(",
                "reqwest::blocking::",
            ],
            Self::Ruby => &["sleep(", "Net::HTTP.get(", "Open3.capture3(", "File.read("],
            Self::Java => &["Thread.sleep(", ".executeQuery(", ".executeUpdate(", ".join("],
        }
    }

    pub fn blocking_suggestion(&self) -> &'static str {
        match self {
            Self::JavaScript | Self::TypeScript => {
                "Prefer non-blocking APIs (fs.promises, async child_process, async crypto) inside async paths."
            }
            Self::Python => {
                "Use async-native libraries (aiohttp/httpx asyncio) and avoid blocking sleep/network APIs in async code."
            }
            Self::Go => {
                "Avoid long blocking operations in goroutines that serve latency-sensitive paths; use context/timeouts and async patterns."
            }
            Self::Rust => {
                "Use async-compatible crates (tokio::time::sleep, tokio::fs, async reqwest) instead of blocking std APIs in async tasks."
            }
            Self::Ruby => {
                "Prefer event-loop friendly async IO patterns and avoid direct blocking file/network calls in async/reactor flows."
            }
            Self::Java => {
                "Prefer non-blocking/reactive APIs in async flows and avoid blocking waits in CompletableFuture or executor tasks."
            }
        }
    }
}

pub fn is_async_context(node: Node, source: &str, language: SupportedLanguage) -> bool {
    match language {
        SupportedLanguage::Python => node.kind() == "async_function_definition",
        SupportedLanguage::Go => {
            node.kind() == "go_statement"
                || node
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|t| t.contains("go func("))
                    .unwrap_or(false)
        }
        SupportedLanguage::Rust => {
            (node.kind() == "function_item" || node.kind() == "closure_expression")
                && node
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|t| t.trim_start().starts_with("async "))
                    .unwrap_or(false)
        }
        SupportedLanguage::Ruby => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|t| t.contains("async") || t.contains("Concurrent::Promise"))
            .unwrap_or(false),
        SupportedLanguage::Java => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|t| t.contains("CompletableFuture") || t.contains("@Async"))
            .unwrap_or(false),
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            (node.kind().contains("function") || node.kind().contains("method"))
                && node
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|t| t.trim_start().starts_with("async "))
                    .unwrap_or(false)
        }
    }
}
