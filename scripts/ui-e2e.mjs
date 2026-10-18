import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn, execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {createRendererServer} from '../renderer/server.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const suites = [
  ['dashboard', 'ui-smoke.cjs', 'fixture'],
  ['pause-controls', 'ui-crawl-control-smoke.cjs', 'fixture'],
  ['recrawl-controls', 'ui-selected-recrawl-smoke.cjs', 'fixture'],
  ['policy-controls', 'ui-policy-smoke.cjs', 'fixture'],
  ['content-controls', 'ui-content-smoke.cjs', 'fixture'],
  ['resource-controls', 'ui-resource-discovery-smoke.cjs', 'fixture'],
  ['resource-ledger', 'ui-resource-ledger-smoke.cjs', 'fixture'],
  ['resource-contract', 'ui-resource-contract-smoke.cjs', 'fixture'],
  ['comparison-controls', 'ui-saved-comparison-smoke.cjs', 'fixture'],
  ['tool-navigation', 'ui-tools-navigation-live.cjs', 'live'],
  ['rendered-crawl', 'ui-renderer-live.cjs', 'live'],
  ['dom-capture', 'ui-dom-live.cjs', 'live'],
  ['resources', 'ui-resource-live.cjs', 'live'],
  ['robots', 'ui-robots-live.cjs', 'live'],
  ['serp', 'ui-serp-live.cjs', 'live'],
  ['list-import', 'ui-list-import-live.cjs', 'live'],
  ['pause-resume', 'ui-crawl-control-live.cjs', 'live'],
  ['selected-recrawl', 'ui-selected-recrawl-live.cjs', 'live'],
  ['policy', 'ui-policy-live.cjs', 'live'],
  ['content', 'ui-content-live.cjs', 'live'],
  ['resource-discovery', 'ui-resource-discovery-live.cjs', 'live'],
  ['resource-persistence', 'ui-resource-contract-live.cjs', 'live'],
  ['saved-spelling', 'ui-spelling-live.cjs', 'live'],
  ['link-workflows', 'ui-link-workflows-live.cjs', 'live'],
  ['api-enrichment', 'ui-enrichment-live.cjs', 'live'],
  ['directory-tree', 'ui-directory-tree-live.cjs', 'live'],
  ['saved-ngrams', 'ui-ngrams-live.cjs', 'live'],
  ['saved-segments', 'ui-segments-live.cjs', 'live'],
  ['resource-sources', 'ui-resource-sources-live.cjs', 'live'],
  ['custom-search', 'ui-custom-search-live.cjs', 'live'],
  ['saved-comparison', 'ui-saved-comparison-live.cjs', 'owned-native'],
  ['comparison-bundles', 'ui-saved-comparison-bundle-live.cjs', 'owned-native'],
];
const args = process.argv.slice(2);
assert(args.length === 0 || (args.length === 2 && args[0] === '--only'), 'Usage: node scripts/ui-e2e.mjs [--only name,name]');
const requested = args.length ? new Set(args[1].split(',')) : null;
if (requested) for (const name of requested) assert(suites.some(s => s[0] === name), 'Unknown suite: ' + name);
const selected = suites.filter(s => !requested || requested.has(s[0]));
const binary = path.resolve(process.env.SPIDER_BINARY || path.join(root, 'target/debug/open-seo-spider' + (process.platform === 'win32' ? '.exe' : '')));
assert(fs.statSync(binary).isFile(), 'Build the current app before running E2E');
const output = process.env.SPIDER_E2E_OUTPUT ? path.resolve(process.env.SPIDER_E2E_OUTPUT) :
  path.join(root, 'artifacts', `ui-e2e-${Date.now()}-${process.pid}`);
assert(!fs.existsSync(output), 'Use a fresh E2E output directory');
fs.mkdirSync(output, {recursive:true});
const hash = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const tracked = execFileSync('git', ['ls-files', '-z'], {cwd:root, encoding:'utf8'}).split('\0').filter(Boolean);
const before = new Map(tracked.map(file => [file, hash(path.join(root, file))]));
const binarySha = hash(binary), results = [];
let worker, app, appClose, appOrigin, shutdown, failure;
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const appLog = fs.createWriteStream(path.join(output, 'app.log'));
try {
  worker = createRendererServer();
  const address = await worker.listen(0), renderer = `http://127.0.0.1:${address.port}/render`;
  const appEnvironment = {...process.env, OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed'};
  // Owned fixture runs must not inherit the user's paid provider credential.
  delete appEnvironment.OPEN_SEO_PAGESPEED_API_KEY;
  app = spawn(binary, ['serve', '--listen', '127.0.0.1:0', '--data-dir', path.join(output, 'data'), '--shutdown-on-stdin-eof'],
    {cwd:root, windowsHide:true, stdio:['pipe','pipe','pipe'], env:appEnvironment});
  appClose = new Promise(resolve => app.once('close', (code,signal) => resolve({code,signal})));
  let startupError, pending = '';
  app.once('error', error => startupError = error);
  for (const stream of [app.stdout,app.stderr]) stream.on('data', bytes => appLog.write(bytes));
  app.stdout.on('data', bytes => {
    pending += bytes.toString('utf8');
    let end;
    while ((end = pending.indexOf('\n')) >= 0) {
      const line = pending.slice(0,end).replace(/\r$/,''); pending = pending.slice(end+1);
      const match = /^Open SEO Spider: (http:\/\/127\.0\.0\.1:([0-9]+))$/.exec(line);
      if (match) { assert(!appOrigin); assert(!['8787','8790'].includes(match[2])); appOrigin = match[1]; }
    }
  });
  for (let i=0; i<200 && !appOrigin; i++) {
    if (startupError) throw startupError;
    assert(app.exitCode === null, 'App exited before binding'); await delay(50);
  }
  assert(appOrigin, 'No owned post-bind endpoint');
  assert.equal((await fetch(appOrigin+'/healthz')).status,200);
  for (const [name,script,kind] of selected) {
    const directory = path.join(output,name); fs.mkdirSync(directory);
    const stdout = fs.createWriteStream(path.join(directory,'stdout.log')), stderr = fs.createWriteStream(path.join(directory,'stderr.log'));
    const started = Date.now();
    const child = spawn(process.execPath,[path.join(root,'web/verification',script)],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],
      env:{...process.env, SPIDER_BINARY:binary, SPIDER_ROOT:root, SPIDER_APP_URL:appOrigin, SPIDER_URL:appOrigin,
        SPIDER_RENDERER_URL:renderer, OPEN_SEO_SPIDER_RENDERER:renderer,
        OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed', SPIDER_TEST_ARTIFACT_DIR:directory,
        SPIDER_COMPARISON_LIVE_DIR:directory, SPIDER_COMPARISON_SMOKE_DIR:directory,
        SPIDER_UI_PROOF:path.join(directory,'native-proof.json')}});
    let error, timedOut=false; child.once('error', value=>error=String(value));
    child.stdout.pipe(stdout); child.stderr.pipe(stderr);
    const timer = setTimeout(()=>{timedOut=true;child.kill();},240000);
    const status = await new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));
    clearTimeout(timer);
    await Promise.all([new Promise(resolve=>stdout.closed?resolve():stdout.once('close',resolve)),new Promise(resolve=>stderr.closed?resolve():stderr.once('close',resolve))]);
    results.push({name,script,kind,...status,error,timed_out:timedOut,duration_ms:Date.now()-started});
    console.log(`${status.code===0?'PASS':'FAIL'} ${name} (${kind})`);
    if (status.code!==0 || error || timedOut) throw Error(name+': '+fs.readFileSync(path.join(directory,'stderr.log'),'utf8').slice(-3000));
  }
} catch (error) {
  failure=error;
  throw error;
} finally {
  if (app) {
    app.stdin.end();
    const timer=setTimeout(()=>app.kill(),20000);
    shutdown=await appClose; clearTimeout(timer);
  }
  if (worker) await worker.close();
  await new Promise(resolve=>appLog.end(resolve));
  const changed=tracked.filter(file=>hash(path.join(root,file))!==before.get(file));
  const proof={kind:'redesign_UI_E2E',results,app_origin:appOrigin,shutdown,binary,binary_sha256:binarySha,
    binary_unchanged:hash(binary)===binarySha,tracked_sources_unchanged:changed.length===0,changed,
    failure:failure?.stack,
    complete:!failure&&changed.length===0&&shutdown?.code===0&&hash(binary)===binarySha&&results.length===selected.length&&results.every(r=>r.code===0&&!r.error&&!r.timed_out),
    scope:'Isolated owned local app/data/renderer and synthetic fixtures. Mocked-API fixture suites are labelled separately from live native flows. No installed app or user crawl store used.'};
  fs.writeFileSync(path.join(output,'proof.json'),JSON.stringify(proof,null,2)+'\n');
  console.log('Evidence: '+output);
  if(!failure){
    assert.equal(shutdown?.code,0,'Owned app did not close naturally');
    assert.deepEqual(changed,[],'E2E mutated tracked source');
    assert.equal(proof.binary_unchanged,true,'E2E binary changed during acceptance');
    assert.equal(proof.complete,true,'E2E acceptance is incomplete');
  }
}
