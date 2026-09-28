import type { Anchor, SourceTarget } from "./types";

export function targetAtSource(
  targets: SourceTarget[],
  file: string,
  offset: number,
) {
  return targets.find(
    (t) => t.file === file && t.startUTF16 <= offset && offset <= t.endUTF16,
  );
}

export function targetsAtLine(
  targets: SourceTarget[],
  file: string,
  line: number,
) {
  return targets.filter(
    (t) => t.file === file && t.line <= line && line <= t.endLine,
  );
}

export function anchorAtPoint(
  anchors: Anchor[],
  page: number,
  x: number,
  y: number,
) {
  const onPage = anchors.filter((a) => a.page === page);
  const contained = onPage.filter(
    (a) =>
      (a.width || 0) > 0 &&
      (a.height || 0) > 0 &&
      x >= a.x - 3 &&
      x <= a.x + a.width! + 3 &&
      y >= a.y - 3 &&
      y <= a.y + a.height! + 3,
  );
  return (
    contained.length ? contained : onPage.filter((a) => Math.abs(a.y - y) <= 28)
  ).sort(
    (a, b) =>
      Math.abs(a.y - y) * 4 +
      Math.abs(a.x - x) -
      (Math.abs(b.y - y) * 4 + Math.abs(b.x - x)),
  )[0];
}

export function artworkBounds(anchor: Anchor) {
  return {
    x: anchor.x,
    y: anchor.height ? anchor.y : anchor.y - 18,
    width: anchor.width || 100,
    height: anchor.height || 30,
  };
}

export function artworkAtPoint(
  anchors: Anchor[],
  targets: SourceTarget[],
  page: number,
  x: number,
  y: number,
  format: string,
) {
  return anchors
    .filter((anchor) => {
      const target = targets.find((target) => target.id === anchor.targetID);
      if (!target || anchor.page !== page) return false;
      if (
        format === "latex" &&
        targetsAtLine(targets, target.file, target.line).length !== 1
      )
        return false;
      const bounds = artworkBounds(anchor);
      return (
        x >= bounds.x - 3 &&
        x <= bounds.x + bounds.width + 3 &&
        y >= bounds.y - 3 &&
        y <= bounds.y + bounds.height + 3
      );
    })
    .sort((a, b) => {
      const aa = artworkBounds(a),
        bb = artworkBounds(b);
      return aa.width * aa.height - bb.width * bb.height;
    })[0];
}
