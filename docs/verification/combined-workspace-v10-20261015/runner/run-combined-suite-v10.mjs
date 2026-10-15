import assert from 'node:assert/strict';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath, pathToFileURL} from 'node:url';

// This driver has no default manifest hash and never accepts old proof as a run.
const argv = process.argv.slice(2);
assert.equal(argv.length, 2, 'Required: --manifest-sha <exact 64 lowercase hex>');
assert.equal(argv[0], '--manifest-sha');
assert.match(argv[1], /^[a-f0-9]{64}$/);
const expectedManifestSha = argv[1];
const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const base = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const source = path.join(base, 'combined-source-v10');
const repo = path.join(base, 'mcp-file-tools-v1');
const runtimeRoot = path.join(task, 'outputs/open-seo-spider');
const out = path.join(base, 'results/combined-suite-v10');
const chrome = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const cargo = 'C:/Users/matth/.cargo/bin/cargo.exe';
const self = fileURLToPath(import.meta.url);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');

// All these checks precede mkdir, imports, sockets and process creation.
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed',
  'A coordinated compute release is required');
assert.equal(path.resolve(self), path.resolve(task, 'work/run-combined-suite-v10.mjs'),
  'Root must promote this exact new driver name before execution');
const enforcedProfileEnv = Object.freeze({
  CARGO_PROFILE_TEST_OPT_LEVEL: '1', CARGO_PROFILE_TEST_DEBUG_ASSERTIONS: 'true',
  CARGO_PROFILE_TEST_OVERFLOW_CHECKS: 'true', CARGO_PROFILE_TEST_DEBUG: '0',
  CARGO_PROFILE_DEV_DEBUG: '0',
});
for (const [name, value] of Object.entries(process.env)) {
  if (/^(?:RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|RUSTDOCFLAGS|CARGO_ENCODED_RUSTDOCFLAGS|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTC|CARGO_BUILD_(?:RUSTFLAGS|TARGET)|CARGO_TARGET_.+_RUSTFLAGS)$/.test(name)) {
    assert(!value, 'Unexpected external compiler setting: ' + name);
  }
  if (/^CARGO_PROFILE_(?:TEST|DEV)_/.test(name) && value) {
    assert(Object.hasOwn(enforcedProfileEnv, name), 'Unexpected inherited profile override: ' + name);
    assert.equal(value, enforcedProfileEnv[name], 'Contrary inherited profile override: ' + name);
  }
}
assert(!fs.existsSync(out), 'Fresh output required; no old results are overwritten');
assert(fs.statSync(cargo).isFile());
assert(fs.statSync(chrome).isFile());
const driverSha = sha(fs.readFileSync(self));
const chromeSha = sha(fs.readFileSync(chrome));
const sourceRaw = fs.readFileSync(path.join(source, 'manifest.json'));
assert.equal(sha(sourceRaw), expectedManifestSha, 'Explicit exact combined manifest required');
const manifest = JSON.parse(sourceRaw);
const validatorPath = path.join(task, 'work/validate-combined-source-v10.mjs');
const validatorSha = '089725726a2d24b3cbcd9807029d7e6ba4f9f971e3ba5301b823a1905134191f';
assert.equal(sha(fs.readFileSync(validatorPath)),validatorSha,'Actual reviewed Source10 validator required before import');
const {validateSource10Manifest} = await import(pathToFileURL(validatorPath));
assert.equal(sha(fs.readFileSync(validatorPath)),validatorSha,'Source10 validator changed during import');
const validation = validateSource10Manifest({manifest,sourceDirectory:source,repo});
const {files,required,baselinePath,baselineSha,rootApprovalReference,pinnedReferences,
  expectedNewFullNames,lineage} = validation;
const rendererGates = [
  'live_browser_worker_executes_javascript_for_core_crawl',
  'live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness',
  'live_browser_worker_pause_drains_render_and_retains_rendered_discoveries',
  'live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion',
];
const rendererFiles = [...files.values()].filter(file => file.path.startsWith('renderer/'));
assert.equal(rendererFiles.length, 4, 'All four actual renderer source/package inputs required');
assert.deepEqual(rendererFiles.map(file => file.path).sort(), [
  'renderer/package-lock.json', 'renderer/package.json', 'renderer/server.mjs',
  'renderer/tests/worker.test.mjs',
]);

function verifyFile(directory, file) {
  const bytes = fs.readFileSync(path.join(directory, file.path));
  assert.equal(bytes.length, file.bytes, directory + '/' + file.path + ' bytes');
  assert.equal(sha(bytes), file.sha256, directory + '/' + file.path + ' SHA');
}
function verifyAllInputs() {
  assert.equal(sha(fs.readFileSync(self)), driverSha, 'Driver changed');
  assert.equal(sha(fs.readFileSync(path.join(source, 'manifest.json'))), expectedManifestSha,
    'Combined manifest changed');
  assert.equal(sha(fs.readFileSync(baselinePath)), baselineSha, 'Predecessor manifest changed');
  assert.equal(sha(fs.readFileSync(rootApprovalReference.path)), rootApprovalReference.sha256, 'Root final-byte approval changed');
  for (const reference of pinnedReferences.values()) {
    const bytes = fs.readFileSync(reference.path);
    assert.equal(sha(bytes),reference.sha256,'Immutable reviewed source reference changed: '+reference.path);
    assert.equal(bytes.length,reference.bytes);
  }
  for (const file of files.values()) {
    verifyFile(source, file);
    verifyFile(repo, file);
  }
  for (const file of rendererFiles) verifyFile(runtimeRoot, file);
  assert.equal(sha(fs.readFileSync(chrome)), chromeSha, 'Chrome changed');
  // Supplemental full-module check uses the same actually closed workspace log.
  // The original required-name/count parser and all owned-resource bodies stay exact.
  if (fs.existsSync(path.join(out,'required-test-results.json'))) {
    const text = fs.readFileSync(path.join(out,'workspace-live.log'),'utf8');
    const actual = [...text.matchAll(/^test ([^\r\n]+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$/gm)];
    for (const expected of expectedNewFullNames) {
      const rows = actual.filter(match=>match[1]===expected);
      assert.equal(rows.length,1,'Missing or duplicate new actual full Cargo module name: '+expected);
      assert.equal(rows[0][2],'ok','New actual full Cargo module case is not green: '+expected);
    }
  }
}
verifyAllInputs();
for (const group of manifest.required_tests) {
  const text = fs.readFileSync(path.join(source, group.path), 'utf8');
  for (const name of group.names) {
    assert(new RegExp('\\bfn\\s+' + name + '\\b').test(text),
      'Required function absent from actual frozen source: ' + group.path + '::' + name);
  }
}

fs.mkdirSync(out);
const temp = path.join(out, 'temp');
fs.mkdirSync(temp);
const env = {...process.env, CARGO_TARGET_DIR: path.join(base, 'cargo-target'),
  CARGO_INCREMENTAL: '0', ...enforcedProfileEnv,
  CARGO_TERM_COLOR: 'never', OPEN_SEO_BROWSER_PATH: chrome, TEMP: temp, TMP: temp};
const runs = [], failures = [];
let worker, fixture, workerAddress, fixtureAddress;
let workerClosed = false, fixtureClosed = false, sourceFinalVerified = false;
let namesVerified = false, rendererVerified = false, minimumVerified = false;
let workerEvidence;
const write = (name, value) => fs.writeFileSync(path.join(out, name),
  JSON.stringify(value, null, 2) + '\n', {flag: 'wx'});

async function run(name, args) {
  const logPath = path.join(out, name + '.log');
  const log = fs.openSync(logPath, 'wx');
  const started = process.hrtime.bigint();
  let child, spawnError = null, exit;
  try {
    child = spawn(cargo, args, {cwd: source, env, windowsHide: true,
      stdio: ['ignore', log, log]});
    child.once('error', error => { spawnError = error.message; });
    // 'close' observes the actual child and its inherited log handles closing.
    exit = await new Promise(resolve => child.once('close', (code, signal) => resolve({code, signal})));
  } finally {
    fs.closeSync(log);
  }
  const bytes = fs.readFileSync(logPath), text = bytes.toString('utf8');
  const summaries = [...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)]
    .map(match => ({state: match[1], passed: +match[2], failed: +match[3], ignored: +match[4]}));
  const record = {name, args, pid: child.pid, actual_close_event: true, exit,
    spawn_error: spawnError, elapsed_ms: Number(process.hrtime.bigint() - started) / 1e6,
    log_sha256: sha(bytes), log_bytes: bytes.length, summaries,
    observed_compiling_lines: text.split(/\r?\n/).filter(line => /^\s*Compiling\s/.test(line))};
  runs.push(record);
  write(name + '.json', record);
  console.log(JSON.stringify({name, exit, passed: summaries.reduce((n, x) => n + x.passed, 0),
    failed: summaries.reduce((n, x) => n + x.failed, 0), ignored: summaries.reduce((n, x) => n + x.ignored, 0)}));
  assert.equal(spawnError, null, spawnError ?? 'spawn');
  assert.equal(exit.code, 0, text.slice(-3500));
  assert.equal(exit.signal, null);
  return text;
}

function verifyWorkspace(text) {
  const seen = new Map();
  for (const match of text.matchAll(/^test ([^\r\n]+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$/gm)) {
    const full = match[1], leaf = full.split('::').at(-1);
    if (!seen.has(leaf)) seen.set(leaf, []);
    seen.get(leaf).push({full, status: match[2]});
  }
  const nameRows = [];
  for (const name of [...required, ...rendererGates]) {
    const rows = seen.get(name) ?? [];
    assert.equal(rows.length, 1, 'Missing or duplicate actual test result: ' + name);
    assert.equal(rows[0].status, 'ok', 'Required actual test not green: ' + name);
    nameRows.push({required: name, actual: rows[0].full, status: rows[0].status});
  }
  for(const expected of expectedNewFullNames){const rows=seen.get(expected.split('::').at(-1))??[];assert.equal(rows.length,1,'Missing/duplicate inherited/new full Cargo module result: '+expected);assert.equal(rows[0].full,expected);assert.equal(rows[0].status,'ok');}
  namesVerified = true;
  rendererVerified = true;
  const test = runs.find(record => record.name === 'workspace-live');
  assert(test.summaries.length > 0, 'No actual Rust test summaries');
  const passed = test.summaries.reduce((n, x) => n + x.passed, 0);
  const failed = test.summaries.reduce((n, x) => n + x.failed, 0);
  const ignored = test.summaries.reduce((n, x) => n + x.ignored, 0);
  assert(passed >= manifest.minimum_expected_workspace_tests,
    'Too few actual tests; zero-tests/filtered/stale-artifact output is not acceptance');
  assert.equal(failed, 0);
  assert.equal(ignored, 0);
  assert(test.summaries.every(summary => summary.state === 'ok'));
  minimumVerified = true;
  write('required-test-results.json', {required_names: required.length,
    renderer_names: rendererGates.length, results: nameRows, passed, failed, ignored});
}

try {
  verifyAllInputs();
  const {createRendererServer} = await import(pathToFileURL(path.join(runtimeRoot, 'renderer/server.mjs')).href);
  worker = createRendererServer({env, concurrency: 4});
  workerAddress = await worker.listen(0);
  assert(![8787, 8790].includes(workerAddress.port));
  fixture = http.createServer((_request, response) => {
    response.setHeader('content-type', 'text/html');
    response.end('<html><head><title>Warmup</title></head><body><script>document.title="Owned ready";</script></body></html>');
  });
  await new Promise((resolve, reject) => {
    fixture.once('error', reject);
    fixture.listen(0, '127.0.0.1', () => { fixture.removeListener('error', reject); resolve(); });
  });
  fixtureAddress = fixture.address();
  assert(![8787, 8790].includes(fixtureAddress.port));
  const origin = 'http://127.0.0.1:' + workerAddress.port;
  const fixtureOrigin = 'http://127.0.0.1:' + fixtureAddress.port;
  const healthResponse = await fetch(origin + '/health', {signal: AbortSignal.timeout(10000)});
  assert.equal(healthResponse.status, 200);
  const health = await healthResponse.json();
  assert.equal(health.browser.available, true);
  const response = await fetch(origin + '/render', {method: 'POST',
    headers: {'content-type': 'application/json'}, body: JSON.stringify({url: fixtureOrigin + '/', wait_ms: 20}),
    signal: AbortSignal.timeout(35000)});
  assert.equal(response.status, 200);
  const warmup = await response.json();
  assert.equal(warmup.title, 'Owned ready');
  assert.equal(warmup.status, 200);
  assert.equal(warmup.final_url, fixtureOrigin + '/');
  workerEvidence = {origin, fixture_origin: fixtureOrigin, health, warmup,
    renderer_runtime_root: runtimeRoot, renderer_input_hashes: rendererFiles,
    chrome_path: chrome, chrome_sha256: chromeSha, user_renderer_contacted: false};
  write('owned-worker.json', workerEvidence);
  env.OPEN_SEO_SPIDER_RENDERER = origin + '/render';
  await run('format', ['fmt', '--all', '--check']);
  const workspace = await run('workspace-live',
    ['test', '--workspace', '--locked', '--', '--include-ignored', '--test-threads=1']);
  verifyWorkspace(workspace);
  await run('clippy', ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']);
  verifyAllInputs();
} catch (error) {
  failures.push({phase: 'suite', message: error.message, stack: error.stack});
  process.exitCode = 1;
  console.error(error.message.slice(-3500));
} finally {
  // Attempt both independent closes even if the first one fails; no fake flag
  // or timeout detach substitutes for actual close completion.
  try {
    if (worker) await worker.close();
    workerClosed = !worker || !worker.server.listening;
    assert(workerClosed, 'Owned renderer still listening after close');
  } catch (error) { failures.push({phase: 'owned renderer close', message: error.message}); }
  try {
    if (fixture) await new Promise((resolve, reject) => fixture.close(error => error ? reject(error) : resolve()));
    fixtureClosed = !fixture || !fixture.listening;
    assert(fixtureClosed, 'Owned HTML fixture still listening after close');
  } catch (error) { failures.push({phase: 'owned fixture close', message: error.message}); }
  try { verifyAllInputs(); sourceFinalVerified = true; }
  catch (error) { failures.push({phase: 'final source guard', message: error.message}); }
  const tests = runs.find(run => run.name === 'workspace-live');
  const passed = tests?.summaries.reduce((n, x) => n + x.passed, 0) ?? 0;
  const failed = tests?.summaries.reduce((n, x) => n + x.failed, 0) ?? 0;
  const ignored = tests?.summaries.reduce((n, x) => n + x.ignored, 0) ?? 0;
  const completeRuns = ['format', 'workspace-live', 'clippy'].every(name =>
    runs.some(run => run.name === name && run.actual_close_event && !run.spawn_error
      && run.exit.code === 0 && run.exit.signal === null));
  const accepted = failures.length === 0 && completeRuns && workerClosed && fixtureClosed
    && sourceFinalVerified && namesVerified && rendererVerified && minimumVerified;
  const result = accepted ? 'PASS_COMBINED_WORKSPACE_V10_REAL_RENDERER_FMT_CLIPPY' : 'FAIL_COMBINED_WORKSPACE_V10';
  write('proof.json', {schema_version: 1, kind: 'combined_full_suite_v10', result,
    recorded_at: new Date().toISOString(), driver_sha256: driverSha,
    source_manifest_sha256: expectedManifestSha, source_directory: source,
    source_worktree: repo, source_inputs: files.size, source_lineage: lineage,
    test_profile: {opt_level: 1, debug_assertions: true, overflow_checks: true, debug_info: 0, environment: enforcedProfileEnv},
    source_manifest_metadata: Object.fromEntries(Object.entries(manifest)
      .filter(([key]) => !['files', 'required_tests'].includes(key))),
    required_test_names: required.length, required_test_names_verified: namesVerified,
    minimum_expected: manifest.minimum_expected_workspace_tests, passed, failed, ignored,
    owned_renderer_gates: rendererGates, owned_renderer_gates_verified: rendererVerified,
    owned_worker_port: workerAddress?.port, owned_fixture_port: fixtureAddress?.port,
    owned_worker_closed: workerClosed, owned_fixture_closed: fixtureClosed,
    source_final_verified: sourceFinalVerified, user_renderer_contacted: false,
    runs, failures, incremental_target_directory: env.CARGO_TARGET_DIR,
    qualification: 'Actual Source10 fullsuite only. All358 frozen Source9 inputs retained with18 reviewed owned paths;21 new Windows tests extend577 to598/min1380. All581 parent full Cargo names and21 new full names remain mandatory. Actual parent PASS is reparsed only from immutable Source9 files and three raw Cargo logs/close receipts/owned renderer evidence; parent optimized build is immutable provenance, never current native acceptance. Fresh exact profile, four actual renderer gates, both owned closes, current direct reviews/formatter/preliminary/private publication and final369 source hashes remain mandatory. Three separate actual UI Node regressions are not Cargo tests. No installation or manufacturer/RSS/performance claim.'});
  console.log(JSON.stringify({result, passed, failed, ignored,
    worker_closed: workerClosed, fixture_closed: fixtureClosed, out}));
  if (!accepted) process.exitCode = 1;
}
