mod analysis;
mod protocol;
#[cfg(not(target_family = "wasm"))]
mod scan;

use crate::analysis::run_all_analyzers;
use crate::protocol::{AnalyzeRequest, AnalyzeResponse, Issue};
use std::io::{self, BufRead, Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    #[cfg(not(target_family = "wasm"))]
    if args.first().is_some_and(|a| a == "scan") {
        scan::run(args.into_iter().skip(1).collect());
    }

    let serve_mode = args.iter().any(|arg| arg == "--serve");
    if serve_mode {
        run_serve_mode();
        return;
    }

    run_single_request_mode();
}

fn run_single_request_mode() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        emit_error_response("Failed to read stdin.");
        return;
    }

    let request = parse_request(&input);
    let issues = run_all_analyzers(&request);

    let response = AnalyzeResponse { issues };
    match serde_json::to_string(&response) {
        Ok(json) => {
            println!("{json}");
        }
        Err(_) => emit_error_response("Failed to serialize analysis response."),
    }
}

fn run_serve_mode() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut locked = stdin.lock();
    let mut line = String::new();

    loop {
        line.clear();
        let read = match locked.read_line(&mut line) {
            Ok(n) => n,
            Err(_) => break,
        };
        if read == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request = parse_request(trimmed);
        let issues = run_all_analyzers(&request);
        let response = AnalyzeResponse { issues };
        let output = serde_json::to_string(&response).unwrap_or_else(|_| {
            serde_json::to_string(&AnalyzeResponse {
                issues: vec![Issue::new(
                    "Engine error",
                    vec!["Failed to serialize analysis response.".to_string()],
                    1.0,
                    None,
                    Some("bug-risk".to_string()),
                )],
            })
            .unwrap_or_else(|_| "{\"issues\":[]}".to_string())
        });

        if writeln!(stdout, "{output}").is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
    }
}

fn parse_request(raw: &str) -> AnalyzeRequest {
    match serde_json::from_str::<AnalyzeRequest>(raw) {
        Ok(request) => request,
        Err(_) => AnalyzeRequest {
            code: raw.to_string(),
            language_id: None,
            document_key: None,
            analysis_context: None,
        },
    }
}

fn emit_error_response(message: &str) {
    let response = AnalyzeResponse {
        issues: vec![Issue::new(
            "Engine error",
            vec![message.to_string()],
            1.0,
            None,
            Some("bug-risk".to_string()),
        )],
    };

    if let Ok(json) = serde_json::to_string(&response) {
        println!("{json}");
    }
}
