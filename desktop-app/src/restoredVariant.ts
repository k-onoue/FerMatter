import type { InstantDraft } from "./instant";
import type { Card, Project } from "./types";

/** Restoration is a new presentation variant, never an edit to a manuscript. */
export function restoredVariant(draft: InstantDraft, project: Project): Card {
  return {
    id: crypto.randomUUID(),
    name: draft.kind === "figure" ? "Imported figure" : "Imported equation",
    tex: draft.source,
    kind: draft.kind,
    origin: "custom",
    status: "pending",
    sourceOrder: project.cards.length,
    file: draft.contextFile || project.main,
    restoredContextFile: draft.contextFile || project.main,
    exportFontSize: draft.fontSize,
    exportFormat: draft.exportFormat,
    instantColor: draft.color.slice(1),
    requiresRender: true,
  };
}
