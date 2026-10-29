import assert from "node:assert/strict";
import { test } from "node:test";
import { execFileSync } from "node:child_process";
import {
  cpSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { zipSync, unzipSync } from "fflate";
import { guideLink } from "../lib/guide-links.mjs";
import { checkPublication } from "../check-publication.mjs";
import { releaseDownloadUrl } from "../verify-release-download.mjs";
import { DEMOS, prepareDemos } from "../prepare-demo.mjs";
import { previewServer } from "../preview-site.mjs";
import { symlinkOrSkip } from "./symlink-helper.mjs";

function fixture(t) {
  const root = mkdtempSync(path.join(tmpdir(), "mathlidelux-docs-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (name, text) => {
    const file = path.join(root, name);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, text);
  };
  const page = "README.md";
  write("user-docs/pages.json", JSON.stringify([page]));
  write("user-docs/README.md", "# Guide\n\n- First instruction.\n");
  write("site/dist/index.html", '<h1 id="download">Home</h1>');
  write(
    "site/src/data/site.json",
    JSON.stringify({ base: "/FerMatter", releaseAvailable: false }),
  );
  write(".gitignore", "site/dist/\n");
  mkdirSync(path.join(root, "site/public"), { recursive: true });
  for (const [demo, names] of Object.entries(DEMOS))
    for (const name of names)
      write(`examples/${demo}/${name}`, "Synthetic sample");
  execFileSync("git", ["init", "-q", root]);
  prepareDemos(root);
  cpSync(
    path.join(root, "docs/downloads"),
    path.join(root, "site/dist/downloads"),
    { recursive: true },
  );
  return {
    root,
    page,
    write,
    read: (name) => readFileSync(path.join(root, name), "utf8"),
  };
}

test("macOS download links to separate checksum and signing notes", (t) => {
  const { root, write } = fixture(t);
  const config = {
    base: "/FerMatter",
    releaseAvailable: true,
    releaseRepository: "example/releases",
    releaseTag: "v1.0-preview.1",
    assetName: "FerMatter.dmg",
    releaseSha256: "a".repeat(64),
    releaseNotarized: false,
  };
  write("site/src/data/site.json", JSON.stringify(config));
  const notesUrl = "/FerMatter/download/macos-notes/";
  const notesPath = "site/dist/download/macos-notes/index.html";
  const good = `<section id="mac"><a href="${releaseDownloadUrl(config)}">Download DMG</a><a href="${notesUrl}">Download verification and opening notes</a></section>`;
  const notes = `<p>Preview is not notarized by Apple. SHA-256: <code>${config.releaseSha256}</code></p>`;
  write("site/dist/download/index.html", good);
  assert.match(
    checkPublication(root).errors.join("\n"),
    /no verification notes/,
  );

  write(notesPath, notes);
  assert.deepEqual(checkPublication(root).errors, []);

  write("site/dist/download/index.html", good.replace(notesUrl, "#mac"));
  assert.match(
    checkPublication(root).errors.join("\n"),
    /must link to its verification notes/,
  );

  write(
    "site/dist/download/index.html",
    good.replace(releaseDownloadUrl(config), "https://example.com/other.dmg"),
  );
  write(
    notesPath,
    notes
      .replace(config.releaseSha256, "b".repeat(64))
      .replace("not notarized by Apple", "ready"),
  );
  const errors = checkPublication(root).errors.join("\n");
  assert.match(errors, /download link differs/);
  assert.match(errors, /reviewed SHA-256/);
  assert.match(errors, /Unnotarized macOS preview/);
});

test("malformed macOS release config cannot bypass publication checks", (t) => {
  const { root, write } = fixture(t);
  const config = {
    base: "/FerMatter",
    releaseAvailable: true,
    releaseNotarized: false,
    releaseRepository: "example/releases",
    releaseTag: "v1.0-preview.1",
    assetName: "FerMatter.dmg",
    releaseSha256: "a".repeat(64),
  };
  write("site/dist/download/index.html", '<section id="mac"></section>');
  for (const invalid of [
    { releaseAvailable: "false" },
    { releaseNotarized: "false" },
    { assetName: "notes.txt" },
  ]) {
    write("site/src/data/site.json", JSON.stringify({ ...config, ...invalid }));
    assert.match(checkPublication(root).errors.join("\n"), /Boolean|DMG asset/);
  }
});

test("only allowlisted guide links and public assets can be published", (t) => {
  const { root, write } = fixture(t);
  const source = path.join(root, "user-docs/README.md");
  write("user-docs/pages.json", JSON.stringify(["README.md", "next.md"]));
  write("user-docs/next.md", "# Next");
  write("site/public/image.png", "image");
  assert.equal(
    guideLink("next.md?view=1#details", source, { root }),
    "/FerMatter/guide/next/?view=1#details",
  );
  assert.equal(guideLink("README.md", source, { root }), "/FerMatter/guide/");
  assert.equal(
    guideLink("../site/public/image.png", source, { root }),
    "/FerMatter/image.png",
  );
  assert.equal(
    guideLink("../docs/downloads/demo-cetz.zip", source, { root }),
    "/FerMatter/downloads/demo-cetz.zip",
  );
  assert.equal(
    guideLink("https://example.com/", source, { root }),
    "https://example.com/",
  );
  write("dev-docs/private.md", "Private");
  write("user-docs/private.md", "Private");
  if (
    !symlinkOrSkip(
      t,
      path.join(root, "dev-docs/private.md"),
      path.join(root, "site/public/escape.md"),
    )
  )
    return;
  for (const link of [
    "private.md",
    "../dev-docs/private.md",
    "../site/public/%2e%2e/%2e%2e/dev-docs/private.md",
    "../site/public/escape.md",
  ]) {
    assert.throws(() => guideLink(link, source, { root }));
  }
});

test("demo ZIPs are deterministic and contain only current allowlisted files", (t) => {
  const { root, write } = fixture(t);
  write("examples/demo-tikz/private.tex", "Not public");
  const zipPath = path.join(root, "docs/downloads/demo-tikz.zip");
  const before = readFileSync(zipPath);
  prepareDemos(root);
  assert.deepEqual(readFileSync(zipPath), before);
  assert.deepEqual(
    Object.keys(unzipSync(before)),
    DEMOS["demo-tikz"].map((name) => `demo-tikz/${name}`),
  );
  assert.deepEqual(checkPublication(root).errors, []);
  write("site/dist/downloads/private.zip", "Not a public sample");
  assert.ok(
    checkPublication(root).errors.some((error) =>
      error.includes("Published downloads"),
    ),
  );
  rmSync(path.join(root, "site/dist/downloads/private.zip"));
  write("examples/demo-tikz/main.tex", "Changed");
  assert.ok(
    checkPublication(root).errors.some((error) =>
      error.includes("ZIP is stale"),
    ),
  );
  writeFileSync(
    zipPath,
    zipSync({ ...unzipSync(before), "private.txt": new Uint8Array([1]) }),
  );
  assert.ok(
    checkPublication(root).errors.some((error) =>
      error.includes("only the allowlisted"),
    ),
  );
});

test("symlink demo sources are rejected without changing existing ZIPs", (t) => {
  const { root, write } = fixture(t);
  const zipPath = path.join(root, "docs/downloads/demo-tikz.zip");
  const before = readFileSync(zipPath);
  write("secret.tex", "Private");
  const source = path.join(root, "examples/demo-tikz/main.tex");
  rmSync(source);
  if (!symlinkOrSkip(t, path.join(root, "secret.tex"), source)) return;
  assert.throws(() => prepareDemos(root), /symlinks/);
  assert.deepEqual(readFileSync(zipPath), before);
});

test("publication checks include ignored tracked files, paths, credentials and broken links", (t) => {
  const { root, write } = fixture(t);
  write(".gitignore", "target/\n");
  write("target/build.txt", "generated");
  execFileSync("git", ["-C", root, "add", "-f", "target/build.txt"]);
  write(
    "bad.md",
    `[Missing](missing.md)\n/${"Users"}/example/file\n${"ghp_"}${"a".repeat(32)}`,
  );
  write(
    "site/dist/bad.html",
    '<a href="/FerMatter/#missing">Bad</a><img src="../bad.md"><a href="/outside/">Bad</a>',
  );
  const errors = checkPublication(root).errors.join("\n");
  for (const expected of [
    "publication candidate: target/build.txt",
    "Personal absolute path",
    "Credential-like",
    "Broken local Markdown",
    "Broken page anchor",
    "out-of-site",
  ])
    assert.ok(errors.includes(expected), expected);
});

test("published download totals, release counts and date must match the snapshot", (t) => {
  const { root, write } = fixture(t);
  const unavailable =
    '<section id="statistics">No public installer statistics yet.</section>';
  write("site/dist/download/index.html", unavailable);
  assert.deepEqual(checkPublication(root).errors, []);
  const snapshot = {
    schemaVersion: 1,
    repository: "example/releases",
    updatedAt: "2026-10-21T12:00:00.000Z",
    assets: [
      {
        id: 1,
        name: "FerMatter.dmg",
        version: "v1",
        platform: "macOS",
        downloads: 1234,
      },
      {
        id: 2,
        name: "FerMatter.pkg",
        version: "v1",
        platform: "macOS",
        downloads: 56,
      },
      {
        id: 3,
        name: "FerMatter.dmg",
        version: "v2-preview",
        platform: "macOS",
        downloads: 0,
      },
    ],
  };
  write("site/dist/stats/downloads.json", JSON.stringify(snapshot));
  assert.ok(
    checkPublication(root).errors.some((error) =>
      error.includes("total does not match"),
    ),
  );
  const rendered =
    '<section id="statistics"><div class="stats-total">1,290</div><time datetime="2026-10-21T12:00:00.000Z">2026-10-21 UTC</time><table><thead><tr><th>Release</th><th>Downloads</th></tr></thead><tbody><tr><th>v2-preview</th><td>0</td></tr><tr><th>v1</th><td>1,290</td></tr></tbody></table></section>';
  write("site/dist/download/index.html", rendered);
  assert.deepEqual(checkPublication(root).errors, []);
  write(
    "site/dist/download/index.html",
    rendered
      .replace('class="stats-total">1,290', 'class="stats-total">0')
      .replace("<td>0</td>", "<td>1</td>")
      .replace(">2026-10-21 UTC", ">2026-10-20 UTC"),
  );
  const errors = checkPublication(root).errors.join("\n");
  for (const expected of [
    "total does not match",
    "release counts do not match",
    "timestamp does not match",
  ])
    assert.ok(errors.includes(expected), expected);
  rmSync(path.join(root, "site/dist/stats/downloads.json"));
  write("site/dist/download/index.html", rendered);
  assert.ok(
    checkPublication(root).errors.some((error) =>
      error.includes("no snapshot"),
    ),
  );
});

test("preview serves only public files and blocks symlink escapes and writes", async (t) => {
  const { root, write } = fixture(t);
  write("secret.txt", "Private");
  if (
    !symlinkOrSkip(
      t,
      path.join(root, "secret.txt"),
      path.join(root, "site/dist/secret.txt"),
    )
  )
    return;
  const server = previewServer(path.join(root, "site/dist"), "/FerMatter");
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  t.after(
    () =>
      new Promise((resolve) => {
        server.closeAllConnections();
        server.close(resolve);
      }),
  );
  const base = `http://127.0.0.1:${server.address().port}`;
  assert.equal((await fetch(base + "/FerMatter/")).status, 200);
  assert.equal((await fetch(base + "/")).status, 404);
  for (const route of [
    "/FerMatter/secret.txt",
    "/FerMatter/..%2fsecret.txt",
    "/%E0%A4%A",
  ])
    assert.equal((await fetch(base + route)).status, 404);
  assert.equal((await fetch(base, { method: "POST" })).status, 405);
});
