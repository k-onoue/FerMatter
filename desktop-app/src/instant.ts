import { builtinTemplates, type ProjectTemplate } from "./templates";
import type { Card } from "./types";

export interface InstantDraft {
  source: string;
  template: ProjectTemplate;
  kind: Card["kind"];
  fontSize: number;
  exportFormat: string;
  color: string;
}
export interface InstantHistory {
  id: string;
  draft: InstantDraft;
}
export interface InstantSession {
  schemaVersion: 1;
  draft: InstantDraft | null;
  history: InstantHistory[];
}
export const initialInstant = (
  template = builtinTemplates[0],
): InstantDraft => ({
  source: "",
  template,
  kind: /tikz|cetz/.test(template.id) ? "figure" : "block",
  fontSize: 24,
  exportFormat: "svg",
  color: "#000000",
});
export const instantRevision = (draft: InstantDraft) => JSON.stringify(draft);
export function instantError(log: string) {
  const line = log.split("\n").find((line) => /^!|^error:|:\d+:\s/.test(line));
  return (
    line?.replace(/^.*?:\d+:\s*/, "") ||
    (log.includes("\n") ? "Check the equation or template." : log)
  ).slice(0, 180);
}
export function readyInstant<T extends { revision: string }>(
  draft: InstantDraft,
  result: T | null,
): T | null {
  return result?.revision === instantRevision(draft) ? result : null;
}
