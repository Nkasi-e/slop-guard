use crate::protocol::AnalyzeRequest;
use std::cell::RefCell;
use std::collections::HashMap;
use tree_sitter::{InputEdit, Parser, Point, Tree};

use super::language::SupportedLanguage;

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

pub fn parse_with_incremental_cache(
    parser: &mut Parser,
    request: &AnalyzeRequest,
    supported_language: SupportedLanguage,
) -> Option<Tree> {
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
    }?;

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
                tree: next_tree.clone(),
            },
        );
    });

    Some(next_tree)
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
