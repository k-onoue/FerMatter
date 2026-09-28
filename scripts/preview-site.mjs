#!/usr/bin/env node
// Dependency-free preview, restricted to the public site on loopback.
import { createServer } from 'node:http';
import { readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { ROOT, publicFile, isMain, run } from './lib/paths.mjs';

const types = { '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript', '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.mp4': 'video/mp4', '.vtt': 'text/vtt', '.zip': 'application/zip' };
export function previewServer(site = path.join(ROOT, 'docs')) {
  return createServer((req, res) => {
    if (!['GET', 'HEAD'].includes(req.method)) { res.writeHead(405, { Allow: 'GET, HEAD' }).end(); return; }
    try {
      let file = path.resolve(site, '.' + decodeURIComponent(new URL(req.url, 'http://localhost').pathname));
      if (statSync(file).isDirectory()) file = path.join(file, 'index.html');
      file = publicFile(site, file);
      const data = readFileSync(file);
      res.writeHead(200, { 'Content-Type': types[path.extname(file)] ?? 'application/octet-stream', 'Content-Length': data.length, 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff' });
      res.end(req.method === 'HEAD' ? undefined : data);
    } catch { res.writeHead(404).end('Not found'); }
  });
}
if (isMain(import.meta.url)) run(() => {
  const port = Number(process.argv[2] ?? 4174);
  if (process.argv.length > 3 || !Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Usage: node scripts/preview-site.mjs [port]');
  previewServer().on('error', error => { console.error(error.message); process.exitCode = 1; }).listen(port, '127.0.0.1', () => console.log(`Preview: http://127.0.0.1:${port}/ (docs/ only)`));
});
