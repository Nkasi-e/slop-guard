import * as https from "https";
import { URL } from "url";
import { AlgorithmAnalysis, EngineIssue, LlmSettings } from "./types";

type EnrichmentResult = {
  issues: EngineIssue[];
};

export async function enrichIssuesWithLlm(
  code: string,
  languageId: string,
  issues: EngineIssue[],
  settings: LlmSettings
): Promise<EngineIssue[]> {
  if (!settings.enabled || !settings.apiKey || issues.length === 0) {
    return issues;
  }

  const prompt = buildPrompt(code, languageId, issues);
  const responseText = await postChatCompletion(settings, prompt);
  const parsed = parseLlmResponse(responseText);
  if (!parsed) {
    throw new Error("LLM response did not match expected JSON format.");
  }

  return mergeEnrichment(issues, parsed.issues);
}

function buildPrompt(code: string, languageId: string, issues: EngineIssue[]): string {
  return [
    "You are a senior engineer refining static-analysis findings.",
    "Return ONLY JSON with this shape:",
    '{"issues":[{"issue":"string","explanation":["string"],"suggestion":"string","algorithmAnalysis":{"timeComplexity":"string","spaceComplexity":"string","suggestedTimeComplexity":"string","suggestedSpaceComplexity":"string","tradeOffSummary":"string","tradeOffs":["string"],"optimizationHint":"string"}}]}',
    "Rules:",
    "- Keep issue names exactly as provided.",
    "- Explain algorithmic complexity and trade-offs when relevant.",
    "- Never claim true O(1) total runtime unless mathematically valid for the full operation.",
    "- If only lookups can be O(1), explicitly say so in optimizationHint.",
    "- Keep explanations practical and concise.",
    "",
    `Language: ${languageId}`,
    `Code:\n${code}`,
    `Issues:\n${JSON.stringify(issues)}`,
  ].join("\n");
}

function postChatCompletion(settings: LlmSettings, prompt: string): Promise<string> {
  const endpoint = new URL(settings.endpoint);
  const body = JSON.stringify({
    model: settings.model,
    temperature: settings.temperature,
    messages: [
      {
        role: "system",
        content: "You must return strict JSON only. No markdown.",
      },
      {
        role: "user",
        content: prompt,
      },
    ],
    response_format: { type: "json_object" },
  });

  return new Promise((resolve, reject) => {
    const req = https.request(
      {
        protocol: endpoint.protocol,
        hostname: endpoint.hostname,
        port: endpoint.port,
        path: `${endpoint.pathname}${endpoint.search}`,
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Content-Length": Buffer.byteLength(body),
          Authorization: `Bearer ${settings.apiKey}`,
        },
        timeout: settings.timeoutMs,
      },
      (res) => {
        let data = "";
        res.on("data", (chunk: Buffer) => {
          data += chunk.toString("utf8");
        });
        res.on("end", () => {
          if ((res.statusCode ?? 500) >= 400) {
            reject(new Error(`LLM request failed (${res.statusCode}): ${data}`));
            return;
          }
          try {
            const payload = JSON.parse(data) as {
              choices?: Array<{ message?: { content?: string | Array<{ text?: string }> } }>;
            };
            const content = payload.choices?.[0]?.message?.content;
            if (typeof content === "string") {
              resolve(content);
              return;
            }
            if (Array.isArray(content)) {
              const joined = content.map((part) => part.text ?? "").join("");
              resolve(joined);
              return;
            }
            reject(new Error("LLM response missing message content."));
          } catch (error) {
            reject(
              new Error(`Failed to parse LLM API response: ${error instanceof Error ? error.message : String(error)}`)
            );
          }
        });
      }
    );

    req.on("timeout", () => {
      req.destroy(new Error(`LLM request timed out after ${settings.timeoutMs}ms`));
    });
    req.on("error", reject);
    req.write(body);
    req.end();
  });
}

function parseLlmResponse(raw: string): EnrichmentResult | null {
  try {
    const parsed = JSON.parse(stripCodeFence(raw)) as EnrichmentResult;
    if (!parsed?.issues || !Array.isArray(parsed.issues)) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

function stripCodeFence(input: string): string {
  const trimmed = input.trim();
  if (!trimmed.startsWith("```")) {
    return trimmed;
  }
  return trimmed.replace(/^```[a-zA-Z]*\n?/, "").replace(/\n?```$/, "");
}

function mergeEnrichment(baseIssues: EngineIssue[], enrichedIssues: EngineIssue[]): EngineIssue[] {
  const byIssue = new Map(enrichedIssues.map((issue) => [issue.issue, issue]));
  return baseIssues.map((base) => {
    const enriched = byIssue.get(base.issue);
    if (!enriched) {
      return base;
    }
    return {
      ...base,
      explanation: enriched.explanation?.length ? enriched.explanation : base.explanation,
      suggestion: enriched.suggestion ?? base.suggestion,
      algorithmAnalysis: mergeAlgorithmAnalysis(base.algorithmAnalysis, enriched.algorithmAnalysis),
    };
  });
}

function mergeAlgorithmAnalysis(
  base?: AlgorithmAnalysis,
  next?: AlgorithmAnalysis
): AlgorithmAnalysis | undefined {
  if (!next) {
    return base;
  }
  if (!base) {
    return next;
  }
  return {
    timeComplexity: next.timeComplexity || base.timeComplexity,
    spaceComplexity: next.spaceComplexity || base.spaceComplexity,
    suggestedTimeComplexity: next.suggestedTimeComplexity ?? base.suggestedTimeComplexity,
    suggestedSpaceComplexity: next.suggestedSpaceComplexity ?? base.suggestedSpaceComplexity,
    tradeOffSummary: next.tradeOffSummary ?? base.tradeOffSummary,
    tradeOffs: next.tradeOffs?.length ? next.tradeOffs : base.tradeOffs,
    optimizationHint: next.optimizationHint ?? base.optimizationHint,
  };
}
