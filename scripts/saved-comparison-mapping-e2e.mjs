// Genuine native two-origin crawls -> join-only mapping -> collisions -> offline restart.
// SPIDER_BINARY must name the binary being accepted. Work drafts additionally set SPIDER_ROOT.
import http from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp, readFile, writeFile, mkdir, readdir, realpath, rm, stat} from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {assertComparisonCsv, verifyBinarySha, assertFinalWitnessState} from './saved-comparison-mapping-oracles.mjs';

const sourcePath = fileURLToPath(import.meta.url);
const root = path.resolve(process.env.SPIDER_ROOT || path.join(path.dirname(sourcePath), '..'));
assert.ok(process.env.SPIDER_BINARY, 'Set SPIDER_BINARY to the exact native binary being accepted');
const binary = path.resolve(process.env.SPIDER_BINARY);
assert.ok((await stat(binary)).isFile(), 'SPIDER_BINARY must be a regular file');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const binarySha = sha(await readFile(binary));
const sourceSha = sha(await readFile(sourcePath));
const proofPath = path.resolve(process.env.SPIDER_COMPARISON_MAPPING_PROOF || path.join(root, 'docs/verification/saved-comparison-mapping-e2e.json'));
const controlsPath = path.resolve(process.env.SPIDER_COMPARISON_MAPPING_CONTROL_PROOF || path.join(root, 'docs/verification/saved-comparison-mapping-oracle-controls.json'));
const helperSha = sha(await readFile(fileURLToPath(new URL('./saved-comparison-mapping-oracles.mjs', import.meta.url))));
const testsSha = sha(await readFile(fileURLToPath(new URL('./saved-comparison-mapping-oracles.test.mjs', import.meta.url))));
const controlsBytes = await readFile(controlsPath), controls = JSON.parse(controlsBytes);
assert.equal(controls.result, 'PASS'); assert.equal(controls.pure_tests, 24);
assert.equal(controls.oracle_helper_sha256, helperSha); assert.equal(controls.test_sha256, testsSha);
assert.equal(controls.native_executable_launched, false); assert.equal(controls.app_launched, false); assert.equal(controls.website_or_http_server_used, false);
const data = await mkdtemp(path.join(os.tmpdir(), 'open-seo-spider-native-mapping-'));
const ownedData = await realpath(data);
const checks = [], fixtures = [], witnesses = [], jobs = [], liveChildren = new Set(), childClosures = new WeakMap();
let api, app, output = '', launchError, acceptedProof, readOnlyBaseline;
const binaryHashChecks = [];
let appStarts = 0, cliStarts = 0;
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const deadline = ms => AbortSignal.timeout(ms);
const engine = 'open-seo-spider/saved-comparison-v1';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function alive() {
  if (launchError || !app || app.exitCode !== null || app.signalCode !== null) {
    throw launchError || Error('Owned mapping App exited: ' + output.slice(-4000));
  }
}
async function until(fn, label) {
  const end = Date.now() + 45_000;
  while (Date.now() < end) {
    alive();
    const value = await fn();
    if (value) return value;
    await delay(25);
  }
  throw Error('Timed out: ' + label + '\n' + output.slice(-4000));
}
function trackChild(child) {
  liveChildren.add(child);
  childClosures.set(child, new Promise(resolve => child.once('close', () => { liveChildren.delete(child); resolve(); })));
  return child;
}
async function stopChild(child) {
  if (!child || !liveChildren.has(child)) return;
  const closed = childClosures.get(child);
  if (child.exitCode === null && child.signalCode === null) child.kill();
  if (!await Promise.race([closed.then(() => true), delay(5000).then(() => false)])) {
    child.kill('SIGKILL');
    assert.ok(await Promise.race([closed.then(() => true), delay(5000).then(() => false)]), 'Owned child must exit before deleting its data directory');
  }
}
async function stopApp() {
  const owned = app;
  await stopChild(owned);
  if (app === owned) app = null;
}
async function launch() {
  output = ''; launchError = undefined;
  binaryHashChecks.push({phase: 'before_app_spawn_' + (++appStarts), sha256: await verifyBinarySha(binary, binarySha)});
  app = trackChild(spawn(binary, ['serve', '--listen', '127.0.0.1:0', '--data-dir', data], {cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe']}));
  app.on('error', error => { launchError = error; });
  const append = chunk => { output = (output + chunk).slice(-1024 * 1024); };
  app.stdout.on('data', append); app.stderr.on('data', append);
  api = await until(() => output.match(/(?:^|\n)Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/)?.[1], 'complete owned port-0 bind line');
  assert.ok(Number(new URL(api).port) > 0, 'Owned bind line must contain a concrete port');
  await until(async () => { try { return (await fetch(api + '/healthz', {signal: deadline(2000)})).ok; } catch { return false; } }, 'owned App health');
}
async function request(route, body) {
  alive();
  const response = await fetch(api + route, body === undefined
    ? {signal: deadline(10_000)}
    : {method: 'POST', headers: {'content-type': 'application/json'}, body: JSON.stringify(body), signal: deadline(10_000)});
  const bytes = Buffer.from(await response.arrayBuffer());
  alive();
  return {status: response.status, bytes, body: JSON.parse(bytes.toString('utf8'))};
}
async function call(route, body) {
  const result = await request(route, body);
  assert.equal(result.status, 200, JSON.stringify(result.body));
  return result;
}
async function closeServer(server) {
  if (!server.listening) return;
  const closed = new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  server.closeAllConnections();
  await closed;
}
async function drainWitnessServer(server) {
  if (!server.listening) return;
  // Do not discard active witness requests. A drain timeout fails acceptance.
  const closed = new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  if (!await Promise.race([closed.then(() => true), delay(5000).then(() => false)])) {
    server.closeAllConnections();
    await closed;
    throw Error('Offline witness did not drain before its close event');
  }
}
async function listen(server, port = 0) {
  await new Promise((resolve, reject) => {
    const failed = error => { server.off('listening', ready); reject(error); };
    const ready = () => { server.off('error', failed); resolve(); };
    server.once('error', failed); server.once('listening', ready); server.listen(port, '127.0.0.1');
  });
  assert.ok(server.address() && typeof server.address() === 'object');
  return 'http://127.0.0.1:' + server.address().port;
}
async function fixture(label, prefix) {
  const owned = {label, prefix, origin: undefined, hits: [], server: undefined};
  owned.server = http.createServer((req, res) => {
    owned.hits.push({method: req.method, path: req.url});
    const url = new URL(req.url, owned.origin);
    if (url.pathname === prefix + '/item') {
      res.writeHead(302, {location: prefix + '/final/item' + url.search}); res.end(); return;
    }
    const source = owned.origin + req.url;
    const entity = owned.origin + prefix + '/entity/item';
    const target = owned.origin + prefix + '/target?source=' + label + '#fragment';
    const ld = {'@context': 'https://schema.org', '@type': 'Thing', '@id': entity, url: source, name: label + ' saved entity'};
    res.setHeader('content-type', 'text/html; charset=utf-8');
    res.end(`<html lang="en"><head><title>${label} ${url.pathname}</title><meta name="description" content="${label} saved description ${url.pathname}"><script type="application/ld+json">${JSON.stringify(ld)}</script></head><body><main><h1>${label} ${url.pathname}</h1><p>Original source ${source}. Original entity ${entity}.</p><a href="${target}">First original link</a><a href="${target}">Second original link</a></main></body></html>`);
  });
  fixtures.push(owned);
  owned.origin = await listen(owned.server);
  return owned;
}
async function crawl(owned, suffixes) {
  const urls = suffixes.map(suffix => owned.origin + suffix);
  const started = (await call('/api/crawls', {
    seed_urls: urls, crawl_links: false, discover_sitemaps: false, obey_robots: false,
    delay_ms: 0, concurrency: 2, content_capture: {sources: ['raw_http_html'], include_selectors: ['main']}
  })).body;
  assert.match(started.id, UUID);
  const status = await until(async () => {
    const job = (await call('/api/crawls/' + started.id)).body;
    if (job.status === 'failed') throw Error(job.error);
    return job.status === 'completed' && job;
  }, 'actual native mapping crawl completed');
  assert.ok(status.generation > 0, 'Native snapshot generation must be positive');
  const exported = await call('/api/crawls/' + started.id + '/project.json');
  assert.equal(exported.body.pages.length, urls.length);
  assert.deepEqual(exported.body.pages.map(page => page.url).sort(), urls.slice().sort());
  for (const page of exported.body.pages) {
    assert.equal(page.error, null); assert.equal(page.final_status, 200);
    assert.ok(owned.hits.some(hit => hit.path === new URL(page.url).pathname + new URL(page.url).search), 'Report row must come from an observed actual native fixture request');
  }
  const job = {id: started.id, generation: status.generation, report: exported.body, projectBytes: exported.bytes};
  jobs.push(job);
  return job;
}
function comparisonBody(before, after, config) {
  return {before: before.id, after: after.id, expected_before_generation: before.generation, expected_after_generation: after.generation, config};
}
async function compare(before, after, config) { return call('/api/compare/saved', comparisonBody(before, after, config)); }
function pageFor(result, key) {
  const page = result.comparison.pages.find(row => row.comparison_key === key);
  assert.ok(page, 'Expected exact comparison key: ' + key);
  return page;
}
function field(page, name) {
  const value = page.fields.find(row => row.field === name);
  assert.ok(value, 'Expected field: ' + name);
  return value;
}
function stableResult(body) {
  // CLI and cold-start App declarations can differ; the compared report facts may not.
  const value = structuredClone(body);
  for (const name of ['before_snapshot', 'after_snapshot']) delete value.comparison[name].caller_binding;
  return value;
}
function hitCount() { return fixtures.reduce((sum, owned) => sum + owned.hits.length, 0); }
async function reportInventory() {
  const inventory = new Map();
  for (const job of jobs) inventory.set(job.id + '.json', await readFile(path.join(data, job.id + '.json')));
  async function revisions(dir) {
    for (const entry of await readdir(dir, {withFileTypes: true})) {
      const filename = path.join(dir, entry.name);
      if (entry.isDirectory()) await revisions(filename);
      else if (entry.isFile() && entry.name.endsWith('.json')) inventory.set(path.relative(data, filename), await readFile(filename));
    }
  }
  await revisions(path.join(data, 'snapshots'));
  return inventory;
}
async function unchangedReports(inventory) {
  for (const [name, bytes] of inventory) assert.deepEqual(await readFile(path.join(data, name)), bytes, 'Saved report bytes changed: ' + name);
  for (const job of jobs) {
    const exported = await call('/api/crawls/' + job.id + '/project.json');
    assert.deepEqual(exported.body, job.report, 'Typed project changed: ' + job.id);
    assert.deepEqual(exported.bytes, job.projectBytes, 'Exported project JSON bytes changed: ' + job.id);
  }
}
async function execute(args) {
  let child;
  try {
    binaryHashChecks.push({phase: 'before_cli_spawn_' + (++cliStarts), sha256: await verifyBinarySha(binary, binarySha)});
    return await new Promise((resolve, reject) => {
      child = trackChild(spawn(binary, args, {cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe']}));
      let stdout = '', stderr = '';
      const timeout = setTimeout(() => { child.kill(); reject(Error('Owned CLI timed out')); }, 30_000);
      child.on('error', error => { clearTimeout(timeout); reject(error); });
      child.stdout.on('data', chunk => { stdout += chunk; }); child.stderr.on('data', chunk => { stderr += chunk; });
      child.on('close', code => { clearTimeout(timeout); resolve({code, stdout, stderr}); });
    });
  } finally { await stopChild(child); }
}
function collision(response, side, key, report, originals) {
  assert.equal(response.status, 400); assert.equal(response.body.website_requests, 0);
  assert.equal(response.body.error.code, 'ambiguous_snapshot_identity');
  assert.equal(Object.hasOwn(response.body, 'comparison'), false, 'Collision may not return a partial comparison');
  const found = response.body.error.collisions.find(value => value.side === side && value.comparison_key === key);
  assert.ok(found, 'Expected structured ' + side + ' collision');
  assert.equal(found.occurrences.length, 2);
  assert.deepEqual(found.occurrences.map(value => value.original_requested_url).sort(), originals.slice().sort());
  for (const occurrence of found.occurrences) {
    assert.ok(Number.isSafeInteger(occurrence.ordinal) && occurrence.ordinal >= 0);
    assert.equal(report.pages[occurrence.ordinal].url, occurrence.original_requested_url);
  }
}

try {
  const stage = await fixture('staging', '/stage');
  const production = await fixture('production', '/site');
  assert.notEqual(stage.origin, production.origin);
  await launch();
  const before = await crawl(stage, ['/stage/item?view=full', '/stage/common']);
  const after = await crawl(production, ['/site/item?view=full', '/site/common']);
  const beforeCollision = await crawl(stage, ['/stage/collision', '/alias/collision']);
  const afterCollision = await crawl(production, ['/site/collision', '/shadow/collision']);
  const inventory = await reportInventory();
  const readOnlyHits = hitCount();
  readOnlyBaseline = readOnlyHits;
  checks.push('Four genuine native crawl cohorts on two distinct port-0 fixture origins; each row has an observed HTTP request, positive generation, and saved report bytes');

  const forwardRule = {side: 'before', from_origin: stage.origin, from_path_prefix: '/stage', to_origin: production.origin, to_path_prefix: '/site'};
  const config = {include_selected_text: true, mappings: [forwardRule]};
  const unmapped = (await compare(before, after, {include_selected_text: true})).body;
  assert.equal(unmapped.comparison.counts.added, 2); assert.equal(unmapped.comparison.counts.removed, 2);
  assert.equal(unmapped.comparison.counts.changed, 0); assert.equal(unmapped.comparison.counts.unchanged, 0);
  const mappedResponse = await compare(before, after, config), mapped = mappedResponse.body;
  assert.equal(mapped.website_requests, 0); assert.equal(mapped.comparison.website_requests, 0);
  assert.equal(mapped.comparison.engine, engine);
  assert.equal(mapped.comparison.identity_profile, 'exact-recorded-requested-url-v1');
  assert.equal(mapped.comparison.mapping_profile, 'origin-segment-path-prefix-join-key-only-v1');
  assert.deepEqual(mapped.comparison.pages.map(page => page.comparison_key).sort(), [production.origin + '/site/item?view=full', production.origin + '/site/common'].sort());
  assert.equal(mapped.comparison.counts.added, 0); assert.equal(mapped.comparison.counts.removed, 0);
  assert.equal(mapped.comparison.counts.changed, 2); assert.equal(mapped.comparison.counts.unchanged, 0);
  for (const name of ['before_snapshot', 'after_snapshot']) assert.equal(mapped.comparison[name].typed_report_sha256, unmapped.comparison[name].typed_report_sha256);
  checks.push('Actual staging /stage URLs map to production /site keys with query preserved: unmapped added2/removed2 becomes mapped changed2 and added0/removed0; typed input hashes stay fixed');

  for (const suffix of ['/common', '/item?view=full']) {
    const joined = pageFor(mapped, production.origin + '/site' + suffix);
    const originals = {before: before.report.pages.find(page => page.url === stage.origin + '/stage' + suffix), after: after.report.pages.find(page => page.url === production.origin + '/site' + suffix)};
    for (const side of ['before', 'after']) {
      const original = originals[side];
      assert.ok(original); assert.equal(joined[side].original_requested_url, original.url);
      assert.equal(joined[side].document_url, original.final_url);
      assert.equal(field(joined, 'final_url')[side].value, original.final_url);
      assert.deepEqual(field(joined, 'links')[side].value, original.links);
      assert.equal(original.links.length, 2); assert.equal(original.links[0].url, original.links[1].url);
      assert.equal(original.links[0].text, 'First original link'); assert.equal(original.links[1].text, 'Second original link');
      assert.equal(original.links[0].fragment, 'fragment');
      assert.deepEqual(field(joined, 'structured_data')[side].value, original.structured_data);
      const capture = original.content_captures.find(row => row.source_kind === 'raw_http_html');
      assert.equal(capture.status, 'complete'); assert.ok(capture.result.text.includes(original.final_url));
      const selected = field(joined, 'text_sources.raw_http_html.selected_text')[side];
      assert.equal(selected.state, 'observed'); assert.equal(selected.value.text, capture.result.text);
      assert.equal(selected.value.text_sha256, capture.result.text_sha256);
      assert.equal(selected.provenance.requested_url, original.url);
    }
    for (const name of ['final_url', 'links', 'structured_data', 'text_sources.raw_http_html.selected_text']) assert.equal(field(joined, name).outcome, 'changed');
  }
  const item = pageFor(mapped, production.origin + '/site/item?view=full');
  assert.equal(item.before.original_requested_url, stage.origin + '/stage/item?view=full');
  assert.equal(field(item, 'final_url').before.value, stage.origin + '/stage/final/item?view=full');
  assert.equal(field(item, 'final_url').after.value, production.origin + '/site/final/item?view=full');
  checks.push('Mapping rewrites join keys only: original requested/final redirect URLs, duplicate ordered link occurrences and fragments, JSON-LD values, selected text/hash and provenance stay byte-for-value equal to each genuine source report');

  const reverseConfig = {include_selected_text: true, mappings: [{side: 'after', from_origin: production.origin, from_path_prefix: '/site', to_origin: stage.origin, to_path_prefix: '/stage'}]};
  const reversed = (await compare(before, after, reverseConfig)).body;
  assert.deepEqual(reversed.comparison.pages.map(page => page.comparison_key).sort(), [stage.origin + '/stage/item?view=full', stage.origin + '/stage/common'].sort());
  for (const suffix of ['/common', '/item?view=full']) {
    const forward = pageFor(mapped, production.origin + '/site' + suffix), reverse = pageFor(reversed, stage.origin + '/stage' + suffix);
    assert.deepEqual(reverse.before, forward.before); assert.deepEqual(reverse.after, forward.after); assert.deepEqual(reverse.fields, forward.fields);
  }
  checks.push('Actual after-side production→staging mapping changes only the chosen join namespace and retains identical original references and field deltas');

  const beforeRules = [forwardRule, {side: 'before', from_origin: stage.origin, from_path_prefix: '/alias', to_origin: production.origin, to_path_prefix: '/site'}];
  const afterRule = {side: 'after', from_origin: production.origin, from_path_prefix: '/shadow', to_origin: production.origin, to_path_prefix: '/site'};
  const collisionKey = production.origin + '/site/collision';
  const beforeError = await request('/api/compare/saved', comparisonBody(beforeCollision, after, {mappings: beforeRules}));
  collision(beforeError, 'before', collisionKey, beforeCollision.report, [stage.origin + '/stage/collision', stage.origin + '/alias/collision']);
  assert.equal(beforeError.body.error.collisions.length, 1);
  checks.push('Two distinct actually crawled before URLs mapped to one key reject400 with the before side, exact mapped key, both original URLs and their correct report ordinals');
  const afterError = await request('/api/compare/saved', comparisonBody(before, afterCollision, {mappings: [forwardRule, afterRule]}));
  collision(afterError, 'after', collisionKey, afterCollision.report, [production.origin + '/site/collision', production.origin + '/shadow/collision']);
  assert.equal(afterError.body.error.collisions.length, 1);
  checks.push('Two distinct actually crawled after URLs mapped to one key reject400 with the after side, exact mapped key, both original URLs and correct ordinals');
  const bothConfig = {mappings: [...beforeRules, afterRule]};
  const bothError = await request('/api/compare/saved', comparisonBody(beforeCollision, afterCollision, bothConfig));
  collision(bothError, 'before', collisionKey, beforeCollision.report, [stage.origin + '/stage/collision', stage.origin + '/alias/collision']);
  collision(bothError, 'after', collisionKey, afterCollision.report, [production.origin + '/site/collision', production.origin + '/shadow/collision']);
  assert.equal(bothError.body.error.collisions.length, 2);
  checks.push('A genuine two-sided mapped collision reports both sides together and returns no truncated or partially joined comparison');

  const beforePath = path.join(data, 'cli-before.json'), afterPath = path.join(data, 'cli-after.json'), configPath = path.join(data, 'cli-config.json');
  const jsonPath = path.join(data, 'cli-comparison.json'), csvPath = path.join(data, 'cli-comparison.csv');
  await writeFile(beforePath, before.projectBytes); await writeFile(afterPath, after.projectBytes); await writeFile(configPath, JSON.stringify(config));
  const cli = await execute(['compare-saved', beforePath, afterPath, '--config', configPath, '--output', jsonPath, '--csv', csvPath]);
  assert.equal(cli.code, 0, cli.stderr);
  const cliBytes = await readFile(jsonPath), csvBytes = await readFile(csvPath);
  assert.deepEqual(stableResult(JSON.parse(cliBytes)), stableResult(mapped));
  const csvOracle = assertComparisonCsv(csvBytes.toString('utf8'), mapped);
  assert.ok(csvOracle.data_rows > 0); assert.equal(csvOracle.columns, 21);
  checks.push('Native CLI JSON matches API projections; every ordered CSV field row and all 21 columns match API keys, classes, original URLs/ordinals/JSON values, states, outcomes, profiles, provenance and typed hashes with formula defence; caller declarations remain distinct');

  const repeated = await compare(before, after, config);
  assert.deepEqual(repeated.bytes, mappedResponse.bytes, 'Repeated API comparison output bytes must be deterministic');
  await unchangedReports(inventory);
  assert.equal(hitCount(), readOnlyHits, 'Mapping, collision validation, CLI and project exports must make zero website requests');
  assert.deepEqual(await readFile(jsonPath), cliBytes); assert.deepEqual(await readFile(csvPath), csvBytes);
  checks.push('All mapping/error/export read-only phases leave API comparison output bytes, four typed/exported reports, original saved/revision JSON bytes and existing CLI exports unchanged; both live fixture hit counters remain fixed');

  for (const owned of fixtures) {
    await closeServer(owned.server);
    const witness = {origin: owned.origin, attempts: [], server: undefined};
    witness.server = http.createServer((req, res) => { witness.attempts.push({method: req.method, path: req.url}); res.writeHead(410); res.end('Original mapping website is offline'); });
    witnesses.push(witness);
    assert.equal(await listen(witness.server, Number(new URL(owned.origin).port)), owned.origin);
  }
  await stopApp(); await launch();
  const restarted = (await compare(before, after, config)).body;
  assert.deepEqual(stableResult(restarted), stableResult(mapped));
  for (const name of ['before_snapshot', 'after_snapshot']) {
    assert.equal(restarted.comparison[name].caller_binding.generation, mapped.comparison[name].caller_binding.generation);
    assert.equal(restarted.comparison[name].caller_binding_attested_by_core, false);
  }
  assert.deepEqual(stableResult((await compare(before, after, reverseConfig)).body), stableResult(reversed));
  for (const [first, second, mappedConfig, expected] of [[beforeCollision, after, {mappings: beforeRules}, beforeError.body], [before, afterCollision, {mappings: [forwardRule, afterRule]}, afterError.body], [beforeCollision, afterCollision, bothConfig, bothError.body]]) {
    const response = await request('/api/compare/saved', comparisonBody(first, second, mappedConfig));
    assert.equal(response.status, 400); assert.deepEqual(response.body, expected);
  }
  const restartedCli = await execute(['compare-saved', beforePath, afterPath, '--config', configPath]);
  assert.equal(restartedCli.code, 0, restartedCli.stderr);
  assert.deepEqual(stableResult(JSON.parse(restartedCli.stdout)), stableResult(mapped));
  await unchangedReports(inventory);
  assert.deepEqual(await readFile(jsonPath), cliBytes); assert.deepEqual(await readFile(csvPath), csvBytes);
  // Keep both offline witnesses open until every App/CLI child has emitted close.
  await stopApp();
  assertFinalWitnessState({liveChildCount: liveChildren.size, fixtureHitCount: hitCount(), fixtureHitBaseline: readOnlyHits, witnesses});
  checks.push('After both website responders are closed and App cold restarts, forward/reverse mappings, CLI deltas/hashes and all three structured collision errors remain identical; both active 410 witnesses stay open through final owned-child close and then each observe zero attempts; all original report/export bytes remain unchanged');

  acceptedProof = {
    tested_at: new Date().toISOString(), result: 'PASS', engine, binary_sha256: binarySha,
    harness_sha256: sourceSha, checks, counts: mapped.comparison.counts,
    origins: {stage: stage.origin, production: production.origin}, owned_api: api,
    actual_native_crawl_count: jobs.length, imported_fixture_report_count: 0,
    jobs: jobs.map(job => ({id: job.id, generation: job.generation, pages: job.report.pages.length, exported_json_sha256: sha(job.projectBytes)})),
    report_files: [...inventory].map(([relative_path, bytes]) => ({relative_path, bytes: bytes.length, sha256: sha(bytes)})),
    typed_before_sha256: mapped.comparison.before_snapshot.typed_report_sha256,
    typed_after_sha256: mapped.comparison.after_snapshot.typed_report_sha256,
    api_repeat_output_sha256: sha(mappedResponse.bytes), cli_json_sha256: sha(cliBytes), cli_csv_sha256: sha(csvBytes),
    csv_oracle: csvOracle,
    oracle_controls: {result: controls.result, pure_tests: controls.pure_tests, proof_sha256: sha(controlsBytes), oracle_helper_sha256: helperSha, test_sha256: testsSha},
    collision_sides: bothError.body.error.collisions.map(value => value.side),
    website_requests_for_read_only_phases: 0,
    limitations: [
      'Own native/API/CLI mapping acceptance on bounded localhost fixtures; browser UI, MCP, release and vendor parity are separate gates.',
      'Mapping changes requested URL join keys only; URL-bearing source facts remain original and can therefore compare as changed.',
      'CLI and restarted App caller declarations may differ; exact compared projections, profiles, configuration, coverage, deltas and typed input hashes are asserted equal.',
      'Report JSON and immutable report revision bytes are covered; caller SQL authenticity and every auxiliary storage file are not attested.',
      'No combined full-original-report bundle or full website completeness claim.'
    ]
  };
} finally {
  const cleanup = [];
  for (const child of [...liveChildren]) { try { await stopChild(child); } catch (error) { cleanup.push(error); } }
  // Verify the actual file again after all starts and closes, while witnesses still observe.
  try { binaryHashChecks.push({phase: 'final_after_owned_children_closed', sha256: await verifyBinarySha(binary, binarySha)}); } catch (error) { cleanup.push(error); }
  if (acceptedProof) {
    try { assertFinalWitnessState({liveChildCount: liveChildren.size, fixtureHitCount: hitCount(), fixtureHitBaseline: readOnlyBaseline, witnesses}); } catch (error) { cleanup.push(error); }
  }
  for (const owned of fixtures) { try { await closeServer(owned.server); } catch (error) { cleanup.push(error); } }
  for (const owned of witnesses) { try { await drainWitnessServer(owned.server); } catch (error) { cleanup.push(error); } }
  if (acceptedProof) {
    try { assertFinalWitnessState({liveChildCount: liveChildren.size, fixtureHitCount: hitCount(), fixtureHitBaseline: readOnlyBaseline, witnesses}); } catch (error) { cleanup.push(error); }
  }
  if (liveChildren.size !== 0) cleanup.push(Error('Owned child remains live; temporary data was preserved'));
  if (cleanup.length === 0) {
    assert.equal(await realpath(data), ownedData);
    assert.equal(path.dirname(ownedData), await realpath(os.tmpdir()));
    assert.ok(path.basename(ownedData).startsWith('open-seo-spider-native-mapping-'));
    await rm(ownedData, {recursive: true, force: true});
  }
  if (cleanup.length) throw new AggregateError(cleanup, 'Mapping harness cleanup did not finish; temporary data was preserved');
}
// A public PASS is only written after every owned child/server and temporary data gate closed.
acceptedProof.cleanup = {owned_children_closed: true, owned_servers_closed: true, temporary_data_removed: true};
acceptedProof.binary_hash_checks = binaryHashChecks;
assert.deepEqual(binaryHashChecks.map(value => value.phase), ['before_app_spawn_1', 'before_cli_spawn_1', 'before_app_spawn_2', 'before_cli_spawn_2', 'final_after_owned_children_closed']);
acceptedProof.offline_endpoint_attempts = assertFinalWitnessState({liveChildCount: liveChildren.size, fixtureHitCount: hitCount(), fixtureHitBaseline: readOnlyBaseline, witnesses});
acceptedProof.native_fixture_requests = fixtures.map(owned => ({origin: owned.origin, count: owned.hits.length, paths: structuredClone(owned.hits)}));
acceptedProof.final_counter_collection = {phase: 'after_all_owned_children_and_witness_server_close_events', live_children: liveChildren.size, website_requests_after_read_only_baseline: hitCount() - readOnlyBaseline, witnesses_open_until_children_closed: true, witness_active_requests_drained_without_forced_disconnect: true};
await mkdir(path.dirname(proofPath), {recursive: true}); await writeFile(proofPath, JSON.stringify(acceptedProof, null, 2) + '\n');
console.log(JSON.stringify({result: 'PASS', checks: checks.length, proof: proofPath, counts: acceptedProof.counts, actual_native_crawls: jobs.length, offline_endpoint_attempts: acceptedProof.offline_endpoint_attempts}, null, 2));
