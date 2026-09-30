import { describe, expect, it } from "vitest";
import { mergeAutomationVariants } from "./automation";
import type { Project } from "./types";
const project = (generation: number, cards: unknown[], fields = {}) =>
  ({
    id: "p",
    cards,
    automationGeneration: generation,
    ...fields,
  }) as unknown as Project;
describe("API additions", () => {
  it("preserves unsaved edits and receives additions from reordered events", () => {
    const original = { id: "a", tex: "unsaved" };
    const one = { id: "api1", automationGeneration: 1 };
    const two = { id: "api2", automationGeneration: 2 };
    const current = project(0, [original], { libraryDraft: { tex: "draft" } });
    const updated = mergeAutomationVariants(
      current,
      project(2, [{ id: "a", tex: "saved" }, one, two]),
    );
    expect(updated.cards).toEqual([original, one, two]);
    expect(updated.libraryDraft).toEqual(current.libraryDraft);
    expect(mergeAutomationVariants(updated, project(1, [one]))).toBe(updated);
  });
  it("does not resurrect already seen deletions or add another project's cards", () => {
    const current = project(1, []);
    expect(
      mergeAutomationVariants(
        current,
        project(2, [
          { id: "old", automationGeneration: 1 },
          { id: "new", automationGeneration: 2 },
        ]),
      ).cards.map((c) => c.id),
    ).toEqual(["new"]);
    expect(
      mergeAutomationVariants(current, { ...project(2, []), id: "other" }),
    ).toBe(current);
  });
});
