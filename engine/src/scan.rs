//! Workspace / directory scan for CI and pre-commit hooks.

use crate::analysis::run_all_analyzers;
use crate::protocol::{AnalyzeRequest, Issue};
use ignore::WalkBuilder;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const DEFAULT_MAX_FILES: usize = 2000;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Hard cap on directory walk entries so unignored build trees cannot hang the scan.
const MAX_WALK_ENTRIES: usize = 50_000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanFileResult {
    path: String,
    issues: Vec<Issue>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanReport {
    version: u32,
    root: String,
    files_scanned: usize,
    issue_count: usize,
    results: Vec<ScanFileResult>,
}

pub fn run(args: Vec<String>) -> ! {
    let mut root = PathBuf::from(".");
    let mut max_files = DEFAULT_MAX_FILES;
    let mut min_confidence = 0.0_f64;
    let mut no_fail = false;

    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--max-files" => {
                let Some(v) = it.next().and_then(|s| s.parse().ok()) else {
                    eprintln!("slopguard-engine scan: --max-files requires a number");
                    process::exit(2);
                };
                max_files = v;
            }
            "--min-confidence" => {
                let Some(v) = it.next().and_then(|s| s.parse().ok()) else {
                    eprintln!("slopguard-engine scan: --min-confidence requires a number");
                    process::exit(2);
                };
                min_confidence = v;
            }
            "--no-fail" => no_fail = true,
            s if s.starts_with('-') => {
                eprintln!("Unknown flag: {s}");
                process::exit(2);
            }
            s => root = PathBuf::from(s),
        }
    }

    let root = match fs::canonicalize(&root) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("slopguard-engine scan: cannot resolve root {:?}: {e}", root);
            process::exit(2);
        }
    };

    let mut results: Vec<ScanFileResult> = Vec::new();
    let mut files_scanned = 0usize;
    let mut walk_entries = 0usize;

    let walker = WalkBuilder::new(&root)
        .hidden(true)
        .git_ignore(true)
        .git_exclude(true)
        .build();

    for entry in walker.flatten() {
        walk_entries += 1;
        if walk_entries > MAX_WALK_ENTRIES {
            break;
        }
        if files_scanned >= max_files {
            break;
        }
        let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
        if !is_file {
            continue;
        }
        let path = entry.path();
        let path_s = path.to_string_lossy();
        if path_s.contains("/target/") || path_s.contains("\\target\\") {
            continue;
        }
        let Some(lang) = language_id_for_path(path) else {
            continue;
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > MAX_FILE_BYTES {
            continue;
        }
        let code = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let rel = path
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.display().to_string());

        let request = AnalyzeRequest {
            code,
            language_id: Some(lang.to_string()),
            document_key: Some(rel.clone()),
            analysis_context: None,
        };

        let mut issues: Vec<Issue> = run_all_analyzers(&request)
            .into_iter()
            .filter(|issue| issue.confidence >= min_confidence)
            .collect();

        if !issues.is_empty() {
            results.push(ScanFileResult {
                path: rel,
                issues: std::mem::take(&mut issues),
            });
        }
        files_scanned += 1;
    }

    let issue_count: usize = results.iter().map(|r| r.issues.len()).sum();

    let report = ScanReport {
        version: 1,
        root: root.display().to_string(),
        files_scanned,
        issue_count,
        results,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("slopguard-engine scan: failed to serialize report: {e}");
            process::exit(2);
        }
    }

    if issue_count > 0 && !no_fail {
        process::exit(1);
    }
    process::exit(0);
}

fn language_id_for_path(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" => Some("javascript"),
        "py" => Some("python"),
        "go" => Some("go"),
        "rs" => Some("rust"),
        "rb" => Some("ruby"),
        "java" => Some("java"),
        _ => None,
    }
}
