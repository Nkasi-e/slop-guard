use super::language::SupportedLanguage;

pub fn deep_nesting_evidence_snippet(
    source: &str,
    language: SupportedLanguage,
) -> Option<(String, usize, usize)> {
    if matches!(
        language,
        SupportedLanguage::JavaScript
            | SupportedLanguage::TypeScript
            | SupportedLanguage::Go
            | SupportedLanguage::Rust
            | SupportedLanguage::Java
    ) {
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
            let starts_block = line.starts_with("if ")
                || line.starts_with("unless ")
                || line.starts_with("while ")
                || line.starts_with("for ")
                || line.ends_with(" do")
                || line.contains(" do |")
                || line.starts_with("begin")
                || line.starts_with("case ");
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

pub fn nested_loop_evidence_snippet(
    source: &str,
    language: SupportedLanguage,
) -> Option<(String, usize, usize)> {
    let lines: Vec<&str> = source.lines().collect();

    let loop_markers: &'static [&'static str] = match language {
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            &["for ", "for(", "while ", "while("]
        }
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

    let start = found[0].saturating_sub(4);
    let end = (found[found.len() - 1] + 4).min(lines.len().saturating_sub(1));
    let snippet = lines[start..=end]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    Some((snippet, start, end))
}
