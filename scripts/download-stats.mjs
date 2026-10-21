#!/usr/bin/env node
// Build-time GitHub counters, never browser analytics. Writes only after every page succeeds.
import { readFileSync, mkdirSync, renameSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { ROOT, isMain } from './lib/paths.mjs';

export const installer = name => /^(?:FerMatter|Mathlidelux)[\w.+ -]*\.(?:dmg|pkg|msi|exe|deb|rpm|appimage)$/i.test(name);
const platform = name => /\.(dmg|pkg)$/i.test(name) ? 'macOS' : /\.(exe|msi)$/i.test(name) ? 'Windows' : 'Linux';
export function validateSnapshot(value) {
  if (value?.schemaVersion !== 1 || !/^[\w.-]+\/[\w.-]+$/.test(value.repository) ||
      !Number.isFinite(Date.parse(value.updatedAt)) || !Array.isArray(value.assets) ||
      value.assets.some(a => !Number.isSafeInteger(a.id) || a.id < 1 || !installer(a.name) ||
        typeof a.version !== 'string' || !['macOS', 'Windows', 'Linux'].includes(a.platform) ||
        !Number.isSafeInteger(a.downloads) || a.downloads < 0) ||
      new Set(value.assets.map(a => a.id)).size !== value.assets.length) throw new Error('Invalid download statistics snapshot.');
  return value;
}
export function accumulate(repository, releases, previous, now = new Date().toISOString()) {
  if (previous && validateSnapshot(previous).repository !== repository) throw new Error('Statistics repository mismatch.');
  const assets = new Map((previous?.assets ?? []).map(a => [a.id, a]));
  for (const release of releases) {
    if (release.draft) continue;
    for (const a of release.assets) {
      if (!installer(a.name)) continue;
      if (!Number.isSafeInteger(a.id) || a.id < 1 || !Number.isSafeInteger(a.download_count) || a.download_count < 0 ||
          typeof release.tag_name !== 'string') throw new Error('Invalid release asset counter.');
      assets.set(a.id, {
        id: a.id, name: a.name, version: release.tag_name, platform: platform(a.name),
        downloads: Math.max(assets.get(a.id)?.downloads ?? 0, a.download_count),
      });
    }
  }
  return validateSnapshot({ schemaVersion: 1, repository, updatedAt: now, assets: [...assets.values()].sort((a,b) => a.id-b.id) });
}
export async function fetchReleases(repository, token, request = fetch) {
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository)) throw new Error('Invalid repository.');
  const headers = { Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28', ...(token ? { Authorization: `Bearer ${token}` } : {}) };
  async function get(endpoint) {
    const response = await request(`https://api.github.com/repos/${repository}${endpoint}`, { headers, signal: AbortSignal.timeout(30_000) });
    if (!response.ok) throw new Error(`GitHub statistics request failed (${response.status}). Previous snapshot retained.`);
    return response.json();
  }
  // A token raises the rate limit, but must never publish private release metadata.
  if ((await get('')).private !== false) throw new Error('Download statistics require a public repository. Previous snapshot retained.');
  async function pages(endpoint) {
    const all = [];
    for (let page = 1; page <= 100; page++) {
      const data = await get(`/${endpoint}?per_page=100&page=${page}`);
      if (!Array.isArray(data)) throw new Error('Unexpected GitHub response.');
      all.push(...data);
      if (data.length < 100) return all;
    }
    throw new Error('GitHub pagination limit reached; previous snapshot retained.');
  }
  const releases = await pages('releases');
  for (const release of releases) {
    if (release.draft) continue;
    if (!Number.isSafeInteger(release.id)) throw new Error('Invalid release ID.');
    release.assets = await pages(`releases/${release.id}/assets`);
  }
  return releases;
}
export function readSnapshot(file) {
  try { return validateSnapshot(JSON.parse(readFileSync(file, 'utf8'))); }
  catch (error) { if (error.code === 'ENOENT') return null; throw error; }
}
if (isMain(import.meta.url)) {
  try {
    const config = JSON.parse(readFileSync(path.join(ROOT, 'site/src/data/site.json'), 'utf8'));
    const file = path.join(ROOT, 'site/public/stats/downloads.json');
    const previous = readSnapshot(file);
    const result = accumulate(config.releaseRepository, await fetchReleases(config.releaseRepository, process.env.GITHUB_TOKEN), previous);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(`${file}.tmp`, JSON.stringify(result, null, 2) + '\n');
    renameSync(`${file}.tmp`, file);
    console.log(`Updated ${result.assets.length} installer counters.`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
