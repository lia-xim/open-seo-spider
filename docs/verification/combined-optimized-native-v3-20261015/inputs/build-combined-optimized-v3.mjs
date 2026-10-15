import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';

const argv = process.argv.slice(2);
assert.equal(argv.length, 2, 'Required: --manifest-sha <exact64hex>');
assert.equal(argv[0], '--manifest-sha');
assert.match(argv[1], /^[a-f0-9]{64}$/);
const expectedManifestSha = argv[1];
const suiteDriverSha = 'e6ccab255c9056f6791a02fde68672a8e91ccc98c9069d9538e7ebfbf1728d4d';
const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const base = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const source = path.join(base, 'combined-source-v3');
const repo = path.join(base, 'mcp-file-tools-v1');
const suiteRoot = path.join(base, 'results/combined-suite-v3');
const out = path.join(base, 'results/combined-optimized-build-v3');
const target = path.join(base, 'cargo-target/release/open-seo-spider.exe');
const cargo = 'C:/Users/matth/.cargo/bin/cargo.exe';
const rustc = 'C:/Users/matth/.cargo/bin/rustc.exe';
const self = fileURLToPath(import.meta.url);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const selfSha = sha(fs.readFileSync(self));
const driverPath = path.join(task, 'work/run-combined-suite-v3.mjs');
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed',
  'Explicit coordinated compute release required');
assert.equal(path.resolve(self), path.resolve(task, 'work/build-combined-optimized-v3.mjs'),
  'Promote this exact new builder name into task/work first');
for (const [name, value] of Object.entries(process.env)) {
  if (/^(?:RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTC|CARGO_BUILD_(?:RUSTFLAGS|TARGET)|CARGO_TARGET_.+_RUSTFLAGS|CARGO_PROFILE_RELEASE_.+)$/.test(name)) {
    assert(!value, 'Unexpected external compiler/profile setting: ' + name);
  }
}
assert(!fs.existsSync(out), 'Fresh output required; never replace an old build');
assert(fs.statSync(cargo).isFile());
assert(fs.statSync(rustc).isFile());
const sourceRaw = fs.readFileSync(path.join(source, 'manifest.json'));
assert.equal(sha(sourceRaw), expectedManifestSha);
const manifest = JSON.parse(sourceRaw);
assert.equal(manifest.kind, 'combined_source_v3');
assert.match(manifest.source_commit, /^[a-f0-9]{40}$/);
assert.equal(path.resolve(manifest.source_worktree), path.resolve(repo));
assert.equal(manifest.parent_source_manifest_sha256, '67aebeb959ac6cfb9f8352162becf3ff98ef90918c825a674ea2e92a5df353d0');
assert(Array.isArray(manifest.files) && manifest.files.length >= 301);
assert.equal(manifest.source_inputs, manifest.files.length);
assert(Array.isArray(manifest.required_tests));
assert(Array.isArray(manifest.approved_product_bindings));
const baselinePath = path.join(base, 'combined-source-v1/manifest.json');
const baselineRaw = fs.readFileSync(baselinePath);
const baselineSha = '67aebeb959ac6cfb9f8352162becf3ff98ef90918c825a674ea2e92a5df353d0';
assert.equal(sha(baselineRaw), baselineSha);
const baseline = JSON.parse(baselineRaw);
assert.equal(baseline.files.length, 301); assert.equal(baseline.required_test_names, 419);
assert.equal(baseline.minimum_expected_workspace_tests, 1201);
const previousAttemptPath = path.join(base, 'combined-source-v2/manifest.json');
const previousAttemptSha = '0bd2a7cc5e429e7d46a1d596e5941d62f5a2b2bd9dfb78199e85b7c5d4f9fd2a';
assert.equal(manifest.previous_attempt_source_manifest_sha256, previousAttemptSha);
const previousAttemptRaw = fs.readFileSync(previousAttemptPath);
assert.equal(sha(previousAttemptRaw), previousAttemptSha);
const previousAttempt = JSON.parse(previousAttemptRaw);
assert.equal(previousAttempt.kind, 'combined_source_v2');
assert.equal(previousAttempt.files.length, 301); assert.equal(previousAttempt.required_test_names, 421);
assert.equal(previousAttempt.minimum_expected_workspace_tests, 1203);
const previousAttemptNames = previousAttempt.required_tests.flatMap(group => group.names);
assert.equal(previousAttemptNames.length, 421); assert.equal(new Set(previousAttemptNames).size, 421);

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
assert.equal(baselineNames.length, 419); assert.equal(new Set(baselineNames).size, 419);
for (const name of baselineNames) assert(required.includes(name), 'Predecessor required name was removed: ' + name);
for (const name of previousAttemptNames) assert(required.includes(name), 'Previous attempt required name was removed: ' + name);
const newRequiredNames = required.filter(name => !baselineNames.includes(name));
assert.equal(manifest.minimum_expected_workspace_tests, 1201 + newRequiredNames.length,
  'Minimum must retain1201 plus every new named Windows function');
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
assert.equal(unchangedInputs + changedInputs, 301);
const lineage = Object.freeze({parent_source_manifest_sha256: baselineSha, previous_attempt_source_manifest_sha256: previousAttemptSha, parent_inputs: 301,
  required_parent_names: 419, unchanged_parent_inputs: unchangedInputs, changed_parent_inputs: changedInputs,
  new_input_files: newInputs, delta_bindings: bindings.size, new_named_windows_functions: newRequiredNames,
  actual_required_names: required.length, actual_source_inputs: files.size,
  minimum_expected_workspace_tests: manifest.minimum_expected_workspace_tests});
const names = required;
const minimumExpected = manifest.minimum_expected_workspace_tests;
const expectedTestProfile = Object.freeze({opt_level: 1, debug_assertions: true,
  overflow_checks: true, debug_info: 0, environment: {
    CARGO_PROFILE_TEST_OPT_LEVEL: '1', CARGO_PROFILE_TEST_DEBUG_ASSERTIONS: 'true',
    CARGO_PROFILE_TEST_OVERFLOW_CHECKS: 'true', CARGO_PROFILE_TEST_DEBUG: '0',
    CARGO_PROFILE_DEV_DEBUG: '0',
  }});
for (const group of manifest.required_tests) {
  const text = fs.readFileSync(path.join(source, group.path), 'utf8');
  for (const name of group.names) {
    assert(new RegExp('\\bfn\\s+' + name + '\\b').test(text),
      'Required function absent from actual frozen source: ' + group.path + '::' + name);
  }
}
const rendererGates = [
  'live_browser_worker_executes_javascript_for_core_crawl',
  'live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness',
  'live_browser_worker_pause_drains_render_and_retains_rendered_discoveries',
  'live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion',
];
const suiteCommands = [
  ['format', ['fmt', '--all', '--check']],
  ['workspace-live', ['test', '--workspace', '--locked', '--', '--include-ignored', '--test-threads=1']],
  ['clippy', ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']],
];
function boundedRead(file, max) {
  const stat = fs.statSync(file);
  assert(stat.isFile() && stat.size <= max, 'Bounded regular file required: ' + file);
  const bytes = fs.readFileSync(file);
  assert.equal(bytes.length, stat.size, 'File changed while reading: ' + file);
  return bytes;
}
const parseSummaries = text => [...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)]
  .map(match => ({state: match[1], passed: +match[2], failed: +match[3], ignored: +match[4]}));

function verifySuite() {
  assert.equal(sha(fs.readFileSync(driverPath)), suiteDriverSha, 'Exact actual suite producer required');
  const proofBytes = boundedRead(path.join(suiteRoot, 'proof.json'), 8 * 1024 * 1024);
  const proof = JSON.parse(proofBytes);
  assert.equal(proof.kind, 'combined_full_suite_v3');
  assert.equal(proof.result, 'PASS_COMBINED_WORKSPACE_V3_REAL_RENDERER_FMT_CLIPPY');
  assert.equal(proof.driver_sha256, suiteDriverSha);
  assert.equal(proof.source_manifest_sha256, expectedManifestSha);
  assert.equal(path.resolve(proof.source_directory), path.resolve(source));
  assert.equal(path.resolve(proof.source_worktree), path.resolve(repo));
  assert.equal(proof.source_inputs, manifest.files.length);
  assert.deepEqual(proof.source_manifest_metadata, Object.fromEntries(Object.entries(manifest)
    .filter(([key]) => !['files', 'required_tests'].includes(key))));
  assert.deepEqual(proof.source_lineage, lineage);
  assert.deepEqual(proof.test_profile, expectedTestProfile);
  assert.equal(proof.required_test_names, names.length);
  assert.equal(proof.required_test_names_verified, true);
  assert.equal(proof.minimum_expected, minimumExpected);
  assert.equal(proof.owned_renderer_gates_verified, true);
  assert.deepEqual(proof.owned_renderer_gates, rendererGates);
  assert.equal(proof.owned_worker_closed, true);
  assert.equal(proof.owned_fixture_closed, true);
  assert.equal(proof.source_final_verified, true);
  assert.equal(proof.user_renderer_contacted, false);
  assert.deepEqual(proof.failures, []);
  for (const port of [proof.owned_worker_port, proof.owned_fixture_port]) {
    assert(Number.isInteger(port) && port >= 1 && port <= 65535 && ![8787, 8790].includes(port));
  }
  assert.equal(proof.runs.length, 3);
  const digests = [{path: 'proof.json', sha256: sha(proofBytes), bytes: proofBytes.length}];
  let workspaceText, workspaceSummary;
  for (const [index, [name, args]] of suiteCommands.entries()) {
    const run = proof.runs[index];
    assert.equal(run.name, name);
    assert.deepEqual(run.args, args);
    assert(Number.isInteger(run.pid) && run.pid > 0);
    assert.equal(run.actual_close_event, true);
    assert.equal(run.spawn_error, null);
    assert.deepEqual(run.exit, {code: 0, signal: null});
    const log = boundedRead(path.join(suiteRoot, name + '.log'), 64 * 1024 * 1024);
    assert.equal(sha(log), run.log_sha256, 'Raw suite log SHA mismatch: ' + name);
    assert.equal(log.length, run.log_bytes);
    const summaries = parseSummaries(log.toString('utf8'));
    assert.deepEqual(summaries, run.summaries, 'Reparse actual suite summaries: ' + name);
    const json = boundedRead(path.join(suiteRoot, name + '.json'), 8 * 1024 * 1024);
    assert.deepEqual(JSON.parse(json), run, 'Actual individual child-close receipt differs');
    digests.push({path: name + '.log', sha256: sha(log), bytes: log.length},
      {path: name + '.json', sha256: sha(json), bytes: json.length});
    if (name === 'workspace-live') { workspaceText = log.toString('utf8'); workspaceSummary = summaries; }
  }
  assert(workspaceSummary.length > 0, 'A PASS label with no actual tests is rejected');
  assert(workspaceSummary.every(summary => summary.state === 'ok'));
  const passed = workspaceSummary.reduce((n, x) => n + x.passed, 0);
  const failed = workspaceSummary.reduce((n, x) => n + x.failed, 0);
  const ignored = workspaceSummary.reduce((n, x) => n + x.ignored, 0);
  assert(passed >= minimumExpected);
  assert.equal(failed, 0); assert.equal(ignored, 0);
  assert.equal(proof.passed, passed); assert.equal(proof.failed, failed); assert.equal(proof.ignored, ignored);
  const seen = new Map();
  for (const match of workspaceText.matchAll(/^test ([^\r\n]+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$/gm)) {
    const leaf = match[1].split('::').at(-1);
    if (!seen.has(leaf)) seen.set(leaf, []);
    seen.get(leaf).push({full: match[1], status: match[2]});
  }
  const rows = [...names, ...rendererGates].map(name => {
    const found = seen.get(name) ?? [];
    assert.equal(found.length, 1, 'Missing/duplicate actual suite test: ' + name);
    assert.equal(found[0].status, 'ok', 'Actual suite test not green: ' + name);
    return {required: name, actual: found[0].full, status: found[0].status};
  });
  const nameBytes = boundedRead(path.join(suiteRoot, 'required-test-results.json'), 8 * 1024 * 1024);
  assert.deepEqual(JSON.parse(nameBytes), {required_names: names.length, renderer_names: 4,
    results: rows, passed, failed, ignored});
  digests.push({path: 'required-test-results.json', sha256: sha(nameBytes), bytes: nameBytes.length});
  const workerBytes = boundedRead(path.join(suiteRoot, 'owned-worker.json'), 8 * 1024 * 1024);
  const worker = JSON.parse(workerBytes);
  assert.equal(worker.origin, 'http://127.0.0.1:' + proof.owned_worker_port);
  assert.equal(worker.fixture_origin, 'http://127.0.0.1:' + proof.owned_fixture_port);
  assert.equal(worker.health.browser.available, true);
  assert.equal(worker.warmup.title, 'Owned ready');
  assert.equal(worker.warmup.status, 200);
  assert.equal(worker.warmup.final_url, worker.fixture_origin + '/');
  assert.equal(worker.user_renderer_contacted, false);
  assert.deepEqual(worker.renderer_input_hashes, manifest.files.filter(file => file.path.startsWith('renderer/')));
  digests.push({path: 'owned-worker.json', sha256: sha(workerBytes), bytes: workerBytes.length});
  return {proof_sha256: sha(proofBytes), source_manifest_sha256: expectedManifestSha,
    suite_driver_sha256: suiteDriverSha, passed, failed, ignored, required_names: names.length, test_profile: expectedTestProfile, source_lineage: lineage,
    renderer_gates: 4, actual_three_closes: true, owned_resources_closed: true,
    final_sources_verified: true, independently_reparsed_logs: true, digests};
}
function verifySource() {
  assert.equal(sha(fs.readFileSync(self)), selfSha, 'Builder changed');
  assert.equal(sha(fs.readFileSync(path.join(source, 'manifest.json'))), expectedManifestSha);
  assert.equal(sha(fs.readFileSync(baselinePath)), baselineSha, 'Predecessor manifest changed');
  assert.equal(sha(fs.readFileSync(previousAttemptPath)), previousAttemptSha, 'Previous attempt manifest changed');
  for (const file of manifest.files) {
    for (const directory of [source, repo]) {
      const bytes = fs.readFileSync(path.join(directory, file.path));
      assert.equal(bytes.length, file.bytes, file.path + ' bytes');
      assert.equal(sha(bytes), file.sha256, directory + '/' + file.path + ' SHA');
    }
  }
}
// A missing/running/failed suite stops here, before mkdir or any child spawn.
verifySource();
const admittedSuite = verifySuite();
fs.mkdirSync(out);
const temp = path.join(out, 'temp'); fs.mkdirSync(temp);
const env = {...process.env, CARGO_TARGET_DIR: path.join(base, 'cargo-target'), CARGO_INCREMENTAL: '0',
  CARGO_PROFILE_DEV_DEBUG: '0', CARGO_PROFILE_TEST_DEBUG: '0', CARGO_TERM_COLOR: 'never', TEMP: temp, TMP: temp};
const runs = [], failures = [];
let binary = null, previous = null, sourceFinal = false, suiteFinal = false;
const write = (name, value) => fs.writeFileSync(path.join(out, name), JSON.stringify(value, null, 2) + '\n', {flag: 'wx'});
async function command(name, executable, args) {
  const logPath = path.join(out, name + '.log'), fd = fs.openSync(logPath, 'wx');
  const start = process.hrtime.bigint();
  let child, spawnError = null, exit;
  try {
    child = spawn(executable, args, {cwd: source, env, windowsHide: true, stdio: ['ignore', fd, fd]});
    child.once('error', error => { spawnError = error.message; });
    exit = await new Promise(resolve => child.once('close', (code, signal) => resolve({code, signal})));
  } finally { fs.closeSync(fd); }
  const bytes = boundedRead(logPath, 64 * 1024 * 1024);
  const record = {name, executable, args, pid: child?.pid ?? null, actual_close_event: true,
    exit, spawn_error: spawnError, elapsed_ms: Number(process.hrtime.bigint() - start) / 1e6,
    log_sha256: sha(bytes), log_bytes: bytes.length};
  runs.push(record); write(name + '.json', record);
  assert.equal(spawnError, null, spawnError ?? 'spawn');
  assert.deepEqual(exit, {code: 0, signal: null}, bytes.toString('utf8').slice(-3500));
  return bytes.toString('utf8');
}
function exclusiveCapture(destination, bytes) {
  const fd = fs.openSync(destination, 'wx');
  try { fs.writeFileSync(fd, bytes); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
  const saved = boundedRead(destination, 256 * 1024 * 1024);
  assert.equal(saved.length, bytes.length); assert.equal(sha(saved), sha(bytes));
  return {path: destination, bytes: saved.length, sha256: sha(saved)};
}
try {
  await command('cargo-version', cargo, ['--version', '--verbose']);
  await command('rustc-version', rustc, ['--version', '--verbose']);
  if (fs.existsSync(target)) previous = exclusiveCapture(path.join(out, 'previous-target.exe'), boundedRead(target, 256 * 1024 * 1024));
  const buildText = await command('build', cargo, ['build', '--release', '-p', 'open-seo-spider', '--locked']);
  assert(/Finished\s+`release`\s+profile\s+\[optimized\]/.test(buildText), 'Actual optimized release completion required');
  verifySource(); assert.deepEqual(verifySuite(), admittedSuite, 'Suite evidence changed during build');
  const bytes = boundedRead(target, 256 * 1024 * 1024);
  assert(bytes.length >= 64 && bytes[0] === 0x4d && bytes[1] === 0x5a, 'Actual Windows PE executable required');
  const pe = bytes.readUInt32LE(60);
  assert(pe <= bytes.length - 4 && bytes.subarray(pe, pe + 4).equals(Buffer.from([0x50, 0x45, 0, 0])));
  const candidate = path.join(out, 'candidate'); fs.mkdirSync(candidate);
  binary = exclusiveCapture(path.join(candidate, 'open-seo-spider.exe'), bytes);
  assert.equal(sha(boundedRead(target, 256 * 1024 * 1024)), binary.sha256, 'Shared target changed during capture');
} catch (error) {
  failures.push({phase: 'build', message: error.message, stack: error.stack});
  process.exitCode = 1;
} finally {
  try { verifySource(); sourceFinal = true; }
  catch (error) { failures.push({phase: 'final source guard', message: error.message}); }
  try { assert.deepEqual(verifySuite(), admittedSuite); suiteFinal = true; }
  catch (error) { failures.push({phase: 'final suite guard', message: error.message}); }
  if (binary) {
    try {
      const bytes = boundedRead(binary.path, 256 * 1024 * 1024);
      assert.equal(bytes.length, binary.bytes); assert.equal(sha(bytes), binary.sha256);
    } catch (error) { failures.push({phase: 'final candidate guard', message: error.message}); }
  }
  const allClosed = runs.length === 3 && runs.every(run => run.actual_close_event && !run.spawn_error
    && run.exit.code === 0 && run.exit.signal === null);
  const accepted = failures.length === 0 && binary !== null && allClosed && sourceFinal && suiteFinal;
  const result = accepted ? 'PASS_COMBINED_OPTIMIZED_BUILD_V3' : 'FAIL_COMBINED_OPTIMIZED_BUILD_V3';
  write('proof.json', {schema_version: 1, kind: 'combined_optimized_build_v3', result,
    recorded_at: new Date().toISOString(), builder_sha256: selfSha,
    source_manifest_sha256: expectedManifestSha, source_commit: manifest.source_commit,
    source_directory: source, source_worktree: repo, source_inputs: manifest.files.length, source_lineage: lineage,
    required_suite: {path: path.join(suiteRoot, 'proof.json'), ...admittedSuite},
    compiler_profile: {mode: 'optimized', lto: 'thin', codegen_units: 1, strip: true,
      declared_in: 'Cargo.toml', external_profile_overrides_rejected: true},
    source_final_verified: sourceFinal, suite_final_verified: suiteFinal,
    runs, previous, binary, failures,
    qualification: `Fresh actual cargo build --release -p open-seo-spider --locked after the independently reparsed same-v3-source PASS suite, all three real suite closes,${names.length} named tests,4 actual owned renderer gates,min${minimumExpected}/0failed/0ignored and owned-resource/final-source evidence. The admitted test profile is opt-level1 with debug assertions and overflow checks true, not release test semantics. All${manifest.files.length} frozen/Root bytes, parent67a manifest lineage, suite evidence and final candidate rehashed after actual build child close. Neither parent67a nor previous0bd2 Clippy-failed v2 is accepted. Shared incremental Cargo target is permitted; no clean rebuild claim. Candidate is a fresh exclusive-file capture in this result folder, never an installed-runtime replacement. Does not execute/install this binary or rebind native SDK/browser/physical harnesses, prove full parity, or establish resource/latency behavior.`});
  console.log(JSON.stringify({result, out, binary, failures: failures.map(failure => failure.message)}));
  if (!accepted) process.exitCode = 1;
}
