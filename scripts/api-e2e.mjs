// Deterministic local HTTP → API → SQLite → export/restart acceptance test.
import http from 'node:http';
import { spawn } from 'node:child_process';
import { mkdtemp, rm, mkdir, rmdir, writeFile, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = process.env.SPIDER_BINARY || path.join(project, 'target', 'debug', `open-seo-spider${process.platform === 'win32' ? '.exe' : ''}`);
const data = await mkdtemp(path.join(os.tmpdir(), 'open-seo-spider-api-'));
const api = 'http://127.0.0.1:18787';
const requests = new Map();
let child;
let output = '';
const fixture = http.createServer(async (req, res) => {
  requests.set(req.url, (requests.get(req.url) || 0) + 1);
  const html = (title, content) => `<html lang="de"><head><title>${title}</title><meta name="description" content="A local deterministic acceptance fixture"></head><body><h1>${title}</h1>${content}</body></html>`;
  if (req.url === '/robots.txt') return res.end('User-agent: *\nDisallow: /blocked\n');
  if (req.url === '/sitemap.xml') { res.writeHead(404); return res.end(); }
  if (req.url === '/redirect') { res.writeHead(301, { location: '/redirect-middle' }); return res.end(); }
  if (req.url === '/redirect-middle') { res.writeHead(302, { location: '/p/1' }); return res.end(); }
  if (req.url === '/missing') { res.writeHead(404, { 'content-type': 'text/html' }); return res.end(html('Missing', '404')); }
  res.setHeader('content-type', 'text/html; charset=utf-8');
  if (req.url === '/') return res.end(html('Fixture home', Array.from({ length: 600 }, (_, i) => `<a href="/p/${i + 1}">Page ${i + 1}</a>`).join('') + '<a href="/redirect">Redirect</a><a href="/missing">Missing</a><a href="/blocked">Blocked</a>'));
  if (req.url === '/slow') return res.end(html('Slow fixture', Array.from({ length: 120 }, (_, i) => `<a href="/slow/${i + 1}">Slow ${i + 1}</a>`).join('')));
  if (req.url.startsWith('/slow/')) await new Promise(resolve => setTimeout(resolve, 45));
  res.end(html(req.url, '<a href="/">Home</a>'));
});
await new Promise(resolve => fixture.listen(19099, '127.0.0.1', resolve));
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function call(route, options = {}) {
  const response = await fetch(api + route, { ...options, headers: { 'content-type': 'application/json', ...options.headers } });
  const body = await response.text();
  if (!response.ok) throw new Error(`${response.status} ${route}: ${body}`);
  return body ? JSON.parse(body) : null;
}
const post = (route, body = {}) => call(route, { method: 'POST', body: JSON.stringify(body) });
async function until(fn, description, timeout = 30000) {
  const start = Date.now();
  while (Date.now() - start < timeout) { const result = await fn(); if (result) return result; await delay(40); }
  throw new Error(`Timeout: ${description}\n${output.slice(-4000)}`);
}
async function launch() {
  output = '';
  child = spawn(binary, ['serve', '--listen', '127.0.0.1:18787', '--data-dir', data], { cwd: project, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { output += chunk; });
  child.on('error', error => { output += error.message; });
  await until(async () => { try { return (await call('/healthz')).ok; } catch { return false; } }, 'server ready');
}
async function stop() { if (!child || child.exitCode !== null) return; const exited = new Promise(resolve => child.once('exit', resolve)); child.kill(); await exited; child = null; }
const status = id => call(`/api/crawls/${id}`);
const terminal = id => until(async () => { const result = await status(id); return ['completed', 'cancelled', 'failed'].includes(result.status) && result; }, 'crawl complete');
try {
  await launch();
  const foreignHostStatus = await new Promise((resolve, reject) => { const request = http.get(api + '/healthz', { headers: { host: 'attacker.example' } }, response => { response.resume(); resolve(response.statusCode); }); request.on('error', reject); });
  assert.equal(foreignHostStatus, 403);
  assert.equal((await fetch(api + '/api/crawls', { method: 'POST', headers: { origin: 'https://attacker.example', 'content-type': 'application/json' }, body: '{}' })).status, 403);
  const offlineUrl='http://127.0.0.1:19099/offline-unrequested';
  const offlineRows=[{url:offlineUrl,title:'Über "uns"',description:'two\nlines'},{url:offlineUrl,title:'😀',description:' \t=FORMULA()'}];
  const offline=await post('/api/serp/preview',{rows:offlineRows});
  assert.equal(offline.website_requests,0);assert.equal(offline.rows.length,2);assert.equal(offline.rows[0].title_characters,10);assert.equal(offline.rows[1].title_pixels,null);
  const offlineCsv=await post('/api/serp/import',{csv:`URL,Title,Description\r\n${offlineUrl},"Über ""uns""","two\nlines"\r\n${offlineUrl},😀," \t=FORMULA()"\r\n`});
  assert.deepEqual(offlineCsv.rows,offline.rows);assert.equal(requests.get('/offline-unrequested'),undefined);
  const offlineExport=await fetch(api+'/api/serp/export.csv',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({rows:offlineRows})});assert.equal(offlineExport.status,200);assert.match(await offlineExport.text(),/' \t=FORMULA\(\)/);
  const robotInput={robots_text:'User-agent: *\nDisallow: /offline-unrequested\nAllow: /offline-unrequested?allow=1$\n',user_agent:'OpenSEOSpider',urls:[offlineUrl,offlineUrl+'?allow=1',offlineUrl,'not a URL']};
  const robotSimulation=await post('/api/robots/simulate',robotInput);
  assert.equal(robotSimulation.website_requests,0);
  assert.deepEqual(robotSimulation.rows.map(row=>row.allowed),[false,true,false,null]);
  assert.ok(robotSimulation.rows[3].error);
  const robotExport=await fetch(api+'/api/robots/export.csv',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(robotInput)});
  assert.equal(robotExport.status,200);assert.match(await robotExport.text(),/Source Row/);
  assert.equal(requests.get('/offline-unrequested'),undefined);
  const invalidRobot=await fetch(api+'/api/robots/simulate',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({...robotInput,limits:{max_urls:0}})});
  assert.equal(invalidRobot.status,400);
  const job = await post('/api/crawls', { seed_url: 'http://127.0.0.1:19099/', delay_ms: 0, concurrency: 16, discover_sitemaps: false, custom_extractions:[{name:'heading',selector:'h1',attribute:null}] });
  const finished = await terminal(job.id);
  assert.equal(finished.status, 'completed');
  assert.equal(finished.stats.fetched, 604);
  assert.equal(requests.get('/blocked'), undefined);
  const pages = await call(`/api/crawls/${job.id}/pages?limit=1000`);
  assert.equal(pages.pages.length, 604);
  assert.equal(pages.pages.find(page => page.url.endsWith('/redirect')).status, 301);
  assert.equal(pages.pages.find(page => page.url.endsWith('/redirect')).final_status, 200);
  const responseView=await call(`/api/crawls/${job.id}/responses`);
  const middle=responseView.responses.find(row=>row.response_url.endsWith('/redirect-middle'));
  assert.equal(middle.status,302);assert.equal(middle.document_metadata_available,false);assert.equal(middle.body_bytes,null);assert.equal(middle.headers,null);
  assert.deepEqual(pages.pages.find(page=>page.url.endsWith('/p/1')).custom_extractions[0].values,['/p/1']);
  const issues = await call(`/api/crawls/${job.id}/issues`);
  assert(issues.issues.some(issue => issue.code === 'broken_outlinks'));
  const analysis = await call(`/api/crawls/${job.id}/analysis`);
  assert(analysis.graph.nodes.length > 500);
  for (const endpoint of ['export.csv', 'issues.csv', 'links.csv', 'extractions.csv', 'javascript.csv', 'cookies.csv', 'pdf.csv', 'images.csv', 'responses.csv', 'fragments.csv', 'dom.csv', 'inputs.csv', 'sitemap.xml', 'project.json']) {
    const response = await fetch(`${api}/api/crawls/${job.id}/${endpoint}`);
    assert.equal(response.status, 200);
    const contents = await response.text();
    assert(contents.length > 0);
    if (endpoint === 'javascript.csv') assert(contents.startsWith('URL,Script Name,Value Type,Value JSON,Error,Elapsed ms'));
    else if (endpoint === 'cookies.csv') assert(contents.startsWith('URL,Name,Domain,Path,Expires,HTTP Only,Secure,Same Site,Value Length'));
    else if (endpoint === 'fragments.csv') assert.equal(contents.trim().split(/\r?\n/).length,1,'Disabled fragment validation exports headers only');
    else if (endpoint === 'dom.csv') assert.ok(contents.includes(',effective,raw_html,true,heading,'),'DOM export preserves effective raw heading provenance');
    else assert(contents.length > 100);
  }
  const snapshot = await call(`/api/crawls/${job.id}/project.json`);
  const imported = await post('/api/crawls/import', snapshot);
  const comparison = await call(`/api/compare?before=${job.id}&after=${imported.id}`);
  assert.equal(comparison.changes.length, 0);
  assert.equal((await fetch(`${api}/api/crawls/${imported.id}/resume`, { method: 'POST' })).status, 409);
  const importedFile=path.join(data,`${imported.id}.json`);
  // Deliberately retain formatting and an unknown future extension: reopening
  // a snapshot must not silently migrate or truncate its original file.
  await writeFile(importedFile,JSON.stringify({...snapshot,_future_extension:{uninterpreted:'preserve on startup'}},null,3)+'\n');
  const preservedFiles=await Promise.all([path.join(data,`${job.id}.json`),importedFile].map(async file=>({file,bytes:await readFile(file)})));
  const slow = await post('/api/crawls', { seed_url: 'http://127.0.0.1:19099/slow', delay_ms: 10, concurrency: 3, crawl_links: true, discover_sitemaps: false });
  await until(async () => (await status(slow.id)).stats.fetched > 3, 'crawl progress');
  await post(`/api/crawls/${slow.id}/cancel`);
  assert.equal((await terminal(slow.id)).status, 'cancelled');
  const temporaryManifest=path.join(data,`${slow.id}.job.tmp`);
  // A terminal API status guarantees its checkpoint write has left the job lock.
  assert.equal(JSON.parse(await readFile(path.join(data,`${slow.id}.job.json`),'utf8')).status,'cancelled');
  await assert.rejects(stat(temporaryManifest), error => error.code === 'ENOENT');
  await mkdir(temporaryManifest);
  assert.equal((await fetch(`${api}/api/crawls/${slow.id}/resume`,{method:'POST'})).status,500);
  assert.equal((await status(slow.id)).status,'cancelled','failed manifest write must retain resumable state');
  assert((await call(`/api/crawls/${slow.id}/project.json`)).pages.length>0);
  await rmdir(temporaryManifest);
  await post(`/api/crawls/${slow.id}/resume`);
  await until(async () => (await status(slow.id)).stats.fetched > 10, 'resumed progress');
  await stop();
  await launch();
  for(const preserved of preservedFiles)assert.ok((await readFile(preserved.file)).equals(preserved.bytes),'Completed/imported snapshot bytes and unknown extensions survive read-only startup');
  const recovered = await status(slow.id);
  assert.equal(recovered.status, 'interrupted');
  assert(recovered.stats.fetched > 0);
  await post(`/api/crawls/${slow.id}/resume`);
  const resumed = await terminal(slow.id);
  assert.equal(resumed.status, 'completed');
  // Slow pages link to the root, which discovers the original 604 URLs as well.
  assert.equal(resumed.stats.fetched, 725);
  assert.equal(requests.get('/slow'), 1, 'completed seed must survive process restart');
  assert.equal((await status(job.id)).stats.fetched, 604);
  const evidence = { tested_at: new Date().toISOString(), platform: process.platform, assertions: ['604 URL HTTP/API crawl', 'robots excluded target', 'redirect initial/final status and separate intermediate-response inventory with unknown unretained body/headers', 'graph and issue API', 'named extraction and all fourteen crawl export endpoints, disabled-fragment headers and effective DOM provenance', 'offline SERP CSV import, Unicode/unknown widths, duplicate input order and formula-safe export without website requests', 'offline robots shared matcher, ordered duplicate and invalid rows, CSV, invalid bounds and zero website requests', 'JSON import and snapshot comparison', 'completed and imported snapshot bytes, formatting and unknown future extension preserved on startup', 'cancel and resume', 'failed resume manifest write retains cancelled state and snapshot', 'process termination and SQLite frontier recovery', 'host/origin protection'], result: 'PASS' };
  console.log(JSON.stringify(evidence, null, 2));
  if (process.env.SPIDER_EVIDENCE) { await mkdir(path.dirname(process.env.SPIDER_EVIDENCE), { recursive: true }); await writeFile(process.env.SPIDER_EVIDENCE, JSON.stringify(evidence, null, 2) + '\n'); }
} finally {
  await stop(); await new Promise(resolve => fixture.close(resolve));
  const cleanupPath = path.resolve(data);
  assert.equal(path.dirname(cleanupPath), path.resolve(os.tmpdir()), 'cleanup must remain directly inside the temporary directory');
  assert.ok(path.basename(cleanupPath).startsWith('open-seo-spider-api-'), 'cleanup must target the owned API fixture directory');
  await rm(cleanupPath, { recursive: true, force: true });
}
