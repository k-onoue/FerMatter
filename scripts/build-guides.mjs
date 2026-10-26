#!/usr/bin/env node
// Compatibility entry point: guides now build with the whole Astro site.
import { execFileSync } from "node:child_process";
import path from "node:path";
import { ROOT } from "./lib/paths.mjs";
// Windows cannot spawn npm.cmd directly without a shell. Run the npm CLI with
// the selected Node executable so paths containing spaces stay ordinary args.
const npm = process.platform === "win32" ? process.execPath : "npm";
const npmArgs =
  process.platform === "win32"
    ? [
        path.join(
          path.dirname(process.execPath),
          "node_modules/npm/bin/npm-cli.js",
        ),
      ]
    : [];
if (process.argv.slice(2).some((arg) => arg !== "--check"))
  throw new Error("Usage: node scripts/build-guides.mjs [--check]");
if (process.argv.includes("--check"))
  execFileSync(npm, [...npmArgs, "--prefix", "site", "run", "check"], {
    cwd: ROOT,
    stdio: "inherit",
  });
execFileSync(npm, [...npmArgs, "--prefix", "site", "run", "build"], {
  cwd: ROOT,
  stdio: "inherit",
});
