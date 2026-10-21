import { cpSync, mkdirSync, rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { DEMOS } from '../../scripts/prepare-demo.mjs';

const destination = fileURLToPath(new URL('../public/downloads/', import.meta.url));
rmSync(destination, { recursive: true, force: true });
mkdirSync(destination, { recursive: true });
for (const name of Object.keys(DEMOS)) {
  cpSync(new URL(`../../docs/downloads/${name}.zip`, import.meta.url), `${destination}/${name}.zip`);
}
