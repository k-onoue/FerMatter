import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { version as osVersion, release as osRelease } from "node:os";
export function hashTree(root, prefix = "") {
  const entries = [];
  for (const name of readdirSync(join(root, prefix)).sort()) {
    const path = join(prefix, name);
    if (statSync(join(root, path)).isDirectory())
      entries.push(...hashTree(root, path));
    else
      entries.push({
        path,
        sha256: createHash("sha256")
          .update(readFileSync(join(root, path)))
          .digest("hex"),
      });
  }
  return entries;
}
export function assertCommitEvidence(report, commit) {
  if (
    report.status !== "passed" ||
    report.dirty !== false ||
    report.commit !== commit ||
    report.endingCommit !== commit
  )
    throw new Error(
      "Public packaging requires successful validation of this exact clean commit.",
    );
  for (const name of ["check", "clippy", "build", "audit", "publication"]) {
    const checks = report.checks?.filter((c) => c.name === name);
    if (checks?.length !== 1 || checks[0].status !== "passed")
      throw new Error(`Missing passing release check: ${name}`);
  }
}
export function assertEvidence(report, commit, app) {
  assertCommitEvidence(report, commit);
  if (JSON.stringify(report.appFiles) !== JSON.stringify(hashTree(app)))
    throw new Error("App differs from the validated bundle.");
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const [mode, ...args] = process.argv.slice(2);
  if (mode === "verify-commit") {
    const [report, commit] = args;
    assertCommitEvidence(JSON.parse(readFileSync(report, "utf8")), commit);
  } else if (mode === "verify") {
    const [report, commit, app] = args;
    assertEvidence(JSON.parse(readFileSync(report, "utf8")), commit, app);
  } else if (mode === "write") {
    const [folder, root, app, status, commit, dirty] = args;
    const run = (cmd, arguments_) => {
      try {
        return execFileSync(cmd, arguments_, {
          cwd: root,
          encoding: "utf8",
          stdio: ["ignore", "pipe", "pipe"],
        }).trim();
      } catch (error) {
        return `unavailable: ${error.message.split("\n")[0]}`;
      }
    };
    const config = JSON.parse(
      readFileSync(join(root, "desktop-app/src-tauri/tauri.conf.json"), "utf8"),
    );
    const checks = readFileSync(join(folder, "steps.tsv"), "utf8")
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => {
        const [name, status] = line.split("\t");
        return { name, status };
      });
    let appFiles = [];
    try {
      appFiles = hashTree(app);
    } catch {}
    const report = {
      schemaVersion: 1,
      status,
      commit,
      endingCommit: run("git", ["rev-parse", "HEAD"]),
      dirty: dirty !== "clean",
      generatedAt: new Date().toISOString(),
      version: config.version,
      build: config.bundle.macOS.bundleVersion,
      platform:
        process.platform === "darwin"
          ? run("sw_vers", [])
          : `${osVersion()} (${osRelease()})`,
      architecture: process.arch,
      checks,
      appFiles,
      tools: Object.fromEntries(
        [
          ["rustc", ["--version"]],
          ["node", ["--version"]],
          ["npm", ["--version"]],
          ["typst", ["--version"]],
          ["latexmk", ["-v"]],
          ["pdflatex", ["--version"]],
          ["dvisvgm", ["--version"]],
          ["pdfcrop", ["--version"]],
          ["synctex", ["--version"]],
          ["gs", ["--version"]],
        ].map(([tool, args]) => [tool, run(tool, args)]),
      ),
      manualValidation:
        "pending: complete dev-docs/reference/release-validation.md on the final distribution artifacts",
    };
    writeFileSync(
      join(folder, "report.json"),
      JSON.stringify(report, null, 2) + "\n",
    );
  } else throw new Error("Use write, verify or verify-commit.");
}
