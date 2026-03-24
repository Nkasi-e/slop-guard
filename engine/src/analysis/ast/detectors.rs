use tree_sitter::Node;

use super::language::SupportedLanguage;
use super::utils::{
    is_assignment_like_statement, is_block_kind, is_loop_node, named_children, node_text,
};

pub fn detect_manual_iteration_ast(
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
            let mut end_line = if end_row == 0 {
                0
            } else {
                (end_row as usize).saturating_sub(1)
            };

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

pub fn detect_redundant_assign_then_return_ast(
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
