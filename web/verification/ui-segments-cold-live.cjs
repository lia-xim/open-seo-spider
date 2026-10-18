'use strict';
// Owns both native processes and their store. Browser localStorage belongs to
// the retained context on the same exact loopback origin, never to the server.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawn} = require('node:child_process');
const {createHash, randomUUID} = require('node:crypto');
const {chromium} = require('../../renderer/node_modules/playwright-core');

const root = path.resolve(__dirname, '../..');
assert(process.env.SPIDER_BINARY, 'Explicit SPIDER_BINARY is required; this test never builds or installs an app');
const binaryPath = path.resolve(process.env.SPIDER_BINARY);
assert(fs.lstatSync(binaryPath).isFile() && !fs.lstatSync(binaryPath).isSymbolicLink());
const binary = fs.realpathSync(binaryPath);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const binarySha = sha(fs.readFileSync(binary));
if (process.env.SPIDER_BINARY_SHA256) assert.equal(binarySha, process.env.SPIDER_BINARY_SHA256);
const output = path.resolve(process.env.SPIDER_SEGMENTS_COLD_OUTPUT || path.join(
  process.env.SPIDER_TEST_ARTIFACT_DIR || path.join(root, 'artifacts'), 'segments-cold-' + Date.now() + '-' + randomUUID().slice(0, 8)));
assert(!fs.existsSync(output), 'A new owned artifact directory is required');
fs.mkdirSync(output, {recursive: true});
process.env.SPIDER_TEST_ARTIFACT_DIR = output;
const artifactPath = require('./artifact-path.cjs');
const store = path.join(output, 'native-store');
const drivers = [__filename, path.join(__dirname, 'artifact-path.cjs')].map(file => ({path: file, sha256: sha(fs.readFileSync(file))}));
const KEY = 'open-seo-saved-segments-v1';
const checks = [], owners = [], errors = [], external = [], readProof = [], exportsProof = [], assets = [], csvCellControls = [], controlledCSVCells = new Set();
const hits = new Map(), witnessHits = [], witnessControls = [], witnessNonce = randomUUID();
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const check = text => { checks.push(text); console.log('PASS ' + text); };
let app, browser, context, page, website, witness, origin, siteOrigin, failure;
let nativeId, emptyId, declaredId, rules, capturedHits, offline = false;

function verifyPins() {
  assert.equal(sha(fs.readFileSync(binary)), binarySha, 'Explicit native executable changed');
  for (const pin of drivers) assert.equal(sha(fs.readFileSync(pin.path)), pin.sha256, 'Owned driver changed: ' + pin.path);
}
async function until(work, label) {
  for (let i = 0; i < 1200; i++) {
    if (app) assert.equal(app.child.exitCode, null, 'Owned app exited: ' + app.log);
    const value = await work(); if (value) return value;
    await sleep(25);
  }
  throw Error('Timeout: ' + label);
}
function deadline(promise, ms, label) {
  let timer;
  return Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Timeout: ' + label)), ms); })])
    .finally(() => clearTimeout(timer));
}
async function launch(phase, listen) {
  verifyPins();
  const env = {...process.env, OPEN_SEO_COORDINATED_EXECUTION_WINDOW: 'confirmed'};
  delete env.OPEN_SEO_PAGESPEED_API_KEY;
  const child = spawn(binary, ['serve', '--listen', listen, '--data-dir', store, '--shutdown-on-stdin-eof'],
    {cwd: root, env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe']});
  const owner = {phase, child, log: '', origin: null, close: null, forced: false};
  owner.closed = new Promise(resolve => child.once('close', (code, signal) => {
    owner.close = {code, signal}; resolve(owner.close);
  }));
  child.once('error', error => { owner.spawnError = error; });
  for (const stream of [child.stdout, child.stderr]) stream.on('data', bytes => { owner.log += bytes.toString(); });
  owners.push(owner); app = owner;
  await until(() => {
    if (owner.spawnError) throw owner.spawnError;
    return owner.log.match(/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?$/m)?.[1];
  }, 'native bind ' + phase).then(value => { owner.origin = value; });
  assert(!['8787', '8790'].includes(new URL(owner.origin).port), 'Never use installed app ports');
  assert.equal((await fetch(owner.origin + '/healthz')).status, 200);
  return owner;
}
async function stop(owner) {
  assert(owner && !owner.close, 'Expected a live owned process');
  owner.child.stdin.end();
  const result = await deadline(owner.closed, 20000, 'natural stdin EOF close ' + owner.phase);
  fs.writeFileSync(artifactPath('native-' + owner.phase + '.log'), owner.log);
  assert.equal(result.code, 0, owner.log); assert.equal(result.signal, null);
  if (app === owner) app = null;
  verifyPins(); return result;
}
async function listen(server, port = 0) {
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', resolve); });
  assert(![8787, 8790].includes(server.address().port));
  return 'http://127.0.0.1:' + server.address().port;
}
async function closeServer(server) {
  if (!server?.listening) return;
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
}
async function probeWitness() {
  const response = await fetch(siteOrigin + '/owned-witness-probe', {headers: {'x-owned-witness-control': witnessNonce}, redirect: 'error'});
  assert.equal(response.status, 410); assert.equal(await response.text(), 'Owned offline witness');
}
async function api(route, body, expected = 200) {
  const response = await fetch(origin + route, {method: body === undefined ? 'GET' : 'POST',
    headers: body === undefined ? {} : {'content-type': 'application/json'},
    body: body === undefined ? undefined : JSON.stringify(body), cache: 'no-store', redirect: 'error'});
  const bytes = Buffer.from(await response.arrayBuffer());
  assert.equal(response.status, expected, bytes.toString('utf8'));
  return {value: JSON.parse(bytes), bytes};
}
function verified(bytes, headers) {
  assert.equal(Number(headers['content-length']), bytes.length);
  assert.equal(headers['x-content-sha256'], sha(bytes));
  assert.equal(headers['cache-control'], 'no-store');
}
function captureRead(route) {
  return page.waitForResponse(response => response.url() === origin + route && response.request().method() === 'POST').then(async response => {
    // Every pure read gets one independent exact native POST oracle. CDP body
    // storage is not a reliable source after the application's fetch consumed it.
    const request = response.request(), body = request.postDataBuffer();
    assert(body); assert.match(new URL(request.url()).pathname, /^\/api\/crawls\/[a-f0-9-]+\/segments(?:\.json|\.csv)?$/i);
    const oracle = await fetch(request.url(), {method: 'POST', headers: {'content-type': request.headers()['content-type']}, body, cache: 'no-store', redirect: 'error'});
    const bytes = Buffer.from(await oracle.arrayBuffer()), headers = await response.allHeaders(), nativeHeaders = Object.fromEntries(oracle.headers);
    assert.equal(response.status(), 200, bytes.toString('utf8')); assert.equal(oracle.status, response.status());
    for (const name of ['content-type', 'content-length', 'x-content-sha256', 'cache-control']) assert.equal(headers[name], nativeHeaders[name]);
    verified(bytes, headers); verified(bytes, nativeHeaders);
    readProof.push({route, request_sha256: sha(body), bytes: bytes.length, sha256: sha(bytes), status: response.status(), body_source: 'independent_exact_native_read'});
    return {value: route.endsWith('.csv') ? null : JSON.parse(bytes), bytes, request: request.postDataJSON()};
  });
}
async function rendered(value) {
  const keys = value.view === 'pages'
    ? ['page_ordinal', 'url', 'document_url', 'status', 'depth', 'stored_title', 'matched_indices', 'unknown_indices']
    : ['code', 'severity', 'url', 'message', 'related_urls'];
  const expected = value.rows.map(row => keys.map(key => {
    if (key === 'matched_indices' || key === 'unknown_indices') return row[key].map(index => value.counts[index].name).join(', ');
    const cell = row[key]; return cell === null ? 'Unbekannt' : typeof cell === 'object' ? JSON.stringify(cell) : String(cell ?? '');
  }));
  await page.waitForFunction(rows => !document.getElementById('result').hidden &&
    JSON.stringify([...document.querySelectorAll('#rows tr')].map(row => [...row.cells].map(cell => cell.textContent))) === JSON.stringify(rows), expected);
  assert.deepEqual(await page.locator('#rows tr').evaluateAll(rows => rows.map(row => [...row.cells].map(cell => cell.textContent))), expected);
  assert.deepEqual(await page.locator('#counts tr').evaluateAll(rows => rows.map(row => [...row.cells].map(cell => cell.textContent))),
    value.counts.map(row => [row.name, row.matched, row.unknown, row.selected_matched, row.selected_unknown].map(String)));
  const s = value.summary;
  assert.equal(await page.locator('#summary').innerText(), `${s.selected_pages}/${s.total_pages} Seiten · ${s.selected_issues} Issues · ${s.multiple_matches} Mehrfachtreffer insgesamt · ${s.unassigned_pages} unzugeordnet · ${s.unknown_pages} unbekannt · ${s.orphan_issues} Issues ohne Seiten-URL`);
  for (const key of ['status', 'depth']) assert.deepEqual(await page.locator('#' + (key === 'status' ? 'statuses' : 'depths') + ' li').allTextContents(),
    Object.entries(value.charts[key]).map(([name, count]) => `${name === 'unknown' ? 'Unbekannt' : name}: ${count}`));
  assert.equal(await page.isDisabled('#next'), !value.next);
}
async function preview(id, selector = 'button[type=submit]') {
  const pending = captureRead(`/api/crawls/${id}/segments`);
  const [result] = await Promise.all([pending, page.click(selector)]);
  assert.deepEqual(result.request.rules, rules, 'Actual UI sends the restored ordered definitions');
  await rendered(result.value); return result;
}
async function download(id, suffix, label) {
  const pending = captureRead(`/api/crawls/${id}/segments.${suffix}`), saving = page.waitForEvent('download');
  const [result, file] = await Promise.all([pending, saving, page.click('#export-' + suffix)]);
  assert.equal(file.suggestedFilename(), 'segments.' + suffix);
  assert.equal(await file.failure(), null);
  assert.deepEqual(fs.readFileSync(await file.path()), result.bytes, 'Actual browser download equals the exact native oracle bytes');
  fs.writeFileSync(artifactPath(label + '.' + suffix), result.bytes);
  exportsProof.push({label, format: suffix, bytes: result.bytes.length, sha256: sha(result.bytes)});
  return result.bytes;
}
function csvRows(text) {
  const rows = []; let row = [], cell = '', quoted = false;
  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    if (char === '"' && quoted && text[i + 1] === '"') { cell += '"'; i++; }
    else if (char === '"') quoted = !quoted;
    else if (char === ',' && !quoted) { row.push(cell); cell = ''; }
    else if (char === '\r' && !quoted) { assert.equal(text[++i], '\n'); row.push(cell); rows.push(row); row = []; cell = ''; }
    else cell += char;
  }
  assert.equal(quoted, false); assert.equal(cell, ''); assert.deepEqual(row, []);
  assert.deepEqual(rows.shift(), ['row_type', 'page_ordinal', 'url', 'segment_indices_json', 'unknown_indices_json', 'data_json']);
  assert(rows.every(row => row.length === 6)); return rows;
}
async function selectJob(id) {
  await until(() => page.locator(`#job option[value="${id}"]`).count(), 'saved job option');
  await page.selectOption('#job', id);
}
function validateSegmentCSV(csvBytes, full) {
  const csv = csvRows(csvBytes.toString('utf8'));
  assert.deepEqual(csv.map(row => row[0]), ['context', 'summary', ...(full.projection.reporting ? ['reporting'] : []),
    ...full.projection.pages.map(() => 'page'), ...full.projection.issues.map(() => 'issue')]);
  assert.deepEqual(JSON.parse(csv.find(row => row[0] === 'context')[5]), full.context);
  assert.deepEqual(JSON.parse(csv.find(row => row[0] === 'summary')[5]),
    Object.fromEntries(Object.entries(full.projection).filter(([key]) => !['pages', 'issues', 'reporting'].includes(key))));
  if (full.projection.reporting) assert.deepEqual(JSON.parse(csv.find(row => row[0] === 'reporting')[5]), full.projection.reporting);
  assert.deepEqual(csv.filter(row => row[0] === 'page').map(row => JSON.parse(row[5])), full.projection.pages);
  assert.deepEqual(csv.filter(row => row[0] === 'issue').map(row => JSON.parse(row[5])), full.projection.issues);
  for (const row of csv.filter(row => row[0] === 'page')) {
    const saved = JSON.parse(row[5]); assert.equal(row[1], String(saved.page_ordinal)); assert.equal(row[2], saved.url);
    assert.deepEqual(JSON.parse(row[3]), saved.matched_indices); assert.deepEqual(JSON.parse(row[4]), saved.unknown_indices);
  }
  for (const row of csv.filter(row => row[0] === 'issue')) {
    const issue = JSON.parse(row[5]);
    assert.deepEqual(row.slice(0, 5), ['issue', '', issue.url, '', ''], 'Issue CSV cells must preserve the URL and empty ordinal/segment columns');
  }
  for (const row of csv.filter(row => ['context', 'summary', 'reporting'].includes(row[0]))) {
    assert.deepEqual(row.slice(0, 5), [row[0], '', '', '', ''], 'Metadata CSV cells must keep all four non-data columns empty');
  }
}
function csvCellNegativeControls(csvBytes, full) {
  const rows = csvRows(csvBytes.toString('utf8'));
  const header = ['row_type', 'page_ordinal', 'url', 'segment_indices_json', 'unknown_indices_json', 'data_json'];
  const encode = values => Buffer.from([header, ...values].map(row => row.map(cell => '"' + cell.replaceAll('"', '""') + '"').join(',')).join('\r\n') + '\r\n');
  for (const [kind, column, replacement] of [['issue', 2, 'http://127.0.0.1:1/incorrect-issue'], ['issue', 1, '999'], ['issue', 3, '[]'], ['issue', 4, '[]'], ['context', 2, 'incorrect-context-url'], ['summary', 3, '[]'], ['reporting', 4, '[]']]) {
    const key = kind + ':' + column, index = rows.findIndex(row => row[0] === kind);
    if (index < 0 || controlledCSVCells.has(key)) continue;
    const changed = structuredClone(rows); changed[index][column] = replacement;
    assert.throws(() => validateSegmentCSV(encode(changed), full), error => /(?:Issue|Metadata) CSV cells must/.test(error.message), key);
    controlledCSVCells.add(key); csvCellControls.push({row_type: kind, column: header[column], rejected: true, actual_validator: 'validateSegmentCSV', basis: 'mutated actual native/browser-verified CSV cells'});
  }
}
async function evaluate(id, {label, mode = '', selection = 'all', view = 'pages', expectedUrls, pageSizes, countPairs, totalPages, unknownPages, unassignedPages}) {
  await selectJob(id); await page.selectOption('#reporting-mode', mode); await page.selectOption('#selection', selection); await page.selectOption('#view', view);
  const pages = [await preview(id)], rows = [...pages[0].value.rows];
  while (pages.at(-1).value.next) {
    const previous = pages.at(-1), next = await preview(id, '#next');
    assert.deepEqual(next.request.cursor, previous.value.next);
    assert.equal(next.request.expected_snapshot_token, previous.value.context.snapshot_token);
    assert.equal(next.request.expected_generation, previous.value.context.app_generation);
    assert.deepEqual(next.value.context, previous.value.context);
    rows.push(...next.value.rows); pages.push(next);
  }
  const first = pages[0].value;
  if (pageSizes) assert.deepEqual(pages.map(result => result.value.rows.length), pageSizes);
  assert.equal(rows.length, first.total);
  if (expectedUrls) assert.deepEqual(rows.map(row => row.url).sort(), [...expectedUrls].sort());
  if (countPairs) assert.deepEqual(first.counts.map(count => [count.matched, count.unknown]), countPairs);
  if (totalPages !== undefined) assert.equal(first.summary.total_pages, totalPages);
  if (unknownPages !== undefined) assert.equal(first.summary.unknown_pages, unknownPages);
  if (unassignedPages !== undefined) assert.equal(first.summary.unassigned_pages, unassignedPages);
  assert.equal(first.context.sql_attested, false); assert.equal(first.context.report_generation_attested, false);
  const jsonBytes = await download(id, 'json', label), full = JSON.parse(jsonBytes);
  assert.deepEqual(full.context, first.context); assert.deepEqual(full.projection.rules, rules);
  assert.deepEqual(view === 'pages' ? full.projection.pages : full.projection.issues, rows);
  assert.deepEqual(full.projection.counts, first.counts); assert.deepEqual(full.projection.charts, first.charts);
  const csvBytes = await download(id, 'csv', label);
  validateSegmentCSV(csvBytes, full); csvCellNegativeControls(csvBytes, full);
  return {pages: pages.map(result => result.value), rows, jsonBytes, csvBytes, full};
}
async function condition(fieldset, index, field, operator, value) {
  const row = fieldset.locator('.condition').nth(index);
  await row.locator('select').nth(0).selectOption(field); await row.locator('select').nth(1).selectOption(operator);
  await row.getByLabel('Wert', {exact: true}).fill(value);
}
async function authorRules() {
  await page.click('#add'); let fieldset = page.locator('#definitions fieldset').nth(0);
  await fieldset.getByLabel('Name', {exact: true}).fill('Products 雪 · AND');
  await condition(fieldset, 0, 'requested_url', 'regex', '/products/[0-9]{3}$');
  await fieldset.getByRole('button', {name: 'Bedingung hinzufügen', exact: true}).click();
  await condition(fieldset, 1, 'status', 'equals', '200');
  await page.click('#add'); fieldset = page.locator('#definitions fieldset').nth(1);
  await fieldset.getByLabel('Name', {exact: true}).fill('Other or 404 · OR');
  await fieldset.locator(':scope > label select').selectOption('any');
  await condition(fieldset, 0, 'stored_title', 'equals', 'Other owned title');
  await fieldset.getByRole('button', {name: 'Bedingung hinzufügen', exact: true}).click();
  await condition(fieldset, 1, 'status', 'equals', '404');
  await page.click('#add'); fieldset = page.locator('#definitions fieldset').nth(2);
  await fieldset.getByLabel('Name', {exact: true}).fill('Known empty title');
  await condition(fieldset, 0, 'stored_title', 'equals', '');
  rules = {version: 1, definitions: [
    {id: 's1', name: 'Products 雪 · AND', join: 'all', conditions: [
      {field: 'requested_url', operator: 'regex', value: '/products/[0-9]{3}$', case_insensitive: false},
      {field: 'status', operator: 'equals', value: '200', case_insensitive: false}]},
    {id: 's2', name: 'Other or 404 · OR', join: 'any', conditions: [
      {field: 'stored_title', operator: 'equals', value: 'Other owned title', case_insensitive: false},
      {field: 'status', operator: 'equals', value: '404', case_insensitive: false}]},
    {id: 's3', name: 'Known empty title', join: 'all', conditions: [
      {field: 'stored_title', operator: 'equals', value: '', case_insensitive: false}]}]};
  await page.click('#save');
  assert.deepEqual(await page.evaluate(key => JSON.parse(localStorage.getItem(key)), KEY), rules);
}
async function restoreRules() {
  assert.equal(await page.locator('#definitions fieldset').count(), 0, 'Definitions are not automatically restored by the server');
  assert.deepEqual(await page.evaluate(key => JSON.parse(localStorage.getItem(key)), KEY), rules);
  await page.click('#restore'); assert.equal(await page.locator('#definitions fieldset').count(), 3);
  for (const [index, rule] of rules.definitions.entries()) assert.equal(await page.locator('#definitions fieldset').nth(index).getByLabel('Name', {exact: true}).inputValue(), rule.name);
}
async function importDeclared(report, name) {
  await page.goto(origin); await page.locator('#connection-state.connected').waitFor();
  // This mutation is never replayed: consume the actual response immediately.
  const pending = page.waitForResponse(response => response.url() === origin + '/api/crawls/import' && response.request().method() === 'POST')
    .then(async response => ({status: response.status(), bytes: await response.body()}));
  const [result] = await Promise.all([pending, page.setInputFiles('#import-file', {name, mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(report))})]);
  assert.equal(result.status, 200, result.bytes.toString());
  const id = JSON.parse(result.bytes).id;
  await page.waitForFunction(id => document.getElementById('crawl-history').value === id, id);
  return id;
}
async function assetHashes(phase) {
  const values = [];
  for (const route of ['/saved-segments', '/saved-segments.js', '/saved-segments.css', '/styles.css']) {
    const response = await fetch(origin + route, {cache: 'no-store', redirect: 'error'}); assert.equal(response.status, 200);
    const bytes = Buffer.from(await response.arrayBuffer()); values.push({route, sha256: sha(bytes), bytes: bytes.length});
  }
  assets.push({phase, values}); return values;
}

(async () => {
  const products = Array.from({length: 105}, (_, index) => '/products/' + String(index).padStart(3, '0'));
  website = http.createServer((request, response) => {
    if (offline) {
      const target = request.headers['x-owned-witness-control'] === witnessNonce ? witnessControls : witnessHits;
      target.push({method: request.method, path: request.url});
      response.writeHead(410, {'content-type': 'text/plain'}); response.end('Owned offline witness'); return;
    }
    hits.set(request.url, (hits.get(request.url) || 0) + 1);
    const known = products.includes(request.url) || ['/hidden', '/missing', '/other', '/untitled'].includes(request.url);
    if (!known) { response.writeHead(404); response.end('Unexpected owned fixture path'); return; }
    response.writeHead(request.url === '/missing' ? 404 : 200, {'content-type': 'text/html; charset=utf-8'});
    const title = request.url.startsWith('/products/') ? 'Product ' + request.url.slice(-3) : request.url === '/other' ? 'Other owned title' : request.url === '/hidden' ? 'Hidden' : null;
    response.end(`<html lang="en"><head>${title === null ? '' : '<title>' + title + '</title>'}${request.url === '/hidden' ? '<meta name="robots" content="noindex">' : ''}</head><body><main>Owned cold segments ${request.url}</main></body></html>`);
  });
  siteOrigin = await listen(website);
  const firstOwner = await launch('first', '127.0.0.1:0'); origin = firstOwner.origin;
  const started = await api('/api/crawls', {seed_urls: [...products, '/hidden', '/missing', '/other', '/untitled'].map(suffix => siteOrigin + suffix),
    crawl_links: false, discover_sitemaps: false, obey_robots: false, check_external: false, delay_ms: 0, concurrency: 8,
    directive_reporting: {version: 1, respect_noindex: true, respect_canonical: false, respect_next_prev: false}});
  nativeId = started.value.id;
  await until(async () => { const job = (await api('/api/crawls/' + nativeId)).value; assert.notEqual(job.status, 'failed', job.error); return job.status === 'completed'; }, 'actual109-page capture');
  const original = await api(`/api/crawls/${nativeId}/project.json`), report = original.value;
  assert.equal(report.pages.length, 109); assert.deepEqual(new Set(report.pages.map(row => row.url)), new Set([...products, '/hidden', '/missing', '/other', '/untitled'].map(suffix => siteOrigin + suffix)));
  assert.equal(report.pages.find(row => row.url.endsWith('/missing')).status, 404);
  assert.equal(report.pages.find(row => row.url.endsWith('/untitled')).titles.length, 0);
  assert.equal(report.pages.filter(row => row.status === 200).length, 108);
  capturedHits = [...hits.values()].reduce((a, b) => a + b, 0); assert.equal(capturedHits, 109); assert([...hits.values()].every(n => n === 1));
  const firstAssets = await assetHashes('first');
  const executablePath = process.env.SPIDER_CHROME || process.env.OPEN_SEO_BROWSER_PATH || await (await import('../../renderer/server.mjs')).findBrowserExecutable();
  browser = await chromium.launch({headless: true, executablePath}); context = await browser.newContext({acceptDownloads: true, viewport: {width: 1440, height: 1000}}); page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    const url = route.request().url();
    if (url.startsWith(origin + '/') || url.startsWith('blob:') || url.startsWith('data:')) return route.continue();
    external.push(url); return route.abort();
  });
  await page.goto(origin + '/saved-segments'); await selectJob(nativeId); await authorRules();
  const empty = structuredClone(report); empty.pages = []; empty.issues = [];
  emptyId = await importDeclared(empty, 'declared-empty-segments.json');
  const declared = structuredClone(empty), base = structuredClone(report.pages.find(row => row.url === siteOrigin + '/products/000'));
  const missing = {...base, url: siteOrigin + '/declared-missing', final_url: siteOrigin + '/declared-missing', fetched_at: '', titles: [], content_type: ''};
  delete missing.status; delete missing.final_status; delete missing.titles;
  const knownEmpty = {...base, url: siteOrigin + '/declared-empty', final_url: siteOrigin + '/declared-empty', status: 200, final_status: 200, titles: ['']};
  declared.pages = [missing, knownEmpty]; declaredId = await importDeclared(declared, 'declared-missing-and-empty-segments.json');
  const normalized = (await api(`/api/crawls/${declaredId}/project.json`)).value;
  assert.equal(normalized.pages[0].status, null); assert.deepEqual(normalized.pages[0].titles, []); assert.deepEqual(normalized.pages[1].titles, ['']);
  await page.goto(origin + '/saved-segments'); await restoreRules();
  const cases = [
    {id: nativeId, label: 'native-products', selection: 'segment:s1', expectedUrls: products.map(suffix => siteOrigin + suffix), pageSizes: [100, 5], countPairs: [[105, 0], [2, 1], [0, 2]], totalPages: 108, unknownPages: 1, unassignedPages: 0},
    {id: nativeId, label: 'native-raw', mode: 'raw', expectedUrls: report.pages.map(row => row.url), pageSizes: [100, 9], countPairs: [[105, 0], [2, 1], [0, 2]], totalPages: 109, unknownPages: 1, unassignedPages: 1},
    {id: nativeId, label: 'native-unknown', selection: 'unknown', expectedUrls: [siteOrigin + '/untitled'], pageSizes: [1]},
    {id: nativeId, label: 'native-unknown-issues', selection: 'unknown', view: 'issues'},
    {id: emptyId, label: 'declared-empty-report', expectedUrls: [], pageSizes: [0], countPairs: [[0, 0], [0, 0], [0, 0]], totalPages: 0, unknownPages: 0, unassignedPages: 0},
    {id: declaredId, label: 'declared-missing', selection: 'unknown', expectedUrls: [siteOrigin + '/declared-missing'], pageSizes: [1], countPairs: [[0, 0], [0, 1], [1, 1]], totalPages: 2, unknownPages: 1, unassignedPages: 0},
    {id: declaredId, label: 'declared-known-empty-title', selection: 'segment:s3', expectedUrls: [siteOrigin + '/declared-empty'], pageSizes: [1]},
  ];
  const warm = [];
  for (const specification of cases) warm.push(await evaluate(specification.id, {...specification, label: 'warm-' + specification.label}));
  assert.deepEqual(warm[0].full.projection.pages.map(row => row.matched_indices), Array.from({length: 105}, () => [0]));
  assert.equal(warm[0].pages[0].reporting.profile_excluded_page_observations, 1);
  assert(warm[3].rows.length > 0); assert.deepEqual(warm[3].rows, report.issues.filter(issue => issue.url === siteOrigin + '/untitled'));
  assert.deepEqual(warm[5].rows[0].matched_indices, []); assert.deepEqual(warm[5].rows[0].unknown_indices, [1, 2]);
  assert.deepEqual(warm[6].rows[0].matched_indices, [2]); assert.deepEqual(warm[6].rows[0].unknown_indices, []);
  check('Actual109-page native capture and UI-authored browser-saved rules prove reported/raw counts, exact100+5 product rows, unknown/issue selection and complete verified JSON/CSV; declared empty/missing/empty-title controls remain separate');
  const savedRules = await page.evaluate(key => localStorage.getItem(key), KEY);
  await page.goto('about:blank');
  // The original server becomes a 410 witness without any unobserved port gap.
  // It remains active during the FIRST native EOF drain as well as the restart.
  offline = true; witness = website; assert(witness.listening); await probeWitness();
  await stop(firstOwner); assert.deepEqual(witnessHits, []);
  assert(firstOwner.close && firstOwner.close.code === 0, 'First child actually closed before restart');
  const secondOwner = await launch('cold', '127.0.0.1:' + new URL(origin).port);
  assert.equal(secondOwner.origin, origin, 'Browser rules need exactly the same owned origin and port'); assert.notEqual(firstOwner.child.pid, secondOwner.child.pid);
  assert.deepEqual(await assetHashes('cold'), firstAssets, 'Both native processes serve identical segment assets');
  await page.goto(origin + '/saved-segments'); assert.equal(await page.evaluate(key => localStorage.getItem(key), KEY), savedRules); await restoreRules();
  const cold = [];
  for (const [index, specification] of cases.entries()) {
    const result = await evaluate(specification.id, {...specification, label: 'cold-' + specification.label}); cold.push(result);
    assert.deepEqual(result.pages, warm[index].pages, 'Cold previews retain every context/count/chart/ordered row: ' + specification.label);
    assert.deepEqual(result.jsonBytes, warm[index].jsonBytes, 'Complete JSON is byte-identical after real restart: ' + specification.label);
    assert.deepEqual(result.csvBytes, warm[index].csvBytes, 'Complete CSV is byte-identical after real restart: ' + specification.label);
  }
  assert.deepEqual((await api(`/api/crawls/${nativeId}/project.json`)).bytes, original.bytes, 'Captured native report is unchanged');
  const missingJob = await api(`/api/crawls/${randomUUID()}/segments`, {rules, selection: {kind: 'all'}, view: 'pages', limit: 100}, 404);
  assert.equal(missingJob.value.code, 'job_not_found');
  check('A second naturally started native process reuses the same owned port/store/browser context; all seven preview projections and every complete JSON/CSV match warm bytes, and missing job remains404 rather than known-empty');
  await selectJob(nativeId); await page.selectOption('#selection', 'segment:s1'); await page.selectOption('#view', 'pages'); await page.selectOption('#reporting-mode', ''); await preview(nativeId);
  for (const width of [1440, 320]) {
    await page.setViewportSize({width, height: 900}); assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.locator('#export-csv').scrollIntoViewIfNeeded(); const box = await page.locator('#export-csv').boundingBox(); assert(box.y >= 0 && box.y + box.height <= 900);
    await page.screenshot({path: artifactPath('segments-cold-' + width + '.png'), fullPage: true});
  }
  assert.deepEqual(errors, []); assert.deepEqual(external, []); assert.deepEqual(witnessHits, []);
  await browser.close(); browser = null; await stop(secondOwner);
  assert.equal(owners.length, 2); assert.equal(csvCellControls.length, 7); assert(owners.every(owner => owner.close?.code === 0 && owner.close.signal === null && !owner.forced));
  assert.deepEqual(witnessHits, [], 'Offline410 witness remains active through browser and both native child closes');
  await probeWitness(); assert.equal(witnessControls.length, 2); await closeServer(witness); assert.deepEqual(witnessHits, []);
  assert.equal([...hits.values()].reduce((a, b) => a + b, 0), capturedHits); verifyPins();
  check('Populated1440/320px exports stay reachable; original report bytes and browser rules survive, while the offline410 witness observes zero website attempts through final natural child shutdown');
  fs.writeFileSync(artifactPath('segments-cold-live.json'), JSON.stringify({result: 'PASS', checks, binary, binary_sha256: binarySha, driver_pins: drivers, output, store,
    native_crawl_id: nativeId, declared_control_ids: {empty: emptyId, missing_and_known_empty_title: declaredId}, origin, site_origin: siteOrigin,
    native: owners.map(owner => ({phase: owner.phase, pid: owner.child.pid, origin: owner.origin, binary_sha256: binarySha, shutdown: owner.close, forced: owner.forced})),
    served_assets: assets, browser_rules: rules, website_capture_requests: capturedHits, offline_witness_attempts: witnessHits, witness_control_requests: witnessControls,
    witness_active_before_first_stdin_eof_and_through_both_child_closes: true, errors, external_requests: external,
    projections: cases.map((specification, index) => ({label: specification.label, pages: cold[index].pages.map(page => page.rows.length), json_sha256: sha(cold[index].jsonBytes), csv_sha256: sha(cold[index].csvBytes)})),
    read_oracles: readProof, exports: exportsProof, csv_cell_negative_controls: csvCellControls,
    scope: 'Two actual native production processes on the same owned loopback origin/store and one retained browser context.109 fresh native pages, exact ordered rules/counts/charts/pagination and complete warm/cold JSON/CSV equality. Empty report, absent status/title and explicit empty title are separate declared imports, never native capture evidence. Pure read oracles use exact browser POST bytes; actual DOM and downloaded bytes are verified. Zero offline website attempts through final producer close. No crash recovery, new-browser rule persistence, SQL authenticity, arbitrary predicate exhaustiveness or vendor parity claim.'}, null, 2) + '\n');
  console.log('PASS saved segments real native cold restart: ' + output);
})().catch(async error => {
  failure = error.stack; console.error(error);
  if (page && !page.isClosed()) await page.screenshot({path: artifactPath('segments-cold-failure.png'), fullPage: true}).catch(() => {});
  fs.writeFileSync(artifactPath('segments-cold-failure.json'), JSON.stringify({result: 'FAIL', failure, checks, output, binary_sha256: binarySha, native_crawl_id: nativeId,
    native: owners.map(owner => ({phase: owner.phase, origin: owner.origin, pid: owner.child.pid, shutdown: owner.close, forced: owner.forced, log: owner.log})), witness_attempts: witnessHits, errors, external_requests: external}, null, 2) + '\n');
  process.exitCode = 1;
}).finally(async () => {
  await browser?.close().catch(error => { console.error('Browser cleanup failed', error); process.exitCode = 1; });
  for (const owner of owners) if (!owner.close) {
    try { await stop(owner); } catch (error) { owner.forced = true; owner.child.kill(); await deadline(owner.closed, 5000, 'failed-test child cleanup').catch(() => {}); process.exitCode = 1; }
  }
  await closeServer(website); await closeServer(witness);
  assert.deepEqual(witnessHits, [], 'Keep the offline website witness until every producer closes');
});
