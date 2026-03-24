use slopguard_engine::analyze_json;

fn run_analysis(code: &str, language_id: &str) -> serde_json::Value {
    let input = serde_json::json!({
        "code": code,
        "languageId": language_id
    });
    let output = analyze_json(&input.to_string()).expect("analysis should succeed");
    serde_json::from_str(&output).expect("engine should return valid json")
}

fn has_issue_type(response: &serde_json::Value, issue_type: &str) -> bool {
    response["issues"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .any(|issue| issue["issueType"].as_str() == Some(issue_type))
}

#[test]
fn detects_blocking_call_in_async_typescript() {
    let code = r#"
async function handler() {
  const data = require("fs").readFileSync("a.txt", "utf8");
  return data.length;
}
"#;

    let response = run_analysis(code, "typescript");
    assert!(
        has_issue_type(&response, "async-blocking"),
        "expected async-blocking issue, got: {response}"
    );
}

#[test]
fn does_not_flag_non_blocking_async_typescript() {
    let code = r#"
async function handler() {
  const fs = await import("fs/promises");
  const data = await fs.readFile("a.txt", "utf8");
  return data.length;
}
"#;

    let response = run_analysis(code, "typescript");
    assert!(
        !has_issue_type(&response, "async-blocking"),
        "did not expect async-blocking issue, got: {response}"
    );
}

#[test]
fn detects_blocking_call_in_async_python() {
    let code = r#"
async def run():
    import time
    time.sleep(1)
"#;

    let response = run_analysis(code, "python");
    assert!(
        has_issue_type(&response, "async-blocking"),
        "expected async-blocking issue, got: {response}"
    );
}
