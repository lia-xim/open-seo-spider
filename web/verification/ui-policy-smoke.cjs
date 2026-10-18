const artifactPath = require('./artifact-path.cjs');
// Isolated browser/API contract fixture. This does not execute the native policy engine.
const http = require('node:http'), fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const { pathToFileURL } = require('node:url'), web = path.resolve(__dirname, '..');
let playwright;
for (const candidate of [process.env.SPIDER_PLAYWRIGHT_MODULE, path.resolve(web, '../renderer/node_modules/playwright-core'), 'playwright-core'].filter(Boolean)) {
  try { playwright = require(candidate); break; } catch (_) {}
}
if (!playwright) throw Error('Run npm ci in renderer or set SPIDER_PLAYWRIGHT_MODULE.');

const ENGINE = 'open-seo-spider/crawl-policy-v1', NATIVE_ENGINE = 'open-seo-spider/native-crawl-policy-v1', HASH = 'a'.repeat(64);
const typedPolicy = {
  engine: ENGINE,
  scope: { roots: [{ origin: 'https://a.fixture', subdomains: false, folders: ['/shop/'], exact_paths: ['/shop'] }], outside_folder: 'check_once', external: 'deny' },
  query: { remove_keys: ['utm_source', 'SessionID'], key_match: 'encoded_bytes', order: 'stable_encoded_key' },
  rewrites: [
    { id: 'first', pattern: '/old/', replacement: { kind: 'literal', text: '/new/$1/' } },
    { id: 'second', pattern: '(?<part>item)', replacement: { kind: 'captures', template: '${part}$$' } }
  ],
  include_patterns: ['^https://a\\.fixture/'], exclude_patterns: ['/private/'], filter_identity: 'both',
  initial_inputs: { rewrite: 'apply', filters: 'apply' }, limits: {}
};
const liveLimits = { max_row_bytes: 262144, max_batch_bytes: 33554432, max_batch_occurrences: 100000, max_ledger_bytes: null, max_page_bytes: 1048576 };
const formulaRaw = '=HYPERLINK("https://inert.invalid","UNTRUSTED_FORMULA")';
const markupRaw = '<img src=x onerror="window.__policyFixtureInjected=1">';
const frozenReason = 'Fixture: imported crawl declares a complete checkpoint, but its frozen policy ledger is unavailable.';
const pageRecord = url => ({ url, final_url: url, internal: true, status: 503, final_status: 503, content_type: 'text/html', titles: ['STORED HTTP PAGE'], h1: [], h2: [], links: [] });
const jobs = {
  a: { id: 'a', status: 'completed', config: { seed_urls: ['https://a.fixture/'] }, stats: { fetched: 1, queued: 0, issues: 0 }, pages: [pageRecord('https://a.fixture/fetch-1')] },
  b: { id: 'b', status: 'completed', config: { seed_urls: ['https://b.fixture/'] }, stats: { fetched: 1, queued: 0, issues: 0 }, pages: [pageRecord('https://b.fixture/fetch-1')] },
  action: { id: 'action', status: 'running', resume_available: true, config: { seed_urls: ['https://action.fixture/'] }, stats: { fetched: 1, queued: 1, issues: 0 }, pages: [pageRecord('https://action.fixture/fetch-1')] },
  legacy: { id: 'legacy', status: 'completed', config: {}, stats: { fetched: 0, queued: 0, issues: 0 }, pages: [] },
  unknown: { id: 'unknown', status: 'completed', config: {}, stats: { fetched: 0, queued: 0, issues: 0 }, pages: [] },
  frozen: { id: 'frozen', status: 'completed', config: {}, stats: { fetched: 0, queued: 0, issues: 0 }, pages: [] },
  broken: { id: 'broken', status: 'completed', config: {}, stats: { fetched: 0, queued: 0, issues: 0 }, pages: [] }
};

function candidate(input, overrides = {}) {
  const raw = input.raw_value;
  let normalized = null;
  try { const url = new URL(raw, input.base_url || 'https://a.fixture/'); url.hash = ''; normalized = url.href; } catch (_) {}
  return {
    occurrence_id: input.occurrence_id, source_kind: input.source_kind || 'anchor', source_url: input.source_url || null,
    source_location: input.source_location || null, raw_value: raw, resolved_discovered_url: normalized,
    normalized_discovered_url: normalized, fetch_candidate_url: normalized, fragment: null, policy_steps: [],
    decision: 'fetch_and_traverse', reason_code: null, error: null, matched_rule_ids: [],
    host_internal: true, folder_internal: true, traverse: true, coalesced_into: null,
    policy_engine: ENGINE, policy_sha256: HASH, warnings: [], policy_complete: true, ...overrides
  };
}
function storedRow(sequence, id = 'a') {
  const inherited = sequence === 2 || sequence === 4;
  const observed = `https://${id}.fixture/observed-${sequence}`;
  const changed = sequence === 1 || sequence === 4;
  const fetch = changed ? `https://${id}.fixture/fetch-${sequence}` : observed;
  const raw = sequence === 2 ? formulaRaw : sequence === 3 ? markupRaw : observed;
  const outcome = sequence === 2 || sequence === 3 ? 'deny' : sequence === 1 ? 'check_only' : 'fetch_and_traverse';
  const decision = candidate({ occurrence_id: `${id}-occurrence-${sequence}`, raw_value: raw, source_kind: sequence === 4 ? 'relationship' : 'anchor', source_url: `https://${id}.fixture/source` }, {
    normalized_discovered_url: outcome === 'deny' ? null : observed,
    resolved_discovered_url: outcome === 'deny' ? null : observed,
    fetch_candidate_url: outcome === 'deny' ? null : fetch, decision: outcome,
    reason_code: outcome === 'deny' ? 'unsafe_url' : null, traverse: outcome === 'fetch_and_traverse',
    policy_steps: changed ? [{ kind: 'rewrite', rule_id: 'first', matched: true, changed: true, before: observed, after: fetch, error: null }] : []
  });
  return {
    sequence, generation: 1, basis: inherited ? 'inherited' : 'current',
    occurrence: {
      source_revision: `${id}-source-revision-${sequence}`, source_ordinal: sequence, input_basis: 'observed_attribute',
      raw_value_observed: true, decision, task_role: sequence === 4 ? 'observation_only' : 'page_document',
      scheduled_fetch_url: outcome === 'deny' || sequence === 4 ? null : fetch,
      coalesced_task_occurrence_id: null,
      admission_reason: outcome === 'deny' ? 'policy_denied' : sequence === 4 ? 'observation_only' : 'scheduled'
    }
  };
}
function actionRows(generation, tag = '') {
  return Array.from({ length: 205 }, (_, i) => {
    const row = storedRow(i + 1, 'action'); row.generation = generation;
    row.occurrence.decision.occurrence_id = `action-${tag ? tag + '-' : ''}occurrence-${i + 1}`;
    row.occurrence.source_revision = `action-source-revision-${generation}`; return row;
  });
}
const ledgers = { a: Array.from({ length: 405 }, (_, i) => storedRow(i + 1)), b: [storedRow(1, 'b')], action: actionRows(1) };
function ledger(id, after = 0, limit = 200) {
  if (id === 'legacy' || id.startsWith('started-')) return { enabled: false, engine: null, policy_sha256: null, checkpoint: null, observations: [], next_after_sequence: after, has_more: false, scope: 'Legacy: keine Policy-Aufzeichnungen.' };
  if (id === 'frozen') return {
    enabled: true, engine: NATIVE_ENGINE, policy_sha256: HASH, ledger_available: false, reason: frozenReason,
    checkpoint: { engine: NATIVE_ENGINE, policy_sha256: HASH, revision: 7, occurrences: 405, committed_bytes: 123456, rolling_sha256: 'd'.repeat(64), incomplete_sources: 0, complete: true },
    observations: [], next_after_sequence: after, has_more: false,
    scope: 'Imported checkpoint metadata; recorded completeness does not establish an available policy ledger.'
  };
  const all = ledgers[id] || [storedRow(1, id)], observations = all.filter(row => row.sequence > after).slice(0, limit);
  return {
    enabled: true, engine: id === 'unknown' ? 'unknown-future-policy-engine' : NATIVE_ENGINE, policy_sha256: HASH,
    checkpoint: { engine: NATIVE_ENGINE, policy_sha256: HASH, revision: 7, occurrences: all.length, committed_bytes: 123456, rolling_sha256: 'b'.repeat(64), incomplete_sources: 2, complete: false, inherited_history: { parent_ledger_sha256: 'c'.repeat(64), expected_occurrences: 5, available_occurrences: 2, complete: false } },
    observations, next_after_sequence: observations.at(-1)?.sequence || after,
    has_more: all.some(row => row.sequence > (observations.at(-1)?.sequence || after)),
    scope: 'Erfasste native Quellen; aktuelle und geerbte Beobachtungen. Keine vollständige Website-Abdeckung.'
  };
}

const previewPosts = [], crawlPosts = [], controlPosts = [], ledgerGets = [], exportGets = [], exportDownloadPaths = [], held = [], checks = [], errors = [], unexpectedNetwork = [];
let exportClientTransports = [];
let nextStart = 0, holdPreview = false, holdLedgerId = null;
const copy = value => JSON.parse(JSON.stringify(value));
const send = (res, value, status = 200) => {
  if (res.destroyed || res.writableEnded) return;
  if (!res.headersSent) { res.statusCode = status; res.setHeader('content-type', 'application/json'); }
  res.end(JSON.stringify(value));
};
const release = (kind, status = 200) => {
  const index = held.findIndex(entry => entry.kind === kind); assert.ok(index >= 0, `${kind} request held`);
  const [entry] = held.splice(index, 1); send(entry.res, status === 200 ? entry.value : { error: 'PRIVATE OLD POLICY RESPONSE' }, status);
};
const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://fixture');
    if (url.pathname === '/api/policy/preview' && req.method === 'POST') {
      let raw = ''; for await (const part of req) raw += part;
      const input = JSON.parse(raw); previewPosts.push(copy(input));
      const value = { engine: ENGINE, replacement_profile: 'rust-first-match-literal-or-braced-captures-v1', policy_sha256: HASH, config: input.config, decisions: input.inputs.map(row => candidate(row)), website_requests: 0, limitations: ['Isolated fixture result, not native policy execution.'] };
      if (holdPreview) { holdPreview = false; held.push({ kind: 'preview', res, value }); return; }
      return send(res, value);
    }
    if (url.pathname === '/api/crawls' && req.method === 'POST') {
      let raw = ''; for await (const part of req) raw += part;
      const config = JSON.parse(raw); crawlPosts.push(config); const id = `started-${++nextStart}`;
      jobs[id] = { id, status: 'completed', config, stats: { fetched: 0, queued: 0, issues: 0 }, pages: [] };
      return send(res, { id, status: 'completed' });
    }
    if (url.pathname === '/api/crawls') return send(res, { crawls: Object.values(jobs).map(({ pages, ...job }) => copy(job)) });
    const match = url.pathname.match(/^\/api\/crawls\/([^/]+)(?:\/(.+))?$/);
    if (match) {
      const [, id, suffix] = match, job = jobs[id]; if (!job) return send(res, { error: 'unknown fixture project' }, 404);
      if (!suffix) { const { pages, ...status } = job; return send(res, status); }
      if (id === 'action' && ['pause', 'resume'].includes(suffix) && req.method === 'POST') {
        controlPosts.push({ id, action: suffix }); job.status = suffix === 'pause' ? 'paused' : 'running';
        ledgers.action = actionRows(suffix === 'pause' ? 2 : 3, `after-${suffix}`);
        return send(res, { id, status: job.status, resume_available: true });
      }
      if (suffix === 'policy') {
        const after = Number(url.searchParams.get('after_sequence') || 0), limit = Number(url.searchParams.get('limit') || 200);
        ledgerGets.push({ id, after, limit }); assert.equal(limit, 200, 'UI ledger response-page size stays bounded');
        if (id === 'broken') return send(res, { error: 'Fixture policy ledger read failed' }, 500);
        const value = ledger(id, after, limit);
        if (holdLedgerId === id) {
          holdLedgerId = null;
          // Chromium can serialize identical GETs while waiting for cache headers. Keep both bodies pending instead.
          if (id === 'action') { res.setHeader('cache-control', 'no-store'); res.setHeader('content-type', 'application/json'); res.flushHeaders(); }
          held.push({ kind: `ledger-${id}`, res, value }); return;
        }
        return send(res, value);
      }
      if (suffix === 'policy.csv' || suffix === 'policy.json') {
        exportGets.push({ id, suffix });
        if (suffix.endsWith('.json')) return send(res, { fixture_native_export: id, observations: ledgers[id] || [] });
        res.setHeader('content-type', 'text/csv'); return res.end('Scope,Fixture native export\nall_saved_rows,' + id + '\n');
      }
      if (suffix === 'pages') { const offset = Number(url.searchParams.get('offset') || 0), limit = Number(url.searchParams.get('limit') || 1000); return send(res, { pages: job.pages.slice(offset, offset + limit), total: job.pages.length }); }
      if (suffix === 'issues') return send(res, { issues: [] });
      if (suffix === 'analysis') return send(res, { issues: [], graph: { nodes: [], edges: [] } });
      if (suffix === 'responses') return send(res, { responses: [] });
      if (suffix === 'fragments') return send(res, { enabled: false, observations: [] });
    }
    const file = url.pathname === '/' ? 'index.html' : url.pathname.slice(1);
    if (!['index.html', 'app.js','dashboard-segments.js','config-profiles.js','screenshots.js', 'list-import.js', 'styles.css', 'saved-comparison.js'].includes(file)) { res.statusCode = 404; return res.end(); }
    res.setHeader('content-type', file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : 'text/html');
    res.end(fs.readFileSync(path.join(web, file)));
  } catch (error) { errors.push('Fixture server: ' + error.message); send(res, { error: error.message }, 500); }
});
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(fn, label) { for (let i = 0; i < 240; i++) { if (await fn()) return; await delay(25); } throw Error('Timeout ' + label); }
const check = name => { checks.push(name); console.log('PASS ' + name); };
function csvRecords(text) {
  const records = []; let row = [], value = '', quoted = false;
  for (let i = 0; i < text.length; i++) { const c = text[i]; if (c === '"') { if (quoted && text[i + 1] === '"') { value += '"'; i++; } else quoted = !quoted; } else if (!quoted && (c === ',' || c === '\n')) { row.push(value); value = ''; if (c === '\n') { records.push(row); row = []; } } else if (quoted || c !== '\r' && c !== '\uFEFF') value += c; }
  row.push(value); if (row.some(item => item !== '')) records.push(row); return records;
}
async function openSettings(page) {
  await page.click('#settings-open'); await page.locator('#settings-dialog[open]').waitFor();
  await page.locator('#advanced-config').evaluate(element => { element.open = true; });
  await page.locator('#policy-settings').evaluate(element => { element.open = true; });
}
async function configure(page, enabled = true) {
  await page.locator('[name=crawl_policy_enabled]').setChecked(enabled);
  await page.fill('[name=crawl_policy_json]', JSON.stringify(typedPolicy, null, 2));
  await page.fill('[name=live_policy_limits_json]', JSON.stringify(liveLimits));
}
async function settingsSubmit(page) { await page.locator('#settings-form').evaluate(form => form.dispatchEvent(new Event('submit', { cancelable: true }))); }
async function probe(page, raw) {
  await page.fill('#policy-preview-raw', raw); await page.fill('#policy-preview-base', 'https://a.fixture/shop/index');
  await page.selectOption('#policy-preview-kind', 'anchor'); await page.click('#policy-preview-button');
}
async function previewContains(page, text) { await page.waitForFunction(value => document.getElementById('policy-preview-table').textContent.includes(value), text); }
async function selectProject(page, id) {
  await page.locator('#crawl-history').evaluate((element, value) => { element.value = value; element.dispatchEvent(new Event('change', { bubbles: true })); }, id);
  await page.waitForFunction(value => document.getElementById('crawl-history').value === value, id);
}
async function ledgerContains(page, text) { await page.waitForFunction(value => document.getElementById('policy-table').textContent.includes(value), text); }
async function download(page, id) {
  const promise = page.waitForEvent('download'); await page.click(id); const item = await promise;
  if (id.startsWith('#policy-export-')) exportDownloadPaths.push(new URL(item.url()).pathname);
  return fs.readFileSync(await item.path(), 'utf8');
}
function report(result, failure) {
  fs.writeFileSync(artifactPath('ui-policy-fixture-results.json'), JSON.stringify({ tested_at: new Date().toISOString(), result, scope: 'isolated browser/API fixture; not native policy execution, persistence, HTTP or export proof', checks, preview_requests: previewPosts.length, crawl_requests: crawlPosts.length, control_requests: controlPosts, ledger_requests: ledgerGets, native_export_requests: exportGets, native_export_download_paths: exportDownloadPaths, native_export_client_transports: exportClientTransports, javascript_runtime_errors: errors, unexpected_network_requests: unexpectedNetwork, ...(failure ? { failure: failure.message } : {}) }, null, 2) + '\n');
}

(async () => {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve)); let executable;
  try { executable = process.env.SPIDER_CHROME || await (await import(pathToFileURL(path.resolve(web, '../renderer/server.mjs')).href)).findBrowserExecutable(); } catch (_) {}
  const browser = await playwright.chromium.launch({ ...(executable ? { executablePath: executable } : {}), headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
  const origin = `http://127.0.0.1:${server.address().port}`;
  // Keep these fixture requests alive after abort so view/token guards must reject genuinely late responses.
  await page.addInitScript(() => {
    const originalFetch = window.fetch;
    window.__policyFixtureExportTransports = [];
    const nativeExport = url => /\/api\/crawls\/[^/]+\/policy\.(csv|json)(?:\?|$)/.test(String(url));
    window.fetch = (input, options) => {
      const url = String(input);
      if (nativeExport(url)) window.__policyFixtureExportTransports.push('fetch');
      return originalFetch(input, /\/api\/policy\/preview(?:\?|$)|\/api\/crawls\/[^/]+\/policy(?:\?|$)/.test(url) && options?.signal ? { ...options, signal: undefined } : options);
    };
    const originalOpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function(method, url, ...rest) {
      if (nativeExport(url)) window.__policyFixtureExportTransports.push('xhr');
      return originalOpen.call(this, method, url, ...rest);
    };
  });
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    if (!request.url().startsWith(origin + '/') && !request.url().startsWith('blob:') && !request.url().startsWith('data:')) unexpectedNetwork.push(request.url());
  });
  try {
    await page.goto(origin); await page.locator('#connection-state.connected').waitFor(); await selectProject(page, 'a');
    await openSettings(page); await configure(page); await probe(page, '/shop/OLD_PREVIEW?utm_source=x#part');
    await previewContains(page, 'OLD_PREVIEW'); assert.equal(crawlPosts.length, 0); assert.deepEqual(previewPosts[0].config, typedPolicy);
    assert.equal(previewPosts[0].inputs[0].source_kind, 'anchor'); assert.equal(previewPosts[0].inputs[0].raw_value, '/shop/OLD_PREVIEW?utm_source=x#part');
    assert.equal(previewPosts[0].inputs[0].base_url, 'https://a.fixture/shop/index'); check('offline preview sends typed ordered config and original probe role without any crawl POST or external request');

    await page.fill('#policy-preview-raw', '/shop/EDITED_PROBE'); assert.equal(await page.isVisible('#policy-preview-results'), false);
    holdPreview = true; await page.click('#policy-preview-button'); await until(() => held.some(entry => entry.kind === 'preview'), 'held edited preview');
    await page.fill('[name=crawl_policy_json]', JSON.stringify({ ...typedPolicy, filter_identity: 'fetch' }));
    holdPreview = true; await probe(page, '/shop/NEWEST_PROBE'); await until(() => held.filter(entry => entry.kind === 'preview').length === 2, 'held newest preview');
    release('preview'); await delay(150); assert.equal(await page.isVisible('#policy-preview-results'), false); assert.equal(await page.isDisabled('#policy-preview-button'), true);
    release('preview'); await previewContains(page, 'NEWEST_PROBE'); assert.equal(await page.isDisabled('#policy-preview-button'), false);
    check('probe and configuration edits invalidate delayed results and an old finally cannot free the newer pending preview');

    await configure(page); await probe(page, '/shop/CLOSE_REOPEN_PREVIEW'); await previewContains(page, 'CLOSE_REOPEN_PREVIEW');
    await page.click('#settings-close'); await openSettings(page); assert.equal(await page.isVisible('#policy-preview-results'), false);
    holdPreview = true; await probe(page, '/shop/OLD_PROJECT_PREVIEW'); await until(() => held.some(entry => entry.kind === 'preview'), 'held project preview');
    await selectProject(page, 'b'); release('preview'); await delay(150); assert.equal(await page.isVisible('#policy-preview-results'), false);
    check('settings close and reopen plus project switch revoke previous preview result identity');

    await configure(page); await page.locator('[name=javascript_rendering]').evaluate(input => { input.checked = true; input.dispatchEvent(new Event('change', { bubbles: true })); });
    await settingsSubmit(page); assert.equal(await page.isVisible('#settings-error'), true); assert.equal(crawlPosts.length, 0);
    await page.locator('[name=javascript_rendering]').evaluate(input => { input.checked = false; input.dispatchEvent(new Event('change', { bubbles: true })); });
    await page.locator('#advanced-config').evaluate(element => { element.open = true; }); await page.fill('[name=include_patterns_text]', '/legacy/');
    await settingsSubmit(page); assert.equal(await page.isVisible('#settings-error'), true); assert.equal(crawlPosts.length, 0);
    await page.fill('[name=include_patterns_text]', ''); await page.fill('[name=crawl_policy_json]', '{'); await settingsSubmit(page);
    assert.equal(await page.isVisible('#settings-error'), true); assert.equal(crawlPosts.length, 0); check('configured policy rejects renderer legacy filters and malformed JSON before creating a crawl');

    await configure(page, false); await settingsSubmit(page); await page.locator('#settings-dialog').waitFor({ state: 'hidden' });
    await selectProject(page, ''); await page.fill('#seed-url', 'https://disabled-policy.fixture/'); await page.click('#start-button'); await until(() => crawlPosts.length === 1, 'disabled policy crawl payload');
    assert.ok(!Object.hasOwn(crawlPosts[0], 'crawl_policy')); assert.ok(!Object.hasOwn(crawlPosts[0], 'live_policy_limits'));
    await openSettings(page); await configure(page); await settingsSubmit(page); await page.locator('#settings-dialog').waitFor({ state: 'hidden' });
    await selectProject(page, ''); await page.fill('#seed-url', 'https://configured-policy.fixture/'); await page.click('#start-button'); await until(() => crawlPosts.length === 2, 'configured policy crawl payload');
    assert.deepEqual(crawlPosts[1].crawl_policy, typedPolicy); assert.deepEqual(crawlPosts[1].live_policy_limits, liveLimits);
    await openSettings(page); assert.deepEqual(JSON.parse(await page.locator('[name=crawl_policy_json]').inputValue()), typedPolicy); await page.click('#settings-close');
    check('disabled fields are omitted while configured crawl and saved settings preserve ordered literal and capture rules separately');

    await selectProject(page, 'a'); const beforePolicy = ledgerGets.length; await page.click('[data-section=policy]'); await ledgerContains(page, 'a-occurrence-1');
    assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 200); assert.equal(ledgerGets.length, beforePolicy + 1);
    assert.deepEqual(ledgerGets.at(-1), { id: 'a', after: 0, limit: 200 }); assert.match(await page.locator('#policy-scope').innerText(), /keine|nicht|unvollständig/i);
    await delay(120); assert.equal(ledgerGets.length, beforePolicy + 1); await page.click('#policy-next'); await ledgerContains(page, 'a-occurrence-201');
    assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 200); assert.deepEqual(ledgerGets.at(-1), { id: 'a', after: 200, limit: 200 });
    await page.click('#policy-next'); await ledgerContains(page, 'a-occurrence-401'); assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 5); assert.equal(await page.isDisabled('#policy-next'), true);
    await page.click('#policy-prev'); await ledgerContains(page, 'a-occurrence-201'); await page.click('#policy-prev'); await ledgerContains(page, 'a-occurrence-1');
    assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 200); check('405 saved occurrences use explicit bounded response pages and backward navigation without loading all rows');

    const beforeSort = ledgerGets.length; await page.click('[data-policy-sort="0"]');
    assert.equal((await page.locator('#policy-table [data-policy-row]').first().locator('td').first().innerText()).trim(), '200');
    assert.equal(ledgerGets.length, beforeSort); await page.click('[data-policy-sort="0"]');
    assert.equal((await page.locator('#policy-table [data-policy-row]').first().locator('td').first().innerText()).trim(), '1');
    await page.selectOption('#policy-filter', 'inherited'); assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 2); await page.selectOption('#policy-filter', 'all');
    assert.match(await page.locator('#policy-scope').innerText(), /2 von 5/); assert.match(await page.locator('#policy-scope').innerText(), /Unvollständige Quellen: 2/);
    check('sorting and inherited filtering stay within the loaded page while partial inherited coverage remains explicit');

    await page.selectOption('#policy-filter', 'deny'); assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 2);
    await page.selectOption('#policy-filter', 'all'); await page.fill('#policy-search', 'UNTRUSTED_FORMULA'); await page.waitForFunction(() => document.querySelectorAll('#policy-table [data-policy-row]').length === 1);
    const visibleCsv = csvRecords(await download(page, '#export-csv')); assert.equal(visibleCsv.length, 2); assert.ok(visibleCsv[1].includes("'" + formulaRaw));
    await page.fill('#policy-search', ''); await page.waitForFunction(() => document.querySelectorAll('#policy-table [data-policy-row]').length === 200);
    assert.equal(await page.evaluate(() => window.__policyFixtureInjected), undefined); assert.equal(await page.locator('#policy-table img').count(), 0);
    await page.locator('#policy-table [data-policy-row]').nth(2).click(); assert.equal(JSON.parse(await page.locator('#policy-inspector-data').innerText()).occurrence.decision.raw_value, markupRaw);
    assert.equal(await page.locator('#policy-inspector img').count(), 0); assert.equal(await page.evaluate(() => window.__policyFixtureInjected), undefined); await page.click('#policy-inspector-close');
    check('filters and CSV cover only current loaded page while untrusted formula and raw markup remain inert text');

    await page.click('[data-section=policyaliases]'); assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 2);
    assert.equal(await page.locator('#policy-table [data-recrawl-url]').count(), 0); assert.equal(await page.locator('#policy-table .observation-age').count(), 0);
    assert.ok(!(await page.locator('#policy-table').innerText()).includes('503'));
    await page.locator('#policy-table [data-policy-row]').first().click(); const inspector = await page.locator('#policy-inspector-data').innerText();
    assert.ok(inspector.includes('check_only') && inspector.includes('current')); assert.ok(!inspector.includes('503')); await page.click('#policy-inspector-close');
    await page.locator('#policy-table [data-policy-row]').nth(1).click(); assert.match(await page.locator('#policy-inspector-data').innerText(), /inherited/); assert.match(await page.locator('#policy-inspector-data').innerText(), /observation_only/); await page.click('#policy-inspector-close');
    check('changed aliases remain policy facts with distinct current inherited check-only and observation-only admissions without fabricated HTTP or source selection');

    await download(page, '#policy-export-csv'); const nativeJson = JSON.parse(await download(page, '#policy-export-json'));
    assert.deepEqual(exportGets.slice(-2), [{ id: 'a', suffix: 'policy.csv' }, { id: 'a', suffix: 'policy.json' }]); assert.equal(nativeJson.observations.length, 405);
    exportClientTransports = await page.evaluate(() => window.__policyFixtureExportTransports);
    assert.deepEqual(exportClientTransports, []); assert.deepEqual(exportDownloadPaths, ['/api/crawls/a/policy.csv', '/api/crawls/a/policy.json']); assert.equal(await page.locator('#crawl-history').inputValue(), 'a');
    check('native ledger export buttons download saved ledger endpoints directly without fetch or XHR separately from filtered visible-page CSV');

    await page.click('[data-section=policy]'); holdLedgerId = 'a'; await page.click('#policy-refresh'); await until(() => held.some(entry => entry.kind === 'ledger-a'), 'held old project ledger');
    holdLedgerId = 'b'; await selectProject(page, 'b'); await until(() => held.some(entry => entry.kind === 'ledger-b'), 'held new project ledger');
    release('ledger-a'); await delay(150); assert.equal(await page.isDisabled('#policy-refresh'), true); assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 0);
    release('ledger-b'); await ledgerContains(page, 'b-occurrence-1');
    assert.ok(!(await page.locator('#policy-table').innerText()).includes('a-occurrence-1')); assert.equal(await page.locator('#crawl-history').inputValue(), 'b');
    holdLedgerId = 'a'; await selectProject(page, 'a'); await until(() => held.some(entry => entry.kind === 'ledger-a'), 'held old ledger error');
    await selectProject(page, 'b'); await ledgerContains(page, 'b-occurrence-1'); release('ledger-a', 500); await delay(150);
    assert.ok(!(await page.locator('#policy-error').innerText()).includes('PRIVATE OLD')); assert.equal(await page.locator('#crawl-history').inputValue(), 'b');
    check('late old-project ledger success error and finally cannot overwrite or free the newer project request');

    await selectProject(page, 'action'); await ledgerContains(page, 'action-occurrence-1');
    let cachedActionId = 'action-occurrence-1';
    for (const [action, generation] of [['pause', 2], ['resume', 3]]) {
      ledgers.action = actionRows(generation - 1, `before-${action}`);
      holdLedgerId = 'action'; await page.click('#policy-refresh'); await until(() => held.some(entry => entry.kind === 'ledger-action'), `held pre-${action} ledger`);
      holdLedgerId = 'action'; await page.click(`#${action}-button`);
      await until(() => held.filter(entry => entry.kind === 'ledger-action').length === 2, `new ledger generation after ${action}`);
      assert.equal(controlPosts.at(-1).action, action); assert.equal(await page.locator('#crawl-history').inputValue(), 'action');
      assert.match(await page.locator('#policy-summary').innerText(), /wird geladen/); assert.equal(await page.isDisabled('#policy-next'), true); assert.equal(await page.isDisabled('#policy-refresh'), true);
      assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 200); assert.ok((await page.locator('#policy-table').innerText()).includes(cachedActionId));
      release('ledger-action'); await delay(150);
      assert.equal(await page.isDisabled('#policy-next'), true); assert.equal(await page.isDisabled('#policy-refresh'), true); assert.match(await page.locator('#policy-summary').innerText(), /wird geladen/);
      assert.ok(!(await page.locator('#policy-table').innerText()).includes(`action-before-${action}-occurrence-1`)); assert.ok((await page.locator('#policy-table').innerText()).includes(cachedActionId));
      release('ledger-action'); cachedActionId = `action-after-${action}-occurrence-1`; await ledgerContains(page, cachedActionId);
      assert.equal(await page.isDisabled('#policy-refresh'), false); assert.equal(await page.isDisabled('#policy-next'), false); assert.ok(!(await page.locator('#policy-summary').innerText()).includes('wird geladen'));
    }
    assert.deepEqual(controlPosts, [{ id: 'action', action: 'pause' }, { id: 'action', action: 'resume' }]);
    check('same-project pause and resume start a new ledger generation while preserving cached rows and reject old responses or finally without leaving loading frozen');

    await selectProject(page, 'legacy'); await page.waitForFunction(() => /keine|legacy|nicht.*aktiv|ohne/i.test(document.getElementById('policy-summary').textContent)); assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 0);
    await selectProject(page, 'unknown'); await page.locator('#policy-error').waitFor({ state: 'visible' }); assert.match(await page.locator('#policy-error').innerText(), /engine|unbekannt|unterstützt/i);
    await selectProject(page, 'broken'); await page.waitForFunction(() => document.getElementById('policy-error').textContent.includes('Fixture policy ledger read failed'));
    check('legacy disabled unknown engine and failed reads stay explicit without implying an empty complete ledger');

    await selectProject(page, 'frozen'); await page.waitForFunction(() => document.getElementById('policy-summary').textContent.includes('Metadaten') || document.getElementById('policy-summary').textContent.includes('deklarativ'));
    const frozenSummary = await page.locator('#policy-summary').innerText();
    const frozenStatus = [frozenSummary, await page.locator('#policy-scope').innerText(), await page.locator('#policy-error').textContent()].join(' ');
    assert.match(frozenStatus, /nicht verfügbar|unverfügbar/i); assert.match(frozenStatus, /deklarativ|Metadaten/i); assert.ok(frozenStatus.includes(frozenReason));
    assert.ok(!frozenSummary.includes('Ledger vollständig')); assert.equal(await page.locator('#policy-table [data-policy-row]').count(), 0);
    for (const id of ['#export-csv', '#policy-export-csv', '#policy-export-json']) assert.ok(!(await page.isVisible(id)) || await page.isDisabled(id), `${id} unavailable for frozen metadata`);
    const beforeFrozenExport = exportGets.length; await delay(100); assert.equal(exportGets.length, beforeFrozenExport);
    check('known native engine with unavailable frozen ledger treats complete imported checkpoint as declarative metadata and disables every policy export');

    await selectProject(page, 'b'); await ledgerContains(page, 'b-occurrence-1'); await page.setViewportSize({ width: 320, height: 740 });
    await page.locator('#policy-page-label').scrollIntoViewIfNeeded(); const pagingBox = await page.locator('#policy-page-label').boundingBox(); assert.ok(pagingBox.y >= 0 && pagingBox.y + pagingBox.height <= 740);
    await openSettings(page); await page.locator('#settings-form button[type=submit]').scrollIntoViewIfNeeded(); const settingsBox = await page.locator('#settings-form button[type=submit]').boundingBox(); assert.ok(settingsBox.y >= 0 && settingsBox.y + settingsBox.height <= 740);
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)); await page.click('#settings-close');
    await page.locator('.main-footer').scrollIntoViewIfNeeded(); const footerBox = await page.locator('.main-footer').boundingBox(); assert.ok(footerBox.y < 740 && footerBox.y + footerBox.height > 0);
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)); assert.deepEqual(errors, []); assert.deepEqual(unexpectedNetwork, []);
    check('320px settings submission paging controls and main footer remain reachable without overflow or JavaScript errors');
    report('PASS'); console.log(`${checks.length} isolated policy UI checks passed.`);
  } catch (error) {
    console.error('Failure UI:', await page.evaluate(() => ({ selected: document.getElementById('crawl-history').value, message: document.getElementById('message').textContent, policy_summary: document.getElementById('policy-summary').textContent, policy_error: document.getElementById('policy-error').textContent, status: document.getElementById('crawl-status')?.textContent, pause: document.getElementById('pause-button').disabled, resume: document.getElementById('resume-button').disabled, refresh: document.getElementById('policy-refresh').disabled })));
    report('FAIL', error); throw error;
  }
  finally { for (const entry of held.splice(0)) send(entry.res, entry.value); await browser.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
})().catch(error => { console.error(error); server.closeAllConnections(); server.close(); process.exitCode = 1; });
