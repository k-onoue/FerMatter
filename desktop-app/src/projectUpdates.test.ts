import { expect, it } from "vitest";
import { mergeScan, mergeRenderedCard } from "./projectUpdates";
import type { Project } from "./types";
it("restores hidden source names and sizes while preserving variants and labels", () => {
  const source = {
    id: "source",
    origin: "source",
    tex: "x",
    name: "My score",
    exportFontSize: 36,
  };
  const variant = {
    id: "variant",
    origin: "custom",
    tex: "styled",
    parentID: "source",
  };
  const current = {
    cards: [source, variant],
    sectionAssignments: { source: "results" },
  } as unknown as Project;
  const hidden = mergeScan(current, {
    cards: [],
    extractionHiddenCards: [{ ...source, name: "generated" }],
    extractionRevision: "new",
    outline: [],
    warnings: [],
    files: [],
  } as unknown as Project);
  expect(hidden.cards).toEqual([variant]);
  expect(hidden.extractionHiddenCards).toEqual([source]);
  expect(hidden.extractionRevision).toBe("new");
  const restored = mergeScan(hidden, {
    cards: [{ ...source, name: "generated" }],
    extractionHiddenCards: [],
    outline: [],
    warnings: [],
    files: [],
  } as unknown as Project);
  expect(restored.cards).toEqual([source, variant]);
  expect(restored.sectionAssignments).toEqual(current.sectionAssignments);
});
it("applies delayed extraction without replacing variants, labels, main or output settings", () => {
  const stale = {
    id: "p",
    main: "main.tex",
    cards: [{ id: "source", origin: "source", tex: "old" }],
    outline: [],
  } as unknown as Project;
  const current = {
    ...stale,
    main: "moved.tex",
    cards: [
      {
        id: "source",
        origin: "source",
        tex: "old",
        name: "Named",
        exportFontSize: 32,
      },
      { id: "new", origin: "custom", tex: "edited" },
    ],
    sections: ["latest"],
    exportFontSize: 48,
    sectionAssignments: { new: "latest" },
  } as unknown as Project;
  const merged = mergeScan(current, {
    ...stale,
    cards: [{ ...stale.cards[0], tex: "new source" }],
    outline: [{ id: "new heading" } as never],
  });
  expect(merged.cards.map((card) => card.tex)).toEqual([
    "new source",
    "edited",
  ]);
  expect(merged.cards[0].name).toBe("Named");
  expect(merged.cards[0].exportFontSize).toBe(32);
  expect(merged.main).toBe("moved.tex");
  expect(merged.sections).toEqual(["latest"]);
  expect(merged.exportFontSize).toBe(48);
  expect(merged.sectionAssignments).toEqual({ new: "latest" });
});

it("ignores late render data after a variant edit and keeps latest output size/name", () => {
  const current = {
    id: "variant",
    origin: "custom",
    tex: "new",
    kind: "block",
    name: "Latest",
    exportFontSize: 40,
  } as Project["cards"][number];
  expect(
    mergeRenderedCard(current, { ...current, tex: "old", svgPath: "old.svg" }),
  ).toBe(current);
  expect(
    mergeRenderedCard(current, {
      ...current,
      name: "Old",
      exportFontSize: 12,
      svgPath: "new.svg",
    }),
  ).toMatchObject({ name: "Latest", exportFontSize: 40, svgPath: "new.svg" });
});
