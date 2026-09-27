import type { Card } from "./types";

export function savedVariant(editing: Card, preview: Card | null): Card {
  return {
    ...editing,
    ...(preview?.tex === editing.tex ? preview : {}),
    // An unchanged preview may still be the original source card.
    // Its cached image is reusable; its identity must never replace the variant.
    id: editing.id,
    origin: "custom",
    parentID: editing.parentID,
    name: editing.name,
    exportFontSize: editing.exportFontSize,
  };
}
