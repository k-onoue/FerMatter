import { cpSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { DEMOS } from '../../scripts/prepare-demo.mjs';
import { readSnapshot } from '../../scripts/download-stats.mjs';

// Import the snapshot as build data; page import.meta.url changes after bundling.
const snapshot = readSnapshot(new URL('../public/stats/downloads.json', import.meta.url));
writeFileSync(new URL('../src/data/downloads.generated.json', import.meta.url), JSON.stringify(snapshot) + '\n');

const destination = fileURLToPath(new URL('../public/downloads/', import.meta.url));
rmSync(destination, { recursive: true, force: true });
mkdirSync(destination, { recursive: true });
for (const name of Object.keys(DEMOS)) {
  cpSync(new URL(`../../docs/downloads/${name}.zip`, import.meta.url), `${destination}/${name}.zip`);
}
