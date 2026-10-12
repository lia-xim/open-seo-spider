// Linux Docker image + unprivileged runtime + SQLite volume acceptance.
// Names are freshly generated; cleanup only removes this script's own objects.
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const suffix = randomUUID().slice(0, 12), name = `open-seo-spider-ci-${suffix}`;
const image = `open-seo-spider-ci:${suffix}`, volume = `${name}-data`;
const docker = args => execFileSync('docker', args, { cwd: root, encoding: 'utf8', windowsHide: true });
const delay = ms => new Promise(done => setTimeout(done, ms));
let volumeCreated = false, containerCreated = false, imageBuilt = false;
async function until(fn, label) {
  for (let attempt = 0; attempt < 400; attempt++) { const result = await fn(); if (result) return result; await delay(50); }
  throw new Error(`Container acceptance timed out: ${label}`);
}
function address() {
  const port = docker(['port', name, '8787/tcp']).trim();
  assert.match(port, /^127\.0\.0\.1:\d+$/);
  return `http://${port}`;
}
async function ready(app) { await until(async () => { try { const result = await fetch(app + '/healthz'); return result.ok && (await result.json()).ok; } catch { return false; } }, 'health'); }
try {
  execFileSync('docker', ['build', '--tag', image, root], { cwd: root, stdio: 'inherit', windowsHide: true }); imageBuilt = true;
  docker(['volume', 'create', volume]); volumeCreated = true;
  docker(['run', '--detach', '--name', name, '--publish', '127.0.0.1::8787', '--mount', `type=volume,source=${volume},target=/data`, '--security-opt', 'no-new-privileges:true', '--cap-drop', 'ALL', image]); containerCreated = true;
  let app = address(); await ready(app);
  assert.equal(docker(['exec', name, 'id', '-u']).trim(), '10001');
  assert.match(await (await fetch(app + '/')).text(), /Open SEO Spider/);
  const preview = await fetch(app + '/api/serp/preview', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ rows: [{ url: 'https://example.invalid/never-fetched', title: 'Container font fixture', description: 'Offline measurement' }] }) });
  assert.equal(preview.status, 200); const metadata = await preview.json();
  assert.equal(metadata.website_requests, 0); assert.equal(typeof metadata.rows[0].title_pixels, 'number');
  const started = await fetch(app + '/api/crawls', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ seed_urls: ['http://127.0.0.1:8787/'], crawl_links: false, discover_sitemaps: false, delay_ms: 0 }) });
  assert.equal(started.status, 200); const job = await started.json();
  const report = await until(async () => { const response = await fetch(`${app}/api/crawls/${job.id}`); const status = await response.json(); if (status.status === 'failed') throw new Error(JSON.stringify(status)); return status.status === 'completed' && status; }, 'self-contained local crawl');
  assert.equal(report.stats.fetched, 1);
  const before = await (await fetch(`${app}/api/crawls/${job.id}/project.json`)).json();
  assert.equal(before.pages.length, 1); assert.ok(before.pages[0].titles.some(title => title.includes('Open SEO Spider')));
  const resourceStarted=await fetch(app+'/api/crawls',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({seed_urls:['http://127.0.0.1:8787/'],crawl_links:false,obey_robots:false,discover_sitemaps:false,delay_ms:0,resource_discovery:{}})});
  assert.equal(resourceStarted.status,200);const resourceJob=await resourceStarted.json();
  const resourceStatus=await until(async()=>{const status=await(await fetch(`${app}/api/crawls/${resourceJob.id}`)).json();if(status.status==='failed')throw Error(JSON.stringify(status));return status.status==='completed'&&status;},'native resource self-crawl');
  assert.ok(resourceStatus.resource_ledger.completed_transports>=3,'own HTML + static CSS/script resources fetched');
  const resourceBefore=await(await fetch(`${app}/api/crawls/${resourceJob.id}/resources.json`)).json();assert.equal(resourceBefore.ledger_available,true);assert.equal(resourceBefore.snapshot.generation,1);assert.equal(resourceBefore.export.all_snapshot_rows_exported,true);
  const resourceTransportsBefore=await(await fetch(`${app}/api/crawls/${resourceJob.id}/resources/transports`)).json();assert.equal(resourceTransportsBefore.ledger_available,true);assert.ok(resourceTransportsBefore.transports.some(row=>row.requested_url.endsWith('/styles.css')));
  const engine='open-seo-spider/resource-contract-v2',capabilities=await(await fetch(app+'/api/capabilities')).json();assert.ok(capabilities.native_resource_discovery.semantic_contracts.includes(engine));
  const v2Started=await fetch(app+'/api/crawls',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({seed_urls:['http://127.0.0.1:8787/'],crawl_links:false,obey_robots:false,discover_sitemaps:false,delay_ms:0,resource_discovery:{contract:{}}})});assert.equal(v2Started.status,200);const v2Job=await v2Started.json();
  await until(async()=>{const status=await(await fetch(`${app}/api/crawls/${v2Job.id}`)).json();if(status.status==='failed')throw Error(JSON.stringify(status));return status.status==='completed'&&status;},'explicit V2 native resource self-crawl');
  const v2Before=await(await fetch(`${app}/api/crawls/${v2Job.id}/resources.json`)).json();assert.equal(v2Before.snapshot.resource_contract_engine,engine);assert.equal(v2Before.export.all_snapshot_rows_exported,true);assert.ok(v2Before.observations.some(row=>row.decision.grant?.reference_kind==='css'));
  const v2TransportsBefore=await(await fetch(`${app}/api/crawls/${v2Job.id}/resources/transports`)).json();assert.equal(v2TransportsBefore.semantic_contract_binding,'validated_full_config_and_generation');assert.ok(v2TransportsBefore.transports.some(row=>row.requested_url.endsWith('/styles.css')&&row.projection.resource_response.contract_engine===engine));
  const toolsResponse=await fetch(app+'/mcp',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/list',params:{}})});assert.equal((await toolsResponse.json()).result.tools.length,22);
  docker(['stop', '--time', '10', name]); docker(['start', name]); app = address(); await ready(app);
  const after = await (await fetch(`${app}/api/crawls/${job.id}/project.json`)).json();
  assert.deepEqual(after, before);
  const resourceAfter=await(await fetch(`${app}/api/crawls/${resourceJob.id}/resources.json`)).json();assert.deepEqual(resourceAfter,resourceBefore);
  const resourceTransportsAfter=await(await fetch(`${app}/api/crawls/${resourceJob.id}/resources/transports`)).json();assert.deepEqual(resourceTransportsAfter,resourceTransportsBefore);
  const v2After=await(await fetch(`${app}/api/crawls/${v2Job.id}/resources.json`)).json();assert.deepEqual(v2After,v2Before);const v2TransportsAfter=await(await fetch(`${app}/api/crawls/${v2Job.id}/resources/transports`)).json();assert.deepEqual(v2TransportsAfter,v2TransportsBefore);
  console.log(JSON.stringify({ tested_at: new Date().toISOString(), result: 'PASS', scope: 'Linux Docker build, UID10001, loopback publication, no privileges/capabilities, bundled font offline API, local legacy/resource V1/explicit V2 self-crawls, 22 MCP tools, exact validated occurrence/transport SQLite snapshots after container restart', resource_contract_v2:engine, external_website_requests: 0, browser_worker: 'not included', local_windows_docker_installation: 'not proven' }));
} catch (error) {
  if (containerCreated) { try { process.stderr.write(docker(['logs', name])); } catch {} }
  throw error;
} finally {
  if (containerCreated) docker(['rm', '--force', name]);
  if (volumeCreated) docker(['volume', 'rm', volume]);
  if (imageBuilt) docker(['image', 'rm', image]);
}
