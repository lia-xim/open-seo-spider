import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath,pathToFileURL} from 'node:url';

const argv = process.argv.slice(2);
assert.equal(argv.length, 4, 'Required: --manifest-sha <exact64hex> --suite-proof-sha <actual64hex>');
assert.equal(argv[0], '--manifest-sha');
assert.match(argv[1], /^[a-f0-9]{64}$/);
assert.equal(argv[2], '--suite-proof-sha');
assert.match(argv[3], /^[a-f0-9]{64}$/);
const expectedManifestSha = argv[1];
const expectedSuiteProofSha = argv[3];
assert(!process.env.NODE_OPTIONS, 'Node preloads are not accepted');
assert.deepEqual(process.execArgv, [], 'Node runtime flags are not accepted');
const suiteDriverSha = '921a8023a3354c42ef28583888734bbe6c4784794161ec5e5f5c8f53dd39f62b';
const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const base = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const source = path.join(base, 'combined-source-v8');
const repo = path.join(base, 'mcp-file-tools-v1');
const suiteRoot = path.join(base, 'results/combined-suite-v8');
const out = path.join(base, 'results/combined-optimized-build-v8');
const target = path.join(base, 'cargo-target/release/open-seo-spider.exe');
const cargo = 'C:/Users/matth/.cargo/bin/cargo.exe';
const rustc = 'C:/Users/matth/.cargo/bin/rustc.exe';
const self = fileURLToPath(import.meta.url);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const selfSha = sha(fs.readFileSync(self));
const driverPath = path.join(task, 'work/run-combined-suite-v8.mjs');
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed',
  'Explicit coordinated compute release required');
assert.equal(path.resolve(self), path.resolve(task, 'work/build-combined-optimized-v8.mjs'),
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
const validatorPath = path.join(task, 'work/validate-combined-source-v8.mjs');
const validatorSha = 'a6ac87b82d367e269f6d021ff0570d50a793d4243511e550f294a471fd3b344f';
assert.equal(sha(fs.readFileSync(validatorPath)),validatorSha,'Exact reviewed Source8 validator required before import');
const {validateSource8Manifest} = await import(pathToFileURL(validatorPath));
assert.equal(sha(fs.readFileSync(validatorPath)),validatorSha,'Source8 validator changed during import');
const validation = validateSource8Manifest({manifest,sourceDirectory:source,repo});
const {files,required,baselinePath,baselineSha,rootApprovalReference,pinnedReferences,
  expectedNewFullNames,lineage} = validation;
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
  assert.equal(sha(proofBytes), expectedSuiteProofSha, 'Explicit actual suite proof hash required');
  const proof = JSON.parse(proofBytes);
  assert.equal(proof.kind, 'combined_full_suite_v8');
  assert.equal(proof.result, 'PASS_COMBINED_WORKSPACE_V8_REAL_RENDERER_FMT_CLIPPY');
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
  for (const expected of expectedNewFullNames) {
    const found = seen.get(expected.split('::').at(-1)) ?? [];
    assert.equal(found.length, 1, 'New authored full-module result missing/duplicate: ' + expected);
    assert.equal(found[0].full, expected, 'New authored function executed in wrong Cargo module');
    assert.equal(found[0].status, 'ok');
  }
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
  assert.equal(path.resolve(worker.renderer_runtime_root), path.resolve(task, 'outputs/open-seo-spider'));
  assert.deepEqual(worker.renderer_input_hashes, manifest.files.filter(file => file.path.startsWith('renderer/')));
  assert.equal(worker.renderer_input_hashes.length, 4);
  for (const file of worker.renderer_input_hashes) {
    const bytes = boundedRead(path.join(worker.renderer_runtime_root, file.path), 64 * 1024 * 1024);
    assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256);
  }
  assert.equal(path.resolve(worker.chrome_path), path.resolve('C:/Program Files/Google/Chrome/Application/chrome.exe'));
  assert.match(worker.chrome_sha256, /^[a-f0-9]{64}$/);
  assert.equal(sha(boundedRead(worker.chrome_path, 256 * 1024 * 1024)), worker.chrome_sha256);
  digests.push({path: 'owned-worker.json', sha256: sha(workerBytes), bytes: workerBytes.length});
  return {proof_sha256: sha(proofBytes), source_manifest_sha256: expectedManifestSha,
    suite_driver_sha256: suiteDriverSha, passed, failed, ignored, required_names: names.length, test_profile: expectedTestProfile, source_lineage: lineage,
    renderer_gates: 4, actual_three_closes: true, owned_resources_closed: true,
    new_authored_full_names: expectedNewFullNames, new_authored_full_names_verified: true,
    final_sources_verified: true, independently_reparsed_logs: true, digests};
}
function verifySource() {
  assert.equal(sha(fs.readFileSync(self)), selfSha, 'Builder changed');
  assert.equal(sha(fs.readFileSync(path.join(source, 'manifest.json'))), expectedManifestSha);
  assert.equal(sha(fs.readFileSync(baselinePath)), baselineSha, 'Predecessor manifest changed');
  assert.equal(sha(fs.readFileSync(rootApprovalReference.path)), rootApprovalReference.sha256, 'Root final-byte approval changed');
  for (const reference of pinnedReferences.values()) {
    const bytes = fs.readFileSync(reference.path);
    assert.equal(bytes.length, reference.bytes); assert.equal(sha(bytes), reference.sha256, 'Reviewed immutable reference changed');
  }
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
  const result = accepted ? 'PASS_COMBINED_OPTIMIZED_BUILD_V8' : 'FAIL_COMBINED_OPTIMIZED_BUILD_V8';
  write('proof.json', {schema_version: 1, kind: 'combined_optimized_build_v8', result,
    recorded_at: new Date().toISOString(), builder_sha256: selfSha,
    source_manifest_sha256: expectedManifestSha, explicit_suite_proof_sha256: expectedSuiteProofSha, source_commit: manifest.source_commit,
    source_directory: source, source_worktree: repo, source_inputs: manifest.files.length, source_lineage: lineage,
    required_suite: {path: path.join(suiteRoot, 'proof.json'), ...admittedSuite},
    compiler_profile: {mode: 'optimized', lto: 'thin', codegen_units: 1, strip: true,
      declared_in: 'Cargo.toml', external_profile_overrides_rejected: true},
    source_final_verified: sourceFinal, suite_final_verified: suiteFinal,
    runs, previous, binary, failures,
    qualification: `Fresh actual optimized Source8 candidate after actual current335 inputs,540 names/min1322,66 full Cargo modules,4 renderer gates and3Cargo/resource closes. Exact formatter/preliminary/private approval and immutable successful Source7 parent raw verification retained. Actual WindowsPE exclusive fsynced candidate and all native results are separate; no installation or manufacturer/RSS/performance claim.`});
  console.log(JSON.stringify({result, out, binary, failures: failures.map(failure => failure.message)}));
  if (!accepted) process.exitCode = 1;
}
