import { expect, it } from "vitest";
import {
  acceptsSVG,
  htmlSVGs,
  readSVGTransfer,
  type SVGTransfer,
} from "./svgTransfer";
const svg =
  '<svg xmlns="http://www.w3.org/2000/svg"><metadata>日本語</metadata></svg>';
const transfer = (value: Partial<SVGTransfer>): SVGTransfer => ({
  files: [],
  svg: "",
  html: "",
  text: "",
  ...value,
});

it("uses one representation and retains metadata, including Unicode", async () => {
  expect(
    await readSVGTransfer(transfer({ svg, html: `<b>${svg}</b>` })),
  ).toEqual([svg]);
  const encoded = btoa(String.fromCharCode(...new TextEncoder().encode(svg)));
  expect(htmlSVGs(`<img src="data:image/svg+xml;base64,${encoded}">`)).toEqual([
    svg,
  ]);
  expect(
    htmlSVGs(
      `<object data='data:image/svg+xml,${encodeURIComponent(svg)}'></object>`,
    ),
  ).toEqual([svg]);
  expect(await readSVGTransfer(transfer({ text: svg }))).toEqual([svg]);
  expect(
    await readSVGTransfer(
      transfer({
        files: [
          {
            name: "preview.png",
            type: "image/png",
            size: 4,
            text: async () => "PNG",
          },
        ],
        html: `<div>${svg}</div>`,
      }),
    ),
  ).toEqual([svg]);
});

it("does not consume Finder project drops or resolve external HTML images", () => {
  const folder = { name: "paper", type: "", size: 0, text: async () => "" };
  expect(acceptsSVG(transfer({ files: [folder] }))).toBe(false);
  expect(acceptsSVG(transfer({ text: "file:///private/source.svg" }))).toBe(
    false,
  );
  expect(
    htmlSVGs(
      '<img src="https://example.com/source.svg"><object data="file:///private/source.svg"></object>',
    ),
  ).toEqual([]);
  expect(htmlSVGs(`<div>${svg}</div>`)).toEqual([svg]);
});

it("bounds the batch before reading file content, and checks actual returned bytes", async () => {
  let reads = 0;
  const file = {
    name: "one.svg",
    type: "image/svg+xml",
    size: 16 * 1024 * 1024 + 1,
    text: async () => {
      reads++;
      return svg;
    },
  };
  await expect(readSVGTransfer(transfer({ files: [file] }))).rejects.toThrow(
    "16 MB",
  );
  expect(reads).toBe(0);
  await expect(
    readSVGTransfer(transfer({ files: Array(17).fill({ ...file, size: 1 }) })),
  ).rejects.toThrow("16 MB");
  expect(reads).toBe(0);
  await expect(
    readSVGTransfer(transfer({ svg: "日".repeat(6 * 1024 * 1024) })),
  ).rejects.toThrow("16 MB");
  expect(
    await readSVGTransfer(transfer({ files: [{ ...file, size: svg.length }] })),
  ).toEqual([svg]);
});

it("rejects malformed SVG data encodings rather than guessing at a source", () => {
  expect(() =>
    htmlSVGs('<img src="data:image/svg+xml;base64,%%%%">'),
  ).toThrow();
  expect(() => htmlSVGs('<img src="data:image/svg+xml,%XX">')).toThrow();
  expect(htmlSVGs("<svg ".repeat(100_000))).toEqual([]);
  expect(() => htmlSVGs(svg.repeat(17))).toThrow("16 MB");
});
