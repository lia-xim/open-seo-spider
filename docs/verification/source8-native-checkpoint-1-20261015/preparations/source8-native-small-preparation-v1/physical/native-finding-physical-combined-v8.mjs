/** AUTHORED ONLY. No fixture, database, process, request or physical proof run. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import readline from 'node:readline';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath, pathToFileURL} from 'node:url';

const OWN = path.dirname(fileURLToPath(import.meta.url));
const ROOT = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const SOURCE = path.join(ROOT, 'combined-source-v8');
const REPO = path.join(ROOT, 'mcp-file-tools-v1');
const BINDINGS_SHA = "c58f1984c6b6593d384915157afdcdae206acd1bb8af72279a9ee82702e1b7e7";
const CSV_CAP = 64 * 1024 * 1024, JSON_CAP = 8 * 1024 * 1024, FIELD_CAP = 65_536;
const ENGINE = 'open-seo-spider/app-sitemap-findings-v1';
const SELECTION = {filter: 'urls-in-sitemap', assessment: 'all', requested_url: null, source_task_url: null, search: null};
const sha = data => createHash('sha256').update(data).digest('hex');
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const bytes = value => Buffer.byteLength(value, 'utf8');
const hashPattern = /^[a-f0-9]{64}$/;
const utf8 = value => Buffer.from(value, 'utf8');
async function within(promise, ms, reason) { let timer; try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(Error(reason)), ms); })]); } finally { clearTimeout(timer); } }
function childOf(base, file) { const r = path.relative(base, file); return r !== '' && r !== '..' && !r.startsWith('..' + path.sep) && !path.isAbsolute(r); }
function regular(file, maximum, allowEmpty = false) { const s = fs.lstatSync(file); assert(s.isFile() && !s.isSymbolicLink() && s.size >= (allowEmpty ? 0 : 1) && s.size <= maximum, 'bounded regular file: ' + file); const data = fs.readFileSync(file); assert.equal(data.length, s.size); return data; }
async function hashFile(file) { let length = 0; const h = createHash('sha256'); for await (const chunk of fs.createReadStream(file)) { length += chunk.length; h.update(chunk); } return {bytes: length, sha256: h.digest('hex')}; }
function pairedArgs() {
  const names = ['binary', 'binary-sha256', 'suite-proof', 'suite-sha256', 'build-proof', 'build-sha256', 'compute-release', 'release-sha256', 'out', 'manifest-sha'];
  const a = {};
  for (let i = 2; i < process.argv.length; i += 2) { const k = process.argv[i]?.slice(2); assert(process.argv[i]?.startsWith('--') && names.includes(k) && !Object.hasOwn(a, k) && process.argv[i + 1], 'exact unique CLI pairs only'); a[k] = process.argv[i + 1]; }
  for (const k of names) { assert(a[k], 'missing --' + k); if (k.endsWith('sha256') || k === 'manifest-sha') assert.match(a[k], hashPattern); else a[k] = path.resolve(a[k]); }
  return a;
}

// No writes, local module evaluation, SQLite import, fixture or process before
// coordinated release, own inputs and exact Binder bytes have been checked.
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed', 'Coordinated compute HOLD remains active');
assert(!process.env.NODE_OPTIONS && !process.env.NODE_PATH, 'No inherited Node preload or dependency redirect');
const a = pairedArgs(), SOURCE_SHA = a['manifest-sha'];assert.equal(SOURCE_SHA,'58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720');
assert(Number(process.versions.node.split('.')[0]) >= 24, 'Node24+ built-in SQLite required');
assert.equal(path.resolve(OWN), path.join(ROOT, 'source8-native-small-preparation-v1/physical'));
const selfSha = sha(regular(fileURLToPath(import.meta.url), 512 * 1024));
const releaseRaw = regular(a['compute-release'], 1024 * 1024); assert.equal(sha(releaseRaw), a['release-sha256']);
const release = JSON.parse(releaseRaw);
assert.equal(release.status, 'RELEASED'); assert.equal(release.kind, 'root_released_combined_finding_physical_v8'); assert.equal(release.scope, 'native_combined_finding_csv_json_physical_v8'); assert.equal(release.build_mode, 'optimized');
for (const [k, v] of Object.entries({source_manifest_sha256: SOURCE_SHA, harness_sha256: selfSha, bindings_sha256: BINDINGS_SHA, suite_proof_sha256: a['suite-sha256'], build_proof_sha256: a['build-sha256'], candidate_binary_sha256: a['binary-sha256']})) assert.equal(release[k], v, 'Root operative release pin: ' + k);
const notBefore = Date.parse(release.not_before), expires = Date.parse(release.expires_at);
assert(Number.isFinite(notBefore) && Number.isFinite(expires) && notBefore <= Date.now() && Date.now() < expires && expires - notBefore <= 4 * 3600_000, 'bounded active Root compute release');
assert.equal(path.resolve(release.output), a.out); assert.equal(path.dirname(a.out), path.join(OWN, 'runs')); assert.match(path.basename(a.out), /^[A-Za-z0-9][A-Za-z0-9_-]{0,95}$/); assert(!fs.existsSync(a.out), 'fresh own result directory');
for (const ancestor of [OWN, path.dirname(a.out)]) if (fs.existsSync(ancestor)) { const s = fs.lstatSync(ancestor); assert(s.isDirectory() && !s.isSymbolicLink()); }
const preparationPath = path.join(OWN, 'manifest.json'), preparationRaw = regular(preparationPath, 1024 * 1024); assert.equal(sha(preparationRaw), release.preparation_manifest_sha256); const preparation = JSON.parse(preparationRaw);
assert.equal(preparation.kind, 'physical_combined_rebind_v8'); const requiredFiles = ['native-finding-physical-combined-v8.mjs', 'combined-bindings.mjs'], seen = new Set(); assert.equal(preparation.files.length, requiredFiles.length);
for (const f of preparation.files) { assert(requiredFiles.includes(f.path) && !seen.has(f.path)); seen.add(f.path); const data = regular(path.join(OWN, f.path), 2 * 1024 * 1024); assert.equal(data.length, f.bytes); assert.equal(sha(data), f.sha256); }
const binderPath = path.join(OWN, 'combined-bindings.mjs'); assert.equal(sha(regular(binderPath, 512 * 1024)), BINDINGS_SHA);
const {bindCombined} = await import(pathToFileURL(binderPath).href); assert.equal(sha(regular(binderPath, 512 * 1024)), BINDINGS_SHA);
const binding = bindCombined(release, SOURCE_SHA), source = binding.manifest, COMMIT = source.source_commit, build = binding.build;
assert.equal(path.resolve(source.source_worktree), path.resolve(REPO)); assert.equal(a['suite-proof'], path.resolve(binding.suitePath)); assert.equal(a['suite-sha256'], sha(binding.suiteRaw)); assert.equal(a['build-proof'], path.join(binding.buildRoot, 'proof.json')); assert.equal(a['build-sha256'], sha(binding.buildRaw)); assert.equal(a.binary, binding.binary); assert.equal(a['binary-sha256'], binding.binarySHA);
const sourceUILineage = binding.sourceLineage, preparationsPublicationLineage = binding.publicationLineage, PUBLICATION_SHA = binding.publicationLineage.sha256;
async function verifySource() { assert.equal(sha(regular(a['compute-release'], 1024 * 1024)), a['release-sha256']); assert(Date.now() >= notBefore && Date.now() < expires); assert.equal(sha(regular(fileURLToPath(import.meta.url), 512 * 1024)), selfSha); assert.equal(sha(regular(preparationPath, 1024 * 1024)), sha(preparationRaw)); for (const f of preparation.files) { const data = regular(path.join(OWN, f.path), 2 * 1024 * 1024); assert.equal(data.length, f.bytes); assert.equal(sha(data), f.sha256); } binding.verify(); }
await verifySource(); assert.deepEqual(await hashFile(a.binary), {bytes: build.binary.bytes, sha256: a['binary-sha256']}); assert.deepEqual([...regular(a.binary, 512 * 1024 * 1024).subarray(0, 2)], [0x4d, 0x5a]);
const disk = fs.statfsSync(OWN); assert(disk.bavail * disk.bsize >= 8 * 1024 ** 3, '8GiB owned scratch required');
const {DatabaseSync} = await import('node:sqlite');
fs.mkdirSync(a.out, {recursive: true});
const runtimeRoot = path.join(a.out, 'runtime'); fs.mkdirSync(runtimeRoot);
function save(name, value) { assert(!name.includes('..') && !path.isAbsolute(name)); fs.writeFileSync(path.join(a.out, name), JSON.stringify(value, null, 2) + '\n', {flag: 'wx'}); }
save('inputs.json', {status: 'RUNNING_NOT_ACCEPTED', source_manifest_sha256: SOURCE_SHA, source_commit: COMMIT, source_publication_receipt_sha256: PUBLICATION_SHA, source_lineage: sourceUILineage, source_publication_lineage: preparationsPublicationLineage, bindings_sha256: BINDINGS_SHA, preparation_manifest_sha256: sha(preparationRaw), harness_sha256: selfSha, args: a, controlled_fact_qualification: 'Only real public Crawler baselines are genuine captures; OFFLINE robots_warning substitutions are synthetic test facts, not website observations or authentic native-warning writer proof.', limits: {csv: CSV_CAP, whole_app_json: JSON_CAP, warning_field: FIELD_CAP, csv_pages: 405, json_pages: 160}});

const appRuns = [], baselines = [], cases = [], failures = [], hits = [];
const siteSockets = new Set(), activeSite = new Set(); let app = null, site = null, siteClosed = false, origin = null;
function stillReleased() { assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed'); assert(Date.now() < expires, 'operative release expired'); assert.equal(sha(regular(a['compute-release'], 1024 * 1024)), a['release-sha256'], 'operative release changed/revoked'); }
async function request(route, {body, maximum = JSON_CAP, timeout = 180_000, saveTo = null} = {}) {
  assert(app?.origin && !app.run.closed); const url = new URL(route, app.origin); assert.equal(url.origin, app.origin); assert(![8787, 8790].includes(Number(url.port)));
  const payload = body === undefined ? null : utf8(JSON.stringify(body)); assert(!payload || payload.length <= 64 * 1024 * 1024);
  const response = await fetch(url, {redirect: 'error', signal: AbortSignal.timeout(timeout), ...(payload ? {method: 'POST', headers: {'content-type': 'application/json'}, body: payload} : {})});
  let length = 0; const h = createHash('sha256'), parts = [], fd = saveTo ? fs.openSync(saveTo, 'wx') : null;
  try { for await (const chunk of response.body) { length += chunk.byteLength; assert(length <= maximum, 'actual response exceeded observation sentinel'); h.update(chunk); if (fd !== null) fs.writeSync(fd, chunk); else parts.push(Buffer.from(chunk)); } } finally { if (fd !== null) fs.closeSync(fd); }
  const digest = h.digest('hex');
  if (saveTo) assert.deepEqual(await hashFile(saveTo), {bytes: length, sha256: digest}, 'saved body must equal all observed network bytes');
  return {status: response.status, headers: Object.fromEntries(response.headers), length, sha256: digest, normal_eof: true, data: saveTo ? null : Buffer.concat(parts)};
}
const decode = response => JSON.parse(response.data);
async function start(label, data, provenance) {
  assert(!app && childOf(runtimeRoot, data)); stillReleased(); assert.deepEqual(await hashFile(a.binary), {bytes: build.binary.bytes, sha256: a['binary-sha256']});
  if (!fs.existsSync(data)) fs.mkdirSync(data);
  const stdout = fs.openSync(path.join(a.out, label + '.stdout.log'), 'wx'), stderr = fs.openSync(path.join(a.out, label + '.stderr.log'), 'wx');
  const run = {label, data, provenance, pid: null, closed: false, forced: false, exit: null, startup_origin: null}; appRuns.push(run);
  let pending = '', logBytes = 0, fatal = null;
  const child = spawn(a.binary, ['serve', '--listen', '127.0.0.1:0', '--data-dir', data], {cwd: SOURCE, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env: {...process.env, TEMP: runtimeRoot, TMP: runtimeRoot}});
  run.pid = child.pid ?? null;
  const close = new Promise(resolve => child.once('close', (code, signal) => { run.closed = true; run.exit = {code, signal}; fs.closeSync(stdout); fs.closeSync(stderr); resolve(); }));
  app = {child, run, close, data, origin: null}; child.once('error', error => { fatal = error; });
  for (const [stream, fd, isOut] of [[child.stdout, stdout, true], [child.stderr, stderr, false]]) stream?.on('data', chunk => {
    logBytes += chunk.length; if (logBytes > 8 * 1024 * 1024) { fatal = Error('bounded child log exceeded'); child.kill('SIGTERM'); return; } fs.writeSync(fd, chunk);
    if (isOut) { pending += chunk.toString('utf8'); if (pending.length > 65_536) { fatal = Error('oversized startup line'); child.kill('SIGTERM'); return; } let end; while ((end = pending.indexOf('\n')) >= 0) { const line = pending.slice(0, end); pending = pending.slice(end + 1); const m = /^Open SEO Spider: (http:\/\/127\.0\.0\.1:([1-9][0-9]*))\r?$/.exec(line); if (m) { if (app?.origin || [8787, 8790].includes(+m[2])) { fatal = Error('invalid own app bind'); child.kill('SIGTERM'); return; } app.origin = m[1]; run.startup_origin = m[1]; } } }
  });
  const until = Date.now() + 180_000; while (!app.origin && Date.now() < until) { if (fatal) throw fatal; assert(!run.closed, 'own app exited during startup'); await delay(50); } assert(app.origin, 'actual owned ephemeral startup receipt'); assert.equal((await request('/healthz', {maximum: 1024 * 1024})).status, 200);
}
async function stop() { if (!app) return; const current = app; app = null; if (!current.run.closed) current.child.kill('SIGTERM'); const force = setTimeout(() => { current.run.forced = true; current.child.kill('SIGKILL'); }, 10_000); try { await within(current.close, 15_000, 'owned native child close missing'); } finally { clearTimeout(force); } assert(current.run.closed && !current.run.forced, 'actual owned child close required'); }
async function files(directory) { const out = []; async function walk(p, relative = '') { for (const name of fs.readdirSync(p).sort()) { const f = path.join(p, name), r = path.join(relative, name); const s = fs.lstatSync(f); assert(!s.isSymbolicLink()); if (s.isDirectory()) await walk(f, r); else { assert(s.isFile()); out.push({path: r.replaceAll('\\', '/'), ...await hashFile(f)}); } } } await walk(directory); return out; }
function quoteIdentifier(value) { assert(/^[A-Za-z_][A-Za-z0-9_]*$/.test(value)); return '"' + value + '"'; }
function fingerprint(db) {
  const h = createHash('sha256'); let rows = 0;
  for (const schema of db.prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name COLLATE BINARY").iterate()) {
    h.update(JSON.stringify(schema) + '\n');
    const pk = db.prepare('PRAGMA table_info(' + quoteIdentifier(schema.name) + ')').all().filter(c => c.pk).sort((a, b) => a.pk - b.pk).map(c => quoteIdentifier(c.name));
    const where = schema.name === 'metadata' ? " WHERE key NOT IN ('report','checkpoint')" : '';
    const order = pk.length ? pk.join(',') : 'rowid';
    for (const row of db.prepare('SELECT * FROM ' + quoteIdentifier(schema.name) + where + ' ORDER BY ' + order).iterate()) { const s = JSON.stringify(row); h.update(String(bytes(s)) + ':' + s + '\n'); rows++; }
  }
  return {rows, sha256: h.digest('hex'), exclusions: ['metadata.report', 'metadata.checkpoint']};
}
function closedBaseline(database, count) {
  const db = new DatabaseSync(database, {readOnly: true}); try {
    const get = key => db.prepare('SELECT value FROM metadata WHERE key=?').get(key).value;
    const report = get('report'), checkpoint = get('checkpoint'), config = get('config');
    assert.equal(JSON.parse(checkpoint).report_sha256, sha(report)); assert.equal(JSON.parse(checkpoint).config_sha256, sha(config)); assert.equal(db.prepare('SELECT count(*) n FROM pages').get().n, count);
    const sourceSummary = JSON.parse(get('sitemap_source_store')); assert.equal(sourceSummary.record_count, 1); assert.equal(sourceSummary.commit_count, 1); assert.equal(sourceSummary.omission, null); assert.equal(sourceSummary.legacy_history_unknown, false);
    assert.equal((report.match(/"robots_warning":null/g) ?? []).length, count, 'only real baseline null warnings are eligible');
    assert.equal(db.prepare("SELECT json_array_length(value,'$.pages') n FROM metadata WHERE key='report'").get().n, count);
    return {count, report_sha256: sha(report), report_bytes: bytes(report), checkpoint_sha256: sha(checkpoint), config_sha256: sha(config), generation: JSON.parse(checkpoint).generation, untouched: fingerprint(db), source_summary: sourceSummary};
  } finally { db.close(); }
}
function warning(p, ordinal, count) { return '"'.repeat(p.base_quotes + (ordinal === count - 1 ? p.last_quotes : 0)) + 'a'.repeat(p.base_ascii + (ordinal === count - 1 ? p.last_ascii : 0)); }
function mutateClosedFork(database, baseline, p) {
  assert(!app, 'app MUST be actually closed before fixture writes');
  for (const n of Object.values(p)) assert(Number.isInteger(n) && n >= 0 && n <= FIELD_CAP);
  assert(bytes(warning(p, baseline.count - 1, baseline.count)) <= FIELD_CAP);
  const db = new DatabaseSync(database); try {
    db.exec('BEGIN IMMEDIATE');
    const get = k => db.prepare('SELECT value FROM metadata WHERE key=?').get(k).value;
    const original = get('report'), checkpoint = get('checkpoint'); assert.equal(sha(original), baseline.report_sha256); assert.equal(sha(checkpoint), baseline.checkpoint_sha256); assert.deepEqual(fingerprint(db), baseline.untouched);
    let ordinal = 0;
    // Byte-fragment-only replacement retains every other original serialization
    // byte, including floats/order/unknown optional fields. No full JSON roundtrip.
    const report = original.replace(/"robots_warning":null/g, () => '"robots_warning":' + JSON.stringify(warning(p, ordinal++, baseline.count)));
    assert.equal(ordinal, baseline.count); assert(bytes(report) <= 256 * 1024 * 1024);
    let inverseCells = 0;
    const inverse = report.replace(/"robots_warning":"(?:[^"\\]|\\.)*"/g, () => { inverseCells++; return '"robots_warning":null'; });
    assert.equal(inverseCells, baseline.count); assert.equal(inverse, original, 'all non-warning report bytes exactly preserved');
    const oldCell = '"report_sha256":"' + baseline.report_sha256 + '"', newCell = '"report_sha256":"' + sha(report) + '"';
    assert.equal(checkpoint.split(oldCell).length - 1, 1, 'actual canonical checkpoint cell required');
    const updatedCheckpoint = checkpoint.replace(oldCell, newCell); assert.equal(updatedCheckpoint.replace(newCell, oldCell), checkpoint);
    db.prepare("UPDATE metadata SET value=? WHERE key='report'").run(report); db.prepare("UPDATE metadata SET value=? WHERE key='checkpoint'").run(updatedCheckpoint);
    assert.equal(db.prepare("SELECT json_valid(value) n FROM metadata WHERE key='report'").get().n, 1);
    assert.deepEqual(fingerprint(db), baseline.untouched, 'raw/A/B/source/config/gen/all other SQL rows unchanged');
    assert.equal(sha(get('config')), baseline.config_sha256); assert.equal(JSON.parse(get('checkpoint')).generation, baseline.generation);
    db.exec('COMMIT');
    return {qualification: 'SYNTHETIC_OFFLINE_WARNING_FACTS_NOT_WEBSITE_OBSERVATIONS', parameters: p, original_report_sha256: baseline.report_sha256, report_sha256: sha(report), report_bytes: bytes(report), original_checkpoint_sha256: baseline.checkpoint_sha256, checkpoint_sha256: sha(updatedCheckpoint), changed_sql_rows: ['metadata.report', 'metadata.checkpoint'], changed_warning_values: ordinal, changed_warning_ordinals: {first: 0, last: ordinal - 1, count: ordinal}, warning_fragment_inverse_exact: true, checkpoint_fragment_inverse_exact: true, unchanged_sql_fingerprint: baseline.untouched, warning_field_bytes_max: bytes(warning(p, baseline.count - 1, baseline.count)), raw_source_config_generation_retained: true};
  } catch (error) { try { db.exec('ROLLBACK'); } catch {} throw error; } finally { db.close(); }
}
async function createBaseline(family, count) {
  const data = path.join(runtimeRoot, 'actual-baseline-' + family); await start('baseline-' + family, data, 'REAL_PUBLIC_CRAWLER_BASELINE');
  let id;
  try {
    const urls = Array.from({length: count}, (_, i) => origin + '/' + family + '/p' + String(i).padStart(3, '0'));
    const input = {seed_urls: urls, sitemap_urls: [origin + '/' + family + '/task.xml'], crawl_links: false, discover_sitemaps: false, obey_robots: false, renderer: null, concurrency: 1, delay_ms: 0, max_pages: count + 1, request_timeout_secs: 10, max_body_bytes: 8 * 1024 * 1024};
    const created = await request('/api/crawls', {body: input, maximum: 1024 * 1024}); assert.equal(created.status, 200); id = decode(created).id; assert.match(id, /^[a-f0-9-]{36}$/);
    let state; const until = Date.now() + 900_000;
    while (Date.now() < until) { await delay(500); const r = await request('/api/crawls/' + id, {maximum: 4 * 1024 * 1024}); assert.equal(r.status, 200); state = decode(r); if (state.status === 'failed') throw Error('real baseline crawl failed: ' + state.error); if (state.status === 'completed') break; }
    assert.equal(state?.status, 'completed'); assert.equal(state.stats.fetched, count);
    const page = await request('/api/crawls/' + id + '/sitemaps/findings', {body: {...SELECTION, limit: 1, candidate_limit: 5000}}); assert.equal(page.status, 200); const value = decode(page); assert.equal(value.engine, ENGINE); assert.equal(value.findings.context.availability, 'available'); assert.equal(value.findings.rows[0].assessment.verdict, 'match'); assert(value.export_token);
  } finally { await stop(); }
  const db = path.join(data, id + '.sqlite'), proof = closedBaseline(db, count);
  const baseline = {family, count, id, data, database: db, ...proof};
  const actualHits = hits.filter(h => h.path.startsWith('/' + family + '/'));
  assert.equal(actualHits.length, count + 1); assert.equal(actualHits.filter(h => h.path.endsWith('/task.xml')).length, 1); assert(actualHits.every(h => h.finished));
  baselines.push(baseline); save('baseline-' + family + '.json', {...baseline, files: await files(data), provenance: 'Actual fresh same-candidate public Crawler and terminal checkpoint before any synthetic fork; no historical proof reused'}); return baseline;
}

async function collectNativeRows(baseline, p, caseDir) {
  const file = path.join(caseDir, 'actual-native-rows.ndjson'), fd = fs.openSync(file, 'wx'); let cursor = null, first = null, count = 0, requests = 0; const tokens = new Set();
  try { do {
    const r = await request('/api/crawls/' + baseline.id + '/sitemaps/findings', {body: {...SELECTION, limit: 10, candidate_limit: 5000, ...(cursor ? {cursor} : {})}}); assert.equal(r.status, 200);
    const page = decode(r); assert.equal(page.website_requests, 0); assert.equal(page.engine, ENGINE); assert.deepEqual(utf8(JSON.stringify(page)), r.data, 'actual ASCII native serialization must match oracle JSON profile');
    assert.equal(page.access_target.job_id, baseline.id); assert.equal(page.access_target.generation, baseline.generation); assert.equal(page.access_target.config_sha256, baseline.config_sha256);
    assert.equal(page.findings.context.availability, 'available'); assert.equal(page.findings.context.basis.analysis_context.available, true); assert.equal(page.findings.context.basis.analysis_context.saved_page_count, baseline.count);
    assert.equal(page.findings.context.basis.sitemap_source_measurement_summary.record_count, 1); assert.equal(page.findings.context.basis.sitemap_source_measurement_summary.commit_count, 1);
    if (!first) first = page; else { assert.deepEqual(page.findings.context, first.findings.context); assert.equal(page.export_token, first.export_token); }
    assert(Array.isArray(page.findings.rows));
    for (const row of page.findings.rows) { assert.equal(row.position.kind, 'saved_page'); assert.equal(row.position.ordinal, count); const candidate = row.evidence.page; assert.equal(row.evidence.domain, 'saved_page'); assert.equal(candidate.finding.observation.report_page_ordinal, count); assert.equal(candidate.finding.membership, 'listed_original'); assert.equal(candidate.finding.original_membership_occurrences, 1); assert.equal(candidate.finding.original_source_document_count, 1); assert.equal(candidate.robots_warning_field_observed, true); assert.equal(candidate.robots_warning, warning(p, count, baseline.count)); assert.equal(row.assessment.verdict, 'match'); assert.deepEqual(row.assessment.reasons, ['listed_original']); fs.writeSync(fd, JSON.stringify(row) + '\n'); count++; }
    cursor = page.next_cursor; if (cursor) { assert(!tokens.has(cursor)); tokens.add(cursor); assert.equal(page.findings.scan.exhausted, false); } else { assert.equal(page.findings.scan.exhausted, true); assert.equal(page.findings.scan.stop, 'exhausted'); }
    requests++; assert(requests <= baseline.count + 1);
  } while (cursor); } finally { fs.closeSync(fd); }
  assert.equal(count, baseline.count); return {first, file, count, requests, rows_file: await hashFile(file)};
}
async function visitRows(file, visit) { const stream = fs.createReadStream(file), lines = readline.createInterface({input: stream, crlfDelay: Infinity}); try { for await (const line of lines) { assert(bytes(line) <= 2 * 1024 * 1024); await visit(JSON.parse(line), line); } } finally { lines.close(); stream.destroy(); } }
async function csvOracle(native) {
  const h = createHash('sha256'); let length = 0, count = 0; const feed = values => { for (const [i, v] of values.entries()) { if (i) { h.update(','); length++; } const cell = '"' + v.replaceAll('"', '""') + '"'; h.update(cell); length += bytes(cell); } h.update('\r\n'); length += 2; };
  feed(['record_type', 'schema_version', 'domain', 'filter', 'assessment', 'original_identity', 'position_json', 'reasons_json', 'typed_finding_json', 'context_json']);
  feed(['context', '1', 'saved_page', SELECTION.filter, SELECTION.assessment, '', '', '', '', JSON.stringify({engine: ENGINE, selection: SELECTION, context: native.first.findings.context})]);
  await visitRows(native.file, (row, canonical) => { const identity = row.evidence.page.finding.observation.requested_url; assert(identity.startsWith(origin + '/'), 'canonical HTTP identities do not need formula prefix in this fixture'); feed(['finding', '1', 'saved_page', SELECTION.filter, row.assessment.verdict, identity, JSON.stringify(row.position), JSON.stringify(row.assessment.reasons), canonical, '']); count++; });
  assert.equal(count, native.count); return {length, sha256: h.digest('hex'), finding_rows: count, context_rows: 1, header_rows: 1, columns: 10, independence: 'Independent uncapped CSV quoting/count/SHA over actual bounded native Finding DTOs and actual metadata context, rowwise; predicate truth and native-warning-writer provenance are not invented'};
}
function templateSegments(value) { const text = JSON.stringify(value), needle = '"rows":[],"scan":', start = text.indexOf(needle); assert(start >= 0 && text.indexOf(needle, start + 1) < 0); const bracket = start + '"rows":'.length; return [text.slice(0, bracket + 1), text.slice(bracket + 1)]; }
async function wholeJsonOracle(native) {
  assert(native.count <= 200, 'whole-native page limit unchanged');
  const envelope = JSON.parse(JSON.stringify(native.first)); envelope.findings.rows = []; envelope.findings.scan = {evaluated_candidates: native.count, exhausted: true, stop: 'exhausted', more_matches: false}; envelope.findings.continuation = null; envelope.next_cursor = null;
  const [prefix, suffix] = templateSegments(envelope), [corePrefix, coreSuffix] = templateSegments(envelope.findings);
  const h = createHash('sha256'), core = createHash('sha256'); let length = bytes(prefix) + bytes(suffix), coreBytes = bytes(corePrefix) + bytes(coreSuffix), count = 0; h.update(prefix); core.update(corePrefix);
  await visitRows(native.file, (_row, canonical) => { if (count) { h.update(','); core.update(','); length++; coreBytes++; } h.update(canonical); core.update(canonical); length += bytes(canonical); coreBytes += bytes(canonical); count++; }); h.update(suffix); core.update(coreSuffix);
  assert.equal(count, native.count); assert(coreBytes < JSON_CAP, 'separate Core page must remain below8MiB; this gate targets whole App wrapper');
  return {length, sha256: h.digest('hex'), core_page_bytes: coreBytes, core_page_sha256: core.digest('hex'), wrapper_bytes: length - coreBytes, rows: count, next_cursor: null, export_token_bytes: bytes(envelope.export_token), complete_envelope_including_tokens: true, oracle: 'Whole actual App serialization shape plus actual native context/token/rows, all stored candidates exhausted, streamed independently; exact native200 is required to validate construction'};
}
async function physicalCase(kind, label, baseline, p, target = null) {
  stillReleased(); assert(!app); assert.match(label, /^[a-z0-9-]+$/); const caseDir = path.join(a.out, label); assert(childOf(a.out, caseDir)); fs.mkdirSync(caseDir); const data = path.join(runtimeRoot, label); assert(childOf(runtimeRoot, data) && !fs.existsSync(data));
  // Source is a CLOSED current-run real baseline, never a user directory.
  const originalFiles = await files(baseline.data); fs.cpSync(baseline.data, data, {recursive: true, errorOnExist: true, force: false}); assert.deepEqual(await files(data), originalFiles);
  const mutation = mutateClosedFork(path.join(data, baseline.id + '.sqlite'), baseline, p); save(label + '.fixture.json', mutation); const beforeHits = hits.length;
  await start(label, data, 'CONTROLLED_OFFLINE_FORK_OF_REAL_BASELINE');
  try {
    const native = await collectNativeRows(baseline, p, caseDir), expected = kind === 'csv' ? await csvOracle(native) : await wholeJsonOracle(native);
    if (target !== null) assert.equal(expected.length, target, 'actual fresh oracle must hit physical size exactly; never hide calibration drift');
    const cap = kind === 'csv' ? CSV_CAP : JSON_CAP; assert(expected.length <= cap + 1); const primedFiles = await files(data), route = '/api/crawls/' + baseline.id + '/sitemaps/findings';
    const physical = await request(kind === 'csv' ? route + '/export.csv' : route, {body: kind === 'csv' ? {...SELECTION, snapshot: native.first.export_token} : {...SELECTION, limit: 200, candidate_limit: 5000}, maximum: cap + 1, timeout: 450_000, saveTo: path.join(caseDir, 'wire-body.bin')});
    if (expected.length <= cap) {
      assert.equal(physical.status, 200); assert.equal(physical.headers['content-length'], String(expected.length)); assert.equal(physical.length, expected.length); assert.equal(physical.sha256, expected.sha256); assert.equal(physical.normal_eof, true);
      if (kind === 'csv') { assert.match(physical.headers['content-type'], /^text\/csv/); assert.equal(physical.headers['x-sitemap-csv-sha256'], expected.sha256); assert.equal(physical.headers['x-sitemap-findings-context-sha256'], native.first.findings.context.context_sha256); assert.equal(physical.headers['x-sitemap-findings-selection-sha256'], native.first.findings.context.selection_sha256); assert.equal(physical.headers['content-disposition'], 'attachment; filename="' + baseline.id + '-sitemap-findings.csv"'); }
      else assert.match(physical.headers['content-type'], /^application\/json/);
    } else {
      assert.equal(expected.length, cap + 1); assert.equal(physical.status, 413); assert(physical.length <= 1024 * 1024);
      for (const header of ['content-disposition', 'x-sitemap-csv-sha256', 'x-sitemap-findings-context-sha256']) assert.equal(physical.headers[header], undefined);
      const error = JSON.parse(regular(path.join(caseDir, 'wire-body.bin'), 1024 * 1024)); assert.equal(error.code, kind === 'csv' ? 'sitemap_findings_csv_byte_budget' : 'sitemap_findings_response_byte_budget'); assert.equal(error.website_requests, 0); assert(!Object.hasOwn(error, 'rows') && !Object.hasOwn(error, 'export_token'));
    }
    const recovery = await request(route, {body: {...SELECTION, limit: 10, candidate_limit: 5000}}); assert.equal(recovery.status, 200); assert.deepEqual(decode(recovery), native.first); assert.equal(hits.length, beforeHits, 'zero website requests during startup/read/export/recovery'); assert.deepEqual(await files(data), primedFiles, 'strict primed read interval leaves saved bytes unchanged');
    const result = {kind, label, target, expected, physical, mutation, native_rows: {count: native.count, requests: native.requests, ...native.rows_file}, native_context_sha256: native.first.findings.context.context_sha256, zero_website_requests: true, saved_bytes_unchanged_in_primed_read_interval: true, public_baseline_id: baseline.id, controlled_offline_test_facts: true}; delete result.physical.data;
    cases.push(result); save(label + '.case.json', result); return result;
  } finally { await stop(); }
}
async function calibrate(kind, baseline) {
  const base = kind === 'csv' ? {base_quotes: 52_000, base_ascii: 0, last_quotes: 0, last_ascii: 0} : {base_quotes: 0, base_ascii: 45_000, last_quotes: 0, last_ascii: 0};
  const first = await physicalCase(kind, kind + '-01-base', baseline, base);
  const quote = await physicalCase(kind, kind + '-02-quote-probe', baseline, {...base, last_quotes: 1});
  const ascii = await physicalCase(kind, kind + '-03-ascii-probe', baseline, {...base, last_ascii: 1});
  const dq = quote.expected.length - first.expected.length, da = ascii.expected.length - first.expected.length, cap = kind === 'csv' ? CSV_CAP : JSON_CAP;
  // Newly observed per-format derivatives only. Old RawCSV5/+2 is never used.
  assert.equal(dq, kind === 'csv' ? 3 : 2, 'actual current quote escaping derivative'); assert.equal(da, 1, 'actual current ASCII derivative');
  const field = kind === 'csv' ? 'base_quotes' : 'base_ascii', step = kind === 'csv' ? dq : da;
  const coarseP = {...base, [field]: base[field] + Math.floor((cap - first.expected.length) / (baseline.count * step))}; assert(coarseP[field] >= base[field]);
  const coarse = await physicalCase(kind, kind + '-04-coarse', baseline, coarseP);
  const remainder = cap - coarse.expected.length; assert(Number.isInteger(remainder) && remainder >= 0 && remainder < baseline.count * step + 128, 'newly measured coarse step; unexpected representation change stops');
  const exactP = {...coarseP, last_ascii: remainder};
  const exact = await physicalCase(kind, kind + '-05-exact', baseline, exactP, cap);
  const plus = await physicalCase(kind, kind + '-06-plus-one', baseline, {...exactP, last_ascii: exactP.last_ascii + 1}, cap + 1);
  save(kind + '-calibration.json', {quote_derivative_newly_observed: dq, ascii_derivative_newly_observed: da, baseline_length: first.expected.length, coarse_length: coarse.expected.length, exact_parameters: exactP, exact_length: exact.expected.length, rejected_plus_one_length: plus.expected.length, native_bodies_measured: true, old_raw_coefficients_reused: false});
}

try {
  site = http.createServer((req, res) => {
    const hit = {path: req.url, method: req.method, finished: false}; hits.push(hit); activeSite.add(hit); req.resume(); req.on('error', () => activeSite.delete(hit)); res.on('error', () => activeSite.delete(hit)); res.once('finish', () => { hit.finished = true; activeSite.delete(hit); }); res.once('close', () => activeSite.delete(hit));
    const match = /^\/(csv|json)\/(task\.xml|p\d{3})$/.exec(req.url ?? '');
    if (!match || req.method !== 'GET' || hits.length > 580) { res.writeHead(404); res.end('unexpected owned fixture request'); return; }
    const count = match[1] === 'csv' ? 405 : 160;
    const body = match[2] === 'task.xml' ? '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">' + Array.from({length: count}, (_, i) => '<url><loc>' + origin + '/' + match[1] + '/p' + String(i).padStart(3, '0') + '</loc></url>').join('') + '</urlset>' : '<html><head><title>Actual current finding boundary fixture</title></head><body>genuine small saved page</body></html>';
    res.writeHead(200, {'content-type': match[2] === 'task.xml' ? 'application/xml' : 'text/html', 'content-length': bytes(body)}); res.end(body);
  });
  site.on('connection', socket => { siteSockets.add(socket); socket.on('error', () => {}); socket.once('close', () => siteSockets.delete(socket)); });
  await new Promise((resolve, reject) => { site.once('error', reject); site.listen(0, '127.0.0.1', resolve); }); const port = site.address().port; assert(![8787, 8790].includes(port)); origin = 'http://127.0.0.1:' + port;
  const csvBaseline = await createBaseline('csv', 405); await calibrate('csv', csvBaseline);
  const jsonBaseline = await createBaseline('json', 160); await calibrate('json', jsonBaseline);
  assert.equal(hits.length, 567, '405+160 actual page requests plus2 actual sitemap requests only');
} catch (error) { failures.push({phase: 'actual-native-fixture/oracle/transport', message: error.message, stack: error.stack}); process.exitCode = 1; }
finally {
  try { await stop(); } catch (error) { failures.push({phase: 'owned-native-close', message: error.message}); process.exitCode = 1; }
  if (site) { try { site.closeIdleConnections(); const close = new Promise(resolve => site.close(() => { siteClosed = true; resolve(); })); const end = Date.now() + 10_000; while ((activeSite.size || siteSockets.size) && Date.now() < end) { site.closeIdleConnections(); await delay(25); } if (activeSite.size || siteSockets.size) { site.closeAllConnections(); throw Error('fixture forced-close required'); } await within(close, 5000, 'fixture close callback missing'); assert(hits.every(h => h.finished)); } catch (error) { failures.push({phase: 'owned-fixture-drain', message: error.message}); process.exitCode = 1; } }
  try { await verifySource(); assert.deepEqual(await hashFile(a.binary), {bytes: build.binary.bytes, sha256: a['binary-sha256']}); for (const b of baselines) assert.deepEqual(closedBaseline(b.database, b.count), {count: b.count, report_sha256: b.report_sha256, report_bytes: b.report_bytes, checkpoint_sha256: b.checkpoint_sha256, config_sha256: b.config_sha256, generation: b.generation, untouched: b.untouched, source_summary: b.source_summary}); } catch (error) { failures.push({phase: 'final-input-and-baseline-preservation', message: error.message}); process.exitCode = 1; }
  const pass = failures.length === 0 && baselines.length === 2 && cases.length === 12 && appRuns.every(r => r.closed && !r.forced) && siteClosed;
  save('proof.json', {schema_version: 1, result: pass ? 'PASS_COMBINED_V8_NATIVE_CONTROLLED_FINDING_CSV64M_APPJSON8M_EXACT_PLUS1' : 'FAIL_COMBINED_V8_NATIVE_FINDING_PHYSICAL_GATE', source_manifest_sha256: SOURCE_SHA, source_commit: COMMIT, source_publication_receipt_sha256: PUBLICATION_SHA, source_lineage: sourceUILineage, source_publication_lineage: preparationsPublicationLineage, bindings_sha256: BINDINGS_SHA, preparation_manifest_sha256: sha(preparationRaw), harness_sha256: selfSha, binary_sha256: a['binary-sha256'], suite_proof_sha256: a['suite-sha256'], build_proof_sha256: a['build-sha256'], compute_release_sha256: a['release-sha256'], baselines, cases: cases.map(c => ({kind: c.kind, label: c.label, expected_bytes: c.expected.length, actual_status: c.physical.status, actual_body_bytes: c.physical.length, actual_body_sha256: c.physical.sha256, normal_eof: c.physical.normal_eof})), app_runs: appRuns, actual_website_requests: hits, owned_children_closed: appRuns.every(r => r.closed && !r.forced), owned_fixture_drained: siteClosed && !activeSite.size && !siteSockets.size, failures, qualification: 'Combined-v8 selected candidate only; frozen current private source approval is publication provenance, never borrowed V3/current suite/build/native acceptance or a live Git HEAD requirement. Physical native transport over genuine same-candidate public Crawler baselines with EXPLICIT SYNTHETIC OFFLINE WARNING FACTS only. Complete HTTP body length/SHA/normal EOF and atomic413 require actual observations. No fake Core session/header/proof, native-warning-writer provenance, historical proof transfer, test-count increase, internal worker Join/TX instrumentation, late-fault/cancel timing, global RSS, rawCSV regression, source50k/50MiB matrix, UI/SDK/platform or manufacturer parity is claimed. Source-bound existing internal barrier tests and separate native gates remain required.'}); if (!pass) process.exitCode = 1;
}
