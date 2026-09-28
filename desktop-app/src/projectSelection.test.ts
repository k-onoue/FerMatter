import { describe, it, expect } from "vitest";
import { projectSelection } from "./projectSelection";
const plain = { shiftKey: false, metaKey: false, ctrlKey: false };
describe("project selection", () => {
  it("toggles with Command and ranges in visible order", () => {
    const ids = ["c", "a", "b", "d"];
    expect([
      ...projectSelection(ids, new Set(["c"]), "b", "c", {
        ...plain,
        shiftKey: true,
      }),
    ]).toEqual(["c", "a", "b"]);
    expect([
      ...projectSelection(ids, new Set(["c"]), "c", "c", {
        ...plain,
        metaKey: true,
      }),
    ]).toEqual([]);
    expect([
      ...projectSelection(ids, new Set(["d"]), "a", "missing", {
        ...plain,
        shiftKey: true,
      }),
    ]).toEqual(["a"]);
    expect([
      ...projectSelection(ids, new Set(["d"]), "a", "c", {
        ...plain,
        shiftKey: true,
        metaKey: true,
      }),
    ]).toEqual(["d", "c", "a"]);
  });
});
