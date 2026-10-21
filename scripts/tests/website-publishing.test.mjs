import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, readFileSync, existsSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { tmpdir } from 'node:os';
import { prepareWebsite } from '../publish-website.mjs';
import { buildLegacySite } from '../build-legacy-site.mjs';
import { previewServer } from '../preview-site.mjs';

function fixture(t) {
  const root = mkdtempSync(path.join(tmpdir(), 'fermatter-publish-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const source = path.join(root, 'site/dist');
  const destination = path.join(root, 'public-repository');
  mkdirSync(source, { recursive: true });
  mkdirSync(destination);
  const write = (base, name, text) => {
    const file = path.join(base, name);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, text);
  };
  write(source, 'index.html', '<a href="/guide/automation/">MCP</a>');
  write(source, 'guide/automation/index.html', '<h1>MCP</h1>');
  write(source, 'downloads/sample.zip', 'Synthetic download');
  return { root, source, destination, write };
}

test('publishing replaces static output while preserving target repository controls', t => {
  const { source, destination, write } = fixture(t);
  for (const name of ['.git/config', '.github/workflows/pages.yml', 'README.md']) write(destination, name, 'Keep');
  write(destination, 'obsolete/index.html', 'Old page');
  prepareWebsite(source, destination, 'a'.repeat(40));
  for (const name of ['.git/config', '.github/workflows/pages.yml', 'README.md']) assert.equal(readFileSync(path.join(destination, name), 'utf8'), 'Keep');
  assert.equal(existsSync(path.join(destination, 'obsolete')), false);
  assert.equal(readFileSync(path.join(destination, 'guide/automation/index.html'), 'utf8'), '<h1>MCP</h1>');
  assert.deepEqual(JSON.parse(readFileSync(path.join(destination, 'deployment.json'), 'utf8')), { commit: 'a'.repeat(40) });
  assert.ok(existsSync(path.join(destination, '.nojekyll')));
});

test('invalid output is rejected before changing an existing deployment', t => {
  const { source, destination, write } = fixture(t);
  write(destination, 'index.html', 'Published');
  symlinkSync(path.join(destination, 'index.html'), path.join(source, 'escape.html'));
  assert.throws(() => prepareWebsite(source, destination, 'a'.repeat(40)), /Unsupported public asset/);
  assert.equal(readFileSync(path.join(destination, 'index.html'), 'utf8'), 'Published');
  rmSync(path.join(source, 'escape.html'));
  write(source, '.github/workflows/pages.yml', 'Unexpected control');
  assert.throws(() => prepareWebsite(source, destination, 'a'.repeat(40)), /repository controls/);
  assert.equal(readFileSync(path.join(destination, 'index.html'), 'utf8'), 'Published');
});

test('legacy pages redirect to matching new paths and keep download bytes unchanged', t => {
  const { root, source } = fixture(t);
  const output = buildLegacySite(root);
  assert.match(readFileSync(path.join(output, 'index.html'), 'utf8'), /url=https:\/\/fermatter.github.io\//);
  assert.match(readFileSync(path.join(output, 'guide/automation/index.html'), 'utf8'), /https:\/\/fermatter.github.io\/guide\/automation\//);
  assert.match(readFileSync(path.join(output, 'guide/automation/index.html'), 'utf8'), /location.search \+ location.hash/);
  assert.deepEqual(readFileSync(path.join(output, 'downloads/sample.zip')), readFileSync(path.join(source, 'downloads/sample.zip')));
});

test('root preview resolves organization-site links and still blocks escaping files', async t => {
  const { source, root } = fixture(t);
  writeFileSync(path.join(root, 'private.txt'), 'Private');
  symlinkSync(path.join(root, 'private.txt'), path.join(source, 'escape.txt'));
  const server = previewServer(source, '');
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise(resolve => { server.closeAllConnections(); server.close(resolve); }));
  const origin = `http://127.0.0.1:${server.address().port}`;
  assert.equal((await fetch(origin + '/')).status, 200);
  assert.equal((await fetch(origin + '/guide/automation/')).status, 200);
  assert.equal((await fetch(origin + '/escape.txt')).status, 404);
});
