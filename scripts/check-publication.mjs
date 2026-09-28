#!/usr/bin/env node
// Read-only publication checks; no uploads.
import { execFileSync } from 'node:child_process';
import { existsSync, lstatSync, readFileSync, readdirSync, realpathSync } from 'node:fs';
import path from 'node:path';
import { parse } from 'parse5';
import { unzipSync } from 'fflate';
import { DEMOS, demoSource } from './prepare-demo.mjs';
import { ROOT, inside, localLink, isMain, run } from './lib/paths.mjs';

function links(html) {
  const references = [], ids = new Set();
  const walk = node => {
    for (const { name, value } of node.attrs ?? []) {
      if (name === 'id') ids.add(value);
      if (['href', 'src', 'poster'].includes(name) && value) references.push(value);
    }
    for (const child of node.childNodes ?? []) walk(child);
    if (node.content) walk(node.content);
  };
  walk(parse(html));
  return { references, ids };
}

function htmlFiles(folder) {
  return readdirSync(folder, { withFileTypes: true }).flatMap(entry => {
    const file = path.join(folder, entry.name);
    return entry.isDirectory() ? htmlFiles(file) : entry.isFile() && file.endsWith('.html') ? [file] : [];
  });
}

export function checkPublication(root = ROOT) {
  const files = [...new Set(execFileSync('git', ['-C', root, 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], { encoding: 'utf8' }).split('\0').filter(Boolean))].sort();
  const errors = [];
  const texts = new Map();
  const decoder = new TextDecoder('utf-8', { fatal: true });
  for (const name of files) {
    const parts = name.split('/');
    const base = path.basename(name);
    if (['paper', '.private', 'ui-prototype'].includes(parts[0]) || parts.some(part => ['build', '.build', 'node_modules', '__pycache__', 'dist', '.venv-docs', 'target'].includes(part)) ||
        (base.startsWith('.env') && base !== '.env.example') || ['.p12', '.p8', '.mobileprovision', '.dmg', '.pkg'].includes(path.extname(name).toLowerCase())) {
      errors.push(`Private or generated file is a publication candidate: ${name}`);
    }
    const file = path.join(root, name);
    let stat;
    try { stat = lstatSync(file); } catch (error) { if (error.code === 'ENOENT') continue; throw error; }
    if (stat.isSymbolicLink()) { errors.push(`Review and remove symlink before publishing: ${name}`); continue; }
    if (!stat.isFile()) continue;
    let content;
    try { content = decoder.decode(readFileSync(file)); } catch (error) { if (error.code === 'ERR_ENCODING_INVALID_ENCODED_DATA') continue; throw error; }
    texts.set(name, content);
    if (/\/(?:Users|home)\/[A-Za-z0-9_.-]+\//.test(content)) errors.push(`Personal absolute path found: ${name}`);
    if (/-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/.test(content)) errors.push(`Private key found: ${name}`);
    if (/(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,})/.test(content)) errors.push(`Credential-like content found: ${name}`);
  }
  const site = realpathSync(path.join(root, 'docs'));
  for (const html of htmlFiles(site)) {
    const parsed = links(readFileSync(html, 'utf8'));
    for (const reference of parsed.references) {
      let url;
      try { url = localLink(reference); } catch { errors.push(`Invalid URL in ${html}: ${reference}`); continue; }
      if (!url) continue;
      if (!url.pathname) {
        if (url.fragment && !parsed.ids.has(url.fragment)) errors.push(`Broken anchor in ${path.basename(html)}: ${reference}`);
        continue;
      }
      if (url.pathname.startsWith('/')) { errors.push(`Use a relative URL for project Pages: ${reference}`); continue; }
      const target = path.resolve(path.dirname(html), url.pathname);
      if (!existsSync(target) || !inside(site, realpathSync(target))) {
        errors.push(`Missing or out-of-site asset in ${path.basename(html)}: ${reference}`);
      } else if (path.extname(target) === '.html' && url.fragment && !links(readFileSync(target, 'utf8')).ids.has(url.fragment)) {
        errors.push(`Broken page anchor in ${path.basename(html)}: ${reference}`);
      }
    }
  }
  for (const [name, text] of texts) {
    if (!name.endsWith('.md')) continue;
    const content = text.replace(/^```[^\n]*\n.*?^```[^\n]*$/gms, '');
    const references = [...content.matchAll(/\]\(([^\s)]+)(?:\s+"[^"]*")?\)/g), ...content.matchAll(/^\[[^\]]+\]:\s*(\S+)/gm)].map(match => match[1]);
    for (const reference of references) {
      let url;
      try { url = localLink(reference.replace(/^<|>$/g, '')); } catch { errors.push(`Invalid Markdown URL in ${name}: ${reference}`); continue; }
      if (!url?.pathname) continue;
      const target = path.resolve(root, path.dirname(name), url.pathname);
      if (!existsSync(target) || !inside(realpathSync(root), realpathSync(target))) errors.push(`Broken local Markdown link in ${name}: ${reference}`);
    }
  }
  for (const [demo, sources] of Object.entries(DEMOS)) {
    try {
      const names = [];
      const archive = unzipSync(readFileSync(path.join(site, 'downloads', `${demo}.zip`)), { filter: entry => { names.push(entry.name); return true; } });
      const expected = sources.map(name => `${demo}/${name}`);
      if (names.length !== expected.length || new Set(names).size !== expected.length || names.some(name => !expected.includes(name))) errors.push(`${demo} ZIP must contain only the allowlisted demo sources.`);
      for (const name of sources) {
        const data = archive[`${demo}/${name}`];
        if (!data || !demoSource(root, demo, name).equals(Buffer.from(data))) errors.push(`${demo} ZIP is stale: ${name}. Run node scripts/prepare-demo.mjs.`);
      }
    } catch (error) { errors.push(`${demo} ZIP: ${error.message}`); }
  }
  return { files: files.length, errors };
}

if (isMain(import.meta.url)) run(() => {
  const result = checkPublication();
  if (result.errors.length) throw new Error(result.errors.map(error => `ERROR: ${error}`).join('\n'));
  console.log(`Checked ${result.files} publication candidates, local Markdown/page links, and ${Object.keys(DEMOS).length} demo ZIPs.`);
  console.log('Private manuscripts, local builds, and dependencies are excluded. Also review git diff --cached before publishing.');
});
