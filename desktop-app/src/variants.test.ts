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
