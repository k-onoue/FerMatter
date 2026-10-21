#!/usr/bin/env node
// Compatibility entry point: guides now build with the whole Astro site.
import { execFileSync } from 'node:child_process';
import { ROOT } from './lib/paths.mjs';
if (process.argv.slice(2).some(arg => arg !== '--check')) throw new Error('Usage: node scripts/build-guides.mjs [--check]');
if (process.argv.includes('--check')) execFileSync('npm', ['--prefix', 'site', 'run', 'check'], { cwd: ROOT, stdio: 'inherit' });
execFileSync('npm', ['--prefix', 'site', 'run', 'build'], { cwd: ROOT, stdio: 'inherit' });
