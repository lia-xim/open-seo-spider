'use strict';
// Real browser/file picker -> pure native profile parser/export -> explicit crawl
// -> stored configuration -> two native processes on one owned store/origin.
// Delayed-file/response controls delay real bytes only; no successful API is mocked.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawn} = require('node:child_process');
const {createHash, randomUUID} = require('node:crypto');
const {chromium} = require('../../renderer/node_modules/playwright-core');
const root = path.resolve(__dirname, '../..');
assert(process.env.SPIDER_BINARY, 'An explicit reviewed SPIDER_BINARY is required; this driver never builds or discovers an installed application');
const binary = fs.realpathSync(path.resolve(process.env.SPIDER_BINARY));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const binaryHash = hash(fs.readFileSync(binary));
if (process.env.SPIDER_BINARY_SHA256) assert.equal(binaryHash, process.env.SPIDER_BINARY_SHA256);
const output = path.resolve(process.env.SPIDER_CONFIG_PROFILES_OUTPUT || path.join(process.env.SPIDER_TEST_ARTIFACT_DIR || path.join(root, 'artifacts'), 'config-profiles-' + Date.now() + '-' + randomUUID().slice(0, 8)));
assert(!fs.existsSync(output), 'A fresh owned evidence directory is required');
fs.mkdirSync(output, {recursive: true});
process.env.SPIDER_TEST_ARTIFACT_DIR = output;
const artifactPath = require('./artifact-path.cjs');
const profileCapture = require('./profile-response-capture.cjs');
const store = path.join(output, 'native-store');
const storageKey = 'open-seo-spider-config-profile-draft-v1';
const pins = [__filename, path.join(__dirname, 'artifact-path.cjs'), path.join(__dirname, 'profile-response-capture.cjs')].map(file => ({file, sha256: hash(fs.readFileSync(file))}));
const servedAssets = [];
const checks = [], owners = [], downloads = [], requests = [], negatives = [], errors = [], external = [], heldResponses = [];
const initialHits = [], offlineHits = [], witnessHits = [], nonce = randomUUID();
const secretValues = ['owned-volatile-one-' + randomUUID(), 'owned-volatile-two-' + randomUUID()];
const headerNames = ['X-Owned-Config', 'x-owned-config'];
let app, siteOrigin, browser, context, page, site, offline = false, failure, jobId;
const capturePolicies = new WeakMap();
const inspectedResponses = [];
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
function check(label) { checks.push(label); console.log('PASS ' + label); }
function deadline(promise, ms, label) {
  let timer;
  return Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Timeout: ' + label)), ms); })]).finally(() => clearTimeout(timer));
}
async function until(work, label, ms = 45000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (app) assert.equal(app.child.exitCode, null, 'Owned native process exited during ' + label);
    const result = await work(); if (result) return result;
    await sleep(25);
  }
  throw Error('Timeout: ' + label);
}
function verifyPins() {
  assert.equal(hash(fs.readFileSync(binary)), binaryHash, 'Native binary changed');
  for (const pin of pins) assert.equal(hash(fs.readFileSync(pin.file)), pin.sha256, 'Owned harness changed');
}
async function listen(server) {
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  assert(![8787, 8790].includes(server.address().port));
  return 'http://127.0.0.1:' + server.address().port;
}
async function launch(phase, address = '127.0.0.1:0') {
  verifyPins(); const env = {...process.env}; delete env.OPEN_SEO_PAGESPEED_API_KEY;
  const args = ['serve', '--listen', address, '--data-dir', store, '--shutdown-on-stdin-eof'];
  const child = spawn(binary, args, {cwd: root, env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe']});
  const owner = {phase, child, args, origin: null, log: '', close: null, forced: false, natural_stdin_eof: false};
  owner.closed = new Promise(resolve => child.once('close', (code, signal) => { owner.close = {code, signal}; resolve(owner.close); }));
  child.once('error', error => { owner.spawnError = error; });
  for (const stream of [child.stdout, child.stderr]) stream.on('data', bytes => { owner.log += bytes.toString(); });
  owners.push(owner); app = owner;
  await until(() => {
    if (owner.spawnError) throw owner.spawnError;
    // A complete post-bind line, including newline, is the readiness witness.
    const match = /(?:^|\n)Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/.exec(owner.log);
    return match && match[1];
  }, 'native bind ' + phase).then(origin => { owner.origin = origin; });
  assert(!['8787', '8790'].includes(new URL(owner.origin).port));
  const health = await fetch(owner.origin + '/healthz', {signal: AbortSignal.timeout(10000)}); assert.equal(health.status, 200); await health.arrayBuffer();
  return owner;
}
async function stop(owner) {
  assert(owner && !owner.close); owner.child.stdin.end();
  const close = await deadline(owner.closed, 20000, 'natural stdin EOF ' + owner.phase);
  assert.deepEqual(close, {code: 0, signal: null}); owner.natural_stdin_eof = true;
  fs.writeFileSync(artifactPath('native-' + owner.phase + '.log'), owner.log);
  if (app === owner) app = null;
  verifyPins();
}
async function raw(route, body, expected = 200, origin = app.origin) {
  const response = await fetch(origin + route, {method: body === undefined ? 'GET' : 'POST', headers: body === undefined ? {} : {'content-type': 'application/json'}, body: body === undefined ? undefined : JSON.stringify(body), cache: 'no-store', signal: AbortSignal.timeout(30000)});
  const bytes = Buffer.from(await response.arrayBuffer());
  assert.equal(response.status, expected, 'Unexpected native status for ' + route + ': ' + response.status);
  requests.push({source: 'direct-native', method: body === undefined ? 'GET' : 'POST', path: route, status: response.status, bytes: bytes.length, sha256: hash(bytes)});
  return {response, bytes};
}
async function json(route, body, expected = 200) { const result = await raw(route, body, expected); return {...result, value: JSON.parse(result.bytes)}; }
async function rejectedWire(bytes, contentType, expected) {
  const response = await fetch(app.origin + '/api/config-profiles/inspect', {method: 'POST', headers: {'content-type': contentType}, body: bytes, signal: AbortSignal.timeout(30000)});
  const result = Buffer.from(await response.arrayBuffer()); assert.equal(response.status, expected);
  requests.push({source: 'direct-native-wire-rejection', method: 'POST', path: '/api/config-profiles/inspect', content_type: contentType, status: response.status, bytes: result.length, sha256: hash(result)});
  return response.status;
}
const documentText = config => JSON.stringify({version: 1, config, required_headers: []});
// Node's source-aware JSON reviver compares numbers without silently rounding u64.
function lossless(text) {
  return JSON.parse(text, (_key, value, source) => typeof value === 'number' ? {__native_number_lexeme: source.source} : value);
}
function sameConfig(left, right, label) { assert.deepEqual(lossless(left), lossless(right), label); }
function noSecrets(text, label) { for (const value of secretValues) assert(!text.includes(value), label + ': volatile header value leaked'); }
function nativeFixtureError(value) {
  let message = typeof value?.error === 'string' ? value.error : 'Native response without an error string';
  for (const secret of secretValues) message = message.split(secret).join('[redacted owned header value]');
  return message.slice(0, 1500);
}
function tree(directory) {
  if (!fs.existsSync(directory)) return [];
  const result = [];
  function visit(dir, prefix) { for (const name of fs.readdirSync(dir).sort()) { const file = path.join(dir, name), stat = fs.lstatSync(file), relative = prefix + name; assert(!stat.isSymbolicLink()); if (stat.isDirectory()) visit(file, relative + '/'); else result.push({path: relative, bytes: stat.size, sha256: hash(fs.readFileSync(file))}); } }
  visit(directory, ''); return result;
}
async function effects() { return {files: tree(store), jobs: (await json('/api/crawls')).value, website: initialHits.length, offline: offlineHits.length}; }
async function pureUnchanged(before, label) { assert.deepEqual(await effects(), before, label); }
async function storage() { return page.evaluate(() => Object.fromEntries(Object.keys(localStorage).sort().map(key => [key, localStorage.getItem(key)]))); }
async function browserTurns() { await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))); }
async function draft() {
  return page.evaluate(() => ({profile: document.getElementById('config-profile-json').value, effective: document.getElementById('config-profile-effective').textContent, controls: [...document.getElementById('settings-form').elements].filter(input => input.name && !['file', 'password', 'submit', 'button'].includes(input.type)).map(input => ({name: input.name, value: input.type === 'checkbox' ? input.checked : input.value, disabled: input.disabled}))}));
}
async function openSettings() {
  if (!await page.locator('#settings-dialog').evaluate(el => el.open)) await page.click('#settings-open');
  const editor = page.locator('#config-profiles > details:has(#config-profile-json)');
  if (!await editor.evaluate(el => el.open)) await editor.locator(':scope > summary').click();
}
async function selectJob(id, capturedPages) {
  await page.selectOption('#crawl-history', id);
  await page.waitForFunction(({id, pages}) => document.getElementById('crawl-history').value === id && document.getElementById('metric-fetched').textContent === String(pages), {id, pages: capturedPages});
}
function captureBrowserResponses() {
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    const url = request.url();
    if (url.startsWith(app.origin + '/')) {
      const p = new URL(url).pathname;
      if (p.startsWith('/api/')) requests.push({source: 'browser', method: request.method(), path: p, request_sha256: hash(Buffer.from(request.postData() || ''))});
    } else if (!url.startsWith('blob:') && !url.startsWith('data:') && url !== 'about:blank') external.push(url);
  });
}
async function browserJSON(pending, marker) {
  const target = page, response = await pending, request = response.request(), raw = request.postDataBuffer();
  const observed = {method: request.method(), path: new URL(response.url()).pathname, url: response.url(), status: response.status(), observed_headers: response.headers(), request_sha256: hash(raw || Buffer.alloc(0)), request_bytes: raw?.length || 0, marker, result: 'BODY_PENDING'};
  inspectedResponses.push(observed); // Preserve original wire metadata even if body capture refuses.
  try {
    const policy = capturePolicies.get(target);
    assert(policy && profileCapture.canCapture(request.method(), response.url(), policy), 'Consumed JSON must belong to this exact owned profile API policy');
    assert(marker, 'An awaited pre-action browser marker is required for every consumed original JSON response');
    const result = await profileCapture.capture(target, response, marker);
    Object.assign(observed, result.evidence, {bytes: result.bytes.length, sha256: hash(result.bytes), result: 'PASS'});
    return {response, bytes: result.bytes, value: JSON.parse(result.bytes)};
  } catch (error) {
    Object.assign(observed, {result: 'FAIL', error: error.message});
    throw error;
  }
}
function waitJSON(route, method = 'POST', marker) { const origin = capturePolicies.get(page)?.origin; assert(origin, 'Owned capture policy must be installed before actions'); return browserJSON(page.waitForResponse(response => response.url() === origin + route && response.request().method() === method), marker); }
async function inspectEditor(text) {
  await page.fill('#config-profile-json', text);
  const marker = await profileCapture.mark(page);
  const pending = waitJSON('/api/config-profiles/inspect', 'POST', marker); await page.click('#config-profile-inspect');
  const result = await pending; assert.equal(result.response.status(), 200); await installed(result.value); return result.value;
}
async function installed(value) {
  assert.equal(value.version, 1); assert.equal(value.engine, 'open-seo-spider/crawl-config-profile-v1'); assert.equal(value.max_bytes, 67108864);
  noSecrets(JSON.stringify(value), 'native sanitized preview');
  await page.waitForFunction(text => document.getElementById('config-profile-effective').textContent === text, value.effective_config_json);
  sameConfig(await page.inputValue('#config-profile-json'), value.profile_json, 'Imported declared profile');
}
async function upload(name, bytes, expected = 200) {
  const marker = await profileCapture.mark(page);
  const pending = waitJSON('/api/config-profiles/inspect', 'POST', marker);
  const chooser = page.waitForEvent('filechooser'); await page.click('#config-profile-import');
  await (await chooser).setFiles({name, mimeType: 'application/json', buffer: Buffer.isBuffer(bytes) ? bytes : Buffer.from(bytes)});
  const result = await pending; assert.equal(result.response.status(), expected, 'Owned file fixture ' + name + ': ' + nativeFixtureError(result.value));
  if (expected === 200) await installed(result.value);
  else await page.locator('#config-profile-error').waitFor({state: 'visible'});
  return result;
}
async function save() {
  const marker = await profileCapture.mark(page);
  const pending = waitJSON('/api/config-profiles/inspect', 'POST', marker); await page.click('#settings-form button[type=submit]');
  const result = await pending; assert.equal(result.response.status(), 200);
  await page.waitForFunction(() => !document.getElementById('settings-dialog').open);
  const stored = (await storage())[storageKey]; assert(stored); noSecrets(stored, 'profile localStorage'); return {result: result.value, stored};
}
async function profileDownload(label) {
  const priorStorage = await storage();
  const responsePending = page.waitForResponse(response => response.url() === app.origin + '/api/config-profiles/export' && response.request().method() === 'POST');
  const downloadPending = page.waitForEvent('download'); await page.click('#config-profile-export');
  const [observed, download] = await Promise.all([responsePending, downloadPending]); assert.equal(observed.status(), 200);
  // Unconditional independent exact native pure-POST oracle, never a mutation retry.
  const request = observed.request(); assert.equal(new URL(observed.url()).origin, app.origin);
  const body = request.postDataJSON(), oracle = await raw('/api/config-profiles/export', body);
  const file = artifactPath(label + '.json'); await download.saveAs(file); const bytes = fs.readFileSync(file);
  assert.deepEqual(bytes, oracle.bytes, 'Actual browser download must equal the exact native export'); noSecrets(bytes.toString('utf8'), 'profile attachment');
  for (const headers of [observed.headers(), Object.fromEntries(oracle.response.headers)]) {
    assert.equal(Number(headers['content-length']), bytes.length); assert.equal(headers['x-content-sha256'], hash(bytes)); assert.equal(headers['cache-control'], 'no-store'); assert.match(headers['content-disposition'], /attachment/);
  }
  const proof = {label, path: file, bytes: bytes.length, sha256: hash(bytes), method: 'POST', native_path: '/api/config-profiles/export', request_sha256: hash(Buffer.from(request.postData())), suggested_filename: download.suggestedFilename()}; downloads.push(proof);
  assert.deepEqual(await storage(), priorStorage, 'Export must not apply the dialog draft to localStorage');
  return bytes;
}
async function requirements() {
  return page.locator('#config-profile-required-headers input[data-profile-header]').evaluateAll(inputs => inputs.map(input => ({name: input.dataset.profileHeader, type: input.type, value: input.value})));
}
async function resolveHeaders() {
  const inputs = page.locator('#config-profile-required-headers input[data-profile-header]');
  assert.deepEqual((await requirements()).map(input => input.name).sort(), headerNames.slice().sort());
  for (let i = 0; i < await inputs.count(); i++) { const input = inputs.nth(i), name = await input.getAttribute('data-profile-header'); assert.equal(await input.getAttribute('type'), 'password'); await input.fill(secretValues[headerNames.indexOf(name)]); }
}
async function witness() {
  const response = await fetch(siteOrigin + '/offline-control', {headers: {'x-owned-config-witness': nonce}, signal: AbortSignal.timeout(10000)});
  assert.equal(response.status, 410); assert.equal(await response.text(), 'Owned offline fixture');
}
async function corruptedRestores(valid) {
  const cases = [
    ['malformed-stored-json', '{"version":1,"profile_text":'],
    ['stored-profile-text-wrong-type', '{"version":1,"profile_text":42}'],
    ['unsupported-stored-wrapper-version', JSON.stringify({version: 2, profile_text: valid})],
  ];
  for (const [name, badBytes] of cases) {
    if (await page.locator('#settings-dialog').evaluate(el => el.open)) await page.click('#settings-close');
    const baseline = await effects();
    await page.evaluate(({key, text}) => localStorage.setItem(key, text), {key: storageKey, text: badBytes});
    await page.reload(); await page.locator('#connection-state.connected').waitFor();
    await page.locator('#config-profile-restore-error').waitFor({state: 'visible'});
    assert(await page.isDisabled('#start-button'), name + ' must block the actual start button after restore has finished');
    assert.equal((await storage())[storageKey], badBytes, 'Restore must not rewrite or silently replace the damaged stored bytes');
    await page.fill('#seed-url', siteOrigin + '/a');
    const startsBefore = requests.filter(request => request.source === 'browser' && ['/api/crawls', '/api/crawls/from-profile'].includes(request.path) && request.method === 'POST').length;
    await page.evaluate(() => document.getElementById('crawl-form').dispatchEvent(new Event('submit', {bubbles: true, cancelable: true})));
    await browserTurns();
    assert.equal(requests.filter(request => request.source === 'browser' && ['/api/crawls', '/api/crawls/from-profile'].includes(request.path) && request.method === 'POST').length, startsBefore, 'Programmatic submit must also refuse silent legacy fallback');
    await openSettings();
    await page.locator('#config-profile-error').waitFor({state: 'visible'}); assert(await page.locator('#config-profile-restore-error').isVisible());
    assert.equal((await storage())[storageKey], badBytes); await pureUnchanged(baseline, name + ' remains effect-free after attempted Start and Settings open');
    await page.click('#settings-close'); assert(await page.locator('#config-profile-restore-error').isVisible()); assert(await page.isDisabled('#start-button'));
    await openSettings(); await upload(name + '-explicit-recovery.json', valid);
    assert(await page.locator('#config-profile-restore-error').isVisible(), 'A valid import alone cannot acknowledge failed stored restoration');
    assert.equal((await storage())[storageKey], badBytes, 'Recovery import stages only a working copy');
    await save(); await page.locator('#config-profile-restore-error').waitFor({state: 'hidden'});
    assert.notEqual((await storage())[storageKey], badBytes, 'Only explicit valid native Save replaces invalid storage');
    noSecrets(JSON.stringify(await storage()), 'explicit restore recovery'); await pureUnchanged(baseline, name + ' recovery is pure');
    negatives.push({name, source: 'actual_owned_browser_storage_reload', damaged_storage_sha256: hash(Buffer.from(badBytes)), button_blocked: true, programmatic_start_blocked: true, storage_unchanged_until_explicit_save: true, valid_import_alone_does_not_clear_guard: true, explicit_valid_save_recovers: true});
  }
  await openSettings();
}
async function pendingRestore(valid) {
  if (await page.locator('#settings-dialog').evaluate(el => el.open)) await page.click('#settings-close');
  const stored = (await storage())[storageKey], wrapper = JSON.parse(stored); sameConfig(wrapper.profile_text, valid, 'Real stored startup profile');
  const baseline = await effects(), pattern = app.origin + '/api/config-profiles/inspect';
  let release, held, settled = false, cancelled = false, armed = true, transportError;
  const barrier = new Promise(resolve => { release = resolve; });
  await page.route(pattern, async route => {
    const input = route.request().postDataJSON();
    if (!armed || input.profile_text !== wrapper.profile_text) return route.continue();
    armed = false;
    try {
      const response = await route.fetch(), bytes = await response.body(); assert.equal(response.status(), 200);
      held = {request: route.request(), status: response.status(), bytes, headers: response.headers()};
      await barrier;
      try { await route.fulfill({status: held.status, headers: held.headers, body: held.bytes}); }
      catch (error) { if (!/abort|cancel/i.test(held.request.failure()?.errorText || '')) throw error; cancelled = true; }
    } catch (error) { transportError = error; }
    finally { settled = true; }
  });
  try {
    await page.reload({waitUntil: 'domcontentloaded'});
    await until(() => { if (transportError) throw transportError; return held; }, 'held actual startup restore response');
    await openSettings(); // This invalidates/aborts the pending restore owner.
    await page.locator('#connection-state.connected').waitFor();
    await page.locator('#config-profile-restore-error').waitFor({state: 'visible'}); await page.locator('#config-profile-error').waitFor({state: 'visible'});
    assert(await page.isDisabled('#start-button')); assert.equal((await storage())[storageKey], stored);
    await page.click('#settings-close'); await page.fill('#seed-url', siteOrigin + '/a');
    const startsBefore = requests.filter(request => request.source === 'browser' && ['/api/crawls', '/api/crawls/from-profile'].includes(request.path) && request.method === 'POST').length;
    await page.evaluate(() => document.getElementById('crawl-form').dispatchEvent(new Event('submit', {bubbles: true, cancelable: true})));
    await browserTurns(); assert(await page.isDisabled('#start-button'));
    assert.equal(requests.filter(request => request.source === 'browser' && ['/api/crawls', '/api/crawls/from-profile'].includes(request.path) && request.method === 'POST').length, startsBefore);
    await pureUnchanged(baseline, 'Aborted valid startup restore cannot silently fall back to legacy Start');
    const replacement = JSON.parse(valid); replacement.config.user_agent = 'OwnedConfigProfile/new-owner-after-startup-restore';
    const replacementText = JSON.stringify(replacement);
    await openSettings(); const imported = await upload('explicit-recovery-after-pending-restore.json', replacementText);
    assert(await page.locator('#config-profile-restore-error').isVisible()); assert.equal((await storage())[storageKey], stored);
    await save(); await page.locator('#config-profile-restore-error').waitFor({state: 'hidden'}); await openSettings();
    const currentDraft = await draft(), currentStorage = await storage(); sameConfig(currentDraft.profile, imported.value.profile_json, 'New current owner before old startup reply release');
    release(); await until(() => { if (transportError) throw transportError; return settled; }, 'old startup response terminal');
    const oldResponse = await deadline(held.request.response(), 10000, 'old startup response completion');
    if (oldResponse) await deadline(oldResponse.finished(), 10000, 'old startup response finished');
    await browserTurns(); assert.deepEqual(await draft(), currentDraft); assert.deepEqual(await storage(), currentStorage); assert(await page.locator('#config-profile-restore-error').isHidden());
    await page.click('#settings-close'); await openSettings();
    sameConfig(await page.inputValue('#config-profile-json'), imported.value.profile_json, 'Old startup reply cannot replace the actual active profile');
    assert.equal(JSON.parse(await page.locator('#config-profile-effective').textContent()).user_agent, replacement.config.user_agent);
    await pureUnchanged(baseline, 'Explicit recovery and late actual startup reply remain pure');
    heldResponses.push({kind: 'actual_startup_restore_delayed_before_settings_open', status: held.status, bytes: held.bytes.length, sha256: hash(held.bytes), cancelled, current_profile_sha256: hash(Buffer.from(imported.value.profile_json)), legacy_start_blocked: true, old_reply_cannot_replace_active_or_storage: true});
    negatives.push({name: 'valid-stored-profile-restore-pending-then-settings-open', source: 'actual_browser_reload_held_native_response', button_blocked: true, programmatic_start_blocked: true, original_storage_unchanged_until_explicit_save: true, valid_import_alone_does_not_clear_guard: true, explicit_valid_save_recovers: true, late_old_reply_cannot_poison_new_active: true});
    await upload('restore-original-profile-after-race.json', valid); await save(); await openSettings();
  } finally { release(); await page.unroute(pattern); }
}
async function invalidCorpus(valid) {
  const config = JSON.parse(valid).config;
  const changed = patch => documentText({...config, ...patch});
  const cases = [
    ['malformed', '{'], ['duplicate-version', '{"version":1,"version":1,"config":{},"required_headers":[]}'],
    ['nested-duplicate', '{"version":1,"config":{"concurrency":65,"concurrency":66},"required_headers":[]}'],
    ['unknown-envelope', JSON.stringify({...JSON.parse(valid), unexpected: true})],
    ['unknown-native', changed({unexpected_native_option: true})], ['version', JSON.stringify({...JSON.parse(valid), version: 2})],
    ['top-array', '[]'], ['struct-array', changed({renderer: []})], ['config-array', '{"version":1,"config":[],"required_headers":[]}'],
    ['unsafe-key', '{"version":1,"config":{"__proto__":{}},"required_headers":[]}'],
    ['credential-url', changed({seed_urls: ['http://owned:credential@127.0.0.1:1/']})],
    ['filesystem-path', changed({data_dir: path.join(output, 'forbidden-import-store')})],
    ['certificate-path', changed({tls: {version: 1, certificate_path: path.join(output, 'never-read.pem')}})],
    ['private-key', changed({tls: {version: 1, trusted_certificates: [{format: 'pem', name: 'private.pem', data: '-----BEGIN PRIVATE KEY-----\nT1dORUQ=\n-----END PRIVATE KEY-----'}]}})],
    ['invalid-regex', changed({include_patterns: ['(']})], ['invalid-selector', changed({content_capture: {sources: ['raw_http_html'], include_selectors: ['[']}})],
    ['duplicate-required', JSON.stringify({...JSON.parse(valid), required_headers: ['X-Owned', 'X-Owned']})],
  ];
  // Real native TXT/XML imports establish valid complete provenance first.
  // Adding exactly one unknown unit-variant property must then fail at the
  // portable guard, rather than because the surrounding import was malformed.
  assert.equal(config.input_imports[0].options.format.kind, 'txt');
  const xml = '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>' + siteOrigin + '/a</loc></url></urlset>';
  const xmlImport = (await json('/api/lists/import', {content_base64: Buffer.from(xml).toString('base64'), options: {format: {kind: 'xml'}}})).value;
  assert.equal(xmlImport.options.format.kind, 'xml'); assert.equal(xmlImport.rows.length, 1);
  for (const [kind, imported] of [['txt', config.input_imports[0]], ['xml', xmlImport]]) {
    const base = {...config, input_imports: [structuredClone(imported)]}, baseText = documentText(base);
    const confirmed = await json('/api/config-profiles/inspect', {profile_text: baseText});
    sameConfig(confirmed.value.config_json, JSON.stringify(base), 'Native accepted the genuine ' + kind + ' unit-variant base');
    base.input_imports[0].options.format.owned_unknown_unit_extra = true;
    cases.push(['tagged-' + kind + '-unknown-extra', documentText(base), {guard: 'internally_tagged_unit', valid_base_status: confirmed.response.status, valid_base_sha256: hash(Buffer.from(baseText))}]);
  }
  return cases;
}
async function staleFile(validA, validB) {
  const bytesA = Buffer.from(validA), bytesB = Buffer.from(validB); assert.notEqual(bytesA.length, bytesB.length);
  await page.evaluate(size => {
    const original = Blob.prototype.arrayBuffer;
    window.__ownedProfileFileGate = {size, waiting: false, release: null};
    Blob.prototype.arrayBuffer = function(...args) {
      const result = original.apply(this, args), gate = window.__ownedProfileFileGate;
      if (!gate || this.size !== gate.size) return result;
      gate.waiting = true;
      return result.then(bytes => new Promise(resolve => { gate.release = () => resolve(bytes); }));
    };
    window.__ownedRestoreProfileFileRead = () => { Blob.prototype.arrayBuffer = original; delete window.__ownedProfileFileGate; };
  }, bytesA.length);
  try {
    let chooser = page.waitForEvent('filechooser'); await page.click('#config-profile-import'); await (await chooser).setFiles({name: 'delayed-A.json', mimeType: 'application/json', buffer: bytesA});
    await page.waitForFunction(() => window.__ownedProfileFileGate.waiting && typeof window.__ownedProfileFileGate.release === 'function');
    const chosen = await upload('current-B.json', bytesB); const accepted = await draft(), stored = await storage();
    const requestCount = requests.filter(r => r.source === 'browser' && r.path === '/api/config-profiles/inspect').length;
    await page.evaluate(() => window.__ownedProfileFileGate.release()); await browserTurns();
    assert.deepEqual(await draft(), accepted); assert.deepEqual(await storage(), stored);
    assert(await page.locator('#config-profile-error').isHidden()); assert(await page.isEnabled('#config-profile-inspect'));
    assert.equal(requests.filter(r => r.source === 'browser' && r.path === '/api/config-profiles/inspect').length, requestCount, 'Superseded file must not even issue validation');
    return chosen.value;
  } finally { await page.evaluate(() => window.__ownedRestoreProfileFileRead()); }
}
async function staleNative(validA, validB) {
  const gates = new Map([validA, validB].map(text => { let release; const barrier = new Promise(resolve => { release = resolve; }); return [text, {barrier, release, held: null, settled: false, cancelled: false}]; }));
  const pattern = app.origin + '/api/config-profiles/inspect';
  await page.route(pattern, async route => {
    const input = route.request().postDataJSON();
    const gate = gates.get(input.profile_text); if (!gate) return route.continue();
    gate.request = route.request();
    // Obtain ONE actual native validation response, then hold its real body.
    const native = await route.fetch(); const bytes = await native.body();
    gate.held = {status: native.status(), bytes, headers: native.headers()};
    assert.equal(gate.held.status, 200); await gate.barrier;
    try { await route.fulfill({status: gate.held.status, headers: gate.held.headers, body: gate.held.bytes}); }
    catch (error) {
      // A genuine abort of superseded A is a valid ownership outcome. No other
      // transport failure is accepted, and current B still must complete200.
      const reason = route.request().failure()?.errorText;
      if (input.profile_text !== validA || !reason || !/abort|cancel/i.test(reason)) throw error;
      gate.cancelled = true;
    } finally { gate.settled = true; }
  });
  try {
    await page.fill('#config-profile-json', validA); await page.click('#config-profile-inspect');
    await until(() => gates.get(validA).held, 'held real native A response');
    await page.fill('#config-profile-json', validB);
    const marker = await profileCapture.mark(page);
    const pending = browserJSON(page.waitForResponse(response => response.url() === pattern && response.request().method() === 'POST' && response.request().postDataJSON().profile_text === validB), marker);
    await page.click('#config-profile-inspect'); await until(() => gates.get(validB).held, 'held real native B response');
    assert(await page.isDisabled('#config-profile-export')); assert(await page.isDisabled('#settings-form button[type=submit]'));
    gates.get(validA).release(); await until(() => gates.get(validA).settled, 'release superseded A while B remains pending');
    const oldResponse = await deadline(gates.get(validA).request.response(), 10000, 'old actual response terminal');
    if (oldResponse) await deadline(oldResponse.finished(), 10000, 'old actual response finished');
    await browserTurns();
    assert(await page.isDisabled('#config-profile-export')); assert(await page.isDisabled('#settings-form button[type=submit]'), 'Old A completion cannot unlock newer pending B');
    gates.get(validB).release(); const current = await pending; assert.equal(current.response.status(), 200); await installed(current.value);
    await until(() => page.isEnabled('#config-profile-export'), 'current B controls');
    const accepted = await draft(), stored = await storage(); await browserTurns();
    assert.deepEqual(await draft(), accepted); assert.deepEqual(await storage(), stored); assert(await page.locator('#config-profile-error').isHidden());
    for (const [text, gate] of gates) heldResponses.push({kind: 'real_native_inspect_delay_only', superseded: text === validA, cancelled: gate.cancelled, status: gate.held.status, bytes: gate.held.bytes.length, sha256: hash(gate.held.bytes), latest_profile_sha256: hash(Buffer.from(current.value.profile_json))});
  } finally { for (const gate of gates.values()) gate.release(); await page.unroute(pattern); }
}
async function run() {
  site = http.createServer((req, res) => {
    if (offline) { (req.headers['x-owned-config-witness'] === nonce ? witnessHits : offlineHits).push({method: req.method, path: req.url}); res.writeHead(410, {'content-type': 'text/plain'}); return res.end('Owned offline fixture'); }
    initialHits.push({method: req.method, path: req.url, explicit_header_bound: req.headers['x-owned-config'] === secretValues[1]});
    if (!['/a', '/b', '/replacement'].includes(req.url)) { res.writeHead(500); return res.end('Unexpected discovery'); }
    res.writeHead(200, {'content-type': 'text/html;charset=utf-8'}); res.end('<html lang="en"><head><title>Owned profile ' + req.url + '</title><meta name="description" content="Independent config profile capture"></head><body><main><h1>Owned profile fixture</h1><p>Exact options survive a real native crawl.</p><a href="/must-not-follow">No outward discovery</a></main></body></html>');
  });
  siteOrigin = await listen(site); const first = await launch('warm');
  const executablePath = process.env.SPIDER_CHROME || process.env.OPEN_SEO_BROWSER_PATH || await (await import('../../renderer/server.mjs')).findBrowserExecutable();
  const chromeHash = hash(fs.readFileSync(executablePath)); browser = await chromium.launch({headless: true, executablePath});
  context = await browser.newContext({acceptDownloads: true, viewport: {width: 1440, height: 1000}}); page = await context.newPage(); captureBrowserResponses();
  const capturePolicy = {name: profileCapture.OWNED_POLICY, origin: app.origin}; await profileCapture.install(page, {}, capturePolicy); capturePolicies.set(page, capturePolicy);
  for (const [file, route] of [['index.html', '/'], ['app.js', '/app.js'], ['styles.css', '/styles.css'], ['config-profiles.js', '/config-profiles.js']]) {
    const source = path.join(root, 'web', file), sourceHash = hash(fs.readFileSync(source));
    const served = await raw(route); assert.equal(hash(served.bytes), sourceHash, 'Actual native embedded asset must match the frozen frontend source');
    servedAssets.push({file: 'web/' + file, route, sha256: sourceHash}); pins.push({file: source, sha256: sourceHash});
  }
  await page.goto(app.origin); await page.locator('#connection-state.connected').waitFor(); await openSettings();
  const pureBaseline = await effects();
  const defaults = (await json('/api/config-profiles/default')).value;
  assert.equal(defaults.requires_targets, true); assert.deepEqual(defaults.missing_headers, []);
  const full = JSON.parse(defaults.effective_config_json);
  const inputText = siteOrigin + '/a\n' + siteOrigin + '/b\n' + siteOrigin + '/a\n';
  const provenance = (await json('/api/lists/import', {content_base64: Buffer.from(inputText).toString('base64'), options: {format: {kind: 'txt'}}})).value;
  const nativeConfig = {...full, seed_urls: [siteOrigin + '/a', siteOrigin + '/b'], sitemap_urls: [], explicit_sitemap_inputs: [], input_imports: [provenance], concurrency: 65, request_timeout_secs: 301, max_body_bytes: 1572865, max_redirects: 3, max_sitemap_documents: 17, discover_sitemaps: false, crawl_links: false, obey_robots: false, check_external: false, delay_ms: 0, user_agent: 'OwnedConfigProfile/1', renderer: null, tls: {version: 1, respect_hsts: false, trusted_certificates: []}, pdf_analysis: {...full.pdf_analysis, enabled: false, max_pages: 19}, custom_headers: {[headerNames[0]]: secretValues[0], [headerNames[1]]: secretValues[1]}};
  const converted = (await json('/api/config-profiles/from-config', {config_text: JSON.stringify(nativeConfig)})).value;
  noSecrets(JSON.stringify(converted), 'from-config response'); assert.deepEqual(JSON.parse(converted.profile_json).required_headers, headerNames);
  const valid = converted.profile_json;
  const imported = await upload('owned-full-native.json', valid); sameConfig(imported.value.config_json, converted.config_json, 'All declared native fields survive picker import');
  await save(); await openSettings(); sameConfig(await page.inputValue('#config-profile-json'), valid, 'Save/reopen retains full declared native document');
  const exported = await profileDownload('full-native-before-crawl'); sameConfig(exported.toString('utf8'), valid, 'Full declared native profile export');
  assert.equal(await page.inputValue('[name=concurrency]'), '65'); assert.equal(await page.inputValue('[name=timeout_seconds]'), '301');
  const snapshot = await storage(); noSecrets(JSON.stringify(snapshot), 'all localStorage'); await pureUnchanged(pureBaseline, 'Default, parser, file import/save and export have no job/store/network effects');
  check('Actual picker import, native sanitizer, full profile export and dialog save preserve65/301/native-only fields, exact non-MiB bytes and disabled profiles without effects');
  // A brand-new origin-local draft is filled from the actual downloaded file.
  await page.locator('#settings-close').click(); await page.evaluate(key => localStorage.removeItem(key), storageKey); await page.reload(); await page.locator('#connection-state.connected').waitFor(); await openSettings();
  const fresh = await upload('actual-downloaded-profile.json', exported); sameConfig(fresh.value.config_json, converted.config_json, 'Fresh-draft full-option equality'); await save(); await page.reload(); await page.locator('#connection-state.connected').waitFor(); await openSettings();
  sameConfig(await page.inputValue('#config-profile-json'), valid, 'Origin-local sanitized draft after reload');
  await corruptedRestores(valid);
  sameConfig(await page.inputValue('#config-profile-json'), valid, 'Explicit corrupted-storage recovery retains the complete valid document');
  check('Three corrupt or unsupported stored wrappers persist visibly and block button/programmatic Start without effects; import alone cannot clear the guard, explicit valid Save recovers');
  await pendingRestore(valid);
  check('Opening Settings aborts a real pending startup restore without permitting legacy Start; explicit current-profile Save recovers and old native reply cannot poison active profile or storage');
  for (const [name, text, control] of await invalidCorpus(valid)) {
    const beforeDraft = await draft(), beforeStorage = await storage(), before = await effects();
    const result = await upload(name + '.json', text, 400); assert.equal(result.response.status(), 400);
    if (control) assert.match(result.value.error, /Unknown property in a unit configuration variant/, 'The genuine valid base must be rejected specifically by the tagged-unit guard');
    assert.deepEqual(await draft(), beforeDraft, name + ' preserves prior working draft'); assert.deepEqual(await storage(), beforeStorage, name + ' preserves stored draft'); await pureUnchanged(before, name + ' rejects before effects');
    negatives.push({name, source: 'actual_file_picker_native_parser', status: result.response.status(), input_sha256: hash(Buffer.from(text)), ...control});
  }
  await inspectEditor(valid); await page.locator('#config-profile-error').waitFor({state: 'hidden'});
  const beforeUTF8 = await draft(), storageUTF8 = await storage(), effectsUTF8 = await effects();
  const inspectRequestsBeforeUTF8 = requests.filter(r => r.source === 'browser' && r.path === '/api/config-profiles/inspect').length;
  const chooser = page.waitForEvent('filechooser'); await page.click('#config-profile-import'); await (await chooser).setFiles({name: 'invalid-utf8.json', mimeType: 'application/json', buffer: Buffer.from([0x7b, 0x22, 0xc3, 0x28, 0x22, 0x7d])});
  await page.locator('#config-profile-error').waitFor({state: 'visible'}); assert.deepEqual(await draft(), beforeUTF8); assert.deepEqual(await storage(), storageUTF8); await pureUnchanged(effectsUTF8, 'Invalid UTF8 file');
  assert.equal(requests.filter(r => r.source === 'browser' && r.path === '/api/config-profiles/inspect').length, inspectRequestsBeforeUTF8, 'Fatal UTF8 file decode must reject before native inspection');
  const invalidNativeStatus = await rejectedWire(Buffer.concat([Buffer.from('{"profile_text":"'), Buffer.from([0xc3, 0x28]), Buffer.from('"}')]), 'application/json', 400);
  await pureUnchanged(effectsUTF8, 'Invalid UTF8 native request');
  negatives.push({name: 'invalid-utf8-file-and-native-wire', source: 'actual_file_picker_and_invalid_native_bytes', native_status: invalidNativeStatus});
  const textPlainStatus = await rejectedWire(Buffer.from(JSON.stringify({profile_text: valid})), 'text/plain', 415);
  await pureUnchanged(effectsUTF8, 'Non-JSON native request rejects before effects');
  negatives.push({name: 'non-json-content-type', source: 'actual_native_request', native_status: textPlainStatus});
  check('Malformed, duplicate, unknown, positional, credential/path/private-key, regex/selector and UTF8 files leave draft/storage/jobs/files/network unchanged');
  const large = valid.replace(/("delay_ms"\s*:\s*)0(?=\s*[,}])/, (_match, prefix) => prefix + '9007199254740993'); assert.notEqual(large, valid, 'Controlled native u64 lexical replacement');
  await upload('exact-u64.json', large); await page.fill('[name=user_agent]', 'OwnedConfigProfile/u64-unrelated-edit'); await save(); await page.reload(); await page.locator('#connection-state.connected').waitFor(); await openSettings();
  const largeDownload = await profileDownload('exact-u64-after-edit-save-reload'); assert.match(largeDownload.toString('utf8'), /"delay_ms"\s*:\s*9007199254740993(?:\s*[,}])/); assert.equal(JSON.parse(largeDownload).config.user_agent, 'OwnedConfigProfile/u64-unrelated-edit');
  check('Untouched native u64 9007199254740993 survives unrelated control edit, Save, page reload and exact real profile attachment without JS rounding');
  const rendererConfig = {...JSON.parse(valid).config, request_timeout_secs: 120, tls: null, renderer: {...(full.renderer || {}), endpoint: siteOrigin + '/never-render', wait_ms: 7, accessibility: false, screenshot: false, collect_cookies: false, custom_javascript: [], max_response_bytes: 1572867}};
  const rendererProfile = documentText(rendererConfig); await upload('renderer-roundtrip-only.json', rendererProfile); await save(); await openSettings();
  const rendererDownload = await profileDownload('renderer-native-only-roundtrip'); assert.equal(JSON.parse(rendererDownload).config.renderer.max_response_bytes, 1572867); assert.equal(initialHits.length, 0);
  check('Renderer-only native response byte limit survives disabled flags and roundtrip; no worker/provider is contacted');
  const a = documentText({...JSON.parse(valid).config, user_agent: 'Owned-delayed-A'}), b = documentText({...JSON.parse(valid).config, user_agent: 'Owned-current-B-longer'});
  await staleFile(a, b); await staleNative(a, b); await pureUnchanged(pureBaseline, 'Delayed actual file/native bytes remain pure');
  check('Actual picker arrayBuffer and actual native-response delay controls reject superseded work without replacing current draft or unlocking stale state');
  await upload('static-crawl-profile.json', valid); await save();
  assert.equal(initialHits.length, 0); const unresolved = await effects();
  if (await page.isEnabled('#start-button')) { await page.click('#start-button'); await until(async () => (await page.locator('#message').innerText()).length > 0 || await page.locator('#settings-dialog').evaluate(el => el.open), 'missing header refusal'); }
  await pureUnchanged(unresolved, 'Missing volatile headers block browser start');
  await raw('/api/crawls/from-profile', {profile_text: valid}, 400); await pureUnchanged(unresolved, 'Missing exact native bindings fail before job/store/HTTP');
  await raw('/api/crawls/from-profile', {profile_text: valid, header_values: {[headerNames[0]]: secretValues[0]}}, 400);
  await pureUnchanged(unresolved, 'Partial exact-case bindings cannot create a job');
  await raw('/api/crawls/from-profile', {profile_text: valid, header_values: {[headerNames[0]]: secretValues[0], [headerNames[1]]: secretValues[1], 'X-Undeclared-Owned': 'owned-extra'}}, 400);
  await pureUnchanged(unresolved, 'Undeclared header bindings cannot create a job');
  negatives.push({name: 'missing-partial-and-undeclared-runtime-bindings', source: 'actual_native_start_requests_before_effects', statuses: [400, 400, 400]});
  await openSettings(); await resolveHeaders(); await save();
  noSecrets(JSON.stringify(await storage()), 'bound values remain volatile');
  const startMarker = await profileCapture.mark(page); const starting = waitJSON('/api/crawls/from-profile', 'POST', startMarker); await page.click('#start-button'); const started = await starting; assert.equal(started.response.status(), 200); jobId = started.value.id || started.value.crawl?.id; assert(jobId);
  const payload = started.response.request().postDataJSON(); assert.equal(payload.profile_text != null, true); assert.deepEqual(Object.keys(payload.header_values).sort(), headerNames.slice().sort());
  await until(async () => { const status = (await json('/api/crawls/' + jobId)).value; assert.notEqual(status.status, 'failed', status.error); return status.status === 'completed'; }, 'actual profile crawl');
  const project = await json('/api/crawls/' + jobId + '/project.json');
  const expectedConfig = JSON.parse(converted.effective_config_json); expectedConfig.custom_headers = nativeConfig.custom_headers;
  assert.deepEqual(project.value.config, expectedConfig, 'Every effective native option and genuine input occurrence is persisted');
  assert.deepEqual(project.value.pages.map(p => p.url).sort(), [siteOrigin + '/a', siteOrigin + '/b']); assert.equal(project.value.config.input_imports[0].rows.length, 3);
  assert.deepEqual(initialHits.map(x => x.path).sort(), ['/a', '/b']); assert(initialHits.every(x => x.explicit_header_bound));
  check('Explicit complete case-sensitive volatile header bindings start one genuine multi-seed crawl; every native option/provenance persists and traversal/discovery remain false');
  // A real reload clears volatile bindings. The natural user path then edits
  // the run target, sees a missing-header refusal and saves reentered headers.
  // That Save must preserve the already edited target: filling it again here
  // would hide the actual toolbar-reset regression.
  await page.reload(); await page.locator('#connection-state.connected').waitFor();
  await until(() => page.locator('#config-profile-active').isVisible(), 'active full profile restored');
  await selectJob(jobId, 2); await openSettings();
  assert.deepEqual((await requirements()).map(input => input.name).sort(), headerNames.slice().sort());
  assert((await requirements()).every(input => input.value === ''), 'A true page reload clears volatile header bindings');
  await page.click('#settings-close');
  const replacementURL = siteOrigin + '/replacement'; await page.fill('#seed-url', replacementURL);
  // Native fresh preparation synthesizes TXT provenance for explicit seed URLs
  // when run-input imports were cleared. Obtain that exact independent parser
  // result (without a trailing newline) before starting the replacement job.
  const replacementImport = (await json('/api/lists/import', {content_base64: Buffer.from(replacementURL).toString('base64'), options: {format: {kind: 'txt'}}})).value;
  assert.equal(replacementImport.rows.length, 1); assert.equal(replacementImport.rows[0].normalized_url, replacementURL);
  const beforeReplacementRefusal = await effects();
  assert(await page.isEnabled('#start-button')); await page.click('#start-button');
  await page.waitForFunction(() => /Header/i.test(document.getElementById('message').textContent));
  await pureUnchanged(beforeReplacementRefusal, 'Explicit replacement with unresolved headers cannot create a job or fetch any source');
  await openSettings(); await resolveHeaders(); await save();
  assert.equal(await page.inputValue('#seed-url'), replacementURL, 'Header reentry plus Settings Save must preserve the user-edited run target without a second fill');
  noSecrets(JSON.stringify(await storage()), 'Reentered replacement-run bindings stay volatile');
  const replacementMarker = await profileCapture.mark(page); const replacementPending = waitJSON('/api/crawls/from-profile', 'POST', replacementMarker); await page.click('#start-button');
  const replacementStart = await replacementPending; assert.equal(replacementStart.response.status(), 200);
  const replacementId = replacementStart.value.id || replacementStart.value.crawl?.id; assert(replacementId); assert.notEqual(replacementId, jobId);
  const replacementPayload = replacementStart.response.request().postDataJSON();
  assert.deepEqual(replacementPayload.patches.map(patch => patch.path), [['seed_urls'], ['sitemap_urls'], ['input_imports'], ['explicit_sitemap_inputs']]);
  assert.deepEqual(JSON.parse(replacementPayload.patches[0].value_json), [replacementURL]);
  assert.deepEqual(JSON.parse(replacementPayload.patches[1].value_json), []); assert.deepEqual(JSON.parse(replacementPayload.patches[2].value_json), []);
  assert.equal(replacementPayload.patches[3].remove, true); assert.deepEqual(Object.keys(replacementPayload.header_values).sort(), headerNames.slice().sort());
  await until(async () => { const status = (await json('/api/crawls/' + replacementId)).value; assert.notEqual(status.status, 'failed', status.error); return status.status === 'completed'; }, 'actual explicit replacement crawl');
  const replacementProject = await json('/api/crawls/' + replacementId + '/project.json');
  assert.deepEqual(replacementProject.value.pages.map(record => record.url), [replacementURL]);
  assert.deepEqual(replacementProject.value.config, {...expectedConfig, seed_urls: [replacementURL], sitemap_urls: [], input_imports: [replacementImport], explicit_sitemap_inputs: []}, 'Only explicit run inputs and native synthesized seed provenance change; all other native options remain');
  assert.deepEqual(replacementProject.value.config.explicit_sitemap_inputs, [], 'Fresh native preparation captures the empty explicit sitemap declarations before input merge');
  assert.equal(replacementProject.value.config.crawl_links, false);
  assert.deepEqual((await json('/api/crawls/' + jobId + '/project.json')).bytes, project.bytes, 'The original multi-seed job remains immutable');
  assert.deepEqual(initialHits.map(x => x.path).sort(), ['/a', '/b', '/replacement']); assert(initialHits.every(x => x.explicit_header_bound));
  check('Natural replacement target → missing-header refusal → reentry and Save retains the target, creates a new one-page job with only run-input patches, preserves native options and original job bytes');
  offline = true; await witness();
  await selectJob(jobId, 2);
  const sanitized = (await json('/api/crawls/' + jobId + '/config-profile')).value; noSecrets(JSON.stringify(sanitized), 'selected saved profile');
  await openSettings(); const loadingMarker = await profileCapture.mark(page); const loading = waitJSON('/api/crawls/' + jobId + '/config-profile', 'GET', loadingMarker); await page.click('#config-profile-load-selected'); const loaded = await loading; assert.equal(loaded.response.status(), 200); await installed(loaded.value);
  const selectedDownload = await profileDownload('selected-saved-sanitized-profile'); sameConfig(selectedDownload.toString('utf8'), sanitized.profile_json, 'Selected profile export has original declared options and only header requirements');
  await save(); const storedBeforeCold = await storage();
  if (await page.locator('#settings-dialog').evaluate(el => el.open)) await page.click('#settings-close');
  await page.goto('about:blank'); const originalOrigin = app.origin;
  await stop(first); assert.equal(offlineHits.length, 0); const second = await launch('cold', new URL(originalOrigin).host); assert.equal(second.origin, originalOrigin); assert.notEqual(second.child.pid, first.child.pid);
  await page.goto(app.origin); await page.locator('#connection-state.connected').waitFor(); await until(async () => await page.locator('#crawl-history option[value="' + jobId + '"]').count(), 'cold native history'); await selectJob(jobId, 2);
  const cold = await json('/api/crawls/' + jobId + '/project.json'); assert.deepEqual(cold.bytes, project.bytes, 'Saved full config/report survives true process death');
  assert.deepEqual((await json('/api/crawls/' + replacementId + '/project.json')).bytes, replacementProject.bytes, 'The separate explicit replacement job also survives cold reopen byte-exactly');
  await openSettings(); assert.deepEqual(await storage(), storedBeforeCold, 'Browser storage belongs to retained context on the same origin');
  const restoredHeaders = await requirements();
  assert.deepEqual(restoredHeaders.map(input => input.name).sort(), headerNames.slice().sort());
  assert(restoredHeaders.every(input => input.value === ''), 'Cold browser restore must not recover header values before selected-load can clear them');
  const coldLoadMarker = await profileCapture.mark(page); const coldLoad = waitJSON('/api/crawls/' + jobId + '/config-profile', 'GET', coldLoadMarker); await page.click('#config-profile-load-selected'); const reloaded = await coldLoad; await installed(reloaded.value);
  assert((await requirements()).every(input => input.value === ''), 'Secret bindings are not restored from storage/server');
  const coldExport = await profileDownload('cold-selected-sanitized-profile'); assert.deepEqual(coldExport, selectedDownload); noSecrets(JSON.stringify(await storage()), 'cold profile storage');
  await page.setViewportSize({width: 320, height: 900});
  const mobileControls = [];
  for (const selector of ['#config-profile-import', '#config-profile-inspect', '#config-profile-export']) {
    await page.locator(selector).scrollIntoViewIfNeeded(); const box = await page.locator(selector).boundingBox(); assert(box); assert(box.x >= 0 && box.x + box.width <= 320 && box.y >= 0 && box.y + box.height <= 900, 'Reachable320px control ' + selector); mobileControls.push({selector, ...box});
  }
  const mobileExport = await profileDownload('cold-selected-320'); assert.deepEqual(mobileExport, coldExport);
  const geometry = await page.evaluate(() => ({viewport: innerWidth, document: document.documentElement.scrollWidth, dialog: document.getElementById('settings-dialog').getBoundingClientRect().width})); assert.equal(geometry.viewport, 320); assert(geometry.document <= 320, 'No document-level horizontal overflow at320px'); geometry.controls = mobileControls;
  await page.screenshot({path: artifactPath('config-profiles-cold-320.png'), fullPage: true}); await page.goto('about:blank'); await stop(second); await witness();
  assert.deepEqual(offlineHits, []); assert.equal(witnessHits.length, 2); assert.deepEqual(errors, []); assert.deepEqual(external, []); verifyPins(); assert.equal(hash(fs.readFileSync(executablePath)), chromeHash);
  assert.equal(owners.length, 2); assert(owners.every(o => o.natural_stdin_eof && !o.forced && o.close.code === 0 && o.close.signal === null));
  check('Same owned origin/store cold native process returns byte-identical report and sanitized profile; browser-local options persist, secret values do not,320px works and both EOF0 closes retain410 witness');
  return {chrome: executablePath, chrome_sha256: chromeHash, job_id: jobId, replacement_job_id: replacementId, replacement_project_sha256: hash(replacementProject.bytes), replacement_project_bytes: replacementProject.bytes.length, explicit_replacement: {natural_header_reentry_save_preserved_target: true, target_fills: 1, run_input_patches: replacementPayload.patches, original_job_unchanged: true, both_jobs_cold_bytes_equal: true}, effective_config_sha256: hash(Buffer.from(converted.effective_config_json)), saved_project_sha256: hash(project.bytes), saved_project_bytes: project.bytes.length, cold_profile_sha256: hash(coldExport), served_assets: servedAssets, browser_storage_scope: 'same-origin retained browser context; sanitized profile only', native_scope: 'two normal native processes, one owned fresh store; no provider or renderer calls', geometry};
}
(async () => {
  const started = Date.now(); let proof;
  try { proof = await run(); } catch (error) { failure = error; }
  finally {
    if (browser) { try { await deadline(browser.close(), 30000, 'owned browser close'); } catch (error) { failure ||= error; } }
    for (const owner of owners) if (!owner.close) { owner.forced = true; owner.child.kill(); try { await deadline(owner.closed, 10000, 'failed owned process close'); } catch (error) { owner.child.stdin.destroy(); owner.child.stdout.destroy(); owner.child.stderr.destroy(); owner.child.unref(); failure ||= error; } }
    if (site?.listening) { site.closeAllConnections(); try { await deadline(new Promise(resolve => site.close(resolve)), 10000, 'owned website close'); } catch (error) { failure ||= error; } }
    for (const owner of owners) fs.writeFileSync(artifactPath('native-' + owner.phase + '.log'), owner.log);
    const receipt = {result: failure ? 'FAIL' : 'PASS', duration_ms: Date.now() - started, binary, binary_sha256: binaryHash, output, driver: __filename, driver_sha256: pins[0].sha256, response_capture_helper_sha256: pins[2].sha256, ...proof, checks, negatives, downloads, requests, inspected_responses: inspectedResponses, held_responses: heldResponses, native: owners.map(o => ({phase: o.phase, pid: o.child.pid, origin: o.origin, args: o.args, close: o.close, forced: o.forced, natural_stdin_eof: o.natural_stdin_eof})), website_capture_hits: initialHits, offline_product_hits: offlineHits, offline_positive_controls: witnessHits.length, browser_errors: errors, external_requests: external, ...(failure ? {error: failure.stack} : {})};
    fs.writeFileSync(artifactPath('ui-config-profiles-live.json'), JSON.stringify(receipt, null, 2) + '\n');
  }
  if (failure) throw failure;
  console.log(checks.length + ' actual config-profile groups passed.');
})().catch(error => { console.error(error); process.exitCode = 1; });
