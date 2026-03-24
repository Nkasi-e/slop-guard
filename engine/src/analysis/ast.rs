use crate::analysis::Analyzer;
use crate::protocol::{AlgorithmAnalysis, AnalyzeRequest, Issue};
use std::cell::RefCell;
use std::collections::HashMap;
use tree_sitter::{InputEdit, Language, Node, Parser, Point, Tree};

const DEFAULT_CACHE_KEY: &str = "__default__";
const MAX_PARSE_CACHE_ENTRIES: usize = 64;

struct CachedParse {
    language: SupportedLanguage,
    code: String,
    tree: Tree,
}

thread_local! {
    static AST_PARSE_CACHE: RefCell<HashMap<String, CachedParse>> = RefCell::new(HashMap::new());
}

pub struct AstAnalyzer;

impl Analyzer for AstAnalyzer {
    fn analyze(&self, request: &AnalyzeRequest) -> Vec<Issue> {
        let supported_language = match SupportedLanguage::from_language_id(request.language_id.as_deref()) {
            Some(language) => language,
            None => return Vec::new(),
        };
        let language = match supported_language.tree_sitter_language() {
            Some(language) => language,
            None => return Vec::new(),
        };

        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }

        let cache_key = request
            .document_key
            .as_deref()
            .unwrap_or(DEFAULT_CACHE_KEY)
            .to_string();
        let previous = AST_PARSE_CACHE.with(|cache| {
            cache.borrow().get(&cache_key).and_then(|entry| {
                if entry.language == supported_language {
                    Some((entry.code.clone(), entry.tree.clone()))
                } else {
                    None
                }
            })
        });

        let next_tree = if let Some((previous_code, mut previous_tree)) = previous {
            if previous_code != request.code {
                if let Some(edit) = compute_single_edit(&previous_code, &request.code) {
                    previous_tree.edit(&edit);
                }
            }
            parser.parse(&request.code, Some(&previous_tree))
        } else {
            parser.parse(&request.code, None)
        };

        let Some(tree) = next_tree else {
            return Vec::new();
        };

        AST_PARSE_CACHE.with(|cache| {
            let mut map = cache.borrow_mut();
            if map.len() >= MAX_PARSE_CACHE_ENTRIES && !map.contains_key(&cache_key) {
                if let Some(oldest_key) = map.keys().next().cloned() {
                    map.remove(&oldest_key);
                }
            }
            map.insert(
                cache_key,
                CachedParse {
                    language: supported_language,
                    code: request.code.clone(),
                    tree: tree.clone(),
                },
            );
        });

        let root = tree.root_node();
        let mut issues = Vec::new();

        if let Some((snippet, start_line, end_line)) = detect_manual_iteration_ast(
            root,
            &request.code,
            supported_language,
        ) {
            issues.push(
                Issue::new(
                    "Manual iteration detected",
                    vec![
                        format!(
                            "AST detected loop-based collection building in {}.",
                            supported_language.display_name()
                        ),
                        "Declarative transforms are usually easier to review and maintain.".to_string(),
                    ],
                    0.96,
                    Some(supported_language.manual_iteration_suggestion().to_string()),
                    Some("ai-slop".to_string()),
                )
                .with_snippet_evidence(snippet, start_line, end_line),
            );
        }

        if let Some((snippet, start_line, end_line)) = detect_redundant_assign_then_return_ast(
            root,
            &request.code,
            supported_language,
        ) {
            issues.push(
                Issue::new(
                    "Redundant variable before return",
                    vec![
                        "AST found a local binding immediately returned.".to_string(),
                        "This is often a temporary variable that can be removed safely.".to_string(),
                    ],
                    0.95,
                    Some("Return the expression directly unless the binding improves clarity.".to_string()),
                    Some("readability".to_string()),
                )
                .with_snippet_evidence(snippet, start_line, end_line),
            );
        }

        let max_depth = compute_control_nesting_depth(root);
        if max_depth > 3 {
            let mut issue = Issue::new(
                "Deep nesting detected",
                vec![
                    format!("AST observed control-flow nesting depth of {max_depth}."),
                    "High nesting raises cognitive load and makes edge cases easier to miss."
                        .to_string(),
                ],
                0.90,
                Some("Use guard clauses or helper functions to flatten nested branches.".to_string()),
                Some("maintainability".to_string()),
            );

            // Best-effort evidence: show a snippet around the first
            // location that reaches the deepest nesting in the raw text.
            if let Some((snippet, start_line, end_line)) =
                deep_nesting_evidence_snippet(&request.code, supported_language)
            {
                issue = issue.with_snippet_evidence(snippet, start_line, end_line);
            }

            issues.push(issue);
        }

        if let Some(algorithm_issue) = analyze_algorithmic_complexity(root, &request.code, supported_language) {
            issues.push(algorithm_issue);
        }

        issues
    }
}

fn compute_single_edit(old_source: &str, new_source: &str) -> Option<InputEdit> {
    if old_source == new_source {
        return None;
    }

    let old_bytes = old_source.as_bytes();
    let new_bytes = new_source.as_bytes();

    let mut prefix = 0usize;
    let prefix_limit = old_bytes.len().min(new_bytes.len());
    while prefix < prefix_limit && old_bytes[prefix] == new_bytes[prefix] {
        prefix += 1;
    }

    let old_remaining = old_bytes.len().saturating_sub(prefix);
    let new_remaining = new_bytes.len().saturating_sub(prefix);
    let mut suffix = 0usize;
    let suffix_limit = old_remaining.min(new_remaining);
    while suffix < suffix_limit
        && old_bytes[old_bytes.len() - 1 - suffix] == new_bytes[new_bytes.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let old_end_byte = old_bytes.len().saturating_sub(suffix);
    let new_end_byte = new_bytes.len().saturating_sub(suffix);

    Some(InputEdit {
        start_byte: prefix,
        old_end_byte,
        new_end_byte,
        start_position: point_for_byte(old_source, prefix),
        old_end_position: point_for_byte(old_source, old_end_byte),
        new_end_position: point_for_byte(new_source, new_end_byte),
    })
}

fn point_for_byte(source: &str, byte_offset: usize) -> Point {
    let mut row = 0usize;
    let mut column = 0usize;
    let clamped = byte_offset.min(source.len());
    for &b in source.as_bytes().iter().take(clamped) {
        if b == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    Point::new(row, column)
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum SupportedLanguage {
    JavaScript,
    TypeScript,
    Python,
    Go,
    Rust,
    Ruby,
    Java,
}

impl SupportedLanguage {
    fn from_language_id(language_id: Option<&str>) -> Option<Self> {
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

    fn tree_sitter_language(&self) -> Option<Language> {
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

    fn display_name(&self) -> &'static str {
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

    fn loop_accumulator_tokens(&self) -> &'static [&'static str] {
        match self {
            Self::JavaScript | Self::TypeScript | Self::Rust => &[".push("],
            Self::Python => &[".append("],
            Self::Go => &["append("],
            Self::Ruby => &["<<", ".push("],
            Self::Java => &[".add("],
        }
    }

    fn manual_iteration_suggestion(&self) -> &'static str {
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

fn detect_manual_iteration_ast(
    root: Node,
    source: &str,
    language: SupportedLanguage,
) -> Option<(String, usize, usize)> {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if is_loop_node(node, source, language) && loop_contains_accumulator(node, source, language) {
            let start_pos = node.start_position();
            let end_pos = node.end_position();
            let start_row = start_pos.row;
            let end_row = end_pos.row;

            let lines: Vec<&str> = source.lines().collect();
            if lines.is_empty() {
                return None;
            }

            let start_line = start_row as usize;
            // end_position is usually exclusive; treat end-1 as inclusive.
            let mut end_line = if end_row == 0 { 0 } else { (end_row as usize).saturating_sub(1) };

            // Clamp to source bounds.
            let max_idx = lines.len().saturating_sub(1);
            let start_line = start_line.min(max_idx);
            end_line = end_line.min(max_idx);
            if end_line < start_line {
                end_line = start_line;
            }

            let snippet = lines[start_line..=end_line]
                .iter()
                .map(|l: &&str| (*l).trim_end())
                .collect::<Vec<&str>>()
                .join("\n");

            return Some((snippet, start_line, end_line));
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push(child);
        }
    }

    None
}

fn loop_contains_accumulator(loop_node: Node, source: &str, language: SupportedLanguage) -> bool {
    let Some(text) = node_text(loop_node, source) else {
        return false;
    };
    language
        .loop_accumulator_tokens()
        .iter()
        .any(|token| text.contains(token))
}

fn detect_redundant_assign_then_return_ast(
    root: Node,
    source: &str,
    language: SupportedLanguage,
) -> Option<(String, usize, usize)> {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if is_block_kind(node.kind()) {
            let statements = named_children(node);
            for window in statements.windows(2) {
                let assignment_like = window[0];
                let ret = window[1];
                if !is_assignment_like_statement(assignment_like.kind()) {
                    continue;
                }
                if ret.kind() != "return_statement" {
                    continue;
                }

                let Some(name) = assigned_identifier(assignment_like, source, language) else {
                    continue;
                };
                let Some(returned) = returned_identifier(ret, source) else {
                    continue;
                };

                if name == returned {
                    let lines: Vec<&str> = source.lines().collect();
                    if lines.is_empty() {
                        return None;
                    }

                    let a_start_pos = assignment_like.start_position();
                    let a_end_pos = assignment_like.end_position();
                    let r_start_pos = ret.start_position();
                    let r_end_pos = ret.end_position();

                    let a_start = a_start_pos.row;
                    let a_end = a_end_pos.row;
                    let r_start = r_start_pos.row;
                    let r_end = r_end_pos.row;

                    let max_idx = lines.len().saturating_sub(1);

                    let start_line = (a_start.min(r_start) as usize).min(max_idx);
                    let mut end_line = if a_end == 0 && r_end == 0 {
                        0
                    } else {
                        (a_end.max(r_end) as usize).saturating_sub(1)
                    };
                    end_line = end_line.min(max_idx);
                    if end_line < start_line {
                        end_line = start_line;
                    }

                    let snippet = lines[start_line..=end_line]
                        .iter()
                        .map(|l: &&str| (*l).trim_end())
                        .collect::<Vec<&str>>()
                        .join("\n");

                    return Some((snippet, start_line, end_line));
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push(child);
        }
    }

    None
}

fn assigned_identifier(
    assignment_like: Node,
    source: &str,
    language: SupportedLanguage,
) -> Option<String> {
    let text = node_text(assignment_like, source)?;
    parse_assigned_identifier(&text, language)
}

fn returned_identifier(return_statement: Node, source: &str) -> Option<String> {
    let text = node_text(return_statement, source)?;
    parse_returned_identifier(&text)
}

fn parse_assigned_identifier(text: &str, language: SupportedLanguage) -> Option<String> {
    let statement = text.trim().trim_end_matches(';').trim();
    if statement.starts_with("return ") || !statement.contains('=') {
        return None;
    }

    let split_token = if statement.contains(":=") { ":=" } else { "=" };
    let left = statement.split(split_token).next()?.trim();

    match language {
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            parse_name_after_prefixes(left, &["const ", "let ", "var "])
        }
        SupportedLanguage::Rust => parse_name_after_prefixes(left, &["let mut ", "let "]),
        SupportedLanguage::Go => {
            parse_name_after_prefixes(left, &["var "]).or_else(|| last_identifier(left))
        }
        SupportedLanguage::Python | SupportedLanguage::Ruby => last_identifier(left),
        SupportedLanguage::Java => {
            parse_name_after_prefixes(
                left,
                &[
                    "final ", "var ", "int ", "long ", "double ", "float ", "boolean ", "String ",
                ],
            )
            .or_else(|| last_identifier(left))
        }
    }
}

fn parse_name_after_prefixes(input: &str, prefixes: &[&str]) -> Option<String> {
    for prefix in prefixes {
        if let Some(rest) = input.strip_prefix(prefix) {
            return last_identifier(rest);
        }
    }
    None
}

fn parse_returned_identifier(text: &str) -> Option<String> {
    let statement = text.trim().trim_end_matches(';').trim();
    let rest = statement.strip_prefix("return ")?.trim();
    if rest.is_empty() {
        return None;
    }
    last_identifier(rest)
}

fn last_identifier(input: &str) -> Option<String> {
    input
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|part| !part.is_empty())
        .last()
        .map(|s| s.to_string())
}

fn compute_control_nesting_depth(root: Node) -> usize {
    let mut max_depth = 0usize;
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        let next_depth = if is_control_kind(node.kind()) {
            depth + 1
        } else {
            depth
        };

        max_depth = max_depth.max(next_depth);

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push((child, next_depth));
        }
    }

    max_depth
}

fn compute_loop_nesting_depth_for_language(
    root: Node,
    source: &str,
    language: SupportedLanguage,
) -> usize {
    let mut max_depth = 0usize;
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        let next_depth = if is_loop_node(node, source, language) {
            depth + 1
        } else {
            depth
        };
        max_depth = max_depth.max(next_depth);

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push((child, next_depth));
        }
    }

    max_depth
}

fn analyze_algorithmic_complexity(
    root: Node,
    source: &str,
    language: SupportedLanguage,
) -> Option<Issue> {
    let mut loop_depth = compute_loop_nesting_depth_for_language(root, source, language);
    if matches!(language, SupportedLanguage::Ruby) && source.matches(".each").count() >= 2 {
        loop_depth = loop_depth.max(2);
    }
    if loop_depth < 2 {
        return None;
    }

    let loop_depth_u32 = loop_depth.min(10) as u32;
    let time_complexity = format!("O(n^{loop_depth_u32})");
    let space_complexity = if contains_any(source, language.loop_accumulator_tokens()) {
        "O(n)".to_string()
    } else {
        "O(1)".to_string()
    };

    let mut trade_offs = vec![
        "Using a hash-based index can reduce repeated lookups but increases memory usage."
            .to_string(),
        "Pre-computing maps can improve runtime while making code less linear to read."
            .to_string(),
    ];
    if matches!(language, SupportedLanguage::Rust) {
        trade_offs.push(
            "In Rust, map-based optimizations may require ownership/borrowing refactors."
                .to_string(),
        );
    }

    let trade_off_summary = format!(
        "Trade-off: spend O(n) memory on a hash map or sorted index to avoid repeated O(n) inner scans — net often O(n) vs {} as data grows.",
        time_complexity
    );

    let mut issue = Issue::new(
            "Algorithmic complexity hotspot",
            vec![
                format!("AST detected nested loop depth of {loop_depth}."),
                "Nested iteration often becomes the dominant runtime cost on large inputs."
                    .to_string(),
            ],
            0.86,
            Some(
                "Where lookups are repeated, build an index/hash map to approach O(n) overall."
                    .to_string(),
            ),
            Some("performance".to_string()),
        )
        .with_algorithm_analysis(AlgorithmAnalysis {
            time_complexity,
            space_complexity,
            suggested_time_complexity: Some(
                "O(n) typical (single pass + O(1) lookups per element via map/set/index)".to_string(),
            ),
            suggested_space_complexity: Some(
                "O(n) auxiliary if you store an index/map (often still better than O(n^k) time)".to_string(),
            ),
            trade_off_summary: Some(trade_off_summary),
            trade_offs,
            optimization_hint: Some(
                "True O(1) for whole-transform workloads is usually not possible; target O(1) lookups via indexing and O(n) total passes."
                    .to_string(),
            ),
        });

    // Best-effort evidence: show the first nested-loop-ish region in the raw text.
    if let Some((snippet, start_line, end_line)) = nested_loop_evidence_snippet(source, language)
    {
        issue = issue.with_snippet_evidence(snippet, start_line, end_line);
    }

    Some(issue)
}

fn deep_nesting_evidence_snippet(
    source: &str,
    language: SupportedLanguage,
) -> Option<(String, usize, usize)> {
    // For curly-brace languages, use brace depth as evidence.
    // For python/ruby, use indentation/control keywords.
    if matches!(language, SupportedLanguage::JavaScript | SupportedLanguage::TypeScript | SupportedLanguage::Go | SupportedLanguage::Rust | SupportedLanguage::Java)
    {
        let mut depth = 0usize;
        let mut max_depth = 0usize;
        let mut max_line_idx: Option<usize> = None;
        let lines: Vec<&str> = source.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                    if depth > max_depth {
                        max_depth = depth;
                        max_line_idx = Some(idx);
                    }
                } else if ch == '}' && depth > 0 {
                    depth -= 1;
                }
            }
        }
        let max_line_idx = max_line_idx?;
        let start = max_line_idx.saturating_sub(4);
        let end = (max_line_idx + 4).min(lines.len().saturating_sub(1));
        let snippet = lines[start..=end]
            .iter()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        return Some((snippet, start, end));
    }

    if matches!(language, SupportedLanguage::Python) {
        let mut stack: Vec<usize> = Vec::new();
        let mut max_indent_depth = 0usize;
        let mut max_line_idx: Option<usize> = None;
        let lines: Vec<&str> = source.lines().collect();
        for (idx, raw) in lines.iter().enumerate() {
            let trimmed = raw.trim_end();
            if trimmed.trim_start().is_empty() || trimmed.trim_start().starts_with('#') {
                continue;
            }
            let indent = raw.len() - raw.trim_start().len();
            while let Some(last) = stack.last().copied() {
                if indent <= last {
                    stack.pop();
                } else {
                    break;
                }
            }

            let t = trimmed.trim_start();
            let keywords = ["if ", "for ", "while ", "try:", "except", "with ", "elif "];
            let starts_control = keywords.iter().any(|kw| t.starts_with(kw));
            if starts_control {
                stack.push(indent);
                if stack.len() > max_indent_depth {
                    max_indent_depth = stack.len();
                    max_line_idx = Some(idx);
                }
            }
        }
        let max_line_idx = max_line_idx?;
        let start = max_line_idx.saturating_sub(4);
        let end = (max_line_idx + 4).min(lines.len().saturating_sub(1));
        let snippet = lines[start..=end]
            .iter()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        return Some((snippet, start, end));
    }

    if matches!(language, SupportedLanguage::Ruby) {
        let mut depth = 0usize;
        let mut max_depth = 0usize;
        let mut max_line_idx: Option<usize> = None;
        let lines: Vec<&str> = source.lines().collect();
        for (idx, raw) in lines.iter().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "end" {
                depth = depth.saturating_sub(1);
                continue;
            }
            let starts_block =
                line.starts_with("if ") ||
                line.starts_with("unless ") ||
                line.starts_with("while ") ||
                line.starts_with("for ") ||
                line.ends_with(" do") ||
                line.contains(" do |") ||
                line.starts_with("begin") ||
                line.starts_with("case ");
            if starts_block {
                depth += 1;
                if depth > max_depth {
                    max_depth = depth;
                    max_line_idx = Some(idx);
                }
            }
        }
        let max_line_idx = max_line_idx?;
        let start = max_line_idx.saturating_sub(4);
        let end = (max_line_idx + 4).min(lines.len().saturating_sub(1));
        let snippet = lines[start..=end]
            .iter()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        return Some((snippet, start, end));
    }

    None
}

fn nested_loop_evidence_snippet(
    source: &str,
    language: SupportedLanguage,
) -> Option<(String, usize, usize)> {
    let lines: Vec<&str> = source.lines().collect();

    let loop_markers: &'static [&'static str] = match language {
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => &["for ", "for(", "while ", "while("],
        SupportedLanguage::Python => &["for ", "while "],
        SupportedLanguage::Go => &["for ", ":= range", "range "],
        SupportedLanguage::Rust => &["for ", "while "],
        SupportedLanguage::Ruby => &[".each", "each do", "while ", "for "],
        SupportedLanguage::Java => &["for ", "for(", "while ", "while("],
    };

    let mut found: Vec<usize> = Vec::new();
    for (i, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        if loop_markers.iter().any(|m| t.contains(m)) {
            found.push(i);
            if found.len() >= 3 {
                break;
            }
        }
    }

    if found.is_empty() {
        return None;
    }

    // Wider window: show preceding/potentially-adjacent loops and conditionals
    // so developers see why it becomes an O(n^k) hotspot.
    let start = found[0].saturating_sub(4);
    let end = (found[found.len() - 1] + 4).min(lines.len().saturating_sub(1));
    let snippet = lines[start..=end]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    Some((snippet, start, end))
}

fn is_loop_kind(kind: &str) -> bool {
    kind.contains("for") || kind.contains("while") || kind == "loop_expression"
}

fn is_loop_node(node: Node, source: &str, language: SupportedLanguage) -> bool {
    if is_loop_kind(node.kind()) {
        return true;
    }
    if matches!(language, SupportedLanguage::Ruby) && node.kind() == "block" {
        return node_text(node, source).map(|t| t.contains(".each")).unwrap_or(false);
    }
    false
}

fn is_control_kind(kind: &str) -> bool {
    is_loop_kind(kind)
        || kind.contains("if")
        || kind.contains("switch")
        || kind.contains("match")
        || kind.contains("case")
        || kind.contains("try")
        || kind.contains("catch")
        || kind.contains("except")
}

fn is_block_kind(kind: &str) -> bool {
    kind.contains("block") || kind.contains("body")
}

fn is_assignment_like_statement(kind: &str) -> bool {
    kind.contains("declaration")
        || kind.contains("assignment")
        || kind == "assignment"
        || kind == "expression_statement"
}

fn contains_any(input: &str, tokens: &[&str]) -> bool {
    tokens.iter().any(|token| input.contains(token))
}

fn node_text(node: Node, source: &str) -> Option<String> {
    node.utf8_text(source.as_bytes()).ok().map(|v| v.to_string())
}

fn named_children<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}
