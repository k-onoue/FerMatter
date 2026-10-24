import { writeFileSync } from "node:fs";
import pages from "../../user-docs/pages.json" with { type: "json" };
import config from "../src/data/site.json" with { type: "json" };

// Emit real .html files: Astro's trailing-slash redirect routes produce directories.
for (const name of pages) {
  if (name === "README.md") continue; // /guide/index.html already serves the guide.
  if (!/^[a-z0-9-]+\.md$/.test(name))
    throw new Error(`Invalid public guide name: ${name}`);
  const slug = name.replace(/\.md$/, "");
  const target = `${config.base}/guide/${slug}/`;
  writeFileSync(
    new URL(`../dist/guide/${slug}.html`, import.meta.url),
    `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="robots" content="noindex"><meta http-equiv="refresh" content="0;url=${target}"><link rel="canonical" href="${config.url}${target}"><title>Page moved</title><a href="${target}">Continue to the guide</a></html>`,
  );
}
