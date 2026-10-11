// Deterministic local HTTP → API → SQLite → export/restart acceptance test.
import http from 'node:http';
import { spawn } from 'node:child_process';
import { mkdtemp, rm, mkdir, rmdir, writeFile } from 'node:fs/promises';
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
  if (req.url === '/redirect') { res.writeHead(301, { location: '/p/1' }); return res.end(); }
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
  const job = await post('/api/crawls', { seed_url: 'http://127.0.0.1:19099/', delay_ms: 0, concurrency: 16, discover_sitemaps: false, custom_extractions:[{name:'heading',selector:'h1',attribute:null}] });
  const finished = await terminal(job.id);
  assert.equal(finished.status, 'completed');
  assert.equal(finished.stats.fetched, 604);
  assert.equal(requests.get('/blocked'), undefined);
  const pages = await call(`/api/crawls/${job.id}/pages?limit=1000`);
  assert.equal(pages.pages.length, 604);
  assert.equal(pages.pages.find(page => page.url.endsWith('/redirect')).status, 301);
  assert.equal(pages.pages.find(page => page.url.endsWith('/redirect')).final_status, 200);
  assert.deepEqual(pages.pages.find(page=>page.url.endsWith('/p/1')).custom_extractions[0].values,['/p/1']);
  const issues = await call(`/api/crawls/${job.id}/issues`);
  assert(issues.issues.some(issue => issue.code === 'broken_outlinks'));
  const analysis = await call(`/api/crawls/${job.id}/analysis`);
  assert(analysis.graph.nodes.length > 500);
  for (const endpoint of ['export.csv', 'issues.csv', 'links.csv', 'extractions.csv', 'sitemap.xml', 'project.json']) {
    const response = await fetch(`${api}/api/crawls/${job.id}/${endpoint}`);
    assert.equal(response.status, 200); assert((await response.text()).length > 100);
  }
  const snapshot = await call(`/api/crawls/${job.id}/project.json`);
  const imported = await post('/api/crawls/import', snapshot);
  const comparison = await call(`/api/compare?before=${job.id}&after=${imported.id}`);
  assert.equal(comparison.changes.length, 0);
  assert.equal((await fetch(`${api}/api/crawls/${imported.id}/resume`, { method: 'POST' })).status, 409);
  const slow = await post('/api/crawls', { seed_url: 'http://127.0.0.1:19099/slow', delay_ms: 10, concurrency: 3, crawl_links: true, discover_sitemaps: false });
  await until(async () => (await status(slow.id)).stats.fetched > 3, 'crawl progress');
  await post(`/api/crawls/${slow.id}/cancel`);
  assert.equal((await terminal(slow.id)).status, 'cancelled');
  const temporaryManifest=path.join(data,`${slow.id}.job.tmp`);
  await mkdir(temporaryManifest);
  assert.equal((await fetch(`${api}/api/crawls/${slow.id}/resume`,{method:'POST'})).status,500);
  assert.equal((await status(slow.id)).status,'cancelled','failed manifest write must retain resumable state');
  assert((await call(`/api/crawls/${slow.id}/project.json`)).pages.length>0);
  await rmdir(temporaryManifest);
  await post(`/api/crawls/${slow.id}/resume`);
  await until(async () => (await status(slow.id)).stats.fetched > 10, 'resumed progress');
  await stop();
  await launch();
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
  const evidence = { tested_at: new Date().toISOString(), platform: process.platform, assertions: ['604 URL HTTP/API crawl', 'robots excluded target', 'redirect initial/final status', 'graph and issue API', 'named CSS extraction and all six export endpoints', 'JSON import and snapshot comparison', 'cancel and resume', 'failed resume manifest write retains cancelled state and snapshot', 'process termination and SQLite frontier recovery', 'host/origin protection'], result: 'PASS' };
  console.log(JSON.stringify(evidence, null, 2));
  if (process.env.SPIDER_EVIDENCE) { await mkdir(path.dirname(process.env.SPIDER_EVIDENCE), { recursive: true }); await writeFile(process.env.SPIDER_EVIDENCE, JSON.stringify(evidence, null, 2) + '\n'); }
} finally {
  await stop(); await new Promise(resolve => fixture.close(resolve));
  await rm(data, { recursive: true, force: true });
}
