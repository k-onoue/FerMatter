import { builtinTemplates, type ProjectTemplate } from "./templates";
import type { Card } from "./types";

export interface InstantDraft {
  requiresRender?: boolean;
  restorationProject?: { uuid: string; name?: string } | null;
  source: string;
  template: ProjectTemplate;
  kind: Card["kind"];
  fontSize: number;
  exportFormat: string;
  color: string;
  contextFile?: string;
}
export interface InstantConfiguration {
  schemaVersion: 1;
  kind: "mathlidelux.instant";
  templates: ProjectTemplate[];
  defaultTemplateID: string;
  fontSize: number;
  exportFormat: string;
  color: string;
  inputMode: Card["kind"];
}
export interface InstantProjectContext {
  id: string;
  root: string;
  main: string;
  format: "latex" | "typst";
  engine: string;
  revision: string;
  sources: { path: string; text: string }[];
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
export function configuredInstant(config: InstantConfiguration): InstantDraft {
  return {
    ...initialInstant(
      config.templates.find(
        (template) => template.id === config.defaultTemplateID,
      ),
    ),
    kind: config.inputMode,
    fontSize: config.fontSize,
    exportFormat: config.exportFormat,
    color: config.color,
  };
}
export function localInstant(context: InstantProjectContext): InstantDraft {
  return {
    ...initialInstant(),
    template: {
      id: "project",
      name: "Project",
      format: context.format,
      compiler: "pdflatex",
      source: "",
    },
    contextFile: context.main,
  };
}
export const instantPendingKey = (project: string) =>
  `mathlidelux.instant.pending.v1${project ? "." + project : ""}`;
export const instantRevision = (draft: InstantDraft, context = "") =>
  JSON.stringify(draft) + context;
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
  context = "",
): T | null {
  return result?.revision === instantRevision(draft, context) ? result : null;
}
