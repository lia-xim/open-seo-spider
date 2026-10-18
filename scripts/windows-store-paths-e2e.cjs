'use strict';
// Owns every process, website, store and output. This is a Windows path test,
// not a statement about UNC shares, system long-path settings or crash recovery.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawn} = require('node:child_process');
const {createHash, randomUUID} = require('node:crypto');
const {chromium} = require('../renderer/node_modules/playwright-core');

assert.equal(process.platform, 'win32', 'This gate measures actual Windows paths');
const root = path.resolve(__dirname, '..');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
for (const key of ['SPIDER_BINARY', 'SPIDER_BINARY_SHA256', 'SPIDER_WINDOWS_PATH_OUTPUT', 'OPEN_SEO_BROWSER_PATH']) {
  assert(process.env[key], 'Explicit ' + key + ' is required');
}
const binary = path.resolve(process.env.SPIDER_BINARY);
assert(fs.lstatSync(binary).isFile() && !fs.lstatSync(binary).isSymbolicLink());
const binaryHash = hash(fs.readFileSync(binary));
assert.equal(binaryHash, process.env.SPIDER_BINARY_SHA256);
const chrome = path.resolve(process.env.OPEN_SEO_BROWSER_PATH);
assert(fs.lstatSync(chrome).isFile() && !fs.lstatSync(chrome).isSymbolicLink());
const chromeHash = hash(fs.readFileSync(chrome));
const driverHash = hash(fs.readFileSync(__filename));
const output = path.resolve(process.env.SPIDER_WINDOWS_PATH_OUTPUT);
assert(!fs.existsSync(output), 'Requested output must be fresh; no reuse or relocation: ' + output);
assert(fs.statSync(path.dirname(output)).isDirectory(), 'Explicit output parent must already exist');
assert(!output.startsWith('\\\\?\\'), 'Use an ordinary explicit artifact path');

// A UUID is 36 UTF-16 units, followed by .sqlite (7) and a separator (1).
function directoryFor(prefix, sqliteUnits, unicode) {
  let current = path.resolve(prefix);
  const target = sqliteUnits - 44;
  assert(current.length < target - 2, 'Requested artifact path cannot fit the 214-unit baseline; choose it explicitly, never relocate');
  while (current.length < target) {
    let width = Math.min(64, target - current.length - 1);
    if (target - current.length - 1 - width === 1) width--;
    assert(width > 0);
    let name = '';
    if (unicode) while (name.length + 'Ä漢😀'.length <= width) name += 'Ä漢😀';
    name += 'p'.repeat(width - name.length);
    current = path.join(current, name);
  }
  assert.equal(current.length, target);
  assert(!current.includes('..'));
  for (const component of current.slice(path.parse(current).root.length).split(path.sep)) assert(component.length <= 255);
  return current;
}
const cases = [
  {name: 'ascii-214', units: 214},
  {name: 'ascii-255', units: 255},
  {name: 'ascii-300', units: 300},
  {name: 'ascii-deep-1150', units: 1150},
  {name: 'unicode-650', units: 650, unicode: true},
].map(item => ({...item, dataDir: directoryFor(path.join(output, item.name), item.units, item.unicode)}));
assert(Buffer.byteLength(cases.find(item => item.unicode).dataDir) > 1040, 'Unicode fixture must independently exceed 1040 UTF-8 bytes');
fs.mkdirSync(output);
const owners = [], cliOwners = [], results = [], cliRuns = [], downloadProofs = [], requests = [], external = [], browserErrors = [];
const controls = [], phaseProofs = [], links = [];
let browser, context, page, site, siteOrigin, offline = false, fixture, failure, app, phaseSerial = 0;
const websiteHits = [], witnessHits = [], witnessControls = [];
const nonce = randomUUID();
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
function pin() {
  assert.equal(hash(fs.readFileSync(binary)), binaryHash, 'Normal executable changed');
  assert.equal(hash(fs.readFileSync(chrome)), chromeHash, 'Browser executable changed');
  assert.equal(hash(fs.readFileSync(__filename)), driverHash, 'Owned driver changed');
}
function writeNew(name, value) {
  const bytes = Buffer.isBuffer(value) ? value : Buffer.from(typeof value === 'string' ? value : JSON.stringify(value, null, 2) + '\n');
  const full = path.join(output, name);
  fs.writeFileSync(full, bytes, {flag: 'wx'});
  return {path: full, bytes: bytes.length, sha256: hash(bytes)};
}
function checkpoint(name) {
  phaseProofs.push(writeNew('phase-' + (++phaseSerial) + '-' + name + '.json', {
    result: 'IN_PROGRESS', completed_phase: name, at: new Date().toISOString(), binary_sha256: binaryHash,
    cases: results, cli_runs: cliRuns, downloads: downloadProofs,
    actual_native_closes: owners.filter(owner => owner.close).map(owner => ({phase: owner.phase, pid: owner.child.pid, close: owner.close})),
  }));
}
function deadline(work, ms, label) {
  let timer;
  return Promise.race([work, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Timeout: ' + label)), ms); })])
    .finally(() => clearTimeout(timer));
}
async function until(work, label, ms = 45000) {
  const end = Date.now() + ms;
  do {
    if (app?.close) throw Error('Owned native process exited: ' + app.log);
    const value = await work();
    if (value) return value;
    await sleep(25);
  } while (Date.now() < end);
  throw Error('Timeout: ' + label);
}
function env() {
  const value = {...process.env, OPEN_SEO_COORDINATED_EXECUTION_WINDOW: 'confirmed'};
  for (const key of Object.keys(value)) if (/^OPEN_SEO_(?:PAGESPEED|AHREFS|MAJESTIC|MOZ|SEMRUSH|GSC|GA4).*KEY|^OPEN_SEO_.*TOKEN$/.test(key)) delete value[key];
  return value;
}
async function launch(item, phase, listen) {
  pin();
  const args = ['serve', '--listen', listen, '--data-dir', item.dataDir, '--shutdown-on-stdin-eof'];
  const child = spawn(binary, args, {cwd: root, env: env(), windowsHide: true, stdio: ['pipe', 'pipe', 'pipe']});
  const owner = {child, phase: item.name + '-' + phase, args, log: '', close: null, natural: false};
  owner.closed = new Promise(resolve => child.once('close', (code, signal) => { owner.close = {code, signal}; resolve(owner.close); }));
  child.once('error', error => { owner.spawnError = error; });
  for (const stream of [child.stdout, child.stderr]) stream.on('data', bytes => { owner.log += bytes.toString(); });
  owners.push(owner); app = owner;
  owner.origin = await until(() => {
    if (owner.spawnError) throw owner.spawnError;
    return owner.log.match(/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/m)?.[1];
  }, 'complete readiness line ' + owner.phase);
  assert(!['8787', '8790'].includes(new URL(owner.origin).port));
  assert.equal((await fetch(owner.origin + '/healthz', {signal: AbortSignal.timeout(10000), redirect: 'error'})).status, 200);
  return owner;
}
async function stop(owner) {
  assert(!owner.close, 'Process must still be live before natural EOF');
  owner.child.stdin.end();
  const close = await deadline(owner.closed, 30000, 'natural EOF ' + owner.phase);
  assert.deepEqual(close, {code: 0, signal: null}, owner.log);
  owner.natural = true;
  writeNew('native-' + owner.phase + '.log', owner.log);
  if (app === owner) app = null;
  pin();
}
async function raw(origin, route, body, status = 200) {
  const response = await fetch(origin + route, {
    method: body === undefined ? 'GET' : 'POST', cache: 'no-store', redirect: 'error',
    signal: AbortSignal.timeout(30000),
    headers: body === undefined ? {} : {'content-type': 'application/json'},
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const bytes = Buffer.from(await response.arrayBuffer());
  assert.equal(response.status, status, route + '\n' + bytes.toString());
  requests.push({origin, route, method: body === undefined ? 'GET' : 'POST', status, bytes: bytes.length, sha256: hash(bytes)});
  return {response, bytes};
}
async function json(origin, route, body, status = 200) {
  const result = await raw(origin, route, body, status);
  return {...result, value: JSON.parse(result.bytes)};
}
async function cli(label, store, outputPath, expected = 0) {
  pin();
  assert(!fs.existsSync(outputPath));
  const args = ['sitemap-rows', store, '--output', outputPath];
  const child = spawn(binary, args, {cwd: root, env: env(), windowsHide: true, stdio: ['ignore', 'pipe', 'pipe']});
  const owner = {child, label, close: null};
  owner.closed = new Promise((resolve, reject) => {
    child.once('error', reject); child.once('close', (code, signal) => { owner.close = {code, signal}; resolve(owner.close); });
  });
  cliOwners.push(owner);
  let stdout = '', stderr = '';
  let outputBytes = 0, rejectOutput;
  const outputFailure = new Promise((_, reject) => { rejectOutput = reject; });
  const collect = (target, bytes) => {
    if (owner.output_budget_error) return;
    outputBytes += bytes.length;
    if (outputBytes > 4 * 1024 * 1024) {
      const error = Error('Read-only CLI output exceeds 4 MiB: ' + label);
      owner.output_budget_error = error.message;
      rejectOutput(error); // Propagates through the awaited flow and its finally.
      return;
    }
    if (target === 'stdout') stdout += bytes.toString(); else stderr += bytes.toString();
  };
  child.stdout.on('data', bytes => { collect('stdout', bytes); });
  child.stderr.on('data', bytes => { collect('stderr', bytes); });
  const close = await deadline(Promise.race([owner.closed, outputFailure]), 30000, 'read-only CLI ' + label);
  cliRuns.push({label, pid: child.pid, args, close, stdout, stderr});
  assert.equal(close.signal, null);
  if (expected === 0) {
    assert.equal(close.code, 0, stderr);
    assert(fs.statSync(outputPath).isFile());
    pin(); return JSON.parse(fs.readFileSync(outputPath));
  }
  assert.notEqual(close.code, 0, 'Expected an actual read-only refusal');
  assert(!fs.existsSync(outputPath), 'Refusal must not leave a partial destination');
  pin(); return null;
}
function csv(bytes) {
  const input = bytes.toString('utf8').replace(/^\uFEFF/, '');
  const rows = []; let row = [], cell = '', quoted = false;
  for (let i = 0; i < input.length; i++) {
    const ch = input[i];
    if (ch === '"') { if (quoted && input[i + 1] === '"') { cell += '"'; i++; } else quoted = !quoted; }
    else if (!quoted && ch === ',') { row.push(cell); cell = ''; }
    else if (!quoted && (ch === '\n' || ch === '\r')) {
      if (ch === '\r' && input[i + 1] === '\n') i++;
      row.push(cell); rows.push(row); row = []; cell = '';
    } else cell += ch;
  }
  assert(!quoted);
  if (cell || row.length) { row.push(cell); rows.push(row); }
  assert(rows.length); for (const values of rows) assert.equal(values.length, rows[0].length);
  return rows;
}
const nativeHeaders = ['URL', 'Final URL', 'Status', 'Final Status', 'Content Type', 'Title', 'Description', 'H1', 'H2', 'Canonical',
  'Indexable', 'Indexability', 'Depth', 'Response ms', 'Bytes', 'Words', 'Outlinks', 'Images', 'Error', 'Rendered', 'Raw Title',
  'Rendered URL', 'Render Error', 'Browser Requests', 'Console Errors', 'HTML Canonical', 'HTTP Canonical', 'Indexability Agent',
  'Effective Robots', 'Pagination Next', 'Pagination Prev', 'AMPHTML', 'Title Pixels', 'Description Pixels', 'Title Pixel Measurements',
  'Description Pixel Measurements', 'Text Metrics Engine', 'Crawl Coverage', 'Observation Basis', 'Selected State', 'Parent Snapshot SHA256', 'Fetched At'];
function nativeCSV(bytes, report) {
  const rows = csv(bytes); assert.deepEqual(rows[0], nativeHeaders); assert.equal(rows.length, 3);
  // This controlled non-rendered, robots-free report has no optional TLS/profile columns.
  const expected = report.pages.map(p => [p.url, p.final_url, String(p.status), String(p.final_status), p.content_type,
    p.titles[0] || '', p.meta_descriptions[0] || '', p.h1.join(' | '), p.h2.join(' | '), p.canonical.join(' | '),
    String(p.indexable), p.indexability_reasons.join(' | '), String(p.depth), String(p.response_ms), String(p.size_bytes), String(p.word_count),
    String(p.links.length), String(p.images.length), p.error || '', String(p.rendered), p.raw_metadata?.titles?.[0] || '',
    p.rendered_url || '', p.renderer_error || '', String(p.network_requests.length), String(p.console_errors.length),
    p.html_canonical.join(' | '), p.header_canonical.join(' | '), p.indexability_agent || 'googlebot', '',
    p.pagination_next.join(' | '), p.pagination_prev.join(' | '), p.amphtml.join(' | '),
    p.title_pixel_widths[0] == null ? '' : String(p.title_pixel_widths[0]),
    p.description_pixel_widths[0] == null ? '' : String(p.description_pixel_widths[0]),
    p.title_pixel_widths.length ? JSON.stringify(p.title_pixel_widths) : '',
    p.description_pixel_widths.length ? JSON.stringify(p.description_pixel_widths) : '', p.text_metrics_engine || '',
    'crawl_snapshot', 'captured', '', '', p.fetched_at]);
  assert.deepEqual(rows.slice(1), expected, 'All 42 native CSV columns, every original page and original order');
  return rows;
}
function validateReport(report) {
  assert.equal(report.pages.length, 2);
  assert.deepEqual(report.pages.map(p => p.url).sort(), fixture.urls.slice().sort());
  for (const p of report.pages) {
    const owned = fixture.pages.get(p.url);
    assert(owned); assert.equal(p.status, 200); assert.equal(p.final_status, 200); assert.equal(p.final_url, p.url);
    assert.deepEqual(p.titles, [owned.title]); assert.deepEqual(p.meta_descriptions, [owned.description]);
    assert.deepEqual(p.h1, [owned.heading]); assert.equal(p.rendered, false);
  }
  assert.deepEqual(report.config.sitemap_urls, [fixture.sitemap]);
  assert(report.sitemap_catalog); assert(report.sitemap_source_measurements); assert(report.sitemap_coverage_inventory);
  assert(report.config.sitemap_source_archive);
}
function validateRaw(window) {
  assert.equal(window.context.engine, 'sitemap-verified-raw-window-v1');
  assert.equal(window.context.generation, 1);
  assert.equal(window.has_more, false); assert.equal(window.next_after_sequence, null);
  assert.deepEqual(window.rows.map(r => r.kind), ['root', 'document', 'entry', 'entry', 'location', 'location']);
  assert.deepEqual(window.rows.map(r => r.row.sequence), [1, 2, 3, 4, 5, 6]);
  assert.equal(window.rows[0].row.normalized_document_url, fixture.sitemap);
  const doc = window.rows[1].row;
  assert.equal(doc.task_url, fixture.sitemap); assert.equal(doc.transport.body_complete, true);
  assert.equal(doc.transport.post_http_decoding_bytes, fixture.xml.length);
  assert.equal(doc.transport.post_http_decoding_sha256, hash(fixture.xml));
  assert.equal(doc.entity_decode.decoded_xml_sha256, hash(fixture.xml));
  assert.equal(doc.parser.complete, true); assert.equal(doc.parser.entry_count, 2); assert.equal(doc.parser.location_count, 2);
  assert.deepEqual(window.rows.slice(4).map(r => r.row.observed.normalized_fetch_url), fixture.urls);
  assert.deepEqual(window.rows.slice(4).map(r => r.row.observed.decoded_value), fixture.urls);
  assert.equal(window.context.ledger.entry_rows, 2); assert.equal(window.context.ledger.location_rows, 2);
  assert.equal(window.context.source_summary.record_count, 1); assert.equal(window.context.source_summary.commit_count, 1);
}
function validateCheckpoint(checkpoint, rawWindow) {
  assert(checkpoint, 'Native terminal/cold status must expose its validated SQLite checkpoint');
  assert.equal(checkpoint.generation, 1); assert.equal(checkpoint.revision, 1); assert.equal(checkpoint.outcome, 'completed');
  assert.equal(checkpoint.completed, 2); assert.equal(checkpoint.pending, 0); assert.equal(checkpoint.discovery_pending, 0);
  assert.equal(checkpoint.discovery_complete, true); assert(Number.isFinite(Date.parse(checkpoint.created_at)));
  assert.match(checkpoint.report_sha256, /^[0-9a-f]{64}$/); assert.match(checkpoint.config_sha256, /^[0-9a-f]{64}$/);
  if (rawWindow) {
    assert.equal(checkpoint.config_sha256, rawWindow.context.crawl_config_sha256);
    assert.deepEqual(checkpoint.sitemap_ledger, rawWindow.context.ledger);
    assert.deepEqual(checkpoint.sitemap_source_measurements, rawWindow.context.source_summary);
  }
  // report_sha256 binds the Core checkpoint before App analysis, not the App
  // export encoding. Never equate it with the physical App JSON file's digest.
}
function fileProof(file) {
  const info = fs.statSync(file, {bigint: true});
  assert(info.isFile());
  const bytes = fs.readFileSync(file);
  return {path: file, bytes: bytes.length, sha256: hash(bytes), ino: String(info.ino), dev: String(info.dev)};
}
function sameFile(left, right) {
  const a = fileProof(left), b = fileProof(right);
  assert.equal(a.ino, b.ino); assert.equal(a.dev, b.dev); assert.equal(a.sha256, b.sha256);
}
function pathFacts(db) {
  const paths = {database: db, rollback_journal: db + '-journal', wal: db + '-wal', shm: db + '-shm', archive: db + '.sitemap-bodies'};
  return Object.fromEntries(Object.entries(paths).map(([kind, value]) => {
    for (const part of value.slice(path.parse(value).root.length).split(path.sep)) assert(part.length <= 255);
    return [kind, {path: value, utf16_units: value.length, utf8_bytes: Buffer.byteLength(value)}];
  }));
}
async function archive(origin, id, db, oldToken) {
  const route = '/api/crawls/' + id + '/sitemaps/source-bodies';
  const inventory = (await json(origin, route)).value;
  assert.equal(inventory.generation, 1); assert.equal(inventory.window.state, 'recorded_metadata');
  assert.equal(inventory.window.summary.captures, 1); assert.equal(inventory.window.summary.body_bytes, fixture.xml.length * 2);
  assert.equal(inventory.window.summary.legacy_history_unknown, false);
  assert.equal(inventory.window.summary.directory_entry_crash_durability_proven, false);
  assert.equal(inventory.byte_availability, 'not_checked'); assert.equal(inventory.download_available, null);
  assert.equal(inventory.window.byte_availability, 'not_checked'); assert.equal(inventory.window.download_available, null);
  assert.equal(inventory.cursor, null); assert.equal(inventory.window.next_sequence, null);
  assert.equal(inventory.window.captures.length, 1);
  const capture = inventory.window.captures[0];
  assert.equal(capture.sequence, 1); assert.equal(capture.capture_generation, 1); assert.equal(capture.omission, null);
  assert.deepEqual(capture.bodies.map(body => body.representation), ['post_http_entity', 'decoded_xml']);
  assert.equal(new Set(capture.bodies.map(body => body.body_id)).size, 2);
  const directory = db + '.sitemap-bodies';
  assert(fs.lstatSync(directory).isDirectory() && !fs.lstatSync(directory).isSymbolicLink());
  assert.deepEqual(fs.readdirSync(directory).sort(), capture.bodies.map(body => body.body_id + '.bin').sort());
  const proofs = [];
  for (const body of capture.bodies) {
    assert.match(body.body_id, /^[0-9a-f]{64}$/); assert.equal(body.extent, 'complete');
    assert.equal(body.bytes, fixture.xml.length); assert.equal(body.sha256, hash(fixture.xml));
    const file = path.join(directory, body.body_id + '.bin');
    assert.deepEqual(fs.readFileSync(file), fixture.xml, 'Original requested archive authority, not a relocated copy');
    const downloaded = await raw(origin, route + '/' + body.body_id + '?snapshot=' + encodeURIComponent(oldToken || inventory.snapshot));
    assert.deepEqual(downloaded.bytes, fixture.xml);
    assert.equal(downloaded.response.headers.get('content-type'), 'application/octet-stream');
    assert.equal(Number(downloaded.response.headers.get('content-length')), fixture.xml.length);
    assert.equal(downloaded.response.headers.get('etag'), '"sha256:' + hash(fixture.xml) + '"');
    proofs.push({...fileProof(file), representation: body.representation});
  }
  return {inventory, files: proofs};
}
async function download(selector, name) {
  const pending = page.waitForEvent('download');
  await page.locator(selector).click();
  const result = await pending;
  assert.equal(await result.failure(), null);
  const bytes = fs.readFileSync(await result.path());
  downloadProofs.push({...writeNew(name, bytes), selector, suggested_filename: result.suggestedFilename()});
  return bytes;
}
async function browserRead(origin, id, item, phase, project, native) {
  await context.route('**/*', route => {
    const url = new URL(route.request().url());
    if (url.origin !== origin) { external.push(url.href); return route.abort('blockedbyclient'); }
    return route.continue();
  });
  await page.goto(origin, {waitUntil: 'domcontentloaded'});
  await page.locator('#crawl-history option[value="' + id + '"]').waitFor({state: 'attached'});
  await page.selectOption('#crawl-history', id);
  await page.locator('#export-project:enabled').waitFor();
  await page.locator('[data-section="internal"]').first().click();
  await until(async () => (await page.locator('#results-table tbody tr[data-row]').count()) === 2, 'all two visible page rows');
  const visibleURLs = await page.locator('#results-table tbody tr[data-row]').evaluateAll(rows => rows.map(row => row.querySelector('td .url-cell')?.textContent));
  assert.deepEqual(visibleURLs.slice().sort(), fixture.urls.slice().sort());
  const downloadedProject = await download('#export-project', item.name + '-' + phase + '-browser-project.json');
  assert.deepEqual(downloadedProject, project.bytes, 'Actual product button downloads complete native project bytes');
  validateReport(JSON.parse(downloadedProject));
  const generic = await download('#export-csv', item.name + '-' + phase + '-browser-section.csv');
  const rows = csv(generic);
  assert.equal(rows.length, 3); assert.equal(rows[0][0], 'URL');
  assert.deepEqual(rows.slice(1).map(row => row[0]).sort(), fixture.urls.slice().sort());
  assert.equal(new Set(rows.slice(1).map(row => row[0])).size, 2);
  const body = await page.locator('body').innerText();
  assert(!body.includes('unable to open database file'));
  await page.screenshot({path: path.join(output, item.name + '-' + phase + '.png'), fullPage: true});
  await context.unroute('**/*');
  return {generic_sha256: hash(generic), generic, native_sha256: hash(native.bytes)};
}
async function witness() {
  const response = await fetch(siteOrigin + '/owned-offline-witness', {headers: {'x-owned-witness': nonce},
    signal: AbortSignal.timeout(10000), redirect: 'error', cache: 'no-store'});
  assert.equal(response.status, 410); assert.equal(await response.text(), 'Owned offline witness');
}
async function readonlyControls(item, db, expected) {
  const before = fileProof(db), namesBefore = fs.readdirSync(item.dataDir).sort();
  const aliases = [
    {name: 'ordinary', value: db},
    {name: 'verbatim', value: '\\\\?\\' + db},
    {name: 'relative-mixed-dot', value: '.\\' + path.relative(root, path.dirname(db)).replace(/\\/g, '/') + '/./' + path.basename(db)},
  ];
  // These legacy aliases intentionally keep the default SQLite VFS; exercise
  // them on the short baseline, without promising new long-path alias support.
  if (item.units === 214) aliases.push({name: 'ordinary-final-dot', value: db + '.'}, {name: 'ordinary-final-space', value: db + ' '});
  for (const alias of aliases) {
    if (alias.name.startsWith('ordinary-final-')) {
      // Node may use the verbatim namespace where the trailing character is
      // literal; the retained legacy SQLite VFS uses ordinary Win32 parsing.
      let nodeStat;
      try { fs.statSync(alias.value); nodeStat = {status: 'observed'}; }
      catch (error) {
        assert.equal(error.code, 'ENOENT');
        nodeStat = {status: 'legacy_file_id_not_observed_by_node', error_code: error.code,
          qualification: 'Node literal/verbatim alias lookup differs from the preserved ordinary SQLite namespace'};
      }
      if (nodeStat.status === 'observed') sameFile(db, alias.value);
      controls.push({name: item.name + '-' + alias.name, original_database: db, alias: alias.value, node_alias_identity: nodeStat,
        native_identity_scope: 'actual read-only CLI, exact dynamic saved raw rows/context, unchanged original DB hash and no second directory entries'});
    } else sameFile(db, alias.value);
    const destination = path.join(item.dataDir, 'readonly-' + alias.name + '.json');
    const window = await cli(item.name + '-' + alias.name, alias.value, destination);
    validateRaw(window); assert.deepEqual(window, expected, 'All raw rows and context are the same saved database through ' + alias.name);
  }
  const junction = path.join(output, item.name + '-owned-junction');
  fs.symlinkSync(item.dataDir, junction, 'junction'); links.push(junction);
  assert(fs.lstatSync(junction).isSymbolicLink());
  const junctionDB = path.join(junction, path.basename(db)); sameFile(db, junctionDB);
  const aliased = await cli(item.name + '-junction-database-only', junctionDB, path.join(item.dataDir, 'readonly-junction.json'));
  assert.deepEqual(aliased, expected);
  // Raw-only CLI does not access archive bodies. This proves DB identity via an
  // owned junction; it deliberately does not claim archive traversal is allowed.
  controls.push({name: item.name + '-junction', scope: 'read-only SQLite/raw rows only; archive nofollow unchanged', junction, target: item.dataDir});
  const created = aliases.map(alias => 'readonly-' + alias.name + '.json').concat('readonly-junction.json');
  assert.deepEqual(fs.readdirSync(item.dataDir).sort(), namesBefore.concat(created).sort(), 'No hidden fallback DB or second archive');
  assert.equal(fileProof(db).sha256, before.sha256, 'Read-only alias queries preserve main database bytes');
  const missing = path.join(item.dataDir, 'missing-readonly.sqlite');
  const invalid = path.join(item.dataDir, 'invalid-readonly.sqlite');
  const invalidBytes = Buffer.from('Owned invalid SQLite file. Do not replace.\r\n');
  fs.writeFileSync(invalid, invalidBytes, {flag: 'wx'});
  const refusalBefore = fs.readdirSync(item.dataDir).sort();
  await cli(item.name + '-missing', missing, path.join(item.dataDir, 'missing-refusal.json'), 1);
  await cli(item.name + '-invalid', invalid, path.join(item.dataDir, 'invalid-refusal.json'), 1);
  assert.deepEqual(fs.readdirSync(item.dataDir).sort(), refusalBefore, 'No read-only DB/WAL/SHM/journal/archive/output creation on missing/invalid input');
  assert(!fs.existsSync(missing)); assert.deepEqual(fs.readFileSync(invalid), invalidBytes);
  assert.equal(fileProof(db).sha256, before.sha256);
  controls.push({name: item.name + '-read-only-refusals', missing, invalid, invalid_sha256: hash(invalidBytes), no_created_entries: true});
}
async function oneCase(item) {
  offline = false; websiteHits.length = 0; witnessHits.length = 0; witnessControls.length = 0;
  fs.mkdirSync(item.dataDir, {recursive: true});
  const first = await launch(item, 'warm', '127.0.0.1:0');
  const created = (await json(first.origin, '/api/crawls', {
    seed_urls: [], sitemap_urls: [fixture.sitemap], discover_sitemaps: false, crawl_links: false,
    obey_robots: false, delay_ms: 0, sitemap_source_archive: {},
  })).value;
  assert.match(created.id, /^[0-9a-f-]{36}$/);
  const id = created.id, db = path.join(item.dataDir, id + '.sqlite');
  assert.equal(db.length, item.units);
  const terminal = await until(async () => {
    const status = (await json(first.origin, '/api/crawls/' + id)).value;
    return ['completed', 'failed', 'cancelled', 'limited'].includes(status.status) && status;
  }, item.name + ' complete small native capture');
  assert.equal(terminal.status, 'completed', JSON.stringify(terminal)); assert.equal(terminal.generation, 1);
  validateCheckpoint(terminal.checkpoint);
  assert.equal(terminal.stats.fetched, 2); assert.equal(terminal.stats.queued, 0); assert.equal(terminal.stats.in_flight, 0);
  assert.deepEqual(websiteHits.slice().sort(), ['/a?x=1&y=two', '/b', '/sitemap.xml'].sort());
  const project = await json(first.origin, '/api/crawls/' + id + '/project.json'); validateReport(project.value);
  const native = await raw(first.origin, '/api/crawls/' + id + '/export.csv'); nativeCSV(native.bytes, project.value);
  writeNew(item.name + '-warm-native-project.json', project.bytes); writeNew(item.name + '-warm-native-pages.csv', native.bytes);
  const warmArchive = await archive(first.origin, id, db);
  const warmBrowser = await browserRead(first.origin, id, item, 'warm', project, native);
  assert.deepEqual(websiteHits.slice().sort(), ['/a?x=1&y=two', '/b', '/sitemap.xml'].sort());
  offline = true; await witness();
  await page.goto('about:blank');
  await stop(first); // The 410 witness is already active before this close.
  const reportPath = path.join(item.dataDir, id + '.json');
  const manifestPath = path.join(item.dataDir, id + '.job.json');
  const generationPath = path.join(item.dataDir, 'snapshots', id, 'generation-1.json');
  const reportBytes = fs.readFileSync(reportPath), generationBytes = fs.readFileSync(generationPath);
  assert.deepEqual(JSON.parse(reportBytes), project.value); assert.deepEqual(JSON.parse(generationBytes), project.value);
  const manifest = JSON.parse(fs.readFileSync(manifestPath));
  assert.equal(manifest.generation, 1); assert.deepEqual(manifest.checkpoint, terminal.checkpoint);
  const dbBefore = fileProof(db); assert(dbBefore.bytes > 0);
  const rawPath = path.join(item.dataDir, 'initial-readonly.json');
  const rawWindow = await cli(item.name + '-initial', db, rawPath); validateRaw(rawWindow);
  validateCheckpoint(terminal.checkpoint, rawWindow);
  await readonlyControls(item, db, rawWindow);
  const second = await launch(item, 'cold', new URL(first.origin).host);
  assert.notEqual(second.child.pid, first.child.pid); assert(first.close && first.natural); assert.equal(second.origin, first.origin);
  const cold = await json(second.origin, '/api/crawls/' + id);
  assert.equal(cold.value.status, 'completed'); assert.equal(cold.value.generation, 1);
  validateCheckpoint(cold.value.checkpoint, rawWindow); assert.deepEqual(cold.value.checkpoint, terminal.checkpoint);
  const coldProject = await json(second.origin, '/api/crawls/' + id + '/project.json');
  assert.deepEqual(coldProject.bytes, project.bytes, 'Same current native JSON bytes after actual process death/reopen');
  const coldCSV = await raw(second.origin, '/api/crawls/' + id + '/export.csv');
  assert.deepEqual(coldCSV.bytes, native.bytes); nativeCSV(coldCSV.bytes, coldProject.value);
  const coldArchive = await archive(second.origin, id, db, warmArchive.inventory.snapshot);
  assert.deepEqual(coldArchive.inventory, warmArchive.inventory, 'Original source descriptors/token preserved');
  assert.deepEqual(coldArchive.files, warmArchive.files, 'Exact original .sitemap-bodies files and identity, no alternative authority');
  const coldBrowser = await browserRead(second.origin, id, item, 'cold', coldProject, coldCSV);
  assert.deepEqual(coldBrowser.generic, warmBrowser.generic, 'All current-section CSV bytes preserved after cold load');
  await page.goto('about:blank');
  await stop(second);
  const finalRaw = await cli(item.name + '-final-cold', db, path.join(item.dataDir, 'final-readonly.json'));
  assert.deepEqual(finalRaw, rawWindow);
  await witness();
  assert.equal(witnessHits.length, 0, 'Native/browser/CLI must not refetch the offline website');
  assert.equal(witnessControls.length, 2);
  assert.deepEqual(websiteHits.slice().sort(), ['/a?x=1&y=two', '/b', '/sitemap.xml'].sort());
  assert.deepEqual(fs.readFileSync(reportPath), reportBytes); assert.deepEqual(fs.readFileSync(generationPath), generationBytes);
  assert.equal(fileProof(db).sha256, dbBefore.sha256);
  const facts = pathFacts(db);
  const row = {name: item.name, requested_data_dir: item.dataDir, paths: facts, job: id, generation: 1,
    sqlite: fileProof(db), raw_report: fileProof(reportPath), generation_report: fileProof(generationPath), manifest: fileProof(manifestPath),
    checkpoint: terminal.checkpoint,
    archived_bodies: coldArchive.files, archive_metadata_sha256: hash(JSON.stringify(coldArchive.inventory)),
    complete_json_bytes: project.bytes.length, complete_json_sha256: hash(project.bytes),
    complete_csv_bytes: native.bytes.length, complete_csv_sha256: hash(native.bytes), native_csv_columns: nativeHeaders.length,
    browser_section_csv_sha256: coldBrowser.generic_sha256, pages: 2, raw_rows: 6,
    warm: {pid: first.child.pid, origin: first.origin, close: first.close, natural_stdin_eof: first.natural},
    cold: {pid: second.child.pid, origin: second.origin, close: second.close, natural_stdin_eof: second.natural},
    website_capture_hits: websiteHits.slice(), offline_product_hits: witnessHits.slice(), offline_control_probes: witnessControls.length,
    sqlite_sidecars_after_both_closes: ['-journal', '-wal', '-shm'].map(suffix => ({path: db + suffix, exists: fs.existsSync(db + suffix)}))};
  results.push(row); checkpoint(item.name + '-cold-complete');
  console.log('PASS ' + item.name + ': ' + db.length + ' UTF-16 / ' + Buffer.byteLength(db) + ' UTF-8; complete crawl/archive/read-only/cold');
}
async function forcedCleanup(owner) {
  if (owner.close) return;
  owner.forced_cleanup = true;
  try { owner.kill_requested = owner.child.kill(); }
  catch (error) { owner.kill_error = String(error); }
  try { await deadline(owner.closed, 10000, 'observe actual post-kill close ' + (owner.phase || owner.label)); }
  catch (error) {
    owner.cleanup_close_not_observed = String(error);
    failure ||= error;
    // A failed cleanup remains a FAIL with close:null. Release our own handles
    // so a missing OS close cannot prevent writing that truthful receipt.
    owner.child.unref();
    for (const stream of [owner.child.stdin, owner.child.stdout, owner.child.stderr]) stream?.destroy();
  }
}

(async () => {
  const started = Date.now();
  try {
    pin();
    site = http.createServer((req, res) => {
      if (offline) {
        if (req.url === '/owned-offline-witness' && req.headers['x-owned-witness'] === nonce) witnessControls.push(req.url);
        else witnessHits.push(req.url);
        res.writeHead(410, {'content-type': 'text/plain', 'cache-control': 'no-store'}); return res.end('Owned offline witness');
      }
      websiteHits.push(req.url);
      if (req.url === '/sitemap.xml') { res.writeHead(200, {'content-type': 'application/xml; charset=utf-8'}); return res.end(fixture.xml); }
      const found = fixture.pages.get(siteOrigin + req.url);
      if (!found) { res.writeHead(404); return res.end('Unexpected owned fixture request'); }
      res.writeHead(200, {'content-type': 'text/html; charset=utf-8'}); res.end(found.html);
    });
    await new Promise((resolve, reject) => { site.once('error', reject); site.listen(0, '127.0.0.1', resolve); });
    assert(![8787, 8790].includes(site.address().port)); siteOrigin = 'http://127.0.0.1:' + site.address().port;
    const urls = [siteOrigin + '/a?x=1&y=two', siteOrigin + '/b'];
    const pages = new Map(urls.map((url, index) => {
      const title = 'Owned long path ' + index + ' – Ä漢😀';
      const description = 'Independent exact description for long-path page ' + index;
      const heading = 'Owned path heading ' + index;
      return [url, {title, description, heading, html: '<!doctype html><html lang="de"><head><meta charset="utf-8"><title>' + title + '</title><meta name="description" content="' + description + '"></head><body><h1>' + heading + '</h1><p>Owned small native path fixture.</p></body></html>'}];
    }));
    fixture = {urls, pages, sitemap: siteOrigin + '/sitemap.xml', xml: Buffer.from('\uFEFF<?xml version="1.0" encoding="UTF-8"?>\r\n<!-- Original Ä漢😀 archive bytes -->\r\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">' + urls.map(url => '<url><loc>' + url.replace(/&/g, '&amp;') + '</loc></url>').join('') + '</urlset>\r\n')};
    browser = await chromium.launch({executablePath: chrome, headless: true});
    context = await browser.newContext({acceptDownloads: true, viewport: {width: 1440, height: 1000}});
    page = await context.newPage(); page.setDefaultTimeout(30000);
    page.on('pageerror', error => { browserErrors.push(error.stack || error.message); });
    for (const item of cases) await oneCase(item);
    await deadline(browser.close(), 30000, 'owned browser close'); browser = null;
    assert.equal(witnessHits.length, 0); assert.equal(external.length, 0); assert.equal(browserErrors.length, 0);
    assert.equal(results.length, cases.length); assert.equal(owners.length, cases.length * 2);
    assert(owners.every(owner => owner.natural && owner.close?.code === 0 && owner.close.signal === null));
    pin();
  } catch (error) {
    failure = error;
    if (page && !page.isClosed()) {
      try { writeNew('failure-dom.html', await page.content()); await page.screenshot({path: path.join(output, 'failure.png'), fullPage: true}); }
      catch (captureError) { writeNew('failure-capture-error.txt', String(captureError)); }
    }
  } finally {
    if (browser) {
      try { await deadline(browser.close(), 30000, 'owned browser cleanup close'); browser = null; }
      catch (error) { browserErrors.push(String(error)); failure ||= error; }
    }
    for (const owner of owners) if (!owner.close) {
      try { await stop(owner); }
      catch (error) { owner.cleanup_error = String(error); failure ||= error; await forcedCleanup(owner); }
    }
    for (const owner of cliOwners) if (!owner.close) {
      await forcedCleanup(owner);
    }
    // Keep the website/witness alive through browser, CLI and every native close.
    if (site?.listening) { site.closeAllConnections(); await new Promise(resolve => site.close(resolve)); }
    for (const owner of owners) if (!fs.existsSync(path.join(output, 'native-' + owner.phase + '.log'))) writeNew('native-' + owner.phase + '.log', owner.log);
    const receipt = {result: failure ? 'FAIL' : 'PASS', error: failure?.stack, duration_ms: Date.now() - started,
      binary, binary_sha256: binaryHash, chrome, chrome_sha256: chromeHash, driver: __filename, driver_sha256: driverHash,
      intended_scope: 'Windows ordinary requested stores; 5 small native sitemap captures, 10 natural native processes, saved JSON/CSV/browser downloads, original archive bytes and read-only alias/refusal controls. No fallback paths. Intended scope is not a success claim on FAIL.',
      actual_counts: {completed_cases: results.length, native_processes_started: owners.length,
        observed_natural_eof_zero_closes: owners.filter(owner => owner.natural && owner.close?.code === 0 && owner.close.signal === null).length,
        read_only_cli_processes_closed: cliOwners.filter(owner => owner.close).length, browser_downloads_verified: downloadProofs.length},
      limits: ['No UNC/network share matrix, device namespace or system settings changes', 'No crash/power-loss durability claim',
        'Owned junction control proves SQLite/raw-reader identity only; original archive nofollow rules remain unchanged',
        'Main report and archive exact byte/identity assertions are current-build cold-reopen proofs, not historical encoding parity',
        'Build provenance must be established by the separate root build/source manifest; this driver pins executable and owned harness bytes'],
      output, results, cli_runs: cliRuns, requests, downloads: downloadProofs, controls, phase_checkpoints: phaseProofs,
      owners: owners.map(owner => ({phase: owner.phase, pid: owner.child.pid, origin: owner.origin, args: owner.args, close: owner.close,
        natural_stdin_eof: owner.natural, cleanup_error: owner.cleanup_error, forced_cleanup: owner.forced_cleanup,
        kill_requested: owner.kill_requested, kill_error: owner.kill_error, cleanup_close_not_observed: owner.cleanup_close_not_observed})),
      cli_processes: cliOwners.map(owner => ({label: owner.label, pid: owner.child.pid, close: owner.close, output_budget_error: owner.output_budget_error,
        forced_cleanup: owner.forced_cleanup, kill_requested: owner.kill_requested, kill_error: owner.kill_error,
        cleanup_close_not_observed: owner.cleanup_close_not_observed})),
      external_requests: external, browser_errors: browserErrors, owned_junctions: links};
    writeNew('windows-store-paths-e2e.json', receipt);
  }
  if (failure) throw failure;
  console.log('PASS windows store paths: ' + results.length + ' exact requested stores / ' + owners.length + ' natural native closes');
})().catch(error => { console.error(error.stack); process.exitCode = 1; });
