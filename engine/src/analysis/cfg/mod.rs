mod builder;
mod ir;
mod lang;
mod rules;
mod util;

use crate::analysis::Analyzer;
use crate::protocol::{AnalyzeRequest, Issue};
use tree_sitter::Parser;

use builder::build_cfg;
use lang::SupportedLanguage;
use rules::detect_blocking_in_async;

pub struct CfgAnalyzer;

impl Analyzer for CfgAnalyzer {
    fn analyze(&self, request: &AnalyzeRequest) -> Vec<Issue> {
        let Some(lang) = SupportedLanguage::from_language_id(request.language_id.as_deref()) else {
            return Vec::new();
        };
        let Some(ts_lang) = lang.tree_sitter_language() else {
            return Vec::new();
        };

        let mut parser = Parser::new();
        if parser.set_language(&ts_lang).is_err() {
            return Vec::new();
        }
        let Some(tree) = parser.parse(&request.code, None) else {
            return Vec::new();
        };

        let root = tree.root_node();
        let cfg = build_cfg(root);
        detect_blocking_in_async(root, &request.code, lang, &cfg)
            .into_iter()
            .collect()
    }
}
