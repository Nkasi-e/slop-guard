import * as vscode from "vscode";
import { AnalysisScope } from "./types";

export type AnalysisTarget = {
  code: string;
  label: string;
};

export function resolveAnalysisTarget(
  editor: vscode.TextEditor,
  scope: AnalysisScope
): AnalysisTarget | null {
  const selected = editor.document.getText(editor.selection).trim();

  if (scope === "selection") {
    if (!selected) {
      return null;
    }
    return { code: selected, label: "selection" };
  }

  if (scope === "file") {
    const code = editor.document.getText().trim();
    return code ? { code, label: "file" } : null;
  }

  if (scope === "function") {
    return detectFunctionLikeScope(editor) ?? detectFallbackScope(editor);
  }

  // auto mode
  if (selected) {
    return { code: selected, label: "selection" };
  }
  return detectFunctionLikeScope(editor) ?? detectFallbackScope(editor);
}

function detectFallbackScope(editor: vscode.TextEditor): AnalysisTarget | null {
  const code = editor.document.getText().trim();
  return code ? { code, label: "file" } : null;
}

function detectFunctionLikeScope(editor: vscode.TextEditor): AnalysisTarget | null {
  const doc = editor.document;
  const lineCount = doc.lineCount;
  if (lineCount === 0) {
    return null;
  }

  const cursorLine = editor.selection.active.line;
  const start = findScopeStart(doc, cursorLine);
  const end = findScopeEnd(doc, cursorLine);
  if (end < start) {
    return null;
  }

  const range = new vscode.Range(start, 0, end, doc.lineAt(end).text.length);
  const code = doc.getText(range).trim();
  if (!code) {
    return null;
  }
  return { code, label: "current block" };
}

function findScopeStart(doc: vscode.TextDocument, cursorLine: number): number {
  let depth = 0;

  for (let line = cursorLine; line >= 0; line--) {
    const text = doc.lineAt(line).text;
    depth += countChar(text, "}");
    depth -= countChar(text, "{");
    if (depth < 0) {
      return line;
    }

    if (looksLikeFunctionHeader(text)) {
      return line;
    }
  }

  return Math.max(0, cursorLine - 30);
}

function findScopeEnd(doc: vscode.TextDocument, cursorLine: number): number {
  const lastLine = doc.lineCount - 1;
  let depth = 0;

  for (let line = cursorLine; line <= lastLine; line++) {
    const text = doc.lineAt(line).text;
    depth += countChar(text, "{");
    depth -= countChar(text, "}");
    if (depth < 0) {
      return line;
    }
  }

  return Math.min(lastLine, cursorLine + 30);
}

function looksLikeFunctionHeader(text: string): boolean {
  const t = text.trim();
  return (
    /^function\s+\w+/.test(t) ||
    /^def\s+\w+/.test(t) ||
    /^fn\s+\w+/.test(t) ||
    /^func\s+\w+/.test(t) ||
    /^[\w<>\[\],\s]+\s+\w+\s*\(.*\)\s*\{?$/.test(t)
  );
}

function countChar(input: string, ch: string): number {
  let count = 0;
  for (const c of input) {
    if (c === ch) {
      count++;
    }
  }
  return count;
}
