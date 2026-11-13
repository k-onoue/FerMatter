import test from "node:test";
import assert from "node:assert/strict";
import {
  mkdtemp,
  mkdir,
  copyFile,
  writeFile,
  readFile,
  rm,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

test(
  "audit keeps caller-relative reports and audits lockfiles without npm install",
  { skip: process.platform === "win32" },
  async () => {
    const root = await mkdtemp(join(tmpdir(), "fermatter-audit-"));
    try {
      const app = join(root, "desktop-app");
      const scripts = join(app, "scripts");
      const bin = join(app, "build", "audit-tools", "bin");
      await mkdir(scripts, { recursive: true });
      await mkdir(bin, { recursive: true });
      for (const file of ["audit.sh", "environment.sh"]) {
        await copyFile(new URL(file, import.meta.url), join(scripts, file));
      }
      await writeFile(
        join(bin, "npm"),
        `#!/bin/bash
set -eu
[[ "$*" == 'audit --omit=dev --json' ]] || exit 90
printf '{"metadata":{"vulnerabilities":{"total":0}}}\\n'
`,
        { mode: 0o755 },
      );
      await writeFile(
        join(bin, "cargo"),
        `#!/bin/bash
set -eu
[[ "$1" == audit ]] || exit 91
if [[ "$2" == --version ]]; then echo 'cargo-audit fixture'; exit 0; fi
[[ "$*" == 'audit --file src-tauri/Cargo.lock --json' ]] || exit 92
if [[ "$AUDIT_FIXTURE_FAIL" == 1 ]]; then
  echo '{"vulnerabilities":{"found":true,"count":1}}'
  exit 1
fi
echo '{"vulnerabilities":{"found":false,"count":0}}'
`,
        { mode: 0o755 },
      );
      const args = [join(scripts, "audit.sh"), "reports/candidate"];
      const passed = spawnSync("bash", args, {
        cwd: root,
        encoding: "utf8",
        env: { ...process.env, AUDIT_FIXTURE_FAIL: "0" },
      });
      assert.equal(passed.status, 0, passed.stderr);
      const reports = join(root, "reports", "candidate");
      assert.equal(
        JSON.parse(await readFile(join(reports, "rust.json"), "utf8"))
          .vulnerabilities.count,
        0,
      );
      assert.equal(
        JSON.parse(await readFile(join(reports, "npm.json"), "utf8")).metadata
          .vulnerabilities.total,
        0,
      );

      const failed = spawnSync("bash", args, {
        cwd: root,
        encoding: "utf8",
        env: { ...process.env, AUDIT_FIXTURE_FAIL: "1" },
      });
      assert.equal(failed.status, 1);
      assert.ok(failed.stderr.includes(reports));
      assert.equal(
        JSON.parse(await readFile(join(reports, "rust.json"), "utf8"))
          .vulnerabilities.count,
        1,
      );
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  },
);
