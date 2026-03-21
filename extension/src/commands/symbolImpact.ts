import * as vscode from "vscode";

/**
 * Uses the workspace language service (LSP-backed reference provider) to show
 * where the symbol under the cursor is referenced — a lightweight
 * "change impact" preview without indexing the repo ourselves.
 */
export async function showSymbolImpact(output: vscode.OutputChannel): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage("SlopGuard: No active editor.");
    return;
  }

  const doc = editor.document;
  if (doc.uri.scheme !== "file") {
    vscode.window.showWarningMessage("SlopGuard: Symbol impact works on files on disk (scheme=file).");
    return;
  }

  const pos = editor.selection.active;
  const wordRange = doc.getWordRangeAtPosition(pos);
  if (!wordRange || wordRange.isEmpty) {
    vscode.window.showWarningMessage(
      "SlopGuard: Place the cursor on a symbol name (function, variable, class, type, etc.)."
    );
    return;
  }

  const symbolText = doc.getText(wordRange);
  if (!symbolText.trim() || !/[\w$]/.test(symbolText)) {
    vscode.window.showWarningMessage("SlopGuard: No recognizable symbol at cursor.");
    return;
  }

  const refPosition = wordRange.start;

  let raw: vscode.Location[] | vscode.LocationLink[] | null | undefined;
  try {
    raw = await vscode.commands.executeCommand<
      vscode.Location[] | vscode.LocationLink[] | null | undefined
    >("vscode.executeReferenceProvider", doc.uri, refPosition);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`SlopGuard: Reference provider failed — ${msg}`);
    output.appendLine("");
    output.appendLine("SlopGuard: Symbol impact failed");
    output.appendLine(msg);
    output.show(true);
    return;
  }

  const locations = normalizeReferenceResults(raw);
  const fileLocations = locations.filter((l) => l.uri.scheme === "file");
  if (fileLocations.length === 0) {
    vscode.window.showInformationMessage(
      "SlopGuard: No references found. Try a different symbol, or ensure a language extension supports this file type."
    );
    return;
  }

  // Count references per file (workspace files only).
  const perFile = new Map<string, number>();
  for (const loc of fileLocations) {
    const key = loc.uri.fsPath;
    perFile.set(key, (perFile.get(key) ?? 0) + 1);
  }

  const sortedFiles = [...perFile.entries()].sort((a, b) => b[1] - a[1]);
  const uniqueFileCount = sortedFiles.length;
  const totalRefs = fileLocations.length;

  output.clear();
  output.appendLine("SlopGuard: Symbol impact (language service references)");
  output.appendLine("=".repeat(52));
  output.appendLine(`Symbol: ${symbolText}`);
  output.appendLine(`Source: ${vscode.workspace.asRelativePath(doc.uri)}`);
  output.appendLine("");
  output.appendLine(`Total reference locations: ${totalRefs}`);
  output.appendLine(`Touches ${uniqueFileCount} file(s) in the workspace.`);
  output.appendLine("");
  output.appendLine(
    "If you rename or change this symbol’s contract, review these call sites before committing."
  );
  output.appendLine("");
  output.appendLine("By file:");
  output.appendLine("-".repeat(52));

  const maxFilesListed = 200;
  let listed = 0;
  for (const [fsPath, count] of sortedFiles) {
    if (listed >= maxFilesListed) {
      const remaining = sortedFiles.length - listed;
      output.appendLine(`  … (${remaining} more file(s) not shown)`);
      break;
    }
    const uri = vscode.Uri.file(fsPath);
    const rel = vscode.workspace.asRelativePath(uri);
    const here = fsPath === doc.uri.fsPath ? " (this file)" : "";
    const link = `${uri.fsPath}:1:1`;
    output.appendLine(`  • ${count}×  ${rel}${here}  (${link})`);
    listed++;
  }

  output.show(true);

  const otherFiles = sortedFiles.filter(([p]) => p !== doc.uri.fsPath).length;
  const choice = await vscode.window.showInformationMessage(
    `SlopGuard: “${symbolText}” — ${totalRefs} reference(s) across ${uniqueFileCount} file(s) (${otherFiles} other file(s)).`,
    "Peek references in editor"
  );
  if (choice === "Peek references in editor") {
    try {
      await vscode.commands.executeCommand(
        "editor.action.showReferences",
        doc.uri,
        refPosition,
        fileLocations
      );
    } catch {
      vscode.window.showWarningMessage(
        "SlopGuard: Could not open peek references (try the built-in Find All References on the same symbol)."
      );
    }
  }
}

function normalizeReferenceResults(
  raw: vscode.Location[] | vscode.LocationLink[] | null | undefined
): vscode.Location[] {
  if (!raw || !Array.isArray(raw)) {
    return [];
  }
  const out: vscode.Location[] = [];
  for (const item of raw) {
    if (item instanceof vscode.Location) {
      out.push(item);
      continue;
    }
    const link = item as vscode.LocationLink;
    if (link.targetUri) {
      const range = link.targetSelectionRange ?? link.targetRange;
      if (range) {
        out.push(new vscode.Location(link.targetUri, range));
      }
    }
  }
  return out;
}
