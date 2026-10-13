// Optional complete typed original reports: owned native/API/CLI/official-SDK evidence.
import assert from 'node:assert/strict';
import http from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp, readFile, writeFile, mkdir, rm, readdir, realpath, stat} from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {verifyBinarySha, assertFinalWitnessState} from './saved-comparison-mapping-oracles.mjs';
import {sha, parseLossless, at, scalar, compactJson, replaceToken, assertLosslessEqual, unsafeIntegerLexemes, assertOriginalBundle, assertDefaultEnvelope, assertComparableResponse, assertBundleCsv} from './saved-comparison-bundle-oracles.mjs';

const sourcePath = fileURLToPath(import.meta.url), root = path.resolve(process.env.SPIDER_ROOT || path.join(path.dirname(sourcePath), '..'));
assert.ok(process.env.SPIDER_BINARY, 'Explicit SPIDER_BINARY is required');
const binary = path.resolve(process.env.SPIDER_BINARY); assert.ok((await stat(binary)).isFile());
const binarySha = sha(await readFile(binary)), sourceSha = sha(await readFile(sourcePath));
const oracleSha = sha(await readFile(fileURLToPath(new URL('./saved-comparison-bundle-oracles.mjs', import.meta.url))));
const testsSha = sha(await readFile(fileURLToPath(new URL('./saved-comparison-bundle-oracles.test.mjs', import.meta.url))));
const lifecycleOracleSha = sha(await readFile(fileURLToPath(new URL('./saved-comparison-mapping-oracles.mjs', import.meta.url))));
const controlsPath = path.resolve(process.env.SPIDER_BUNDLE_CONTROL_PROOF || path.join(root, 'docs/verification/saved-comparison-original-bundle-oracle-controls.json'));
const controlsBytes = await readFile(controlsPath), controls = JSON.parse(controlsBytes.toString('utf8'));
assert.equal(controls.result, 'PASS'); assert.equal(controls.pure_tests, 28);
assert.equal(controls.helper_sha256, oracleSha); assert.equal(controls.tests_sha256, testsSha);
assert.equal(controls.native_executable_launched, false); assert.equal(controls.app_or_child_or_http_server_launched, false);
const proofPath = path.resolve(process.env.SPIDER_BUNDLE_PROOF || path.join(root, 'docs/verification/saved-comparison-original-bundle-e2e.json'));
const requireSdk = createRequire(path.join(root, 'scripts/package.json'));
const {Client, StreamableHTTPClientTransport} = await import(pathToFileURL(requireSdk.resolve('@modelcontextprotocol/client')).href);
const {StdioClientTransport} = await import(pathToFileURL(requireSdk.resolve('@modelcontextprotocol/client/stdio')).href);
const sdkPackage = JSON.parse(await readFile(path.join(root, 'scripts/node_modules/@modelcontextprotocol/client/package.json'), 'utf8'));
assert.equal(sdkPackage.version, '2.3.1', 'Observed SDK child ownership adapter is bound to SDK2.3.1');
const data = await mkdtemp(path.join(os.tmpdir(), 'open-seo-spider-original-bundle-')), ownedData = await realpath(data);
const fixtures = [], witnesses = [], liveChildren = new Set(), closures = new WeakMap(), jobs = new Map(), checks = [], hashChecks = [], sdkChecks = [];
let app, api, output = '', launchError, starts = 0, acceptedProof, baselineHits;
const UUID = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i;
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const stringAt = (doc, route) => { const node = at(doc, route); assert.equal(node.type, 'string'); return node.value; };
const safeInt = (doc, route) => { const node = at(doc, route); assert.equal(node.type, 'number'); assert.match(node.lexeme, /^\d+$/); const value = BigInt(node.lexeme); assert.ok(value <= BigInt(Number.MAX_SAFE_INTEGER)); return Number(value); };
const hits = () => fixtures.reduce((sum, owned) => sum + owned.hits.length, 0);
async function binaryCheck(phase) { hashChecks.push({phase, sha256: await verifyBinarySha(binary, binarySha)}); }
function track(child) { liveChildren.add(child); closures.set(child, new Promise(resolve => child.once('close', () => { liveChildren.delete(child); resolve(); }))); return child; }
async function waitClosed(child) {
  if (!child || !liveChildren.has(child)) return;
  const closed = closures.get(child);
  if (child.exitCode === null && child.signalCode === null) child.kill();
  if (!await Promise.race([closed.then(() => true), delay(5000).then(() => false)])) {
    child.kill('SIGKILL'); assert.ok(await Promise.race([closed.then(() => true), delay(5000).then(() => false)]), 'Owned child did not emit close');
  }
}
async function stopApp() { const owned = app; await waitClosed(owned); if (app === owned) app = undefined; }
function alive() { if (launchError || !app || app.exitCode !== null || app.signalCode !== null) throw launchError || Error('Owned bundle App exited: ' + output.slice(-4000)); }
async function until(fn, label) {
  const end = Date.now() + 45_000;
  while (Date.now() < end) { alive(); const value = await fn(); if (value) return value; await delay(25); }
  throw Error('Timeout ' + label + '\n' + output.slice(-4000));
}
async function launch() {
  await binaryCheck('before_app_spawn_' + (++starts)); output = ''; launchError = undefined;
  app = track(spawn(binary, ['serve', '--listen', '127.0.0.1:0', '--data-dir', data], {cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe']}));
  app.on('error', error => { launchError = error; });
  const append = chunk => { output = (output + chunk).slice(-1024 * 1024); };
  app.stdout.on('data', append); app.stderr.on('data', append);
  api = await until(() => output.match(/(?:^|\n)Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/)?.[1], 'complete owned port0 bind');
  await until(async () => { try { return (await fetch(api + '/healthz', {signal: AbortSignal.timeout(2000)})).ok; } catch { return false; } }, 'owned health');
}
async function request(route, body) {
  alive(); const response = await fetch(api + route, body === undefined ? {signal: AbortSignal.timeout(15_000)} : {method: 'POST', headers: {'content-type': 'application/json'}, body: Buffer.isBuffer(body) ? body : JSON.stringify(body), signal: AbortSignal.timeout(15_000)});
  const bytes = Buffer.from(await response.arrayBuffer()); alive();
  return {status: response.status, bytes, document: parseLossless(bytes)};
}
async function call(route, body) { const response = await request(route, body); assert.equal(response.status, 200, response.bytes.toString('utf8').slice(0, 2000)); return response; }
async function listen(server, port = 0) {
  await new Promise((resolve, reject) => { const failed = error => { server.off('listening', ready); reject(error); }; const ready = () => { server.off('error', failed); resolve(); }; server.once('error', failed); server.once('listening', ready); server.listen(port, '127.0.0.1'); });
  assert.ok(server.address() && typeof server.address() === 'object'); return 'http://127.0.0.1:' + server.address().port;
}
async function closeServer(server, witness = false) {
  if (!server.listening) return;
  const closed = new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  if (!witness) server.closeAllConnections();
  if (!await Promise.race([closed.then(() => true), delay(5000).then(() => false)])) { server.closeAllConnections(); await closed; throw Error('Owned server did not drain'); }
}
async function fixture(label, prefix) {
  const owned = {label, prefix, revision: 0, hits: [], hold: new Set(), held: new Map(), origin: undefined, server: undefined};
  owned.server = http.createServer((req, res) => {
    owned.hits.push({method: req.method, path: req.url});
    const send = () => {
      if (req.url === prefix + '/decimal.svg') {
        res.setHeader('content-type', 'image/svg+xml');
        res.end('<svg xmlns="http://www.w3.org/2000/svg" width="51.248178375505404" height="2"></svg>'); return;
      }
      const target = owned.origin + prefix + '/target#original';
      res.setHeader('content-type', 'text/html; charset=utf-8');
      // Literal u64 survives real HTML -> Rust JSON-LD -> typed report -> bundle.
      const ld = `{"@type":"Thing","counter":9007199254740993,"url":${JSON.stringify(owned.origin + req.url)},"revision":${owned.revision}}`;
      const image = req.url === prefix + '/resource-page' ? `<img src="${prefix}/decimal.svg" alt="Actual decimal-width SVG">` : '';
      res.end(`<html lang="en"><head><title>${label} ${req.url} ${owned.revision}</title><meta name="description" content="${label} ${req.url} ${owned.revision}"><script type="application/ld+json">${ld}</script></head><body><main><h1>${label} ${req.url} ${owned.revision}</h1>Original raw selected text from ${owned.origin}${req.url}. <a href="${target}">First occurrence</a><a href="${target}">Second occurrence</a>${image}</main></body></html>`);
    };
    if (owned.hold.has(req.url)) owned.held.set(req.url, {send, res}); else send();
  });
  fixtures.push(owned); owned.origin = await listen(owned.server); return owned;
}
function release(owned, route) { owned.hold.delete(route); const pending = owned.held.get(route); if (pending) { owned.held.delete(route); pending.send(); } }
async function terminal(id, states = ['completed']) {
  return until(async () => { const response = await call('/api/crawls/' + id); const status = stringAt(response.document, ['status']); if (status === 'failed') throw Error(response.bytes.toString('utf8').slice(0, 2000)); return states.includes(status) && response; }, 'native terminal ' + id);
}
async function record(id, generation) {
  assert.match(id, UUID); const exported = await call('/api/crawls/' + id + '/project.json');
  const job = {id, generation, document: exported.document, bytes: exported.bytes, compactBytes: compactJson(exported.bytes)}; jobs.set(id, job); return job;
}
async function crawl(owned, suffixes, extra = {}) {
  const started = await call('/api/crawls', {seed_urls: suffixes.map(suffix => owned.origin + suffix), crawl_links: false, discover_sitemaps: false, obey_robots: false, delay_ms: 0, concurrency: 1, content_capture: {sources: ['raw_http_html', 'rendered_dom_html'], include_selectors: ['main']}, ...extra});
  const id = stringAt(started.document, ['id']), ended = await terminal(id, ['completed', 'limited']);
  const job = await record(id, safeInt(ended.document, ['generation'])); assert.ok(job.generation > 0);
  for (const page of at(job.document, ['pages']).elements) assert.ok(owned.hits.some(hit => owned.origin + hit.path === stringAt(page, ['url'])), 'Every native row requires an actual fixture request');
  return job;
}
function body(before, after, config = {}, include = true) { return {before: before.id, after: after.id, expected_before_generation: before.generation, expected_after_generation: after.generation, config, include_original_reports: include}; }
async function bundle(before, after, config = {}) {
  const prior = hits(), response = await call('/api/compare/saved', body(before, after, config));
  const bindings = assertOriginalBundle(response.document, before, after); assert.equal(hits(), prior); return {...response, bindings};
}
function error(response, status, code) { assert.equal(response.status, status); assert.equal(stringAt(response.document, ['error', 'code']), code); assert.equal(scalar(at(response.document, ['website_requests'])), '0'); assert.equal(response.document.root.properties.has('original_reports'), false); }
async function execute(args) {
  await binaryCheck('before_cli_spawn_' + (++starts)); let child;
  try {
    return await new Promise((resolve, reject) => {
      child = track(spawn(binary, args, {cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe']})); let stdout = '', stderr = '';
      const timeout = setTimeout(() => { child.kill(); reject(Error('Owned CLI timeout')); }, 30_000);
      child.on('error', failure => { clearTimeout(timeout); reject(failure); }); child.stdout.on('data', chunk => { stdout += chunk; }); child.stderr.on('data', chunk => { stderr += chunk; }); child.once('close', code => { clearTimeout(timeout); resolve({code, stdout, stderr}); });
    });
  } finally { await waitClosed(child); }
}
class OwnedSdkStdio extends StdioClientTransport {
  async start() {
    await binaryCheck('before_official_sdk_stdio_spawn_' + (++starts));
    const started = super.start();
    // SDK2.3.1 source was inspected: _process is its spawned ChildProcess handle.
    assert.ok(this._process && typeof this._process.kill === 'function', 'Observed official SDK child handle must exist');
    this.owned = track(this._process); await started;
  }
  async close() { await super.close(); await waitClosed(this.owned); }
}
function mcpText(result) { assert.equal(result.content.length, 1); assert.equal(result.content[0].type, 'text'); return parseLossless(result.content[0].text); }
async function sdk(kind, before, after, config, expected, phase) {
  const client = new Client({name: 'original-report-bundle-acceptance', version: '0.1.0'});
  const transport = kind === 'stdio' ? new OwnedSdkStdio({command: binary, args: ['mcp', '--server', api], cwd: root, stderr: 'pipe', maxBufferSize: 64 * 1024 * 1024}) : new StreamableHTTPClientTransport(new URL(api + '/mcp'));
  let stderrTail = '';
  if (transport.stderr) transport.stderr.on('data', chunk => { stderrTail = (stderrTail + chunk).slice(-16000); });
  try {
    alive(); await client.connect(transport); const inventory = (await client.listTools()).tools;
    assert.equal(inventory.length, 24); const tool = inventory.find(value => value.name === 'spider_compare_saved');
    assert.equal(tool.inputSchema.properties.include_original_reports.type, 'boolean'); assert.equal(tool.annotations.readOnlyHint, true); assert.equal(tool.annotations.openWorldHint, false);
    const saved = await client.callTool({name: 'spider_compare_saved', arguments: body(before, after, config)});
    assert.equal(saved.isError, false); const text = mcpText(saved);
    assertOriginalBundle(text, before, after, {directTypedSlices: false}); assertComparableResponse(text, expected.document);
    assert.equal(saved.structuredContent.website_requests, 0); assert.equal(saved.structuredContent.original_reports.schema_version, 1);
    for (const side of ['before', 'after']) {
      assert.equal(saved.structuredContent.original_reports[side].typed_report_sha256, stringAt(expected.document, ['comparison', side + '_snapshot', 'typed_report_sha256']));
      assert.equal(saved.structuredContent.original_reports[side].typed_report_json_bytes, safeInt(expected.document, ['comparison', side + '_snapshot', 'typed_report_json_bytes']));
    }
    const absent = body(before, after, config); delete absent.include_original_reports;
    const defaultResult = await client.callTool({name: 'spider_compare_saved', arguments: absent}); assert.equal(defaultResult.isError, false); assertDefaultEnvelope(mcpText(defaultResult)); assertComparableResponse(mcpText(defaultResult), expected.document);
    const disabled = await client.callTool({name: 'spider_compare_saved', arguments: body(before, after, config, false)}); assert.equal(disabled.isError, false); assertDefaultEnvelope(mcpText(disabled)); assertLosslessEqual(mcpText(disabled).root, mcpText(defaultResult).root);
    const conflict = await client.callTool({name: 'spider_compare_saved', arguments: {...body(before, after, config), expected_after_generation: after.generation + 1}});
    assert.equal(conflict.isError, true); assert.equal(conflict.structuredContent.error.code, 'generation_conflict'); assert.equal(conflict.structuredContent.website_requests, 0); assert.equal(conflict.structuredContent.context.observed_generation, after.generation);
    for (const invalid of [{...body(before, after), include_original_reports: null}, {...body(before, after), include_original_reports: 'yes'}]) {
      const refused = await client.callTool({name: 'spider_compare_saved', arguments: invalid}); assert.equal(refused.isError, true); assert.equal(refused.structuredContent.error.code, 'invalid_comparison_request'); assert.equal(refused.structuredContent.website_requests, 0);
    }
    await assert.rejects(client.callTool({name: 'spider_compare_saved', arguments: {...body(before, after), unexpected: true}}), failure => failure.code === -32602, 'Unknown tool argument is a JSON-RPC parameter error');
    sdkChecks.push({transport: kind, phase, result: 'PASS', full_report_values_and_numeric_lexemes: 'MCP text verified losslessly', raw_mcp_report_slice_typed_sha_claim: false, structured_content_scope: 'safe envelope/profile/hash/count scalars; unsafe JS numeric facts not attested'});
  } catch (failure) {
    throw new Error(`Owned SDK ${kind}/${phase} failed; child=${transport.owned?.pid} exit=${transport.owned?.exitCode} signal=${transport.owned?.signalCode}; stderr=${stderrTail}`, {cause: failure});
  } finally { await client.close(); }
}
async function reportInventory() {
  const result = new Map();
  for (const job of jobs.values()) result.set(job.id + '.json', await readFile(path.join(data, job.id + '.json')));
  async function collect(directory) { for (const entry of await readdir(directory, {withFileTypes: true})) { const filename = path.join(directory, entry.name); if (entry.isDirectory()) await collect(filename); else if (entry.isFile() && entry.name.endsWith('.json')) result.set(path.relative(data, filename), await readFile(filename)); } }
  await collect(path.join(data, 'snapshots')); return result;
}
async function unchangedReports(inventory) {
  for (const [name, bytes] of inventory) assert.deepEqual(await readFile(path.join(data, name)), bytes, 'Saved report/revision bytes changed: ' + name);
  for (const job of jobs.values()) { const response = await call('/api/crawls/' + job.id + '/project.json'); assertLosslessEqual(response.document.root, job.document.root); assert.deepEqual(response.bytes, job.bytes); }
}

try {
  const stage = await fixture('staging', '/stage'), production = await fixture('production', '/site'); assert.notEqual(stage.origin, production.origin);
  await launch();
  const before = await crawl(stage, ['/stage/item', '/stage/common']), after = await crawl(production, ['/site/item', '/site/common']);
  for (const job of [before, after]) {
    assert.equal(at(job.document, ['pages']).elements.length, 2);
    assert.ok(unsafeIntegerLexemes(job.document.root).some(value => value.lexeme === '9007199254740993' && value.path.includes('structured_data')));
  }
  checks.push('Two actual persisted native crawls on distinct owned origins retain complete typed reports and literal JSON-LD u64 counter9007199254740993 without JS numeric reserialization');
  const resource = await crawl(stage, ['/stage/resource-page'], {resource_discovery: {}});
  const svg = at(resource.document, ['pages']).elements.find(page => stringAt(page, ['url']) === stage.origin + '/stage/decimal.svg');
  assert.ok(svg, 'Actual discovered and fetched SVG must have a retained resource page');
  assert.equal(scalar(at(svg, ['image_metadata', 'width'])), '51.248178375505404');
  assert.ok(stage.hits.some(hit => hit.path === '/stage/decimal.svg'));
  const resourceBundle = await bundle(resource, resource);
  checks.push('Genuine discovered/fetched SVG resource retains exact typed f64 width lexeme 51.248178375505404 in full bound original reports');
  const omitted = body(before, after); delete omitted.include_original_reports;
  const defaultResponse = await call('/api/compare/saved', omitted), falseResponse = await call('/api/compare/saved', body(before, after, {}, false));
  assertDefaultEnvelope(defaultResponse.document); assertDefaultEnvelope(falseResponse.document); assert.deepEqual(defaultResponse.bytes, falseResponse.bytes);
  const initial = await bundle(before, after); assertLosslessEqual(at(initial.document, ['comparison']), at(defaultResponse.document, ['comparison']));
  checks.push('Absent/false API option retains exact old two-field envelope and bytes; opt-in adds two complete ordered typed report objects whose direct compact slices match every source value, Core SHA and byte count');
  const config = {include_selected_text: false, mappings: [{side: 'before', from_origin: stage.origin, from_path_prefix: '/stage', to_origin: production.origin, to_path_prefix: '/site'}]};
  const mapped = await bundle(before, after, config);
  assert.equal(scalar(at(mapped.document, ['comparison', 'counts', 'added'])), '0'); assert.equal(scalar(at(mapped.document, ['comparison', 'counts', 'removed'])), '0');
  for (const side of ['before', 'after']) assertLosslessEqual(at(mapped.document, ['original_reports', side]), at(initial.document, ['original_reports', side]));
  assert.equal(at(mapped.document, ['original_reports', 'before', 'report', 'pages', 0, 'links']).elements.length, 2);
  assert.ok(stringAt(mapped.document, ['original_reports', 'before', 'report', 'pages', 0, 'content_captures', 0, 'result', 'text']).includes(stage.origin));
  checks.push('Actual origin/prefix join mapping and selected-text projection opt-out leave both full original reports, duplicate link occurrences, selected source text and u64 lexical values unchanged');
  const same = await bundle(before, before); assert.equal(scalar(at(same.document, ['comparison', 'counts', 'unchanged'])), '2');
  assertLosslessEqual(at(same.document, ['original_reports', 'before']), at(same.document, ['original_reports', 'after']));
  const priorStrict = hits();
  for (const flag of [null, 'yes', 1, {}, []]) error(await request('/api/compare/saved', {...body(before, after), include_original_reports: flag}), 400, 'invalid_comparison_request');
  for (const extra of [{unexpected: true}, {config: {unexpected: true}}]) error(await request('/api/compare/saved', {...body(before, after), ...extra}), 400, 'invalid_comparison_request');
  const conflict = await request('/api/compare/saved', {...body(before, after), expected_after_generation: after.generation + 1}); error(conflict, 409, 'generation_conflict'); assert.equal(safeInt(conflict.document, ['context', 'observed_generation']), after.generation);
  error(await request('/api/compare/saved', {...body(before, before), expected_after_generation: before.generation + 1}), 409, 'generation_conflict'); assert.equal(hits(), priorStrict);
  checks.push('Same-job opt-in preserves identical originals; wrong option types, unknown fields/config and both ordinary/same-job expected-generation conflicts reject explicitly without website requests or partial bundles');

  const parentBytes = await readFile(path.join(data, before.id + '.json')); stage.revision = 1;
  const selectedStart = await call('/api/crawls/' + before.id + '/recrawl', {urls: [stage.origin + '/stage/item']});
  const selectedId = stringAt(selectedStart.document, ['id']), selectedTerminal = await terminal(selectedId), selected = await record(selectedId, safeInt(selectedTerminal.document, ['generation']));
  assert.equal(stringAt(selected.document, ['operation', 'coverage']), 'mixed_age_selected_recrawl'); assert.equal(stringAt(selected.document, ['operation', 'parent', 'snapshot_sha256']), sha(parentBytes));
  const inheritedBefore = at(before.document, ['pages']).elements.find(page => stringAt(page, ['url']) === stage.origin + '/stage/common');
  const inheritedAfter = at(selected.document, ['pages']).elements.find(page => stringAt(page, ['url']) === stage.origin + '/stage/common'); assertLosslessEqual(inheritedAfter, inheritedBefore);
  const selectedBundle = await bundle(before, selected); assert.equal(scalar(at(selectedBundle.document, ['comparison', 'after_snapshot', 'coverage', 'mixed_age_selected_recrawl'])), true);
  assert.deepEqual(await readFile(path.join(data, before.id + '.json')), parentBytes);
  checks.push('Genuine completed selected recrawl bundle retains mixed-age operation/parent binding and every inherited page value/capture exactly, while exposing the refreshed selected row; immutable native parent bytes stay unchanged');

  const partial = await crawl(stage, ['/stage/partial'], {content_capture: {sources: ['raw_http_html'], include_selectors: ['main'], limits: {max_text_utf8_bytes: 8}}});
  const partialBundle = await bundle(partial, partial, {include_selected_text: true});
  const partialFields = at(partialBundle.document, ['comparison', 'pages', 0, 'fields']).elements;
  const partialText = partialFields.find(field => stringAt(field, ['field']) === 'text_sources.raw_http_html.selected_text');
  assert.equal(stringAt(partialText, ['before', 'state']), 'partial'); assert.equal(stringAt(partialText, ['outcome']), 'unknown');
  assert.equal(stringAt(partialBundle.document, ['original_reports', 'before', 'report', 'pages', 0, 'content_captures', 0, 'status']), 'incomplete');
  let importedBytes = replaceToken(before.document, ['pages', 0, 'response_ms'], '9007199254740993');
  const viewportDepth = 123;
  const deepViewport = '['.repeat(viewportDepth) + String.raw`{"counter":9007199254740993,"escaped":"quotes \" and backslash \\"}` + ']'.repeat(viewportDepth);
  importedBytes = replaceToken(parseLossless(importedBytes), ['pages', 0, 'rendered_viewport'], deepViewport);
  const importedDoc = parseLossless(importedBytes); importedBytes = Buffer.from(importedDoc.source.slice(0, importedDoc.root.end - 1) + ',"future_extension":{"counter":9007199254740993,"source":"owned raw input"}}' + importedDoc.source.slice(importedDoc.root.end), 'utf8');
  const importedStart = await call('/api/crawls/import', importedBytes), imported = await record(stringAt(importedStart.document, ['id']), 0);
  assert.equal(imported.document.root.properties.has('future_extension'), false); assert.equal(scalar(at(imported.document, ['pages', 0, 'response_ms'])), '9007199254740993');
  assertLosslessEqual(at(imported.document, ['pages', 0, 'rendered_viewport']), parseLossless(deepViewport).root);
  const importedBundle = await bundle(imported, imported); assert.equal(at(importedBundle.document, ['comparison', 'before_snapshot', 'caller_binding', 'generation']).type, 'null');
  assert.equal(stringAt(importedBundle.document, ['comparison', 'before_snapshot', 'caller_binding', 'origin']), 'app_declared_imported_or_detached_report');
  checks.push('Actual partial captures remain incomplete/partial/unknown inside complete typed-report wrappers; separate raw controlled import retains unsafe response_ms and123-nested-array rendered_viewport exact lexemes/escaping, drops unknown input extension under typed loading and invents no generation/authenticity');

  const limited = await crawl(stage, ['/stage/limit/one', '/stage/limit/two', '/stage/limit/three'], {max_pages: 1});
  const limitedBundle = await bundle(limited, limited); assert.equal(scalar(at(limitedBundle.document, ['comparison', 'before_snapshot', 'coverage', 'limited'])), true);
  stage.hold.add('/stage/limit/two'); await call('/api/crawls/' + limited.id + '/resume', {max_pages: 3}); await until(() => stage.held.has('/stage/limit/two'), 'resumed ordinary crawl admitted barrier');
  const activeStatus = await call('/api/crawls/' + limited.id), activeGeneration = safeInt(activeStatus.document, ['generation']);
  const activeBody = {...body(before, limited), expected_after_generation: activeGeneration};
  const activeHits = hits(); error(await request('/api/compare/saved', activeBody), 409, 'unstable_snapshot');
  await call('/api/crawls/' + limited.id + '/pause', {}); const pausing = await request('/api/compare/saved', activeBody); error(pausing, 409, 'unstable_snapshot'); assert.equal(stringAt(pausing.document, ['context', 'status']), 'pausing'); assert.equal(hits(), activeHits);
  release(stage, '/stage/limit/two'); const pausedStatus = await terminal(limited.id, ['paused']), paused = await record(limited.id, safeInt(pausedStatus.document, ['generation']));
  const pausedBundle = await bundle(paused, paused); assert.equal(at(paused.document, ['pages']).elements.length, 2);
  error(await request('/api/compare/saved', body(limited, limited)), 409, 'generation_conflict');
  stage.hold.add('/stage/limit/three'); await call('/api/crawls/' + paused.id + '/resume', {max_pages: 3}); await until(() => stage.held.has('/stage/limit/three'), 'cancel admitted barrier');
  const cancellingStatus = await call('/api/crawls/' + paused.id); await call('/api/crawls/' + paused.id + '/cancel', {});
  const cancelling = await request('/api/compare/saved', {...body(before, paused), expected_after_generation: safeInt(cancellingStatus.document, ['generation'])}); error(cancelling, 409, 'unstable_snapshot'); assert.equal(stringAt(cancelling.document, ['context', 'status']), 'cancelling');
  release(stage, '/stage/limit/three'); const cancelledStatus = await terminal(paused.id, ['cancelled']), cancelled = await record(paused.id, safeInt(cancelledStatus.document, ['generation']));
  const cancelledBundle = await bundle(cancelled, cancelled); assert.equal(scalar(at(cancelledBundle.document, ['comparison', 'before_snapshot', 'coverage', 'cancelled'])), true);
  checks.push('Genuine limited/paused/cancelled saved cohorts can include full retained reports; held native resume barriers reject running/pausing/cancelling sources and stale generations with structured conflicts');

  const beforePath = path.join(data, 'cli-before.json'), afterPath = path.join(data, 'cli-after.json'), configPath = path.join(data, 'cli-config.json'), jsonPath = path.join(data, 'cli-bundle.json'), csvPath = path.join(data, 'cli-bundle.csv');
  await writeFile(beforePath, before.bytes); await writeFile(afterPath, after.bytes); await writeFile(configPath, JSON.stringify(config));
  const cliDefault = await execute(['compare-saved', beforePath, afterPath, '--config', configPath]); assert.equal(cliDefault.code, 0, cliDefault.stderr); assertDefaultEnvelope(parseLossless(cliDefault.stdout)); assertComparableResponse(parseLossless(cliDefault.stdout), mapped.document, {ignoreCallerBinding: true});
  const cli = await execute(['compare-saved', beforePath, afterPath, '--config', configPath, '--include-original-reports', '--output', jsonPath, '--csv', csvPath]); assert.equal(cli.code, 0, cli.stderr);
  const cliBytes = await readFile(jsonPath), csvBytes = await readFile(csvPath), cliDoc = parseLossless(cliBytes); assertOriginalBundle(cliDoc, before, after); assertComparableResponse(cliDoc, mapped.document, {ignoreCallerBinding: true});
  const csvProof = assertBundleCsv(csvBytes.toString('utf8'), mapped.document);
  for (const destination of [jsonPath, beforePath]) { const bytes = await readFile(destination); const refused = await execute(['compare-saved', beforePath, afterPath, '--include-original-reports', '--output', destination]); assert.equal(refused.code, 2); assert.equal(stringAt(parseLossless(refused.stderr), ['error', 'code']), 'output_destination_exists'); assert.deepEqual(await readFile(destination), bytes); }
  const alias = path.join(data, 'same-destination.json'); const duplicate = await execute(['compare-saved', beforePath, afterPath, '--include-original-reports', '--output', alias, '--csv', alias]); assert.equal(duplicate.code, 2); assert.equal(stringAt(parseLossless(duplicate.stderr), ['error', 'code']), 'duplicate_output_destination'); await assert.rejects(stat(alias), {code: 'ENOENT'});
  checks.push('Native CLI default omits originals; opt-in new JSON/CSV fully matches API including direct original slice SHA/count and lossless CSV numeric cells; existing input/output and duplicate targets refuse without rewriting bytes or reserving aliases');

  baselineHits = hits(); const inventory = await reportInventory();
  for (const kind of ['stdio', 'streamable_http']) {
    await sdk(kind, before, after, config, mapped, 'websites_live_native_u64');
    await sdk(kind, resource, resource, {}, resourceBundle, 'websites_live_native_svg_f64');
    await sdk(kind, imported, imported, {}, importedBundle, 'websites_live_declared_import_u64_depth123');
  }
  assert.equal(hits(), baselineHits); await unchangedReports(inventory); assert.deepEqual(await readFile(jsonPath), cliBytes); assert.deepEqual(await readFile(csvPath), csvBytes);
  checks.push('Both official SDK transports expose optional boolean on 24 tools, including the existing read-only comparison tool; MCP text retains all original values/exact u64 tokens, default omission and structured errors, while safe structured descriptors match API');
  for (const owned of fixtures) {
    await closeServer(owned.server); const witness = {origin: owned.origin, attempts: [], server: undefined};
    witness.server = http.createServer((req, res) => { witness.attempts.push({method: req.method, path: req.url}); res.writeHead(410); res.end('Original website offline'); }); witnesses.push(witness); assert.equal(await listen(witness.server, Number(new URL(owned.origin).port)), owned.origin);
  }
  await stopApp(); await launch();
  const restarted = await bundle(before, after, config); assertComparableResponse(restarted.document, mapped.document);
  for (const [first, second, saved, savedConfig] of [[before, selected, selectedBundle, {}], [partial, partial, partialBundle, {include_selected_text: true}], [imported, imported, importedBundle, {}], [resource, resource, resourceBundle, {}], [cancelled, cancelled, cancelledBundle, {}]]) { const response = await bundle(first, second, savedConfig); assertComparableResponse(response.document, saved.document); }
  const restartedDefault = await call('/api/compare/saved', omitted); assertDefaultEnvelope(restartedDefault.document); assertComparableResponse(restartedDefault.document, defaultResponse.document);
  for (const kind of ['stdio', 'streamable_http']) {
    await sdk(kind, before, after, config, restarted, 'cold_restart_native_u64_two_witnesses');
    await sdk(kind, resource, resource, {}, resourceBundle, 'cold_restart_native_svg_f64_two_witnesses');
    await sdk(kind, imported, imported, {}, importedBundle, 'cold_restart_declared_import_u64_depth123_two_witnesses');
  }
  const coldCli = await execute(['compare-saved', beforePath, afterPath, '--config', configPath, '--include-original-reports']); assert.equal(coldCli.code, 0, coldCli.stderr); const coldCliDoc = parseLossless(coldCli.stdout); assertOriginalBundle(coldCliDoc, before, after); assertComparableResponse(coldCliDoc, mapped.document, {ignoreCallerBinding: true});
  await unchangedReports(inventory); assert.deepEqual(await readFile(jsonPath), cliBytes); assert.deepEqual(await readFile(csvPath), csvBytes);
  await stopApp(); assertFinalWitnessState({liveChildCount: liveChildren.size, fixtureHitCount: hits(), fixtureHitBaseline: baselineHits, witnesses});
  checks.push('Both websites closed: cold App restart/API/CLI/both SDKs retain complete bound originals, native mixed-age/partial/import/cancelled facts and default envelope; both active410 witnesses remain open until every native/SDK child closes and observe zero attempts');
  acceptedProof = {
    result: 'PASS', tested_at: new Date().toISOString(), engine: 'open-seo-spider/original-report-bundle-v1', checks,
    binary_sha256: binarySha, harness_sha256: sourceSha, oracle_helper_sha256: oracleSha,
    oracle_tests_sha256: testsSha, lifecycle_oracle_sha256: lifecycleOracleSha, pure_control_proof_sha256: sha(controlsBytes), pure_control_count: controls.pure_tests,
    sdk_version: sdkPackage.version, sdk_checks: sdkChecks, csv_oracle: csvProof,
    initial_native_original_bindings: initial.bindings, mapped_original_bindings: mapped.bindings,
    complete_native_source_reports: [before, after].map(job => ({id: job.id, generation: job.generation, exported_json_sha256: sha(job.bytes), typed_compact_report_sha256: sha(job.compactBytes), typed_compact_report_bytes: job.compactBytes.length, unsafe_integer_lexemes: unsafeIntegerLexemes(job.document.root)})),
    saved_report_files: [...inventory].map(([relative_path, bytes]) => ({relative_path, bytes: bytes.length, sha256: sha(bytes)})),
    imported_control_is_separate_from_genuine_native_crawl_evidence: true,
    native_svg_resource_evidence: {id: resource.id, generation: resource.generation, fetched_url: stringAt(svg, ['url']), width_lexeme: scalar(at(svg, ['image_metadata', 'width'])), bindings: resourceBundle.bindings},
    declared_import_depth_control: {id: imported.id, nested_viewport_arrays: viewportDepth, response_ms_lexeme: scalar(at(imported.document, ['pages', 0, 'response_ms'])), bindings: importedBundle.bindings, establishes_native_rendering: false},
    stable_terminal_bundles: [limitedBundle, pausedBundle, cancelledBundle].map(response => ({bindings: response.bindings, before_generation: scalar(at(response.document, ['comparison', 'before_snapshot', 'caller_binding', 'generation']))})),
    cli_json_sha256: sha(cliBytes), cli_csv_sha256: sha(csvBytes), website_requests_for_saved_comparison_and_exports: 0,
    open_gates: ['Actual exact64MiB/+1 API/CLI/full-JSON-RPC envelope boundaries and oversized escaped RPC IDs are separate Root-owned native/unit gates, not executed by this harness.', 'Concurrent resume specifically between pair-lock clone and serialization requires App directed interleaving evidence; this harness covers genuine sequential resume and active barriers.', 'Browser opt-in/export UI and served asset evidence are separate.', 'Full vendor parity, SQL/generation authenticity, absent HTML/DOM/body reconstruction and raw legacy input-file fidelity are not established.'],
    mcp_hash_profile_boundary: 'MCP Value ordering differs. Full text report semantics and numeric lexemes plus descriptors are verified; raw MCP report slices are not claimed equal to Core typed-report SHA. SDK parsed unsafe numeric values are not attested.'
  };
} finally {
  const failures = [];
  for (const child of [...liveChildren]) { try { await waitClosed(child); } catch (failure) { failures.push(failure); } }
  for (const owned of fixtures) for (const route of [...owned.held.keys()]) release(owned, route);
  try { await binaryCheck('final_after_all_native_and_sdk_children_closed'); } catch (failure) { failures.push(failure); }
  if (acceptedProof) { try { assertFinalWitnessState({liveChildCount: liveChildren.size, fixtureHitCount: hits(), fixtureHitBaseline: baselineHits, witnesses}); } catch (failure) { failures.push(failure); } }
  for (const owned of fixtures) { try { await closeServer(owned.server); } catch (failure) { failures.push(failure); } }
  for (const owned of witnesses) { try { await closeServer(owned.server, true); } catch (failure) { failures.push(failure); } }
  if (acceptedProof) { try { assertFinalWitnessState({liveChildCount: liveChildren.size, fixtureHitCount: hits(), fixtureHitBaseline: baselineHits, witnesses}); } catch (failure) { failures.push(failure); } }
  if (liveChildren.size !== 0) failures.push(Error('Owned child remains live'));
  if (failures.length === 0) { assert.equal(await realpath(data), ownedData); assert.equal(path.dirname(ownedData), await realpath(os.tmpdir())); assert.ok(path.basename(ownedData).startsWith('open-seo-spider-original-bundle-')); await rm(ownedData, {recursive: true, force: true}); }
  if (failures.length) throw new AggregateError(failures, 'Bundle harness cleanup failed; owned temporary data preserved');
}
acceptedProof.binary_hash_checks = hashChecks; assert.equal(hashChecks.length, starts + 1);
acceptedProof.offline_endpoint_attempts = assertFinalWitnessState({liveChildCount: liveChildren.size, fixtureHitCount: hits(), fixtureHitBaseline: baselineHits, witnesses});
acceptedProof.final_counter_collection = {phase: 'after_all_native_and_sdk_children_and_witness_close_events', live_children: liveChildren.size, witnesses_open_until_child_close: true, witness_drain_without_discarding_active_requests: true};
acceptedProof.native_fixture_requests = fixtures.map(owned => ({origin: owned.origin, count: owned.hits.length, paths: structuredClone(owned.hits)}));
acceptedProof.cleanup = {owned_children_closed: true, owned_servers_drained: true, temporary_data_removed: true};
await mkdir(path.dirname(proofPath), {recursive: true}); await writeFile(proofPath, JSON.stringify(acceptedProof, null, 2) + '\n');
console.log(JSON.stringify({result: 'PASS', checks: checks.length, sdk_transports_checked: sdkChecks.length, proof: proofPath, offline_endpoint_attempts: acceptedProof.offline_endpoint_attempts}, null, 2));
