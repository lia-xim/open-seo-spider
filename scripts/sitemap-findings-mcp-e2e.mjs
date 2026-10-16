import {assertDefaultMcpInventory} from './mcp-inventory-contract.mjs';
// AUTHORED / NOT_RUN. Explicit compute release is mandatory before any fixture/process.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import http from 'node:http';
import net from 'node:net';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Client, StreamableHTTPClientTransport } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';

assert.equal(process.env.SPIDER_COMPUTE_RELEASED, 's6-cli-mcp', 'Compute HOLD: explicitly release this native gate first');
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = process.env.SPIDER_BINARY;
const expectedSHA = process.env.SPIDER_BINARY_SHA256;
assert(binary && path.isAbsolute(binary), 'An explicit absolute candidate binary is required');
assert.match(expectedSHA || '', /^[0-9a-f]{64}$/);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
assert.equal(sha(await readFile(binary)), expectedSHA, 'Candidate binary SHA guard');
const sdk = JSON.parse(await readFile(path.join(root, 'scripts/package.json'), 'utf8'));
assert.equal(sdk.devDependencies['@modelcontextprotocol/client'], '2.3.1');
const directory = await mkdtemp(path.join(os.tmpdir(), 'oss-findings-native-'));
let app, cli, activeClient;
let websiteRequests = 0;
const website = http.createServer((request, response) => {
  websiteRequests++;
  response.setHeader('content-type', request.url === '/source.xml' ? 'application/xml' : 'text/html');
  response.end(request.url === '/source.xml'
    ? `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>${websiteOrigin}/p0</loc></url></urlset>`
    : '<html><head><title>Actual native finding</title></head><body>Stored only</body></html>');
});
await new Promise(resolve => website.listen(0, '127.0.0.1', resolve));
const websiteOrigin = `http://127.0.0.1:${website.address().port}`;
const reserve = net.createServer();
await new Promise(resolve => reserve.listen(0, '127.0.0.1', resolve));
const port = reserve.address().port;
await new Promise(resolve => reserve.close(resolve));
const api = `http://127.0.0.1:${port}`;
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const bounded = (promise, ms = 30000) => Promise.race([promise, new Promise((_, reject) => { const timer = setTimeout(() => reject(Error('Native findings deadline')), ms); timer.unref(); })]);
async function body(response, limit) {
  const chunks = []; let total = 0;
  for await (const chunk of response.body || []) { total += chunk.length; assert(total <= limit, 'Native response byte cap'); chunks.push(chunk); }
  return Buffer.concat(chunks);
}
async function apiJSON(route, value) {
  const response = await fetch(api + route, { signal: AbortSignal.timeout(30000), redirect: 'error', ...(value === undefined ? {} : { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(value) }) });
  const bytes = await body(response, 8 * 1024 * 1024);
  assert.equal(response.status, 200, bytes.toString('utf8').slice(0, 2000));
  return JSON.parse(bytes);
}
async function until(operation) {
  const end = Date.now() + 60000;
  while (Date.now() < end) { const result = await operation(); if (result) return result; await delay(25); }
  throw Error('Native findings readiness/crawl deadline');
}
function child(args) {
  const process = spawn(binary, args, { cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = '', stderr = '';
  process.stdout.on('data', bytes => { stdout += bytes; if (stdout.length > 65536) process.kill(); });
  process.stderr.on('data', bytes => { stderr = (stderr + bytes).slice(-65536); });
  const owned = { process, closed: null, actualClose: false, spawnError: null };
  owned.closed = new Promise(resolve => {
    // An error event does not prove close. Retain it separately while the
    // ownership promise resolves only on the actual close event.
    process.once('error', error => { owned.spawnError = error; });
    process.once('close', (code, signal) => {
      owned.actualClose = true;
      resolve({ code, signal, stdout, stderr });
    });
  });
  return owned;
}
async function stopOwned(owned) {
  if (!owned) return;
  const live = () => !owned.actualClose && Number.isInteger(owned.process.pid) && owned.process.pid > 0 && owned.process.exitCode === null && owned.process.signalCode === null;
  if (live()) owned.process.kill('SIGTERM');
  try { await bounded(owned.closed, 15000); }
  catch (error) {
    if (live()) owned.process.kill('SIGKILL');
    await bounded(owned.closed, 15000);
    assert(owned.actualClose, 'Actual owned child close event required');
    throw Error('Owned native child required forced shutdown; this run is not a clean PASS', { cause: error });
  }
  assert(owned.actualClose, 'Actual owned child close event required');
}
const checks = [];
let evidence;
try {
  app = child(['serve', '--listen', `127.0.0.1:${port}`, '--data-dir', directory]);
  await until(async () => { if (app.spawnError) throw app.spawnError; assert.equal(app.process.exitCode, null, 'Owned App exited'); try { return (await fetch(api + '/healthz', { signal: AbortSignal.timeout(1000) })).ok; } catch { return false; } });
  const job = await apiJSON('/api/crawls', { seed_urls: Array.from({ length: 5 }, (_, n) => `${websiteOrigin}/p${n}`), sitemap_urls: [`${websiteOrigin}/source.xml`], obey_robots: false, discover_sitemaps: false, crawl_links: false, concurrency: 1, delay_ms: 0 });
  await until(async () => { const state = await apiJSON(`/api/crawls/${job.id}`); assert.notEqual(state.status, 'failed', state.error); return state.status === 'completed'; });
  const before = websiteRequests;
  let csvOracle;
  for (const kind of ['stdio', 'streamable_http']) {
    activeClient = new Client({ name: 'oss-findings-native', version: '1' });
    const transport = kind === 'stdio'
      ? new StdioClientTransport({ command: binary, args: ['mcp', '--server', api], cwd: root, stderr: 'pipe' })
      : new StreamableHTTPClientTransport(new URL(api + '/mcp'));
    let actualClose = false;
    if (kind === 'stdio') {
      const start = transport.start.bind(transport);
      transport.start = async (...args) => { const previous = transport.onclose; transport.onclose = () => { actualClose = true; previous?.(); }; return start(...args); };
    }
    await bounded(activeClient.connect(transport));
    const inventory = await bounded(activeClient.listTools()); assertDefaultMcpInventory(inventory.tools);
    const definition = inventory.tools.find(tool => tool.name === 'spider_sitemap_findings');
    assert(definition && definition.annotations.readOnlyHint && !definition.annotations.openWorldHint);
    assert.equal(definition.inputSchema.additionalProperties, false);
    const invoke = async args => {
      const result = await bounded(activeClient.callTool({ name: 'spider_sitemap_findings', arguments: { id: job.id, ...args } }));
      assert.equal(result.isError, false, JSON.stringify(result));
      assert(result.content[0].text.length < 200); return result.structuredContent;
    };
    const first = await invoke({ filter: 'urls-not-in-sitemap', limit: 1, include_export_descriptor: true });
    assert.equal(first.website_requests, 0); assert.equal(first.findings.rows.length, 1);
    assert.equal(first.access_target.kind, 'job'); assert.equal(first.access_target.job_id, job.id);
    assert(first.next_cursor); assert(first.export_token);
    const second = await invoke({ filter: 'urls-not-in-sitemap', limit: 2, cursor: first.next_cursor });
    assert.equal(second.findings.rows.length, 2);
    assert.notDeepEqual(second.findings.rows[0].position, first.findings.rows[0].position);
    const descriptor = first.export_descriptor;
    assert.equal(descriptor.method, 'POST'); assert.equal(descriptor.path, `/api/crawls/${job.id}/sitemaps/findings/export.csv`);
    assert.equal(descriptor.body.snapshot, first.export_token); assert(!('cursor' in descriptor.body));
    const response = await fetch(api + descriptor.path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(descriptor.body), redirect: 'error', signal: AbortSignal.timeout(30000) });
    assert.equal(response.status, 200); const csv = await body(response, 64 * 1024 * 1024);
    if (csvOracle) assert.deepEqual(csv, csvOracle); else csvOracle = csv;
    for (const filter of definition.inputSchema.properties.filter.enum) {
      const result = await invoke({ filter, assessment: 'all', limit: 2 });
      assert.match(result.findings.context.context_sha256, /^[0-9a-f]{64}$/);
      assert.equal(result.website_requests, 0);
    }
    const invalid = await bounded(activeClient.callTool({ name: 'spider_sitemap_findings', arguments: { id: job.id, filter: 'orphan-urls', source_task_url: 'bad-domain' } }));
    assert.equal(invalid.isError, true); assert.equal(invalid.structuredContent.status, 400);
    await bounded(activeClient.close()); activeClient = null;
    if (kind === 'stdio') assert(actualClose, 'Actual stdio child close event required');
    assert.equal(websiteRequests, before, 'Read-only tool/API export fetched website');
    checks.push(`${kind}: official SDK, actual App route, seven filters, independent page cursor, pinned full CSV descriptor, strict domain failure, actual close`);
  }
  const request = path.join(directory, 'offline-request.json'), output = path.join(directory, 'offline-page.json'), csv = path.join(directory, 'offline-full.csv');
  await writeFile(request, JSON.stringify({ filter: 'urls-not-in-sitemap', limit: 1 }), { flag: 'wx' });
  cli = child(['sitemap-findings', path.join(directory, job.id + '.sqlite'), '--request', request, '--output', output, '--csv', csv]);
  const result = await bounded(cli.closed, 60000); assert.equal(result.code, 0, result.stderr);
  const receipt = JSON.parse(result.stdout); assert.equal(path.basename(receipt.json_page.path), path.basename(output)); assert.equal(receipt.csv_full_selection.data_rows, 4);
  const page = JSON.parse(await readFile(output)); assert.equal(page.access_target.kind, 'offline'); assert.equal(page.findings.rows.length, 1);
  assert.deepEqual(await readFile(csv), csvOracle); assert.equal(websiteRequests, before);
  checks.push('actual CLI new files: one-page JSON versus full4-row CSV, same actual Core CSV bytes as API, offline access role');
  evidence = { result: 'PASS', tested_at: new Date().toISOString(), binary_sha256: expectedSHA, sdk: '@modelcontextprotocol/client2.3.1', checks, website_requests_during_read: websiteRequests - before, scope: 'Only this directed candidate run; physical64MiB/large-RSS/cancellation-latency/full-feature parity remain separate gates' };
} finally {
  if (activeClient) await activeClient.close().catch(() => {});
  let closeError;
  // The CLI may still be running after its outer request timeout. Keep its
  // actual ownership outside try and await close BEFORE App/data teardown.
  for (const owned of [cli, app]) {
    try { await stopOwned(owned); } catch (error) { closeError ??= error; }
  }
  const allClosed = [cli, app].every(owned => !owned || owned.actualClose);
  if (!allClosed) {
    website.closeAllConnections();
    await new Promise(resolve => website.close(resolve));
    throw Error(`Owned child has not closed; data retained at ${directory}; no cleanup or PASS`, { cause: closeError });
  }
  await new Promise(resolve => website.close(resolve));
  await rm(directory, { recursive: true, force: true });
  if (closeError) throw closeError;
}
// A positive receipt follows actual App/SDK/fixture shutdown and own cleanup.
assert(evidence);
console.log(JSON.stringify(evidence, null, 2));
