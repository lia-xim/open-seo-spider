'use strict';
// Actual browser CRUD -> native catalog SQLite -> fresh native process/origin.
// Negative catalog files are prepared offline in separate owned stores only.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawn} = require('node:child_process');
const {DatabaseSync} = require('node:sqlite');
const {createHash, randomUUID} = require('node:crypto');
const {chromium} = require('../../renderer/node_modules/playwright-core');
const profileCapture = require('./profile-response-capture.cjs');
const root = path.resolve(__dirname, '../..');
assert(process.env.SPIDER_BINARY, 'Explicit reviewed normal SPIDER_BINARY required; no build or installed-app discovery');
assert(process.env.SPIDER_BINARY_SHA256, 'Explicit normal binary SHA required');
const binary = fs.realpathSync(path.resolve(process.env.SPIDER_BINARY));
const hash = value => createHash('sha256').update(value).digest('hex');
const binaryHash = hash(fs.readFileSync(binary)); assert.equal(binaryHash, process.env.SPIDER_BINARY_SHA256);
const chrome = process.env.OPEN_SEO_BROWSER_PATH || process.env.SPIDER_CHROME;
assert(chrome && fs.statSync(chrome).isFile(), 'Explicit owned browser required');
const chromeHash = hash(fs.readFileSync(chrome));
const output = path.resolve(process.env.SPIDER_CONFIG_PROFILE_CATALOG_OUTPUT || path.join(process.env.SPIDER_TEST_ARTIFACT_DIR || path.join(root, 'artifacts'), 'config-profile-catalog-' + Date.now() + '-' + randomUUID().slice(0, 8)));
assert(!fs.existsSync(output), 'Fresh owned output required'); fs.mkdirSync(output, {recursive: true});
process.env.SPIDER_TEST_ARTIFACT_DIR = output;
const artifactPath = require('./artifact-path.cjs');
const CATALOG = '/api/config-profile-catalog', ENGINE = 'open-seo-spider/config-profile-catalog-v1';
const key = 'open-seo-spider-config-profile-draft-v1', positiveStore = path.join(output, 'positive-store');
const header = 'X-Owned-Catalog', secret = 'owned-volatile-catalog-' + randomUUID(), nonce = randomUUID();
const checks = [], owners = [], requests = [], negatives = [], downloads = [], sqlProofs = [], held = [], errors = [], external = [], captureHits = [], offlineHits = [], witnesses = [];
const capturePolicies = new WeakMap();
const inspectedResponses = [];
const pins = [__filename, path.join(__dirname, 'artifact-path.cjs'), binary, chrome, path.join(__dirname, 'profile-response-capture.cjs')].map(file => ({file, sha256: hash(fs.readFileSync(file))}));
let app, browser, site, siteOrigin, offline = false, failure, proof;
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
function check(text) { checks.push(text); console.log('PASS ' + text); }
function deadline(promise, ms, label) {
  let timer; return Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Timeout: ' + label)), ms); })]).finally(() => clearTimeout(timer));
}
async function until(work, label, ms = 45000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (app) assert.equal(app.child.exitCode, null, 'Owned native exited: ' + label);
    const value = await work(); if (value) return value; await sleep(25);
  }
  throw Error('Timeout: ' + label);
}
function verifyPins() { for (const pin of pins) assert.equal(hash(fs.readFileSync(pin.file)), pin.sha256, 'Protected input changed: ' + pin.file); }
function lossless(text) { return JSON.parse(text, (_key, value, context) => typeof value === 'number' ? {native_number: context.source} : value); }
function sameJSON(a, b, label) { assert.deepEqual(lossless(a), lossless(b), label); }
function noSecret(text, label) { assert(!text.includes(secret), label + ': volatile header value leaked'); }
function dbFile(directory) { return path.join(directory, 'configuration-profiles', 'catalog.sqlite'); }
// Node's bundled SQLite needs an extended Windows filename for long owned
// fixture outputs. The native application's store adapter is tested separately.
function sqlitePath(file) { return process.platform === 'win32' ? path.toNamespacedPath(file) : file; }
function files(directory) {
  const found = []; if (!fs.existsSync(directory)) return found;
  const visit = (dir, prefix) => { for (const name of fs.readdirSync(dir).sort()) { const file = path.join(dir, name), stat = fs.lstatSync(file); assert(!stat.isSymbolicLink()); if (stat.isDirectory()) visit(file, prefix + name + '/'); else found.push({path: prefix + name, bytes: stat.size, sha256: hash(fs.readFileSync(file))}); } };
  visit(directory, ''); return found;
}
async function listen(server, port = 0) {
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', resolve); });
  assert(![8787, 8790].includes(server.address().port)); return 'http://127.0.0.1:' + server.address().port;
}
async function launch(phase, directory) {
  verifyPins(); const env = {...process.env}; delete env.OPEN_SEO_PAGESPEED_API_KEY;
  const args = ['serve', '--listen', '127.0.0.1:0', '--data-dir', directory, '--shutdown-on-stdin-eof'];
  const child = spawn(binary, args, {cwd: root, env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe']});
  const owner = {phase, directory, child, args, log: '', close: null, natural_eof: false, forced: false, origin: null};
  owner.closed = new Promise(resolve => child.once('close', (code, signal) => { owner.close = {code, signal}; resolve(owner.close); }));
  child.once('error', error => { owner.spawnError = error; });
  for (const stream of [child.stdout, child.stderr]) stream.on('data', bytes => { owner.log += bytes.toString(); });
  owners.push(owner); app = owner;
  owner.origin = await until(() => { if (owner.spawnError) throw owner.spawnError; return /(?:^|\n)Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/.exec(owner.log)?.[1]; }, 'complete post-bind line ' + phase);
  assert(!['8787', '8790'].includes(new URL(owner.origin).port));
  const health = await native('/healthz'); assert.equal(health.response.status, 200); return owner;
}
async function stop(owner) {
  assert(owner && !owner.close); owner.child.stdin.end();
  assert.deepEqual(await deadline(owner.closed, 20000, 'natural native EOF ' + owner.phase), {code: 0, signal: null});
  owner.natural_eof = true; if (app === owner) app = null;
  fs.writeFileSync(artifactPath('native-' + owner.phase + '.log'), owner.log); verifyPins();
}
async function native(route, body, status = 200, method = body === undefined ? 'GET' : 'POST') {
  const response = await fetch(app.origin + route, {method, headers: body === undefined ? {} : {'content-type': 'application/json'}, body: body === undefined ? undefined : JSON.stringify(body), redirect: 'error', cache: 'no-store', signal: AbortSignal.timeout(30000)});
  const bytes = Buffer.from(await response.arrayBuffer()); assert.equal(response.status, status, 'Native ' + method + ' ' + route + ': ' + bytes.toString().slice(0, 500));
  requests.push({source: 'independent-native', phase: app.phase, path: route, method, status, bytes: bytes.length, sha256: hash(bytes)});
  return {response, bytes};
}
async function api(route, body, status, method) { const result = await native(route, body, status, method); return {...result, value: JSON.parse(result.bytes)}; }
async function effects() { return {files: files(app.directory), jobs: (await api('/api/crawls')).value, capture: captureHits.length, offline: offlineHits.length}; }
function observe(page) {
  const owner = app;
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    const url = request.url(); if (url.startsWith(owner.origin + '/api/')) requests.push({source: 'browser', phase: owner.phase, method: request.method(), path: new URL(url).pathname, request_sha256: hash(Buffer.from(request.postData() || ''))});
  });
}
async function makeContext() {
  const origin = app.origin;
  const context = await browser.newContext({acceptDownloads: true, viewport: {width: 1440, height: 1000}});
  await context.route('**/*', route => { const url = route.request().url(); if (url.startsWith(origin + '/') || /^(about:|blob:|data:)/.test(url)) return route.continue(); external.push(url); return route.abort(); });
  return context;
}
async function makePage(context) {
  const page = await context.newPage(); observe(page); const policy = {name: profileCapture.OWNED_POLICY, origin: app.origin}; await profileCapture.install(page, {}, policy); capturePolicies.set(page, policy); await page.goto(app.origin); await page.locator('#connection-state.connected').waitFor();
  if (await page.evaluate(name => localStorage.getItem(name) !== null, key)) await page.locator('#config-profile-active').waitFor({state: 'visible'});
  return page;
}
async function browserJSON(page, pending, marker) {
  const response = await pending, request = response.request(), raw = request.postDataBuffer();
  const observed = {method: request.method(), path: new URL(response.url()).pathname, url: response.url(), status: response.status(), observed_headers: response.headers(), request_sha256: hash(raw || Buffer.alloc(0)), request_bytes: raw?.length || 0, marker, result: 'BODY_PENDING'};
  inspectedResponses.push(observed); // Preserve original wire metadata even if body capture refuses.
  try {
    const policy = capturePolicies.get(page);
    assert(policy && profileCapture.canCapture(request.method(), response.url(), policy), 'Consumed JSON must belong to this exact owned profile API policy');
    assert(marker, 'An awaited pre-action browser marker is required for every consumed original JSON response');
    const result = await profileCapture.capture(page, response, marker);
    Object.assign(observed, result.evidence, {bytes: result.bytes.length, sha256: hash(result.bytes), result: 'PASS'});
    return {response, bytes: result.bytes, value: JSON.parse(result.bytes)};
  } catch (error) {
    Object.assign(observed, {result: 'FAIL', error: error.message});
    throw error;
  }
}
function waitJSON(page, route, method = 'POST', marker) { const origin = capturePolicies.get(page)?.origin; assert(origin, 'Owned capture policy must be installed before actions'); return browserJSON(page, page.waitForResponse(response => response.url() === origin + route && response.request().method() === method), marker); }
async function clickJSON(page, selector, route, method = 'POST', expected = 200) {
  const marker = await profileCapture.mark(page);
  const pending = waitJSON(page, route, method, marker); await page.click(selector); const result = await pending;
  assert.equal(result.response.status(), expected, 'Browser ' + method + ' ' + route + ': ' + result.bytes.toString().slice(0, 500));
  noSecret(result.bytes.toString(), 'native browser response'); return result;
}
async function openSettings(page) {
  if (!await page.locator('#settings-dialog').evaluate(element => element.open)) await page.click('#settings-open');
  const editor = page.locator('#config-profile-json').locator('xpath=ancestor::details[1]'); if (!await editor.evaluate(element => element.open)) await editor.locator(':scope > summary').click();
}
async function openCatalog(page) { await openSettings(page); const details = page.locator('#config-profile-catalog'); if (!await details.evaluate(element => element.open)) await details.locator(':scope > summary').click(); }
async function storage(page) { return page.evaluate(() => Object.fromEntries(Object.keys(localStorage).sort().map(name => [name, localStorage.getItem(name)]))); }
async function working(page) {
  return page.evaluate(() => ({profile: document.getElementById('config-profile-json').value, effective: document.getElementById('config-profile-effective').textContent,
    toolbar: {target: document.getElementById('seed-url').value, mode: document.getElementById('crawl-mode').value},
    headers: [...document.querySelectorAll('#config-profile-required-headers input[data-profile-header]')].map(input => ({name: input.dataset.profileHeader, value: input.value, type: input.type})),
    controls: [...document.getElementById('settings-form').elements].filter(input => input.name && !input.id.startsWith('config-catalog-') && !['password', 'file', 'button', 'submit'].includes(input.type)).map(input => ({name: input.name, value: input.type === 'checkbox' ? input.checked : input.value, disabled: input.disabled}))}));
}
async function prepared(page, text) {
  await openSettings(page); await page.fill('#config-profile-json', text);
  const result = await clickJSON(page, '#config-profile-inspect', '/api/config-profiles/inspect');
  await page.waitForFunction(value => document.getElementById('config-profile-effective').textContent === value, result.value.effective_config_json);
  sameJSON(await page.inputValue('#config-profile-json'), result.value.profile_json, 'Actual native-installed draft'); return result.value;
}
async function save(page) {
  const result = await clickJSON(page, '#settings-form button[type=submit]', '/api/config-profiles/inspect');
  await page.waitForFunction(() => !document.getElementById('settings-dialog').open);
  noSecret(JSON.stringify(await storage(page)), 'sanitized browser storage'); return result.value;
}
async function bindHeader(page) {
  const input = page.locator('#config-profile-required-headers input[data-profile-header]'); assert.equal(await input.count(), 1);
  assert.equal(await input.getAttribute('data-profile-header'), header); assert.equal(await input.getAttribute('type'), 'password'); await input.fill(secret);
}
function entryShape(entry) {
  assert.deepEqual(Object.keys(entry).sort(), ['id', 'name', 'revision', 'profile_sha256', 'profile_bytes', 'created_at', 'updated_at'].sort());
  assert.match(entry.id, /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/); assert.match(entry.revision, /^[1-9]\d*$/); assert(BigInt(entry.revision) <= 9223372036854775807n);
  assert.match(entry.profile_sha256, /^[0-9a-f]{64}$/); assert(Number.isSafeInteger(entry.profile_bytes));
  for (const field of ['created_at', 'updated_at']) { assert.match(entry[field], /^\d{4}-\d\d-\d\dT.*Z$/); assert(Buffer.byteLength(entry[field]) <= 64); assert(Number.isFinite(Date.parse(entry[field]))); }
  return entry;
}
async function list(page, expected = 200) {
  await openCatalog(page); const result = await clickJSON(page, '#config-catalog-list', CATALOG, 'GET', expected);
  if (expected !== 200) { await page.locator('#config-catalog-error').waitFor({state: 'visible'}); return result; }
  assert.equal(result.value.engine, ENGINE); assert.equal(result.value.version, 1); assert.equal(result.value.scope, 'selected_application_data_directory');
  assert.deepEqual(result.value.limits, {max_entries: 128, max_name_bytes: 256, max_profile_bytes: 67108864, max_total_profile_bytes: 268435456});
  for (const item of result.value.entries) entryShape(item);
  await page.waitForFunction(ids => JSON.stringify([...document.querySelectorAll('#config-catalog-select option')].filter(option => option.value).map(option => option.value).sort()) === JSON.stringify(ids), result.value.entries.map(entry => entry.id).sort());
  return result;
}
async function select(page, id) { await page.selectOption('#config-catalog-select', id); assert.equal(await page.inputValue('#config-catalog-select'), id); }
async function load(page, entry) {
  await select(page, entry.id); const result = await clickJSON(page, '#config-catalog-load', CATALOG + '/' + entry.id, 'GET');
  assert.equal(result.value.entry.id, entry.id); assert.equal(result.value.entry.profile_sha256, hash(Buffer.from(result.value.profile.profile_json)));
  assert.equal(result.value.entry.profile_bytes, Buffer.byteLength(result.value.profile.profile_json));
  await page.waitForFunction(text => document.getElementById('config-profile-effective').textContent === text, result.value.profile.effective_config_json);
  sameJSON(await page.inputValue('#config-profile-json'), result.value.profile.profile_json, 'Catalog Load stages full original text');
  assert((await working(page)).headers.every(input => input.value === ''), 'Catalog Load clears volatile header bindings'); return result;
}
async function mutationUnchanged(page, action, label) {
  const before = await working(page), stored = await storage(page), jobs = (await api('/api/crawls')).value, hitCount = captureHits.length;
  const result = await action(); await page.waitForFunction(() => !document.getElementById('config-catalog-list').disabled);
  assert.deepEqual(await working(page), before, label + ' preserves working config, header bindings and toolbar'); assert.deepEqual(await storage(page), stored, label + ' never adopts the active draft');
  assert.deepEqual((await api('/api/crawls')).value, jobs); assert.equal(captureHits.length, hitCount); return result;
}
function sqlSnapshot(label, directory = app.directory) {
  const file = dbFile(directory); assert(fs.statSync(file).isFile()); const before = hash(fs.readFileSync(file));
  const database = new DatabaseSync(sqlitePath(file), {readOnly: true}); let rows, applicationId, version;
  try {
    applicationId = database.prepare('PRAGMA application_id').get().application_id; version = database.prepare('PRAGMA user_version').get().user_version;
    assert.equal(applicationId, 0x4f535043); assert.equal(version, 1); assert.equal(database.prepare('PRAGMA integrity_check').get().integrity_check, 'ok');
    rows = database.prepare('SELECT id,name,CAST(revision AS TEXT) AS revision,profile_sha256,profile_bytes,profile_text,created_at,updated_at FROM configuration_profiles ORDER BY id').all().map(row => ({...row}));
  } finally { database.close(); }
  assert.equal(hash(fs.readFileSync(file)), before, 'Independent SQLite read is read-only');
  for (const row of rows) { assert.equal(hash(Buffer.from(row.profile_text)), row.profile_sha256); assert.equal(Buffer.byteLength(row.profile_text), row.profile_bytes); noSecret(row.profile_text, 'actual catalog stored text'); }
  const record = {label, path: file, application_id: applicationId, user_version: version, file_sha256: before, rows};
  fs.writeFileSync(artifactPath('catalog-sql-' + label + '.json'), JSON.stringify(record, null, 2) + '\n');
  sqlProofs.push({label, file, file_sha256: before, rows: rows.map(({profile_text, ...metadata}) => metadata)}); return record;
}
async function storedEntry(entry, label) {
  const snapshot = sqlSnapshot(label), row = snapshot.rows.find(row => row.id === entry.id); assert(row, 'Actual persisted row');
  const {profile_text, ...metadata} = row; assert.deepEqual(metadata, entryShape(entry));
  const result = await api(CATALOG + '/' + entry.id); assert.deepEqual(result.value.entry, entry);
  assert.equal(result.value.profile.profile_json, profile_text, 'Native catalog GET has exact stored UTF8 text'); return row;
}
async function download(page, label, expectedText) {
  const stored = await storage(page), responsePending = page.waitForResponse(response => response.url() === app.origin + '/api/config-profiles/export' && response.request().method() === 'POST');
  const pending = page.waitForEvent('download'); await page.click('#config-profile-export'); const [observed, actual] = await Promise.all([responsePending, pending]); assert.equal(observed.status(), 200);
  const request = observed.request(); assert.equal(new URL(observed.url()).origin, app.origin); const body = request.postDataBuffer(); assert(body);
  const oracle = await fetch(observed.url(), {method: 'POST', headers: {'content-type': 'application/json'}, body, redirect: 'error', cache: 'no-store', signal: AbortSignal.timeout(30000)});
  const bytes = Buffer.from(await oracle.arrayBuffer()); assert.equal(oracle.status, 200);
  const file = artifactPath(label + '.json'); await actual.saveAs(file); assert.deepEqual(fs.readFileSync(file), bytes); assert.equal(bytes.toString(), expectedText, 'Actual download preserves exact stored canonical profile bytes');
  for (const headers of [await observed.allHeaders(), Object.fromEntries(oracle.headers)]) { assert.equal(headers['x-content-sha256'], hash(bytes)); assert.equal(Number(headers['content-length']), bytes.length); assert.equal(headers['cache-control'], 'no-store'); assert.match(headers['content-disposition'], /attachment/); }
  noSecret(bytes.toString(), 'real downloaded profile'); assert.deepEqual(await storage(page), stored);
  downloads.push({label, file, bytes: bytes.length, sha256: hash(bytes), request_sha256: hash(body), route: '/api/config-profiles/export', oracle: 'unconditional_exact_native_pure_post_not_mutation_replay'}); return bytes;
}
async function remove(page, entry, expected = 200) {
  await select(page, entry.id); const requestCount = requests.filter(request => request.source === 'browser' && request.method === 'DELETE').length;
  await page.click('#config-catalog-delete'); await page.locator('#config-catalog-delete-question').waitFor({state: 'visible'});
  assert.equal(requests.filter(request => request.source === 'browser' && request.method === 'DELETE').length, requestCount, 'Opening inline deletion question is not deletion');
  assert((await page.locator('#config-catalog-delete-question-text').textContent()).includes(entry.name));
  return clickJSON(page, '#config-catalog-delete-confirm', CATALOG + '/' + entry.id, 'DELETE', expected);
}
async function witness() {
  const response = await fetch(siteOrigin + '/offline-witness', {headers: {'x-owned-witness': nonce}, signal: AbortSignal.timeout(10000)}); assert.equal(response.status, 410); assert.equal(await response.text(), 'Owned offline catalog fixture');
}
async function assetBinding() {
  for (const [name, route] of [['index.html', '/'], ['app.js', '/app.js'], ['styles.css', '/styles.css'], ['config-profiles.js', '/config-profiles.js']]) {
    const file = path.join(root, 'web', name), bytes = fs.readFileSync(file), served = await native(route);
    assert.equal(hash(served.bytes), hash(bytes), 'Fresh normal binary must embed current asset ' + name); pins.push({file, sha256: hash(bytes)});
  }
}
async function negativeCatalogControls(profileText) {
  for (const variant of ['empty-corrupt', 'unsupported-version']) {
    const directory = path.join(output, 'negative-' + variant), file = dbFile(directory);
    fs.mkdirSync(path.dirname(file), {recursive: true});
    if (variant === 'empty-corrupt') fs.writeFileSync(file, Buffer.alloc(0));
    else {
      const db = new DatabaseSync(sqlitePath(file));
      try { db.exec('PRAGMA application_id=1330860099; PRAGMA user_version=2; CREATE TABLE unsupported_control(value TEXT);'); } finally { db.close(); }
    }
    const installedHash = hash(fs.readFileSync(file)), owner = await launch('negative-' + variant, directory);
    if (!pins.some(pin => pin.file === path.join(root, 'web', 'index.html'))) await assetBinding();
    const context = await makeContext(), page = await makePage(context);
    await prepared(page, profileText); await save(page); await openSettings(page);
    const before = await working(page), stored = await storage(page), effect = await effects();
    const catalogReads = requests.filter(request => request.source === 'browser' && request.phase === owner.phase && request.path.startsWith(CATALOG)).length;
    assert.equal(catalogReads, 0, 'Catalog files are not automatically read on startup, Settings or profile Save');
    const refused = await list(page, 503); assert.equal(typeof refused.value.error, 'string'); assert(refused.value.error.length);
    assert.deepEqual(await working(page), before, variant + ' never resets the verified active draft'); assert.deepEqual(await storage(page), stored);
    assert.deepEqual(await effects(), effect); assert.equal(hash(fs.readFileSync(file)), installedHash, 'Unavailable catalog is never repaired or reset');
    assert.equal(await page.getAttribute('#config-catalog-error', 'role'), 'alert');
    negatives.push({name: variant, scope: 'offline-prepared-negative-only-owned-store; no positive profile seeded', path: file, sha256: installedHash, status: 503, preserves_active_working_storage_jobs_files: true});
    await page.goto('about:blank'); await context.close(); await stop(owner);
  }
}
async function delayedLoad(page, first, second, secondText) {
  const pattern = app.origin + CATALOG + '/' + first.id; let release, reached, settled;
  const gate = new Promise(resolve => { release = resolve; }), ready = new Promise(resolve => { reached = resolve; }), done = new Promise(resolve => { settled = resolve; });
  let captured;
  await page.route(pattern, async route => {
    assert.equal(route.request().method(), 'GET');
    try {
      const response = await route.fetch({timeout: 30000}), bytes = await response.body(); assert.equal(response.status(), 200);
      const value = JSON.parse(bytes); assert.equal(value.entry.id, first.id); assert.equal(hash(Buffer.from(value.profile.profile_json)), value.entry.profile_sha256);
      captured = {url: pattern, method: 'GET', source: 'actual_native_response_body_only_delayed', bytes: bytes.length, sha256: hash(bytes)};
      reached(); await gate;
      try { await route.fulfill({response, body: bytes}); captured.delivered = true; } catch (error) { captured.delivered = false; captured.delivery_error = String(error.message).slice(0, 250); }
    } finally { settled(); }
  });
  try {
    const stored = await storage(page), beforeDisk = files(app.directory);
    await select(page, first.id); await page.click('#config-catalog-load'); await deadline(ready, 10000, 'held actual native catalog Load A');
    await select(page, second.id); await load(page, second);
    sameJSON(await page.inputValue('#config-profile-json'), secondText, 'Newer actual Load B wins while A is held');
    const installed = await working(page); release(); await deadline(done, 10000, 'release actual held Load A');
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    assert.deepEqual(await working(page), installed, 'Late actual A body never replaces B or header state'); assert.deepEqual(await storage(page), stored);
    assert.equal(await page.inputValue('#config-catalog-select'), second.id); assert.equal(await page.isEnabled('#config-catalog-load'), true);
    assert.deepEqual(files(app.directory), beforeDisk); held.push(captured);
  } finally { release(); await page.unroute(pattern); }
}
async function missingHeaderStart(page, label) {
  await save(page); const before = await effects(), starts = requests.filter(request => request.source === 'browser' && request.path === '/api/crawls/from-profile').length;
  // The real form submission exercises the same programmatic entry guard even if the UI disables its button.
  await page.locator('#crawl-form').evaluate(form => form.requestSubmit());
  await page.locator('#message').waitFor({state: 'visible'}); assert.match(await page.locator('#message').textContent(), /Header|header/);
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(requests.filter(request => request.source === 'browser' && request.path === '/api/crawls/from-profile').length, starts, label + ' rejects before a native Start POST');
  assert.deepEqual(await effects(), before, label + ' creates no job/files/website requests');
}
async function run() {
  site = http.createServer((request, response) => {
    if (offline) {
      if (request.url === '/offline-witness' && request.headers['x-owned-witness'] === nonce) witnesses.push({at: Date.now(), path: request.url});
      else offlineHits.push(request.url);
      response.writeHead(410, {'content-type': 'text/plain; charset=utf-8'}); response.end('Owned offline catalog fixture'); return;
    }
    captureHits.push({url: request.url, header_received: request.headers[header.toLowerCase()] === secret});
    response.writeHead(request.url === '/a' || request.url === '/b' ? 200 : 404, {'content-type': 'text/html; charset=utf-8'});
    response.end('<html lang="en"><head><title>Owned catalog ' + request.url + '</title><meta name="description" content="Unique owned ' + request.url + '"></head><body><main>Independent profile catalog capture.</main><a href="/must-not-follow">Unfollowed</a></body></html>');
  });
  siteOrigin = await listen(site); browser = await chromium.launch({headless: true, executablePath: chrome});
  // Positive catalog rows are created exclusively through actual browser-native operations below.
  const bootstrap = await launch('positive-warm', positiveStore); await assetBinding();
  const defaults = (await api('/api/config-profiles/default')).value, full = JSON.parse(defaults.effective_config_json);
  const imported = (await api('/api/lists/import', {content_base64: Buffer.from(siteOrigin + '/a\n' + siteOrigin + '/b\n' + siteOrigin + '/a\n').toString('base64'), options: {format: {kind: 'txt'}}})).value;
  const config = {...full, seed_urls: [siteOrigin + '/a', siteOrigin + '/b'], sitemap_urls: [], explicit_sitemap_inputs: [], input_imports: [imported], concurrency: 65, request_timeout_secs: 301, max_body_bytes: 1572865, max_redirects: 3, discover_sitemaps: false, crawl_links: false, obey_robots: false, check_external: false, delay_ms: 0, user_agent: 'OwnedCatalog/operational', renderer: null, tls: {version: 1, respect_hsts: false, trusted_certificates: []}, pdf_analysis: {...full.pdf_analysis, enabled: false, max_pages: 19}, custom_headers: {[header]: secret}};
  const nativePrepared = (await api('/api/config-profiles/from-config', {config_text: JSON.stringify(config)})).value;
  const textB = nativePrepared.profile_json; noSecret(textB, 'sanitized operational profile');
  assert.deepEqual(JSON.parse(textB).required_headers, [header]);
  const textA = textB.replace(/("delay_ms"\s*:\s*)0(?=\s*[,}])/, (_match, prefix) => prefix + '9007199254740993'); assert.notEqual(textA, textB);
  const context = await makeContext(), page = await makePage(context);
  await prepared(page, textB); await save(page); await openCatalog(page);
  const initial = await effects(); assert(!fs.existsSync(path.dirname(dbFile(positiveStore))), 'Pure profile Save does not create catalog parent');
  assert.equal(requests.filter(request => request.source === 'browser' && request.phase === bootstrap.phase && request.path.startsWith(CATALOG)).length, 0, 'No automatic catalog list on startup or Settings');
  const empty = await list(page); assert.deepEqual(empty.value.entries, []); assert.deepEqual(await effects(), initial);
  await api(CATALOG + '/' + randomUUID(), undefined, 404); assert.deepEqual(await effects(), initial);
  const unsafe = JSON.stringify({version: 1, config: {...config}, required_headers: []});
  for (const [label, body] of [['malformed-profile', {name: 'Invalid malformed', profile_text: '{'}], ['unsafe-header-values', {name: 'Unsafe header', profile_text: unsafe}], ['unknown-field', {name: 'Unknown field', profile_text: textB, unknown: true}], ['unsupported-profile', {name: 'Unsupported profile', profile_text: JSON.stringify({version: 2, config: {}, required_headers: []})}]]) {
    await api(CATALOG, body, 400); assert.deepEqual(await effects(), initial); assert(!fs.existsSync(path.dirname(dbFile(positiveStore)))); negatives.push({name: label, status: 400, no_catalog_parent_or_job_or_network: true});
  }
  check('Explicit catalog list and missing/invalid/unsafe requests preserve an absent catalog directory, active profile, jobs and all network counters');
  // Corrupt-file controls use separately prepared stores. Keep the warm positive app alive, never mutate its SQLite files.
  const warmApp = app;
  await negativeCatalogControls(textB); app = warmApp;
  const nameA = ' ../<img src=x onerror="window.catalogInjection=true"> 雪 \\ profile ', nameB = 'Operational profile 雪';
  await prepared(page, textA); await bindHeader(page); await openCatalog(page); await page.fill('#config-catalog-name', nameA);
  const createA = await mutationUnchanged(page, () => clickJSON(page, '#config-catalog-create', CATALOG), 'Create A'); let entryA = entryShape(createA.value.entry);
  assert.equal(entryA.name, nameA); let rowA = await storedEntry(entryA, 'created-a'); assert.match(rowA.profile_text, /"delay_ms"\s*:\s*9007199254740993(?:\s*[,}])/);
  assert.deepEqual(createA.response.request().postDataJSON(), {name: nameA, profile_text: rowA.profile_text}, 'Actual browser Create A submits the exact canonical text independently read from SQLite');
  await prepared(page, textB); await bindHeader(page); await openCatalog(page); await page.fill('#config-catalog-name', nameB);
  const createB = await mutationUnchanged(page, () => clickJSON(page, '#config-catalog-create', CATALOG), 'Create B'); let entryB = entryShape(createB.value.entry), rowB = await storedEntry(entryB, 'created-b');
  assert.deepEqual(createB.response.request().postDataJSON(), {name: nameB, profile_text: rowB.profile_text}, 'Actual browser Create B submits the exact canonical text independently read from SQLite');
  assert.equal(entryA.revision, '1'); assert.equal(entryB.revision, '1'); assert.notEqual(entryA.id, entryB.id);
  assert.equal(await page.evaluate(() => window.catalogInjection), undefined); assert(!files(positiveStore).some(item => /<img|profile |\.\.\//.test(item.path)), 'Display names are never filesystem paths');
  await page.click('#settings-close'); await openSettings(page); sameJSON(await page.inputValue('#config-profile-json'), textB, 'Catalog create does not adopt A as the active profile');
  await openCatalog(page); const listA = await list(page); assert.equal(listA.value.entries.length, 2);
  const tab = await makePage(context); await openCatalog(tab); const listB = await list(tab); assert.deepEqual(listB.value.entries, listA.value.entries);
  await select(page, entryA.id); const renamed = nameA + ' renamed'; await page.fill('#config-catalog-name', renamed);
  const rename = await mutationUnchanged(page, () => clickJSON(page, '#config-catalog-rename', CATALOG + '/' + entryA.id + '/rename'), 'Rename A'); const previousA = entryA; entryA = entryShape(rename.value.entry);
  assert.equal(entryA.name, renamed); assert.equal(entryA.revision, '2'); assert.equal(entryA.profile_sha256, previousA.profile_sha256); assert.equal(entryA.created_at, previousA.created_at);
  rowA = await storedEntry(entryA, 'renamed-a');
  await select(tab, previousA.id); await tab.fill('#config-catalog-name', previousA.name);
  const staleRows = sqlSnapshot('before-stale-controls'), beforeStaleFiles = files(positiveStore);
  const stalePut = await mutationUnchanged(tab, () => clickJSON(tab, '#config-catalog-replace', CATALOG + '/' + previousA.id, 'PUT', 409), 'Stale two-tab PUT'); await tab.locator('#config-catalog-error').waitFor({state: 'visible'});
  const putWire = stalePut.response.request().postDataJSON();
  assert.deepEqual(Object.keys(putWire).sort(), ['expected_revision', 'expected_sha256', 'name', 'profile_text']);
  assert.equal(putWire.expected_revision, previousA.revision); assert.equal(putWire.expected_sha256, previousA.profile_sha256); assert.equal(putWire.name, previousA.name);
  assert.equal(putWire.profile_text, rowB.profile_text, 'Stale replacement content itself is a genuine valid canonical profile, not an unrelated parser refusal');
  assert.deepEqual(files(positiveStore), beforeStaleFiles); assert.deepEqual(sqlSnapshot('after-stale-put').rows, staleRows.rows);
  const staleDelete = await mutationUnchanged(tab, () => remove(tab, previousA, 409), 'Stale two-tab DELETE'); await tab.locator('#config-catalog-error').waitFor({state: 'visible'});
  assert.deepEqual(staleDelete.response.request().postDataJSON(), {expected_revision: previousA.revision, expected_sha256: previousA.profile_sha256}, 'Actual stale DELETE retains the old tab revision/hash authority');
  assert.deepEqual(files(positiveStore), beforeStaleFiles); assert.deepEqual(sqlSnapshot('after-stale-delete').rows, staleRows.rows);
  negatives.push({name: 'actual-two-tab-stale-put-and-delete', id: previousA.id, request_revision: previousA.revision, request_sha256: previousA.profile_sha256, current_revision: entryA.revision, statuses: [stalePut.response.status(), staleDelete.response.status()], persisted_rows_and_bytes_unchanged: true});
  check('Two real tabs enforce revision/hash CAS: stale PUT and confirmed DELETE return409 without modifying catalog bytes, active/working profile or browser storage');
  await tab.close();
  const changedConfig = {...JSON.parse(textB).config, user_agent: 'OwnedCatalog/replaced', max_redirects: 4};
  const changedText = JSON.stringify({version: 1, config: changedConfig, required_headers: [header]});
  await prepared(page, changedText); await bindHeader(page); await openCatalog(page); await select(page, entryB.id); await page.fill('#config-catalog-name', nameB);
  const replace = await mutationUnchanged(page, () => clickJSON(page, '#config-catalog-replace', CATALOG + '/' + entryB.id, 'PUT'), 'Overwrite B'); const firstB = entryB; entryB = entryShape(replace.value.entry);
  assert.equal(entryB.id, firstB.id); assert.equal(entryB.revision, '2'); assert.notEqual(entryB.profile_sha256, firstB.profile_sha256); assert.equal(entryB.created_at, firstB.created_at);
  rowB = await storedEntry(entryB, 'overwritten-b');
  await mutationUnchanged(page, () => remove(page, entryB), 'Confirmed delete B'); await api(CATALOG + '/' + entryB.id, undefined, 404);
  assert(!sqlSnapshot('deleted-b').rows.some(row => row.id === entryB.id));
  await page.fill('#config-catalog-name', nameB); const recreate = await mutationUnchanged(page, () => clickJSON(page, '#config-catalog-create', CATALOG), 'Recreate B');
  const overwrittenText = rowB.profile_text;
  entryB = entryShape(recreate.value.entry); assert.notEqual(entryB.id, firstB.id); assert.equal(entryB.revision, '1'); rowB = await storedEntry(entryB, 'recreated-b'); assert.equal(rowB.profile_text, overwrittenText);
  await list(page); await delayedLoad(page, entryA, entryB, rowB.profile_text);
  const storageBeforeLoad = await storage(page); await bindHeader(page); await load(page, entryA); assert.deepEqual(await storage(page), storageBeforeLoad, 'Load stages only, without adopting active draft');
  const warmA = await download(page, 'warm-exact-u64-catalog', rowA.profile_text); assert.match(warmA.toString(), /9007199254740993/);
  await load(page, entryB); const warmB = await download(page, 'warm-operational-catalog', rowB.profile_text);
  check('Real browser create, rename, overwrite, delete, recreate and delayed actual native Load preserve exact stored profile bytes/u64, source provenance and volatile-secret boundaries');
  await missingHeaderStart(page, 'Warm catalog reapply'); await openSettings(page); await bindHeader(page); await save(page);
  const startMarker = await profileCapture.mark(page); const starting = waitJSON(page, '/api/crawls/from-profile', 'POST', startMarker); await page.click('#start-button'); const started = await starting; assert.equal(started.response.status(), 200); const jobId = started.value.id;
  assert(jobId); await until(async () => { const job = (await api('/api/crawls/' + jobId)).value; assert.notEqual(job.status, 'failed', job.error); return job.status === 'completed'; }, 'owned catalog profile capture');
  const project = await native('/api/crawls/' + jobId + '/project.json'), report = JSON.parse(project.bytes); assert.equal(report.pages.length, 2);
  assert.deepEqual(new Set(report.pages.map(row => row.url)), new Set(config.seed_urls)); assert.equal(captureHits.length, 2); assert(captureHits.every(hit => hit.header_received));
  const expectedEffective = JSON.parse((await api('/api/config-profiles/inspect', {profile_text: rowB.profile_text})).value.effective_config_json); expectedEffective.custom_headers = {[header]: secret};
  sameJSON(JSON.stringify(report.config), JSON.stringify(expectedEffective), 'Actual saved crawl config equals every operational native field from the loaded catalog plus explicit volatile binding');
  assert.equal(report.config.concurrency, 65); assert.equal(report.config.request_timeout_secs, 301); assert.equal(report.config.max_body_bytes, 1572865); assert.equal(report.config.crawl_links, false); assert.deepEqual(report.config.explicit_sitemap_inputs, []);
  fs.writeFileSync(artifactPath('warm-project.json'), project.bytes); const warmSQL = sqlSnapshot('warm-before-eof');
  offline = true; await witness(); await page.goto('about:blank'); await context.close(); await stop(bootstrap); assert.deepEqual(offlineHits, []);
  // Holding the old owned port prevents the OS from accidentally reusing the old browser origin.
  const reserved = http.createServer((_request, response) => { response.writeHead(410); response.end(); }); await listen(reserved, Number(new URL(bootstrap.origin).port));
  let cold;
  try { cold = await launch('positive-cold', positiveStore); } finally { reserved.closeAllConnections(); await new Promise(resolve => reserved.close(resolve)); }
  assert.notEqual(cold.origin, bootstrap.origin); assert.notEqual(cold.child.pid, bootstrap.child.pid);
  const coldSQL = sqlSnapshot('cold-reopened'); assert.deepEqual(coldSQL.rows, warmSQL.rows); assert.equal(coldSQL.file_sha256, warmSQL.file_sha256, 'Cold startup does not rewrite catalog');
  const coldContext = await makeContext(), coldPage = await makePage(coldContext);
  assert.deepEqual(await storage(coldPage), {}, 'Fresh context and new origin have no browser-local profile or catalog');
  assert.equal(requests.filter(request => request.source === 'browser' && request.phase === cold.phase && request.path.startsWith(CATALOG)).length, 0, 'Cold native catalog is not auto-loaded');
  await openCatalog(coldPage); const coldList = await list(coldPage); assert.deepEqual(new Set(coldList.value.entries.map(entry => entry.id)), new Set([entryA.id, entryB.id]));
  const coldStorage = await storage(coldPage); await load(coldPage, entryA); assert.deepEqual(await storage(coldPage), coldStorage);
  assert.deepEqual(await download(coldPage, 'cold-exact-u64-catalog', rowA.profile_text), warmA);
  await load(coldPage, entryB); assert.deepEqual(await download(coldPage, 'cold-operational-catalog', rowB.profile_text), warmB);
  await missingHeaderStart(coldPage, 'Cold catalog explicit Load and Save');
  await openSettings(coldPage); const coldHeaders = (await working(coldPage)).headers; assert.deepEqual(coldHeaders.map(input => input.name), [header]); assert(coldHeaders.every(input => input.value === ''));
  assert.deepEqual((await native('/api/crawls/' + jobId + '/project.json')).bytes, project.bytes, 'Saved warm report and full config are byte-identical after process death');
  const geometry = [];
  await coldPage.setViewportSize({width: 320, height: 900}); await openCatalog(coldPage); await select(coldPage, entryB.id);
  for (const selector of ['#config-catalog-list', '#config-catalog-select', '#config-catalog-name', '#config-catalog-load', '#config-catalog-create', '#config-catalog-replace', '#config-catalog-rename', '#config-catalog-delete', '#config-profile-export']) {
    await coldPage.locator(selector).scrollIntoViewIfNeeded(); const box = await coldPage.locator(selector).boundingBox(); assert(box && box.x >= 0 && box.x + box.width <= 320 && box.y >= 0 && box.y + box.height <= 900, 'Reachable320px ' + selector); geometry.push({selector, ...box});
  }
  await coldPage.click('#config-catalog-delete'); await coldPage.locator('#config-catalog-delete-question').waitFor({state: 'visible'});
  for (const selector of ['#config-catalog-delete-confirm', '#config-catalog-delete-cancel']) { await coldPage.locator(selector).scrollIntoViewIfNeeded(); const box = await coldPage.locator(selector).boundingBox(); assert(box && box.x >= 0 && box.x + box.width <= 320); geometry.push({selector, ...box}); }
  await coldPage.click('#config-catalog-delete-cancel'); assert.equal((await api(CATALOG)).value.entries.length, 2);
  await coldPage.locator('#config-profile-export').scrollIntoViewIfNeeded(); assert.deepEqual(await download(coldPage, 'cold-operational-320', rowB.profile_text), warmB);
  const documentGeometry = await coldPage.evaluate(() => ({viewport: innerWidth, document: document.documentElement.scrollWidth})); assert.equal(documentGeometry.viewport, 320); assert(documentGeometry.document <= 320);
  await coldPage.screenshot({path: artifactPath('catalog-cold-320.png'), fullPage: true}); await coldPage.goto('about:blank'); await coldContext.close(); await stop(cold); await witness();
  assert.deepEqual(offlineHits, []); assert.equal(witnesses.length, 2); assert.deepEqual(errors, []); assert.deepEqual(external, []); verifyPins();
  assert.equal(owners.filter(owner => owner.phase.startsWith('positive-')).length, 2); assert.equal(owners.filter(owner => owner.phase.startsWith('negative-')).length, 2);
  assert(owners.every(owner => owner.natural_eof && !owner.forced && owner.close.code === 0 && owner.close.signal === null));
  check('Two positive normal native processes share the actual catalog across different origins and fresh browser contexts; cold explicit reapply preserves full bytes and missing-header refusal,320px works,410 remains through both EOF0 closes');
  return {job_id: jobId, positive_store: positiveStore, profile_ids: [entryA.id, entryB.id], warm_project: {bytes: project.bytes.length, sha256: hash(project.bytes)}, warm_catalog_sha256: warmSQL.file_sha256, cold_catalog_sha256: coldSQL.file_sha256,
    origin_scope: 'Two different owned positive origins and a fresh cold browser context; localStorage initially empty. Catalog is native data-dir storage, not browser storage.', capture_scope: 'One successful warm catalog-loaded crawl; cold explicit load/save and missing-header Start refusal only. No successful second crawl while410 witness is active.', geometry: {controls: geometry, ...documentGeometry}};
}
(async () => {
  const started = Date.now();
  try { proof = await run(); } catch (error) { failure = error; }
  finally {
    if (failure && browser) { const pages = browser.contexts().flatMap(context => context.pages()); for (let index = 0; index < pages.length; index++) { try { await pages[index].screenshot({path: artifactPath('catalog-failure-' + index + '.png'), timeout: 10000}); } catch {} } }
    if (browser) { try { await deadline(browser.close(), 30000, 'owned browser close'); } catch (error) { failure ||= error; } }
    for (const owner of owners) if (!owner.close) { owner.forced = true; owner.child.kill(); try { await deadline(owner.closed, 10000, 'forced owned cleanup close'); } catch (error) { owner.child.stdin.destroy(); owner.child.stdout.destroy(); owner.child.stderr.destroy(); owner.child.unref(); failure ||= error; } }
    if (site?.listening) { site.closeAllConnections(); try { await deadline(new Promise(resolve => site.close(resolve)), 10000, 'owned site close'); } catch (error) { failure ||= error; } }
    for (const owner of owners) fs.writeFileSync(artifactPath('native-' + owner.phase + '.log'), owner.log);
    fs.writeFileSync(artifactPath('ui-config-profile-catalog-live.json'), JSON.stringify({result: failure ? 'FAIL' : 'PASS', duration_ms: Date.now() - started, binary, binary_sha256: binaryHash, browser: chrome, browser_sha256: chromeHash, output, pins, ...proof, checks, requests, inspected_responses: inspectedResponses, negatives, downloads, sql_proofs: sqlProofs, held_actual_responses: held,
      native: owners.map(owner => ({phase: owner.phase, directory: owner.directory, pid: owner.child.pid, origin: owner.origin, close: owner.close, natural_stdin_eof: owner.natural_eof, forced: owner.forced})), website_capture_hits: captureHits, offline_product_hits: offlineHits, offline_positive_controls: witnesses, browser_errors: errors, external_requests: external, scope: 'Bounded native named-profile catalog CRUD, CAS, exact profile bytes, explicit activation and warm-capture/cold-reapply. Not128-entry/64MiB/256MiB boundary acceptance, crash durability, cross-platform matrix or full vendor parity.', ...(failure ? {error: failure.stack} : {})}, null, 2) + '\n');
  }
  if (failure) throw failure; console.log(checks.length + ' actual config-profile catalog groups passed.');
})().catch(error => { console.error(error.stack); process.exitCode = 1; });
