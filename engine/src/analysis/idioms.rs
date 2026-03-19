use crate::analysis::Analyzer;
use crate::protocol::{AnalyzeRequest, Issue};

pub struct IdiomaticAnalyzer;

impl Analyzer for IdiomaticAnalyzer {
    fn analyze(&self, request: &AnalyzeRequest) -> Vec<Issue> {
        match request
            .language_id
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "javascript" | "typescript" | "javascriptreact" | "typescriptreact" => {
                analyze_js_ts(&request.code)
            }
            "python" => analyze_python(&request.code),
            "go" => analyze_go(&request.code),
            "rust" => analyze_rust(&request.code),
            "ruby" => analyze_ruby(&request.code),
            "java" => analyze_java(&request.code),
            _ => Vec::new(),
        }
    }
}

fn analyze_js_ts(code: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let has_index_loop = code.contains("for (let i = 0;")
        || code.contains("for(let i = 0;")
        || code.contains("for (var i = 0;");
    let has_array_access = code.contains("[i]");
    let has_push = code.contains(".push(");

    if has_index_loop && has_array_access && has_push {
        issues.push(Issue::new(
            "JavaScript iteration can be more idiomatic",
            vec![
                "Indexed loops with push are verbose for simple transformations.".to_string(),
                "Array helpers improve intent clarity and reduce mutable state.".to_string(),
            ],
            0.87,
            Some("Use map/filter/reduce where possible.".to_string()),
            Some("readability".to_string()),
        ));
    }

    issues
}

fn analyze_python(code: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let uses_range_len = code.contains("range(len(");
    let uses_append = code.contains(".append(");

    if uses_range_len && uses_append {
        issues.push(Issue::new(
            "Python loop can be simplified",
            vec![
                "range(len(...)) with append is often an anti-pattern in Python.".to_string(),
                "Comprehensions are usually faster and easier to read.".to_string(),
            ],
            0.88,
            Some("Use direct iteration or a list comprehension.".to_string()),
            Some("performance".to_string()),
        ));
    }

    issues
}

fn analyze_rust(code: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let clone_calls = code.matches(".clone()").count();

    if clone_calls >= 2 {
        issues.push(Issue::new(
            "Potential over-cloning in Rust",
            vec![
                format!("Detected {clone_calls} clone calls in selected code."),
                "Frequent cloning can add avoidable allocations and copies.".to_string(),
            ],
            0.75,
            Some("Prefer borrowing or iterator adapters when ownership allows.".to_string()),
            Some("performance".to_string()),
        ));
    }

    issues
}

fn analyze_go(code: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let has_range_loop = code.contains("for _,") || code.contains("for _, ") || code.contains("for i := range");
    let has_append = code.contains("append(");

    if has_range_loop && has_append {
        issues.push(Issue::new(
            "Go loop may hide transform intent",
            vec![
                "Range loops with append can be hard to reuse across call sites.".to_string(),
                "Encapsulating transformation logic improves readability and testability.".to_string(),
            ],
            0.79,
            Some("Extract a small helper function for mapping/filtering behavior.".to_string()),
            Some("maintainability".to_string()),
        ));
    }

    issues
}

fn analyze_ruby(code: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let has_each = code.contains(".each do") || code.contains(".each {");
    let has_accumulate = code.contains("<<") || code.contains(".push(");

    if has_each && has_accumulate {
        issues.push(Issue::new(
            "Ruby iteration can use Enumerable idioms",
            vec![
                "Manual accumulation inside each blocks is often verbose Ruby style.".to_string(),
                "Enumerable chains communicate transformation intent more directly.".to_string(),
            ],
            0.84,
            Some("Prefer map/select/reject when transforming collections.".to_string()),
            Some("readability".to_string()),
        ));
    }

    issues
}

fn analyze_java(code: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let has_for_loop = code.contains("for (") || code.contains("for(");
    let mutating_collection = code.contains(".add(");

    if has_for_loop && mutating_collection {
        issues.push(Issue::new(
            "Java collection transform is imperative",
            vec![
                "Loop-based add patterns can obscure simple transformation pipelines.".to_string(),
                "Stream API often reduces boilerplate for pure data transformations.".to_string(),
            ],
            0.82,
            Some("Use stream().map(...).filter(...).toList() when it improves readability.".to_string()),
            Some("readability".to_string()),
        ));
    }

    issues
}
