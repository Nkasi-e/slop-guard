use tree_sitter::Language;

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

    pub fn loop_accumulator_tokens(&self) -> &'static [&'static str] {
        match self {
            Self::JavaScript | Self::TypeScript | Self::Rust => &[".push("],
            Self::Python => &[".append("],
            Self::Go => &["append("],
            Self::Ruby => &["<<", ".push("],
            Self::Java => &[".add("],
        }
    }

    pub fn manual_iteration_suggestion(&self) -> &'static str {
        match self {
            Self::JavaScript | Self::TypeScript => {
                "Prefer map/filter/reduce for collection transforms when side effects are unnecessary."
            }
            Self::Python => {
                "Prefer list/dict comprehensions for collection transforms when readability improves."
            }
            Self::Go => "Extract transform logic into reusable helpers to avoid repetitive loop plumbing.",
            Self::Rust => "Prefer iterator chains (map/filter/collect) when ownership remains clear.",
            Self::Ruby => "Prefer Enumerable methods like map/select/reject over manual accumulation.",
            Self::Java => "Prefer Stream API for pure transformations when it reduces boilerplate.",
        }
    }
}
