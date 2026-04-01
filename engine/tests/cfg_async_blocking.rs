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

#[test]
fn detects_blocking_wrapper_propagation_from_analysis_context() {
    let code = r#"
async function handler() {
  return readConfigSyncWrapper();
}
"#;
    let input = serde_json::json!({
        "code": code,
        "languageId": "typescript",
        "analysisContext": {
            "currentFile": "src/handler.ts",
            "dependencyNeighbors": ["src/io.ts"],
            "blockingWrapperHints": [
                {
                    "symbol": "readConfigSyncWrapper",
                    "sourceFile": "src/io.ts",
                    "confidenceTier": "high"
                }
            ],
            "unresolvedDynamicCalls": 1,
            "unresolvedDynamicImports": 0
        }
    });
    let output = analyze_json(&input.to_string()).expect("analysis should succeed");
    let response: serde_json::Value = serde_json::from_str(&output).expect("valid JSON response");
    assert!(
        has_issue_type(&response, "async-blocking-propagated"),
        "expected async-blocking-propagated issue, got: {response}"
    );
}

#[test]
fn detects_cross_boundary_n_plus_one_signal() {
    let code = r#"
async function handle(ids: string[]) {
  for (const id of ids) {
    await userRepoFindById(id);
  }
}
"#;
    let input = serde_json::json!({
        "code": code,
        "languageId": "typescript",
        "analysisContext": {
            "currentFile": "src/handler.ts",
            "dependencyNeighbors": ["src/repository/userRepo.ts"],
            "blockingWrapperHints": [],
            "nPlusOneHints": [
                {
                    "symbol": "userRepoFindById",
                    "sourceFile": "src/repository/userRepo.ts",
                    "boundary": "repository",
                    "confidenceTier": "high"
                }
            ],
            "unresolvedDynamicCalls": 0,
            "unresolvedDynamicImports": 0
        }
    });
    let output = analyze_json(&input.to_string()).expect("analysis should succeed");
    let response: serde_json::Value = serde_json::from_str(&output).expect("valid JSON response");
    assert!(
        has_issue_type(&response, "n-plus-one-cross-boundary"),
        "expected n-plus-one-cross-boundary issue, got: {response}"
    );
}

#[test]
fn detects_retry_policy_inconsistency_across_call_chain() {
    let code = r#"
async function syncUsers(ids: string[]) {
  for (const id of ids) {
    await userServiceRetryFetch(id);
  }
}
"#;
    let input = serde_json::json!({
        "code": code,
        "languageId": "typescript",
        "analysisContext": {
            "currentFile": "apps/api/src/handlers/sync.ts",
            "dependencyNeighbors": ["packages/shared/src/retry.ts"],
            "retryPolicyHints": [
                {
                    "symbol": "userServiceRetryFetch",
                    "sourceFile": "packages/shared/src/retry.ts",
                    "confidenceTier": "high",
                    "hasBackoff": true,
                    "hasJitter": false,
                    "hasCap": false,
                    "propagatesCancellation": false,
                    "filtersTransientErrors": true
                }
            ],
            "callGraphEdges": [
                {
                    "caller": "syncUsers",
                    "callee": "userServiceRetryFetch",
                    "sourceFile": "apps/api/src/handlers/sync.ts",
                    "targetFile": "packages/shared/src/retry.ts",
                    "boundary": "package-boundary",
                    "confidenceTier": "high"
                }
            ]
        }
    });
    let output = analyze_json(&input.to_string()).expect("analysis should succeed");
    let response: serde_json::Value = serde_json::from_str(&output).expect("valid JSON response");
    assert!(
        has_issue_type(&response, "retry-policy-cross-chain"),
        "expected retry-policy-cross-chain issue, got: {response}"
    );
}

#[test]
fn detects_n_plus_one_across_package_boundary() {
    let code = r#"
async function hydrateUsers(ids: string[]) {
  for (const id of ids) {
    await fetchUserById(id);
  }
}
"#;
    let input = serde_json::json!({
        "code": code,
        "languageId": "typescript",
        "analysisContext": {
            "currentFile": "apps/web/src/hydrate.ts",
            "dependencyNeighbors": ["packages/data/src/usersRepo.ts"],
            "nPlusOneHints": [
                {
                    "symbol": "fetchUserById",
                    "sourceFile": "packages/data/src/usersRepo.ts",
                    "boundary": "package-boundary",
                    "confidenceTier": "high"
                }
            ],
            "callGraphEdges": [
                {
                    "caller": "hydrateUsers",
                    "callee": "fetchUserById",
                    "sourceFile": "apps/web/src/hydrate.ts",
                    "targetFile": "packages/data/src/usersRepo.ts",
                    "boundary": "package-boundary",
                    "confidenceTier": "high"
                }
            ]
        }
    });
    let output = analyze_json(&input.to_string()).expect("analysis should succeed");
    let response: serde_json::Value = serde_json::from_str(&output).expect("valid JSON response");
    assert!(
        has_issue_type(&response, "n-plus-one-cross-boundary"),
        "expected n-plus-one-cross-boundary issue for package boundary, got: {response}"
    );
}
