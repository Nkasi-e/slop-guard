mod analysis;
mod protocol;

use crate::analysis::run_all_analyzers;
use crate::protocol::{AnalyzeRequest, AnalyzeResponse, Issue};
use std::io::{self, Read};

fn main() {
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

fn parse_request(raw: &str) -> AnalyzeRequest {
    match serde_json::from_str::<AnalyzeRequest>(raw) {
        Ok(request) => request,
        Err(_) => AnalyzeRequest {
            code: raw.to_string(),
            language_id: None,
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
