import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { assertEvidence, hashTree } from './release-report.mjs';
test('release evidence rejects skipped gates, dirty inputs, commits and changed bundles', () => {
  const app = mkdtempSync(join(tmpdir(), 'mathlidelux-release-'));
  try {
    writeFileSync(join(app, 'binary'), 'first');
    const report = { status: 'passed', commit: 'abc', endingCommit: 'abc', dirty: false, checks: ['check','clippy','build','audit','publication'].map(name => ({name,status:'passed'})), appFiles: hashTree(app) };
    assertEvidence(report, 'abc', app);
    assert.throws(() => assertEvidence({...report,dirty:true},'abc',app));
    assert.throws(() => assertEvidence({...report,endingCommit:'def'},'abc',app));
    assert.throws(() => assertEvidence({...report,checks:report.checks.slice(1)},'abc',app));
    writeFileSync(join(app,'binary'),'second');
    assert.throws(() => assertEvidence(report,'abc',app));
  } finally { rmSync(app,{recursive:true,force:true}); }
});
