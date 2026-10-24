#!/usr/bin/env node
import { lstatSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { zipSync } from "fflate";
import { ROOT, publicFile, isMain, run } from "./lib/paths.mjs";

export const DEMOS = {
  "demo-tikz": ["README.md", "main.tex", "latexmkrc"],
  "demo-cetz": ["README.md", "main.typ"],
  "demo-paper": [
    "README.md",
    "main.tex",
    "latexmkrc",
    "sections/model.tex",
    "sections/fitting.tex",
    "sections/geometry.tex",
    "sections/summary.tex",
  ],
  "demo-typst": [
    "README.md",
    "main.typ",
    "symbols.typ",
    "sections/model.typ",
    "sections/fitting.typ",
  ],
};

export function demoSource(root, demo, name) {
  const folder = path.join(root, "examples", demo);
  const source = path.join(folder, name);
  let current = path.join(root, "examples");
  for (const part of [demo, ...name.split("/")]) {
    current = path.join(current, part);
    if (lstatSync(current).isSymbolicLink())
      throw new Error(`Demo source must not contain symlinks: ${current}`);
  }
  return readFileSync(publicFile(folder, source));
}

export function prepareDemos(root = ROOT) {
  const archives = Object.entries(DEMOS).map(([demo, names]) => {
    const entries = Object.fromEntries(
      names.map((name) => [
        `${demo}/${name}`,
        [
          demoSource(root, demo, name),
          {
            mtime: new Date(2026, 0, 1, 0, 0, 0),
            os: 3,
            attrs: 0o100644 << 16,
          },
        ],
      ]),
    );
    return [
      path.join(root, "docs/downloads", `${demo}.zip`),
      zipSync(entries, { level: 6 }),
    ];
  });
  for (const [output, data] of archives) {
    mkdirSync(path.dirname(output), { recursive: true });
    writeFileSync(output, data);
  }
  return archives.length;
}

if (isMain(import.meta.url))
  run(() => console.log(`Prepared ${prepareDemos()} allowlisted demo ZIPs.`));
