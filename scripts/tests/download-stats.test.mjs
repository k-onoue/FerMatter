import {test} from 'node:test';
import assert from 'node:assert/strict';
import {accumulate, fetchReleases, installer} from '../download-stats.mjs';
import {execFileSync} from 'node:child_process';
import {mkdtempSync, mkdirSync, writeFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const repo = 'example/FerMatter';
const release = (assets, extra = {}) => ({tag_name:'v1.0', assets, ...extra});
const asset = (id, downloads, name = 'FerMatter.dmg') => ({id, download_count:downloads, name});
test('published installer downloads include previews and exclude samples, signatures and drafts', () => {
  const result = accumulate(repo, [release([asset(1,5),asset(2,8,'FerMatter.pkg'),asset(3,90,'demo.zip'),asset(4,12,'FerMatter.dmg.sig')]), release([asset(5,99)],{draft:true}),release([asset(6,99)],{prerelease:true})]);
  assert.equal(result.assets.reduce((n,a)=>n+a.downloads,0),112);
  assert.equal(installer('FerMatter-linux.AppImage'),true);
  assert.equal(installer('Mathlidelux-0.1.dmg'),true);
  assert.equal(installer('../FerMatter.dmg'),false);
});
test('asset IDs retain deleted counters and distinguish replacement uploads', () => {
  const previous = accumulate(repo,[release([asset(1,10),asset(2,5)])]);
  const result = accumulate(repo,[release([asset(1,9),asset(3,2)])],previous);
  assert.deepEqual(result.assets.map(a=>a.downloads),[10,5,2]);
  assert.throws(()=>accumulate('other/repo',[],previous),/mismatch/);
  assert.throws(()=>accumulate(repo,[release([asset(3,-1)])]),/Invalid/);
});
test('fetching published previews includes their assets while keeping drafts private', async () => {
  const fetched = [];
  const releases = await fetchReleases(repo, null, async url => {
    if (url.endsWith(repo)) return {ok:true,json:async()=>({private:false})};
    if (url.includes('/releases?')) return {ok:true,json:async()=>[
      {id:1,tag_name:'v1.0-preview.1',prerelease:true,draft:false},
      {id:2,tag_name:'v1.0',prerelease:false,draft:true},
    ]};
    fetched.push(url);
    return {ok:true,json:async()=>[asset(1,3)]};
  });
  assert.equal(fetched.length,1);
  assert.match(fetched[0],/releases\/1\/assets/);
  assert.equal(accumulate(repo,releases).assets[0].downloads,3);
});
test('pagination includes every release and asset, rejecting partial API failure', async () => {
  const calls=[];
  const result = await fetchReleases(repo,null,async url=>{
    calls.push(url);
    if (url.endsWith(repo)) return {ok:true,json:async()=>({private:false})};
    const data = url.includes('/assets?') ? (url.endsWith('page=1') ? Array.from({length:100},(_,i)=>asset(i+1,1)) : [asset(101,1)]) : [{id:1,tag_name:'v1.0',assets:[]}];
    return {ok:true,json:async()=>data};
  });
  assert.equal(result[0].assets.length,101);
  assert.equal(calls.length,4);
  await assert.rejects(fetchReleases(repo,null,async()=>({ok:false,status:403})),/403/);
  let privateCalls = 0;
  await assert.rejects(fetchReleases(repo,'token',async()=>{ privateCalls++; return {ok:true,json:async()=>({private:true})}; }),/public repository/);
  assert.equal(privateCalls,1);
});
test('statistics publish only to their data branch, preserving the application checkout', t => {
  const root = mkdtempSync(path.join(tmpdir(), 'fermatter-counts-'));
  t.after(()=>rmSync(root,{recursive:true,force:true}));
  const git = (...args) => execFileSync('git',args,{cwd:root,encoding:'utf8',stdio:['ignore','pipe','pipe']}).trim();
  git('init','-q','--initial-branch=main');
  git('config','user.name','Statistics test');
  git('config','user.email','test@example.invalid');
  writeFileSync(path.join(root,'app.txt'),'Application');
  git('add','app.txt'); git('commit','-qm','Application baseline');
  const head = git('rev-parse','HEAD');
  git('init','--bare','-q','remote.git');
  git('remote','add','origin',path.join(root,'remote.git'));
  mkdirSync(path.join(root,'site/public/stats'),{recursive:true});
  const snapshot = path.join(root,'site/public/stats/downloads.json');
  const script = fileURLToPath(new URL('../save-download-stats.sh',import.meta.url));
  const publish = () => execFileSync('bash',[script],{cwd:root,stdio:'pipe'});
  writeFileSync(snapshot, JSON.stringify(accumulate(repo,[release([asset(1,2)])])));
  publish();
  git('fetch','-q','origin','download-stats:refs/remotes/origin/download-stats');
  const first = git('rev-parse','origin/download-stats');
  publish();
  assert.equal(git('rev-parse','origin/download-stats'),first);
  writeFileSync(snapshot, JSON.stringify(accumulate(repo,[release([asset(1,3)])])));
  publish();
  git('fetch','-q','origin','download-stats:refs/remotes/origin/download-stats');
  assert.equal(git('rev-parse','origin/download-stats^'),first);
  assert.equal(git('ls-tree','--name-only','origin/download-stats'),'downloads.json');
  assert.equal(git('rev-parse','HEAD'),head);
  assert.equal(git('diff','--cached'),'');
  assert.equal(git('branch','--show-current'),'main');
});
