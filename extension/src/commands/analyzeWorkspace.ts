import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { resolveAnalysisSettings } from "../config";
import { runEngineHybrid } from "../engineClient";
import { EngineIssue } from "../types";
import { WorkspaceContextIndexer } from "../workspaceContext";

const SCAN_GLOB = "**/*.{ts,tsx,js,jsx,py,go,rs,java,rb}";
const SCAN_EXCLUDE = "**/{node_modules,target,dist,out,.git,vendor}/**";

function languageIdForPath(filePath: string): string | undefined {
  const ext = path.extname(filePath).toLowerCase();
  switch (ext) {
    case ".ts":
    case ".tsx":
      return "typescript";
    case ".js":
    case ".jsx":
      return "javascript";
    case ".py":
      return "python";
    case ".go":
      return "go";
    case ".rs":
      return "rust";
    case ".rb":
      return "ruby";
    case ".java":
      return "java";
    default:
      return undefined;
  }
}

function issueToDiagnostics(uri: vscode.Uri, issues: EngineIssue[]): vscode.Diagnostic[] {
  const out: vscode.Diagnostic[] = [];
  for (const issue of issues) {
    const startLine = issue.snippetStartLine ?? 0;
    const endLine = issue.snippetEndLine ?? startLine;
    const range = new vscode.Range(
      Math.max(0, startLine),
      0,
      Math.max(0, endLine),
      Number.MAX_SAFE_INTEGER
    );
    const msg = issue.explanation.length > 0 ? `${issue.issue}: ${issue.explanation[0]}` : issue.issue;
    const sev =
      issue.confidence >= 0.85 ? vscode.DiagnosticSeverity.Error : vscode.DiagnosticSeverity.Warning;
    const d = new vscode.Diagnostic(range, msg, sev);
    d.source = "SlopGuard";
    if (issue.issueType) {
      d.code = issue.issueType;
    }
    out.push(d);
  }
  return out;
}

export async function analyzeWorkspace(
  output: vscode.OutputChannel,
  diagnostics: vscode.DiagnosticCollection,
  indexer: WorkspaceContextIndexer
): Promise<void> {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders?.length) {
    vscode.window.showWarningMessage("SlopGuard: Open a workspace folder to scan.");
    return;
  }

  const settings = resolveAnalysisSettings();
  const maxFiles = settings.maxWorkspaceScanFiles;
  const maxLines = settings.maxAnalyzeLines;

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "SlopGuard: scanning workspace",
      cancellable: true,
    },
    async (progress, token) => {
      diagnostics.clear();

      const uris = await vscode.workspace.findFiles(SCAN_GLOB, SCAN_EXCLUDE, maxFiles);
      if (token.isCancellationRequested) return;

      let filesDone = 0;
      let totalIssues = 0;
      const errors: string[] = [];

      for (const uri of uris) {
        if (token.isCancellationRequested) break;
        if (uri.scheme !== "file") continue;

        const lang = languageIdForPath(uri.fsPath);
        if (!lang) continue;

        progress.report({
          message: `${path.basename(uri.fsPath)} (${filesDone + 1}/${uris.length})`,
        });

        let code: string;
        try {
          code = await fs.promises.readFile(uri.fsPath, "utf8");
        } catch {
          continue;
        }

        const lines = code.split("\n");
        if (lines.length > maxLines) {
          code = lines.slice(0, maxLines).join("\n");
        }

        try {
          const analysisContext = await indexer.getAnalysisContextForFilePath(uri.fsPath, code, lang, 8);
          const { response } = await runEngineHybrid({
            code,
            languageId: lang,
            documentKey: `workspace-scan::${uri.fsPath}`,
            analysisContext,
          });

          if (response.issues.length > 0) {
            diagnostics.set(uri, issueToDiagnostics(uri, response.issues));
            totalIssues += response.issues.length;
          }
        } catch (e) {
          const msg = e instanceof Error ? e.message : String(e);
          errors.push(`${vscode.workspace.asRelativePath(uri)}: ${msg}`);
        }

        filesDone += 1;
      }

      output.clear();
      output.appendLine("SlopGuard workspace scan");
      output.appendLine("=".repeat(48));
      output.appendLine(`Files scanned: ${filesDone}`);
      output.appendLine(`Issues: ${totalIssues}`);
      if (errors.length > 0) {
        output.appendLine("");
        output.appendLine("Errors:");
        for (const line of errors.slice(0, 20)) {
          output.appendLine(`  ${line}`);
        }
        if (errors.length > 20) {
          output.appendLine(`  … ${errors.length - 20} more`);
        }
      }
      output.appendLine("");
      output.appendLine("Open the Problems panel (View → Problems) to see findings in closed files.");
      output.show(true);

      if (totalIssues > 0) {
        vscode.window.showInformationMessage(
          `SlopGuard: ${totalIssues} issue(s) in workspace — see Problems panel.`,
          "Show Problems"
        ).then((choice) => {
          if (choice === "Show Problems") {
            void vscode.commands.executeCommand("workbench.actions.view.problems");
          }
        });
      } else if (!token.isCancellationRequested) {
        vscode.window.showInformationMessage("SlopGuard: No issues found in scanned files.");
      }
    }
  );
}
