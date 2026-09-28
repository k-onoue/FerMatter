import { expect, it } from "vitest";
import { savedVariant } from "./variants";
import type { Card } from "./types";

it("saving before the first preview completes keeps the presentation variant separate", () => {
  const source = {
    id: "source-1",
    tex: "x",
    origin: "source",
    status: "ready",
    svgPath: "cached.svg",
  } as Card;
  const editing = {
    ...source,
    id: "custom-1",
    origin: "custom",
    parentID: source.id,
    name: "Slide",
  } as Card;
  const variant = savedVariant(editing, source);
  expect(variant.id).toBe("custom-1");
  expect(variant.origin).toBe("custom");
  expect(variant.parentID).toBe(source.id);
  expect(variant.svgPath).toBe("cached.svg");
  expect(source.id).toBe("source-1");
});

it("does not attach the old preview to newly edited math", () => {
  const editing = {
    id: "custom-1",
    tex: "y",
    origin: "custom",
    status: "pending",
  } as Card;
  const preview = {
    ...editing,
    tex: "x",
    status: "ready",
    svgPath: "old.svg",
  } as Card;
  expect(savedVariant(editing, preview).svgPath).toBeUndefined();
});

it.each([{ instantColor: "dc2626" }, { kind: "block" }, { tex: "y" }])(
  "invalidates old artwork when saving before the new preview: %j",
  (change) => {
    const preview = {
      id: "custom-1",
      tex: "x",
      kind: "inline",
      origin: "custom",
      status: "ready",
      svgPath: "old.svg",
      pngPath: "old.png",
      pdfPath: "old.pdf",
    } as Card;
    const editing = { ...preview, ...change } as Card;
    const saved = savedVariant(editing, preview);
    expect(saved).toMatchObject({ ...change, status: "pending" });
    expect(saved.svgPath).toBeUndefined();
    expect(saved.pngPath).toBeUndefined();
    expect(saved.pdfPath).toBeUndefined();
    expect(preview.svgPath).toBe("old.svg");
  },
);

it("keeps all edited metadata while accepting matching colored artifacts", () => {
  const preview = {
    id: "source",
    tex: "x",
    kind: "inline",
    origin: "source",
    name: "Old name",
    sourceOrder: 0,
    status: "ready",
    instantColor: "ABCDEF",
    sourceFontSize: 11,
    exportFontSize: 12,
    exportFormat: "svg",
    svgPath: "new.svg",
    parentID: "old-parent",
    userField: "older",
  } as Card;
  const editing = {
    ...preview,
    id: "variant",
    origin: "custom",
    name: "New name",
    instantColor: "abcdef",
    exportFontSize: 32,
    exportFormat: "pdf",
    parentID: "source",
    userField: "latest",
  } as Card;
  expect(savedVariant(editing, preview)).toEqual(editing);
});

it("resetting a color never reuses the overridden artwork", () => {
  const preview = {
    id: "variant",
    tex: "x",
    kind: "inline",
    origin: "custom",
    instantColor: "000000",
    status: "ready",
    svgPath: "black.svg",
  } as Card;
  const saved = savedVariant({ ...preview, instantColor: undefined }, preview);
  expect(saved.instantColor).toBeUndefined();
  expect(saved.status).toBe("pending");
  expect(saved.svgPath).toBeUndefined();
});

it("retains errors but drops artwork when the matching render failed", () => {
  const editing = {
    id: "variant",
    tex: "x",
    kind: "inline",
    origin: "custom",
    status: "ready",
    svgPath: "old.svg",
  } as Card;
  const saved = savedVariant(editing, {
    ...editing,
    status: "failed",
    error: "Compile failed",
  });
  expect(saved).toMatchObject({ status: "failed", error: "Compile failed" });
  expect(saved.svgPath).toBeUndefined();
});
