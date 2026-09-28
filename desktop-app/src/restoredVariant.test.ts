import { expect, it } from "vitest";
import { initialInstant } from "./instant";
import { restoredVariant } from "./restoredVariant";
import type { Project } from "./types";

it("restores source/style/context into independent, uncompiled presentation variants", () => {
  const project = {
    id: "demo",
    main: "main.typ",
    cards: [{ id: "original", tex: "x" }],
  } as Project;
  const draft = {
    ...initialInstant(),
    source: "custom(x)",
    contextFile: "sections/model.typ",
    color: "#123abc",
    fontSize: 40,
    exportFormat: "pdf",
  };
  const before = JSON.stringify(project);
  const first = restoredVariant(draft, project);
  const second = restoredVariant(draft, project);
  expect(first.id).not.toBe(second.id);
  expect(first).toMatchObject({
    tex: "custom(x)",
    origin: "custom",
    status: "pending",
    requiresRender: true,
    restoredContextFile: "sections/model.typ",
    instantColor: "123abc",
    exportFontSize: 40,
    exportFormat: "pdf",
  });
  expect(first.svgPath).toBeUndefined();
  expect(JSON.stringify(project)).toBe(before);
  expect(
    restoredVariant({ ...draft, contextFile: undefined }, project)
      .restoredContextFile,
  ).toBe("main.typ");
});
