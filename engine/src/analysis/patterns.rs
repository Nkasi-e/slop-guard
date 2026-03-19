use crate::analysis::Analyzer;
use crate::protocol::{AnalyzeRequest, Issue};

pub struct PatternAnalyzer;

impl Analyzer for PatternAnalyzer {
    fn analyze(&self, request: &AnalyzeRequest) -> Vec<Issue> {
        let mut issues = Vec::new();
        let code = request.code.as_str();
        let language = normalize_language(request.language_id.as_deref());

        if let Some((snippet, start_line, end_line)) = find_manual_iteration_snippet(code, &language) {
            issues.push(Issue::new(
                "Manual iteration detected",
                vec![
                    "Loop-based collection building was detected for this language.".to_string(),
                    "This pattern is harder to scan than declarative transformations.".to_string(),
                ],
                0.90,
                Some(language_manual_iteration_suggestion(&language).to_string()),
                Some("ai-slop".to_string()),
            ).with_snippet_evidence(snippet, start_line, end_line));
        }

        if let Some((snippet, start_line, end_line)) = find_redundant_assign_then_return_snippet(code, &language) {
            issues.push(Issue::new(
                "Redundant variable before return",
                vec![
                    "A variable is assigned and returned immediately in the same local scope."
                        .to_string(),
                    "This adds local noise without improving intent clarity.".to_string(),
                ],
                0.92,
                Some("Return the expression directly.".to_string()),
                Some("readability".to_string()),
            ).with_snippet_evidence(snippet, start_line, end_line));
        }

        let (max_depth, nesting_snippet) = compute_nesting_depth(code, &language);
        if max_depth > 3 {
            let issue = Issue::new(
                "Deep nesting detected",
                vec![
                    format!("Observed block nesting depth of {max_depth}, which is above 3."),
                    "Nested control flow increases cognitive load and bug risk.".to_string(),
                ],
                0.85,
                Some("Extract helper functions or use guard clauses to flatten branches.".to_string()),
                Some("maintainability".to_string()),
            );

            issues.push(match nesting_snippet {
                Some(snippet) => issue.with_snippet(snippet),
                None => issue,
            });
        }

        issues
    }
}

fn normalize_language(language_id: Option<&str>) -> String {
    let id = language_id.unwrap_or_default().to_ascii_lowercase();
    match id.as_str() {
        "javascript" | "javascriptreact" => "javascript".to_string(),
        "typescript" | "typescriptreact" => "typescript".to_string(),
        "python" => "python".to_string(),
        "go" => "go".to_string(),
        "rust" => "rust".to_string(),
        "ruby" => "ruby".to_string(),
        "java" => "java".to_string(),
        _ => "unknown".to_string(),
    }
}

fn language_manual_iteration_suggestion(language: &str) -> &'static str {
    match language {
        "python" => "Prefer list/dict comprehensions or built-ins like map/filter where appropriate.",
        "go" => "Extract reusable transform/filter helpers instead of inline accumulation loops.",
        "rust" => "Prefer iterator chains (map/filter/collect) when they keep ownership clear.",
        "ruby" => "Prefer Enumerable methods like map/select/reject over manual push-style loops.",
        "java" => "Prefer Stream API for transformations when readability improves.",
        _ => "Prefer map/filter pipelines for data transformation.",
    }
}

fn find_manual_iteration_snippet(code: &str, language: &str) -> Option<(String, usize, usize)> {
    // Best-effort evidence: return the first "loop" line we can find.
    let lines: Vec<&str> = code.lines().collect();

    let (loop_token, acc_tokens): (&str, Vec<&str>) = match language {
        "python" => ("for ", vec![".append("]),
        "go" => ("for ", vec!["append("]),
        "rust" => ("for ", vec![".push("]),
        "ruby" => (".each do", vec!["<<", ".push("]),
        "java" => ("for ", vec![".add("]),
        _ => ("for ", vec![".push("]),
    };

    let mut for_idx: Option<usize> = None;
    for (i, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        if t.contains(loop_token) {
            for_idx = Some(i);
            break;
        }
    }

    let for_idx = for_idx?;
    let acc_present = acc_tokens.iter().any(|tok| {
        let lower = lines.iter().skip(for_idx).take(20).map(|s| s.trim()).collect::<Vec<_>>().join("\n");
        lower.contains(tok)
    });

    if !acc_present {
        return None;
    }

    // Include a wider window so the developer can see the loop's structure,
    // accumulation, and at least a bit of surrounding code.
    let start = for_idx.saturating_sub(2);
    let end = (for_idx + 6).min(lines.len().saturating_sub(1));
    let snippet_lines = lines[start..=end]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>();
    Some((snippet_lines.join("\n"), start, end))
}

fn find_redundant_assign_then_return_snippet(
    code: &str,
    language: &str,
) -> Option<(String, usize, usize)> {
    let lines: Vec<&str> = code.lines().collect();

    for i in 0..lines.len() {
        let assign = lines[i].trim();
        let Some(var_name) = parse_assignment_var_name(assign, language) else {
            continue;
        };

        let mut j = i + 1;
        while j < lines.len() && lines[j].trim().is_empty() {
            j += 1;
        }
        if j >= lines.len() {
            continue;
        }

        let ret = lines[j].trim().trim_end_matches(';');
        let expected = format!("return {var_name}");
        if ret == expected {
            // Provide surrounding context (often includes the return site and
            // the assignment that creates the redundant local binding).
            let start = i.saturating_sub(2);
            let end = (j + 2).min(lines.len().saturating_sub(1));
            let snippet = lines[start..=end]
                .iter()
                .map(|l| l.trim_end())
                .collect::<Vec<_>>()
                .join("\n");
            return Some((snippet, start, end));
        }
    }

    None
}

fn parse_assignment_var_name<'a>(line: &'a str, language: &str) -> Option<&'a str> {
    let mut candidates: Vec<&str> = Vec::new();
    match language {
        "javascript" | "typescript" => candidates.extend(["const ", "let ", "var "]),
        "rust" => candidates.extend(["let mut ", "let "]),
        "go" => candidates.extend(["var "]),
        "java" => candidates.extend(["final ", "var ", "int ", "long ", "double ", "float ", "boolean ", "String "]),
        "python" | "ruby" => {}
        _ => candidates.extend(["const ", "let ", "var ", "let mut ", "var "]),
    }

    for prefix in candidates {
        if let Some(rest) = line.strip_prefix(prefix) {
            let mut parts = rest.splitn(2, '=');
            let candidate = parts.next()?.trim();
            if !candidate.is_empty() && parts.next().is_some() {
                let name = candidate.split_whitespace().last().unwrap_or(candidate);
                return Some(name.trim_end_matches(':'));
            }
        }
    }

    if matches!(language, "python" | "ruby" | "go") && !line.starts_with("return ") {
        let mut parts = line.splitn(2, '=');
        let left = parts.next()?.trim();
        let _right = parts.next()?;
        if !left.is_empty() && !left.contains(' ') && !left.contains('.') {
            return Some(left);
        }
    }

    None
}

fn compute_nesting_depth(code: &str, language: &str) -> (usize, Option<String>) {
    if language == "python" {
        return (compute_python_nesting_depth(code), None);
    }
    if language == "ruby" {
        return (compute_ruby_nesting_depth(code), None);
    }

    let mut depth = 0usize;
    let mut max_depth = 0usize;
    let mut max_depth_line: Option<String> = None;

    for raw_line in code.lines() {
        let line = raw_line;
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
                if depth > max_depth {
                    max_depth = depth;
                    max_depth_line = Some(line.trim_end().to_string());
                }
            } else if ch == '}' && depth > 0 {
                depth -= 1;
            }
        }
    }

    (max_depth, max_depth_line)
}

fn compute_python_nesting_depth(code: &str) -> usize {
    let mut stack: Vec<usize> = Vec::new();
    let mut max_depth = 0usize;

    for raw in code.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.trim_start().starts_with('#') {
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

        let trimmed = line.trim_start();
        if starts_control_keyword_python(trimmed) {
            stack.push(indent);
            max_depth = max_depth.max(stack.len());
        }
    }

    max_depth
}

fn starts_control_keyword_python(line: &str) -> bool {
    let keywords = ["if ", "for ", "while ", "try:", "except", "with ", "elif "];
    keywords.iter().any(|kw| line.starts_with(kw))
}

fn compute_ruby_nesting_depth(code: &str) -> usize {
    let mut depth = 0usize;
    let mut max_depth = 0usize;

    for raw in code.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "end" {
            depth = depth.saturating_sub(1);
            continue;
        }
        if line.starts_with("if ")
            || line.starts_with("unless ")
            || line.starts_with("while ")
            || line.starts_with("for ")
            || line.ends_with(" do")
            || line.contains(" do |")
            || line.starts_with("begin")
            || line.starts_with("case ")
        {
            depth += 1;
            max_depth = max_depth.max(depth);
        }
    }

    max_depth
}
