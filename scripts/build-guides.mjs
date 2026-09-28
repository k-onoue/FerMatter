#!/usr/bin/env node
// Only the explicit user-guide manifest may be published.
import { existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import MarkdownIt from 'markdown-it';
import { ROOT, publicFile, localLink, isMain, run } from './lib/paths.mjs';

function markdown(source, output, targets, site) {
  const md = new MarkdownIt(); // Raw HTML is text, so it cannot bypass link validation.
  const rewrite = value => {
    const url = localLink(value);
    if (!url?.pathname) return value;
    let target = path.resolve(path.dirname(source), url.pathname);
    try {
      target = targets.get(target) ?? publicFile(site, target);
    } catch {
      throw new Error(`${path.basename(source)}: link is not a public guide or asset: ${value}`);
    }
    return path.relative(path.dirname(output), target).split(path.sep).map(encodeURIComponent).join('/') + url.suffix;
  };
  for (const [type, attribute] of [['link_open', 'href'], ['image', 'src']]) {
    const original = md.renderer.rules[type] ?? ((tokens, i, options, env, renderer) => renderer.renderToken(tokens, i, options));
    md.renderer.rules[type] = (tokens, i, ...args) => {
      const value = tokens[i].attrGet(attribute);
      if (value) tokens[i].attrSet(attribute, rewrite(value));
      return original(tokens, i, ...args);
    };
  }
  md.core.ruler.push('heading_ids', state => {
    const ids = new Set();
    state.tokens.forEach((token, i) => {
      if (token.type !== 'heading_open') return;
      const text = state.tokens[i + 1].children.map(child => child.content).join('');
      const base = text.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').replace(/[^\w\s-]/g, '').trim().toLowerCase().replace(/[\s-]+/g, '-');
      let id = base;
      for (let n = 1; ids.has(id); n++) id = `${base}_${n}`;
      ids.add(id);
      token.attrSet('id', id);
    });
  });
  return md;
}

function videoBlock(name, site) {
  if (!/^[a-z0-9-]+$/.test(name)) throw new Error(`Invalid media name: ${name}`);
  for (const suffix of ['.mp4', '.jpg', '.vtt']) publicFile(site, path.join(site, 'assets/tutorials', name + suffix));
  return `<figure class="tutorial">
      <video controls playsinline preload="none" width="1440" height="920" poster="../assets/tutorials/${name}.jpg" aria-label="Three-step demonstration using Mathlidelux and PowerPoint">
        <source src="../assets/tutorials/${name}.mp4" type="video/mp4">
        <track kind="captions" src="../assets/tutorials/${name}.vtt" srclang="en" label="English">
        <a href="../assets/tutorials/${name}.mp4">Download the demonstration video</a>.
      </video>
      <figcaption>18-second step-through · earlier Swift prototype · the same steps are listed below.</figcaption>
    </figure>`;
}

export function buildGuides({ root = ROOT, check = false } = {}) {
  root = realpathSync(root);
  const sourceDir = path.join(root, 'user-docs');
  const site = path.join(root, 'docs');
  const outputDir = path.join(site, 'guide');
  const pages = JSON.parse(readFileSync(path.join(sourceDir, 'pages.json'), 'utf8'));
  const targets = new Map();
  const slugs = new Set();
  for (const page of pages) {
    const source = publicFile(sourceDir, path.resolve(sourceDir, page.source));
    if (path.extname(source) !== '.md' || !/^[a-z0-9-]+$/.test(page.slug) || slugs.has(page.slug) || targets.has(source)) {
      throw new Error(`Invalid or duplicate page: ${page.slug}`);
    }
    if (typeof page.title !== 'string' || typeof page.summary !== 'string') throw new Error('Missing page title or summary');
    targets.set(source, path.join(outputDir, `${page.slug}.html`));
    slugs.add(page.slug);
  }
  const template = readFileSync(new URL('./guide-template.html', import.meta.url), 'utf8');
  // Validate and render every page before writing any output.
  const generated = pages.map(page => {
    const source = publicFile(sourceDir, path.resolve(sourceDir, page.source));
    const output = targets.get(source);
    const [heading, ...body] = readFileSync(source, 'utf8').split('\n');
    if (heading !== `# ${page.title}`) throw new Error(`${page.source}: first heading must match the manifest title`);
    const md = markdown(source, output, targets, site);
    const values = {
      source: md.utils.escapeHtml(page.source), title: md.utils.escapeHtml(page.title),
      summary: md.utils.escapeHtml(page.summary), content: md.render(body.join('\n')).trimEnd(),
      media: page.video ? videoBlock(page.video, site) : '',
    };
    return [output, template.replace(/\{\{(source|title|summary|content|media)\}\}/g, (_, key) => values[key])];
  });
  const extras = existsSync(outputDir) ? readdirSync(outputDir).filter(name => name.endsWith('.html') && !slugs.has(name.slice(0, -5))) : [];
  if (extras.length) throw new Error(`Remove obsolete guide output explicitly: ${extras.join(', ')}`);
  const stale = generated.filter(([file, content]) => !existsSync(file) || readFileSync(file, 'utf8') !== content);
  if (check && stale.length) throw new Error(`Stale guides; run node scripts/build-guides.mjs: ${stale.map(([file]) => path.basename(file)).join(', ')}`);
  if (!check) {
    mkdirSync(outputDir, { recursive: true });
    for (const [file, content] of generated) writeFileSync(file, content);
  }
  return generated.length;
}

if (isMain(import.meta.url)) run(() => {
  if (process.argv.slice(2).some(arg => arg !== '--check')) throw new Error('Usage: node scripts/build-guides.mjs [--check]');
  const check = process.argv.includes('--check');
  console.log(`${check ? 'Checked' : 'Built'} ${buildGuides({ check })} public guides from the explicit Markdown manifest.`);
});
