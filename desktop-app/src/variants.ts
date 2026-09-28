import type { Card } from "./types";

function colorKey(value: unknown) {
  return typeof value === "string" ? value.toLowerCase() : (value ?? undefined);
}

export function savedVariant(editing: Card, preview: Card | null): Card {
  const matches =
    preview?.tex === editing.tex &&
    preview.kind === editing.kind &&
    colorKey(preview.instantColor) === colorKey(editing.instantColor);
  const rendered = matches ? preview : null;
  return {
    ...editing,
    // Only render artifacts may come from the preview. It may be the original
    // source card or a response from before another editor field changed.
    status: rendered?.status || "pending",
    svgPath: rendered?.status === "ready" ? rendered.svgPath : undefined,
    pngPath: rendered?.status === "ready" ? rendered.pngPath : undefined,
    pdfPath: rendered?.status === "ready" ? rendered.pdfPath : undefined,
    error: rendered?.error,
    sourceFontSize: rendered?.sourceFontSize ?? editing.sourceFontSize,
    origin: "custom",
  };
}
