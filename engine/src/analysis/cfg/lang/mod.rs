mod go;
mod java;
mod javascript;
mod python;
mod ruby;
mod rust;

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
            Self::JavaScript | Self::TypeScript => javascript::blocking_call_tokens(),
            Self::Python => python::blocking_call_tokens(),
            Self::Go => go::blocking_call_tokens(),
            Self::Rust => rust::blocking_call_tokens(),
            Self::Ruby => ruby::blocking_call_tokens(),
            Self::Java => java::blocking_call_tokens(),
        }
    }

    pub fn blocking_suggestion(&self) -> &'static str {
        match self {
            Self::JavaScript | Self::TypeScript => javascript::blocking_suggestion(),
            Self::Python => python::blocking_suggestion(),
            Self::Go => go::blocking_suggestion(),
            Self::Rust => rust::blocking_suggestion(),
            Self::Ruby => ruby::blocking_suggestion(),
            Self::Java => java::blocking_suggestion(),
        }
    }
}

pub fn is_async_context(node: Node, source: &str, language: SupportedLanguage) -> bool {
    match language {
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            javascript::is_async_context(node, source)
        }
        SupportedLanguage::Python => python::is_async_context(node, source),
        SupportedLanguage::Go => go::is_async_context(node, source),
        SupportedLanguage::Rust => rust::is_async_context(node, source),
        SupportedLanguage::Ruby => ruby::is_async_context(node, source),
        SupportedLanguage::Java => java::is_async_context(node, source),
    }
}
