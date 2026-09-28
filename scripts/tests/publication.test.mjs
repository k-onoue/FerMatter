import assert from 'node:assert/strict';
import { test } from 'node:test';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { zipSync, unzipSync } from 'fflate';
import { buildGuides } from '../build-guides.mjs';
import { checkPublication } from '../check-publication.mjs';
import { DEMOS, prepareDemos } from '../prepare-demo.mjs';
import { previewServer } from '../preview-site.mjs';

function fixture(t) {
  const root = mkdtempSync(path.join(tmpdir(), 'mathlidelux-docs-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (name, text) => { const file = path.join(root, name); mkdirSync(path.dirname(file), { recursive: true }); writeFileSync(file, text); };
  const page = { source: 'README.md', slug: 'index', title: 'Guide', summary: 'Public guide' };
  write('user-docs/pages.json', JSON.stringify([page]));
  write('user-docs/README.md', '# Guide\n\n- First instruction.\n');
  write('docs/index.html', '<h1 id="download">Home</h1>');
  for (const [demo, names] of Object.entries(DEMOS)) for (const name of names) write(`examples/${demo}/${name}`, 'Synthetic sample');
  execFileSync('git', ['init', '-q', root]);
  prepareDemos(root);
  return { root, page, write, read: name => readFileSync(path.join(root, name), 'utf8') };
}

test('only manifest pages are generated; check mode never rewrites stale output', t => {
  const { root, write, read } = fixture(t);
  write('user-docs/private-draft.md', 'Confidential draft');
  assert.equal(buildGuides({ root }), 1);
  assert.deepEqual(readdirSync(path.join(root, 'docs/guide')), ['index.html']);
  const before = read('docs/guide/index.html');
  assert.ok(!before.includes('Confidential'));
  write('user-docs/README.md', '# Guide\n\nChanged');
  assert.throws(() => buildGuides({ root, check: true }), /Stale guides/);
  assert.equal(read('docs/guide/index.html'), before);
  buildGuides({ root });
  assert.equal(buildGuides({ root, check: true }), 1);
});

test('private links, escaped paths and symlink assets fail before changing output', t => {
  const { root, write, read } = fixture(t);
  buildGuides({ root });
  const before = read('docs/guide/index.html');
  write('dev-docs/private.md', 'Private');
  symlinkSync(path.join(root, 'dev-docs/private.md'), path.join(root, 'docs/escaped.md'));
  for (const link of ['../dev-docs/private.md', '../docs/%2e%2e/dev-docs/private.md', '../docs/escaped.md']) {
    write('user-docs/README.md', `# Guide\n\n[Private](${link})`);
    assert.throws(() => buildGuides({ root }), /not a public guide or asset/);
    assert.equal(read('docs/guide/index.html'), before);
  }
});

test('guide links, images, tables, code and heading IDs work under a Pages subpath', t => {
  const { root, page, write, read } = fixture(t);
  write('user-docs/pages.json', JSON.stringify([page, { ...page, source: 'next.md', slug: 'next', title: 'Next' }]));
  write('docs/sample.png', 'image');
  write('user-docs/next.md', '# Next\n\n## Details\n\n## Details');
  write('user-docs/README.md', '# Guide\n\n[Next](next.md?view=1#details) ![Sample](../docs/sample.png)\n\n| A | B |\n| - | - |\n| x | y |\n\n```sh\nnode --version\n```');
  buildGuides({ root });
  const html = read('docs/guide/index.html');
  for (const value of ['href="next.html?view=1#details"', 'src="../sample.png"', '<table>', '<pre><code class="language-sh"']) assert.ok(html.includes(value), value + "\n" + html);
  assert.ok(read('docs/guide/next.html').includes('id="details_1"'));
});

test('invalid or duplicate manifests, missing media and obsolete pages fail', t => {
  const { root, page, write } = fixture(t);
  for (const pages of [[page, page], [{ ...page, source: '../docs/index.html' }], [{ ...page, video: 'missing' }]]) {
    write('user-docs/pages.json', JSON.stringify(pages));
    assert.throws(() => buildGuides({ root }));
  }
  write('user-docs/pages.json', JSON.stringify([page]));
  write('docs/guide/old.html', 'Old page');
  assert.throws(() => buildGuides({ root }), /obsolete guide/);
});

test('demo ZIPs are deterministic and contain only current allowlisted files', t => {
  const { root, write } = fixture(t);
  write('examples/demo-tikz/private.tex', 'Not public');
  const zipPath = path.join(root, 'docs/downloads/demo-tikz.zip');
  const before = readFileSync(zipPath);
  prepareDemos(root);
  assert.deepEqual(readFileSync(zipPath), before);
  assert.deepEqual(Object.keys(unzipSync(before)), DEMOS['demo-tikz'].map(name => `demo-tikz/${name}`));
  assert.deepEqual(checkPublication(root).errors, []);
  write('examples/demo-tikz/main.tex', 'Changed');
  assert.ok(checkPublication(root).errors.some(error => error.includes('ZIP is stale')));
  writeFileSync(zipPath, zipSync({ ...unzipSync(before), 'private.txt': new Uint8Array([1]) }));
  assert.ok(checkPublication(root).errors.some(error => error.includes('only the allowlisted')));
});

test('symlink demo sources are rejected without changing existing ZIPs', t => {
  const { root, write } = fixture(t);
  const zipPath = path.join(root, 'docs/downloads/demo-tikz.zip');
  const before = readFileSync(zipPath);
  write('secret.tex', 'Private');
  const source = path.join(root, 'examples/demo-tikz/main.tex');
  rmSync(source);
  symlinkSync(path.join(root, 'secret.tex'), source);
  assert.throws(() => prepareDemos(root), /symlinks/);
  assert.deepEqual(readFileSync(zipPath), before);
});

test('publication checks include ignored tracked files, paths, credentials and broken links', t => {
  const { root, write } = fixture(t);
  write('.gitignore', 'target/\n');
  write('target/build.txt', 'generated');
  execFileSync('git', ['-C', root, 'add', '-f', 'target/build.txt']);
  write('bad.md', `[Missing](missing.md)\n/${'Users'}/example/file\n${'ghp_'}${'a'.repeat(32)}`);
  write('docs/bad.html', '<a href="index.html#missing">Bad</a><img src="../bad.md">');
  const errors = checkPublication(root).errors.join('\n');
  for (const expected of ['publication candidate: target/build.txt', 'Personal absolute path', 'Credential-like', 'Broken local Markdown', 'Broken page anchor', 'out-of-site']) assert.ok(errors.includes(expected), expected);
});

test('preview serves only public files and blocks symlink escapes and writes', async t => {
  const { root, write } = fixture(t);
  write('secret.txt', 'Private');
  symlinkSync(path.join(root, 'secret.txt'), path.join(root, 'docs/secret.txt'));
  const server = previewServer(path.join(root, 'docs'));
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise(resolve => { server.closeAllConnections(); server.close(resolve); }));
  const base = `http://127.0.0.1:${server.address().port}`;
  assert.equal((await fetch(base)).status, 200);
  for (const route of ['/secret.txt', '/..%2fsecret.txt', '/%E0%A4%A']) assert.equal((await fetch(base + route)).status, 404);
  assert.equal((await fetch(base, { method: 'POST' })).status, 405);
});
