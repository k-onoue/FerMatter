import { describe, it, expect } from "vitest";
import { leaf, leaves, removePane, update, type Pane } from "./panes";
describe("editor groups", () => {
  it("preserves tabs in mixed horizontal and vertical splits", () => {
    const original = leaf("main.typ");
    original.tabs.push("symbols.typ");
    const next = update(original, original.id, (p) => ({
      id: "horizontal",
      direction: "row",
      ratio: 0.5,
      children: [
        p,
        {
          id: "vertical",
          direction: "column",
          ratio: 0.4,
          children: [leaf("main.typ"), leaf("sections/model.typ")],
        },
      ],
    }));
    expect(leaves(next).map((p) => p.active)).toEqual([
      "main.typ",
      "main.typ",
      "sections/model.typ",
    ]);
    expect(leaves(next)[0].tabs).toEqual(["main.typ", "symbols.typ"]);
  });
  it("closing a nested pane collapses only its parent", () => {
    const a = leaf("main.tex"),
      b = leaf("a.tex"),
      c = leaf("b.tex");
    const layout: Pane = {
      id: "row",
      direction: "row",
      ratio: 0.6,
      children: [
        a,
        { id: "column", direction: "column", ratio: 0.3, children: [b, c] },
      ],
    };
    const result = removePane(layout, b.id)!;
    expect(leaves(result).map((p) => p.active)).toEqual(["main.tex", "b.tex"]);
    expect("children" in result && result.ratio).toBe(0.6);
    expect(removePane(a, a.id)).toBeNull();
  });
  it("updates one group without changing another group showing the same file", () => {
    const a = leaf("main.typ"),
      b = leaf("main.typ");
    const layout: Pane = {
      id: "row",
      direction: "row",
      ratio: 0.5,
      children: [a, b],
    };
    const changed = update(layout, a.id, (p) => ({
      ...p,
      tabs: [...p.tabs, "symbols.typ"],
      active: "symbols.typ",
    }));
    expect(leaves(changed).map((p) => p.active)).toEqual([
      "symbols.typ",
      "main.typ",
    ]);
    expect(leaves(layout).map((p) => p.active)).toEqual([
      "main.typ",
      "main.typ",
    ]);
  });
});
