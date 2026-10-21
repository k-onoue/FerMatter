#!/usr/bin/env node
// Publish reviewed static output only; never copy the application's Git history.
import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, lstatSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { ROOT, inside, isMain, run } from './lib/paths.mjs';

const controls = new Set(['.git', '.github', 'README.md', 'LICENSE']);
export function prepareWebsite(source, destination, commit) {
  if (!/^[a-f0-9]{40}$/.test(commit)) throw new Error('Expected a full source commit ID.');
  source = path.resolve(source);
  destination = path.resolve(destination);
  if (inside(source, destination) || inside(destination, source)) throw new Error('Publishing folders must be separate.');
  function validate(folder) {
    for (const entry of readdirSync(folder)) {
      const file = path.join(folder, entry);
      const stat = lstatSync(file);
      if (stat.isSymbolicLink() || (!stat.isFile() && !stat.isDirectory())) throw new Error(`Unsupported public asset: ${file}`);
      if (stat.isDirectory()) validate(file);
    }
  }
  validate(source);
  if (!existsSync(path.join(source, 'index.html'))) throw new Error('Build the public site before publishing.');
  for (const entry of readdirSync(source)) {
    if (controls.has(entry)) throw new Error(`Public output must not replace repository controls: ${entry}`);
  }
  for (const entry of readdirSync(destination)) {
    if (!controls.has(entry)) rmSync(path.join(destination, entry), { recursive: true, force: true });
  }
  for (const entry of readdirSync(source)) cpSync(path.join(source, entry), path.join(destination, entry), { recursive: true });
  writeFileSync(path.join(destination, '.nojekyll'), '');
  writeFileSync(path.join(destination, 'deployment.json'), JSON.stringify({ commit }) + '\n');
}

if (isMain(import.meta.url)) run(() => {
  const commit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim();
  const work = mkdtempSync(path.join(tmpdir(), 'fermatter-website-'));
  const git = args => execFileSync('git', args, { cwd: work, stdio: 'inherit' });
  try {
    git(['clone', '--branch', 'main', '--single-branch', 'git@github.com:fermatter/fermatter.github.io.git', '.']);
    prepareWebsite(path.join(ROOT, 'site/dist'), work, commit);
    git(['config', 'user.name', 'github-actions[bot]']);
    git(['config', 'user.email', '41898282+github-actions[bot]@users.noreply.github.com']);
    git(['add', '--all']);
    try { git(['diff', '--cached', '--quiet']); return; }
    catch (error) { if (error.status !== 1) throw error; }
    git(['commit', '-m', `Publish website (${commit.slice(0, 7)})`]);
    git(['push', 'origin', 'HEAD:main']);
  } finally { rmSync(work, { recursive: true, force: true }); }
});
