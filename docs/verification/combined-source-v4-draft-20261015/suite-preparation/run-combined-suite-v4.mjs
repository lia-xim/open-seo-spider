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
const source = path.join(base, 'combined-source-v4');
const repo = path.join(base, 'mcp-file-tools-v1');
const runtimeRoot = path.join(task, 'outputs/open-seo-spider');
const out = path.join(base, 'results/combined-suite-v4');
const chrome = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const cargo = 'C:/Users/matth/.cargo/bin/cargo.exe';
const self = fileURLToPath(import.meta.url);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');

// All these checks precede mkdir, imports, sockets and process creation.
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed',
  'A coordinated compute release is required');
assert.equal(path.resolve(self), path.resolve(task, 'work/run-combined-suite-v4.mjs'),
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
assert.equal(manifest.kind, 'combined_source_v4');
assert.match(manifest.source_commit, /^[a-f0-9]{40}$/);
assert.equal(path.resolve(manifest.source_worktree), path.resolve(repo));
assert.equal(manifest.parent_source_manifest_sha256, '01bfa8efcb6745c5521be5e533ae63f2f15679318b095a29eb3ab925fef95bdf');
assert(Array.isArray(manifest.files) && manifest.files.length >= 301);
assert.equal(manifest.source_inputs, manifest.files.length);
assert(Array.isArray(manifest.required_tests));
assert(Array.isArray(manifest.approved_product_bindings));
const baselinePath = path.join(base, 'combined-source-v3/manifest.json');
const baselineRaw = fs.readFileSync(baselinePath);
const baselineSha = '01bfa8efcb6745c5521be5e533ae63f2f15679318b095a29eb3ab925fef95bdf';
assert.equal(sha(baselineRaw), baselineSha);
const baseline = JSON.parse(baselineRaw);
assert.equal(baseline.kind, 'combined_source_v3');
assert.equal(baseline.files.length, 301); assert.equal(baseline.required_test_names, 421);
assert.equal(baseline.minimum_expected_workspace_tests, 1203);
// Root's explicit final-byte approval binds authored handoffs and formatter-only changes.
const rootApprovalReference = manifest.root_approval;
assert(rootApprovalReference && path.isAbsolute(rootApprovalReference.path));
assert.match(rootApprovalReference.sha256, /^[a-f0-9]{64}$/);
const rootApprovalRaw = fs.readFileSync(rootApprovalReference.path);
assert.equal(sha(rootApprovalRaw), rootApprovalReference.sha256);
const rootApproval = JSON.parse(rootApprovalRaw);
assert.equal(rootApproval.kind, 'root_approved_combined_source_v4');
assert.equal(rootApproval.status, 'REVIEWED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED');
assert.equal(rootApproval.source_commit, manifest.source_commit);
assert.equal(rootApproval.source_branch, manifest.source_branch);
assert.equal(path.resolve(rootApproval.source_worktree), path.resolve(repo));
assert.equal(rootApproval.parent_source_manifest_sha256, baselineSha);
assert.deepEqual(manifest.source_proposals, rootApproval.proposals);
assert.deepEqual(manifest.new_authored_tests, rootApproval.new_authored_tests);
assert(Array.isArray(rootApproval.files) && rootApproval.files.length > 0);
assert(Array.isArray(manifest.references) && manifest.references.length > 0);
const pinnedReferences = new Map();
for (const reference of manifest.references) {
  assert(path.isAbsolute(reference.path));
  assert.match(reference.sha256, /^[a-f0-9]{64}$/);
  assert(Number.isSafeInteger(reference.bytes) && reference.bytes >= 0);
  assert(!pinnedReferences.has(path.resolve(reference.path).toLowerCase()), 'Duplicate immutable reference');
  pinnedReferences.set(path.resolve(reference.path).toLowerCase(), reference);
}
for (const proposal of manifest.source_proposals) {
  for (const [referencePath, referenceSha] of [[proposal.path,proposal.sha256],[proposal.review_path,proposal.review_sha256]]) {
    const reference = pinnedReferences.get(path.resolve(referencePath).toLowerCase());
    assert(reference && reference.sha256 === referenceSha, 'Reviewed proposal must be an immutable final-guard reference');
  }
}
assert.equal(manifest.source_proposals.length, 2);
assert.deepEqual(manifest.source_proposals.map(proposal => proposal.role), ['sql','ledger']);
assert.equal(manifest.source_proposals[0].sha256, 'b355222a8fe61353122a22b754b59936bab269a248cdce07d2a7d8b85da2dba3');
assert.equal(manifest.source_proposals[0].review_sha256, '46d1166a7cceb5c8a1715034b96e76ddf54508bbc4f2fc24a4ccfe4e72b6d8e3');
assert.equal(manifest.source_proposals[1].review_sha256, 'd9504ec7b44aa5d102cef7c6b270b0445412aa813f67978113edb6c53981f18b');

function relativeFile(value) {
  assert.equal(typeof value, 'string');
  assert(value.length > 0 && !value.includes('\\') && !value.includes(':'));
  assert(!path.posix.isAbsolute(value));
  assert(value.split('/').every(part => part.length > 0 && part !== '.' && part !== '..'));
  return value;
}
const files = new Map();
for (const file of manifest.files) {
  relativeFile(file.path);
  assert(!files.has(file.path), 'Duplicate manifest path: ' + file.path);
  assert.match(file.sha256, /^[a-f0-9]{64}$/);
  assert(Number.isSafeInteger(file.bytes) && file.bytes >= 0);
  files.set(file.path, file);
}
assert.equal(new Set([...files.keys()].map(p => p.toLowerCase())).size, files.size,
  'Windows path aliases are not separate inputs');
const required = [];
const requiredPaths = new Set();
for (const group of manifest.required_tests) {
  relativeFile(group.path);
  assert(files.has(group.path), 'Required test source not frozen: ' + group.path);
  assert(!requiredPaths.has(group.path), 'Duplicate required-test group');
  requiredPaths.add(group.path);
  assert(Array.isArray(group.names) && group.names.length > 0);
  for (const name of group.names) {
    assert.match(name, /^[A-Za-z_][A-Za-z_0-9]*$/);
    required.push(name);
  }
}
assert.equal(required.length, manifest.required_test_names);
assert.equal(new Set(required).size, required.length, 'Required names must be unambiguous');
const baselineNames = baseline.required_tests.flatMap(group => group.names);
assert.equal(baselineNames.length, 421); assert.equal(new Set(baselineNames).size, 421);
for (const name of baselineNames) assert(required.includes(name), 'Predecessor required name was removed: ' + name);
for (const previousGroup of baseline.required_tests) {
  const currentGroup = manifest.required_tests.find(group => group.path === previousGroup.path);
  assert(currentGroup, 'Predecessor required source group removed');
  for (const name of previousGroup.names) assert(currentGroup.names.includes(name), 'Predecessor test moved out of its actual frozen source path: ' + name);
}
const newRequiredNames = required.filter(name => !baselineNames.includes(name));
assert.equal(manifest.inherited_required_test_names, 421);
assert.equal(manifest.new_authored_test_functions, newRequiredNames.length);
const declaredNewNames = manifest.new_authored_tests.flatMap(group => group.names);
assert.deepEqual([...declaredNewNames].sort(), [...newRequiredNames].sort());
assert.equal(new Set(declaredNewNames).size, declaredNewNames.length);
for (const group of manifest.new_authored_tests) {
  relativeFile(group.path);
  const actualGroup = manifest.required_tests.find(item => item.path === group.path);
  assert(actualGroup);
  for (const name of group.names) assert(actualGroup.names.includes(name));
}
assert.equal(manifest.minimum_expected_workspace_tests, 1203 + newRequiredNames.length,
  'Minimum must retain1203 plus every actual newly authored function');
const expectedNewFullNames = manifest.new_authored_full_test_names;
assert(Array.isArray(expectedNewFullNames));
for (const name of expectedNewFullNames) assert.match(name,/^[A-Za-z_][A-Za-z_0-9]*(?:::[A-Za-z_][A-Za-z_0-9]*)+$/);
assert.equal(new Set(expectedNewFullNames).size,expectedNewFullNames.length);
assert.deepEqual(expectedNewFullNames.map(name=>name.split('::').at(-1)).sort(),[...newRequiredNames].sort());
const declaredSqlProposal = JSON.parse(fs.readFileSync(manifest.source_proposals[0].path));
const declaredLedgerProposal = JSON.parse(fs.readFileSync(manifest.source_proposals[1].path));
assert.notEqual(manifest.source_proposals[1].sha256,'ee7f4782991d40bbe50da48675cab501c96b4fef6d1f4346296be1be29396652','Known rejected capacity v1 cannot be admitted');
assert.equal(manifest.source_proposals[1].sha256,'9da86f091f5bba9ba70eca5415542cfaf9fb43947c9f5bb1259c192b2d170a82','Only final actual capacity-v3 source is admitted');
assert.deepEqual([...expectedNewFullNames].sort(),[
  ...declaredSqlProposal.new_named_tests.map(name=>'sitemap_membership_analysis::tests::'+name),
  ...declaredLedgerProposal.new_test_names,
].sort());
const previousFiles = new Map(baseline.files.map(file => [file.path, file]));
for (const filePath of previousFiles.keys()) assert(files.has(filePath), 'Predecessor input path removed: ' + filePath);
const bindings = new Map();
for (const item of manifest.approved_product_bindings) {
  relativeFile(item.path); assert(!bindings.has(item.path), 'Duplicate delta binding');
  assert(files.has(item.path), 'Delta path not in current input set');
  assert.match(item.after_sha256, /^[a-f0-9]{64}$/);
  assert(item.before_sha256 === 'ABSENT' || /^[a-f0-9]{64}$/.test(item.before_sha256));
  assert(Number.isSafeInteger(item.bytes) && item.bytes >= 0);
  bindings.set(item.path, item);
}
let changedInputs = 0, newInputs = 0, unchangedInputs = 0;
for (const [filePath, current] of files) {
  const before = previousFiles.get(filePath), delta = bindings.get(filePath);
  if (before && before.sha256 === current.sha256 && before.bytes === current.bytes) {
    assert(!delta, 'Unchanged input cannot be claimed as a delta: ' + filePath); unchangedInputs++;
  } else {
    assert(delta, 'Changed/new input lacks explicit Root delta pin: ' + filePath);
    assert.equal(delta.before_sha256, before ? before.sha256 : 'ABSENT');
    assert.equal(delta.after_sha256, current.sha256); assert.equal(delta.bytes, current.bytes);
    if (before) { assert.notEqual(before.sha256, current.sha256); changedInputs++; } else newInputs++;
  }
}
assert.equal(changedInputs + newInputs, bindings.size);
assert.equal(rootApproval.files.length, bindings.size);
for (const item of rootApproval.files) {
  relativeFile(item.path);
  const actual = bindings.get(item.path);
  assert(actual, 'Root approval delta absent from exact current source');
  assert.equal(actual.before_sha256,item.before_sha256);
  assert.equal(actual.after_sha256,item.after_sha256);
  assert.equal(actual.bytes,item.bytes);
}
assert.equal(manifest.changed_parent_inputs,changedInputs);
assert.equal(manifest.unchanged_parent_inputs,unchangedInputs);
assert.equal(manifest.new_input_files,newInputs);
assert.equal(files.size,301+newInputs);
assert.equal(unchangedInputs + changedInputs, 301);
const lineage = Object.freeze({parent_source_manifest_sha256: baselineSha, root_approval_sha256: rootApprovalReference.sha256, parent_inputs: 301,
  required_parent_names: 421, unchanged_parent_inputs: unchangedInputs, changed_parent_inputs: changedInputs,
  new_input_files: newInputs, delta_bindings: bindings.size, new_named_windows_functions: newRequiredNames,
  actual_required_names: required.length, actual_source_inputs: files.size,
  minimum_expected_workspace_tests: manifest.minimum_expected_workspace_tests});
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
  const result = accepted ? 'PASS_COMBINED_WORKSPACE_V4_REAL_RENDERER_FMT_CLIPPY' : 'FAIL_COMBINED_WORKSPACE_V4';
  write('proof.json', {schema_version: 1, kind: 'combined_full_suite_v4', result,
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
    qualification: 'Only these fresh actual Cargo logs establish combined-v4 acceptance. Actual frozen/Root input bytes, explicit manifest, driver, Chrome, all reviewed proposal/Root approval references and four runtime renderer input files are reverified after actual owned-resource closure. All301 parent paths and421 parent named cases are retained in their actual source paths; only exact reviewed SQL/ledger delta pins, every new required name and corresponding1203-plus-new-functions minimum are admitted. Every new authored test also has its exact full Cargo module name verified in the actual closed workspace raw log. Known rejected capacity v1 is explicitly barred. Requires four actual owned renderer gates and zero failed/ignored, fmt and Clippy -Dwarnings. Test profile requests opt-level1, debug assertions true, overflow checks true and debuginfo0; it does not switch to release semantics. Incremental Cargo cache is permitted; no clean rebuild, timing, native/browser/physical, install, full parity or RSS proof is claimed. Historical V3 source/full/native evidence remains lineage only and cannot establish any V4 runtime acceptance.'});
  console.log(JSON.stringify({result, passed, failed, ignored,
    worker_closed: workerClosed, fixture_closed: fixtureClosed, out}));
  if (!accepted) process.exitCode = 1;
}
