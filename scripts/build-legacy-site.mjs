#!/usr/bin/env node
import { cpSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { ROOT, isMain, run } from "./lib/paths.mjs";
import config from "../site/src/data/site.json" with { type: "json" };

// Keep old public assets/downloads and redirect every old page to its new path.
export function buildLegacySite(root = ROOT) {
  const source = path.join(root, "site/dist");
  const output = path.join(root, "site/legacy-dist");
  rmSync(output, { recursive: true, force: true });
  mkdirSync(output, { recursive: true });
  cpSync(source, output, { recursive: true });
  function redirect(folder) {
    for (const entry of readdirSync(folder, { withFileTypes: true })) {
      const file = path.join(folder, entry.name);
      if (entry.isDirectory()) redirect(file);
      else if (entry.isFile() && entry.name.endsWith(".html")) {
        const relative = path
          .relative(output, file)
          .split(path.sep)
          .join("/")
          .replace(/(^|\/)index\.html$/, "$1");
        const target = new URL(relative, config.url + "/").href;
        writeFileSync(
          file,
          `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="robots" content="noindex"><meta http-equiv="refresh" content="0;url=${target}"><link rel="canonical" href="${target}"><title>Page moved</title><a href="${target}">Continue to FerMatter</a><script>location.replace(${JSON.stringify(target)} + location.search + location.hash)</script></html>`,
        );
      }
    }
  }
  redirect(output);
  return output;
}
if (isMain(import.meta.url))
  run(() => console.log(`Legacy site: ${buildLegacySite()}`));
