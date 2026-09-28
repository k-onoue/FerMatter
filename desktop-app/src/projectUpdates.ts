import type { Project } from "./types";

/** Scan owns extracted source structure only; all editable metadata belongs to current. */
export function mergeScan(current: Project, scanned: Project): Project {
  const old = new Map(current.cards.map((card) => [card.id, card]));
  const cards = scanned.cards
    .filter((card) => card.origin !== "custom")
    .map((card) => {
      const previous = old.get(card.id);
      return previous
        ? {
            ...card,
            name: previous.name,
            exportFontSize: previous.exportFontSize,
          }
        : card;
    });
  return {
    ...current,
    cards: [
      ...cards,
      ...current.cards.filter((card) => card.origin === "custom"),
    ],
    outline: scanned.outline,
    warnings: scanned.warnings,
    files: scanned.files,
  };
}

/** Rendering may finish after the same card has been edited. Only attach matching artifacts. */
export function mergeRenderedCard<T extends Project["cards"][number]>(
  current: T,
  rendered: Partial<T> | undefined,
): T {
  if (
    !rendered ||
    (rendered.tex !== undefined && rendered.tex !== current.tex) ||
    (rendered.kind !== undefined && rendered.kind !== current.kind)
  )
    return current;
  return {
    ...current,
    ...rendered,
    name: current.name,
    exportFontSize: current.exportFontSize,
  };
}
