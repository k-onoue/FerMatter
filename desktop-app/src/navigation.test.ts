import { expect, it } from "vitest";
import {
  anchorAtPoint,
  artworkAtPoint,
  targetAtSource,
  targetsAtLine,
} from "./navigation";
import type { SourceTarget } from "./types";
const targets = [
  {
    id: "a",
    file: "main.typ",
    line: 2,
    endLine: 2,
    startUTF16: 10,
    endUTF16: 14,
  },
  {
    id: "b",
    file: "main.typ",
    line: 2,
    endLine: 2,
    startUTF16: 22,
    endUTF16: 27,
  },
  {
    id: "c",
    file: "part.typ",
    line: 2,
    endLine: 5,
    startUTF16: 8,
    endUTF16: 45,
  },
] as SourceTarget[];
it("distinguishes two equations on one source line by their editor offsets", () => {
  expect(targetAtSource(targets, "main.typ", 24)?.id).toBe("b");
  expect(targetAtSource(targets, "main.typ", 19)).toBeUndefined();
  expect(targetAtSource(targets, "part.typ", 24)?.id).toBe("c");
});
it("keeps ambiguous SyncTeX lines explicit and includes multiline equations", () => {
  expect(targetsAtLine(targets, "main.typ", 2).map((t) => t.id)).toEqual([
    "a",
    "b",
  ]);
  expect(targetsAtLine(targets, "part.typ", 4).map((t) => t.id)).toEqual(["c"]);
});
it("prioritizes the clicked PDF line, rejects blank regions, and respects figure bounds", () => {
  const anchors = [
    { targetID: "a", page: 1, x: 100, y: 100 },
    { targetID: "b", page: 1, x: 150, y: 118 },
    { targetID: "figure", page: 2, x: 50, y: 50, width: 200, height: 200 },
  ];
  expect(anchorAtPoint(anchors, 1, 145, 101)?.targetID).toBe("a");
  expect(anchorAtPoint(anchors, 1, 145, 250)).toBeUndefined();
  expect(anchorAtPoint(anchors, 2, 150, 200)?.targetID).toBe("figure");
});
it("offers PDF artwork only inside a mapped target, excluding headings and distant prose", () => {
  const anchors = [
    { targetID: "a", page: 1, x: 100, y: 100 },
    { targetID: "heading", page: 1, x: 400, y: 100 },
    { targetID: "c", page: 2, x: 50, y: 50, width: 200, height: 200 },
  ];
  expect(artworkAtPoint(anchors, targets, 1, 110, 100, "typst")?.targetID).toBe(
    "a",
  );
  expect(
    artworkAtPoint(anchors, targets, 1, 600, 100, "typst"),
  ).toBeUndefined();
  expect(
    artworkAtPoint(anchors, targets, 1, 405, 100, "typst"),
  ).toBeUndefined();
  expect(artworkAtPoint(anchors, targets, 2, 240, 240, "typst")?.targetID).toBe(
    "c",
  );
  expect(
    artworkAtPoint(anchors, targets, 1, 240, 240, "typst"),
  ).toBeUndefined();
  // A SyncTeX line containing two equations cannot safely identify one image.
  expect(
    artworkAtPoint(anchors, targets, 1, 110, 100, "latex"),
  ).toBeUndefined();
  expect(artworkAtPoint(anchors, targets, 2, 240, 240, "latex")?.targetID).toBe(
    "c",
  );
});
