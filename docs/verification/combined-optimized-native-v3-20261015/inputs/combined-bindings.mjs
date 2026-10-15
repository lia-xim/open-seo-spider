import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';

// This module performs only bounded local reads and hashes. Its caller validates
// the exact operational release and module SHA before importing/calling it.
export function bindCombined(release, expectedManifestSha) {
assert.match(expectedManifestSha, /^[a-f0-9]{64}$/);
const base = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const source = path.join(base, 'combined-source-v3'), repo = path.join(base, 'mcp-file-tools-v1');
const suiteRoot = path.join(base, 'results/combined-suite-v3');
const buildRoot = path.join(base, 'results/combined-optimized-build-v3');
const driverPath = path.join(task, 'work/run-combined-suite-v3.mjs');
const suiteDriverSha = 'e6ccab255c9056f6791a02fde68672a8e91ccc98c9069d9538e7ebfbf1728d4d';
const builderPath = path.join(task, 'work/build-combined-optimized-v3.mjs');
const builderSha = '878c981a1645205f2ba958775ca318b95abbde62b6dda226ff55dcf2de82f983';
const publicationPath = path.join(task, 'work/combined-source-published-v1.json');
const publicationSha = '65855e2610ec378ce986fee2d6cbfba204b547bd59c7bf6227843d96b48a29c6';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function boundedRead(file, max) {
  const stat = fs.statSync(file);
  assert(stat.isFile() && stat.size <= max, 'Bounded regular file required: ' + file);
  const bytes = fs.readFileSync(file);
  assert.equal(bytes.length, stat.size);
  return bytes;
}
const sourceRaw = boundedRead(path.join(source, 'manifest.json'), 8 * 1024 * 1024);
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
  assert.equal(sha(boundedRead(path.join(source, 'manifest.json'), 8 * 1024 * 1024)), expectedManifestSha);
  assert.equal(sha(boundedRead(baselinePath, 8 * 1024 * 1024)), baselineSha);
  assert.equal(sha(boundedRead(previousAttemptPath, 8 * 1024 * 1024)), previousAttemptSha);
  for (const file of manifest.files) for (const directory of [source, repo]) {
    const bytes = fs.readFileSync(path.join(directory, file.path));
    assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256);
  }
  assert.equal(sha(boundedRead(publicationPath, 1024 * 1024)), publicationSha);
  assert.equal(sha(boundedRead(builderPath, 1024 * 1024)), builderSha);
}
verifySource();
const publication = JSON.parse(boundedRead(publicationPath, 1024 * 1024));
assert.equal(publication.kind, 'private_combined_source_publication_v1');
assert.equal(publication.commit, baseline.source_commit);
assert.equal(publication.source_manifest_sha256, baselineSha);
assert.equal(publication.source_inputs, 301);
assert.equal(publication.repository.isPrivate, true);
assert.equal(publication.repository.nameWithOwner, 'lia-xim/open-seo-spider');
assert.equal(publication.remote_exact, true);
// Publication says the suite was running; it is provenance, never acceptance.
const suitePath = path.join(suiteRoot, 'proof.json');
const suiteRaw = boundedRead(suitePath, 8 * 1024 * 1024), suite = JSON.parse(suiteRaw);
const admittedSuite = verifySuite();
assert.equal(sha(suiteRaw), admittedSuite.proof_sha256, 'Suite bytes changed during binding');
function verifyBuild() {
  assert.equal(sha(boundedRead(builderPath, 1024 * 1024)), builderSha);
  const raw = boundedRead(path.join(buildRoot, 'proof.json'), 8 * 1024 * 1024), build = JSON.parse(raw);
  assert.equal(sha(raw), release.build_proof_sha256);
  assert.equal(build.kind, 'combined_optimized_build_v3');
  assert.equal(build.result, 'PASS_COMBINED_OPTIMIZED_BUILD_V3');
  assert.equal(build.builder_sha256, builderSha);
  assert.equal(build.source_manifest_sha256, expectedManifestSha);
  assert.equal(build.source_commit, manifest.source_commit);
  assert.equal(path.resolve(build.source_directory), path.resolve(source));
  assert.equal(path.resolve(build.source_worktree), path.resolve(repo));
  assert.equal(build.source_inputs, manifest.files.length);
  assert.deepEqual(build.source_lineage, lineage);
  assert.equal(build.source_final_verified, true); assert.equal(build.suite_final_verified, true);
  assert.deepEqual(build.failures, []);
  assert.deepEqual(build.required_suite, {path: suitePath, ...admittedSuite});
  assert.deepEqual(build.compiler_profile, {mode: 'optimized', lto: 'thin', codegen_units: 1, strip: true,
    declared_in: 'Cargo.toml', external_profile_overrides_rejected: true});
  const expected = [
    ['cargo-version', 'C:/Users/matth/.cargo/bin/cargo.exe', ['--version', '--verbose']],
    ['rustc-version', 'C:/Users/matth/.cargo/bin/rustc.exe', ['--version', '--verbose']],
    ['build', 'C:/Users/matth/.cargo/bin/cargo.exe', ['build', '--release', '-p', 'open-seo-spider', '--locked']],
  ];
  assert.equal(build.runs.length, 3);
  const digests = [{path: 'proof.json', sha256: sha(raw), bytes: raw.length}];
  for (const [i, [name, executable, args]] of expected.entries()) {
    const run = build.runs[i]; assert.equal(run.name, name); assert.deepEqual(run.args, args);
    assert.equal(run.executable, executable); assert(Number.isSafeInteger(run.pid) && run.pid > 0);
    assert.equal(run.actual_close_event, true); assert.equal(run.spawn_error, null);
    assert.deepEqual(run.exit, {code: 0, signal: null});
    const log = boundedRead(path.join(buildRoot, name + '.log'), 64 * 1024 * 1024);
    assert.equal(sha(log), run.log_sha256); assert.equal(log.length, run.log_bytes);
    const json = boundedRead(path.join(buildRoot, name + '.json'), 8 * 1024 * 1024);
    assert.deepEqual(JSON.parse(json), run);
    if (name === 'build') assert(/Finished\s+`release`\s+profile\s+\[optimized\]/.test(log.toString('utf8')));
    digests.push({path: name + '.log', sha256: sha(log), bytes: log.length}, {path: name + '.json', sha256: sha(json), bytes: json.length});
  }
  const binary = path.join(buildRoot, 'candidate/open-seo-spider.exe');
  assert.equal(path.resolve(build.binary.path), path.resolve(binary));
  const bytes = boundedRead(binary, 256 * 1024 * 1024), hash = sha(bytes);
  assert.equal(hash, build.binary.sha256); assert.equal(hash, release.candidate_binary_sha256);
  assert.equal(bytes.length, build.binary.bytes);
  assert(bytes.length >= 64 && bytes[0] === 0x4d && bytes[1] === 0x5a);
  const pe = bytes.readUInt32LE(60);
  assert(pe <= bytes.length - 4 && bytes.subarray(pe, pe + 4).equals(Buffer.from([0x50, 0x45, 0, 0])));
  return {proof_sha256: sha(raw), binary_path: binary, binary_sha256: hash, binary_bytes: bytes.length, digests};
}
const admittedBuild = verifyBuild();
const buildRaw = boundedRead(path.join(buildRoot, 'proof.json'), 8 * 1024 * 1024), build = JSON.parse(buildRaw);
assert.equal(sha(buildRaw), admittedBuild.proof_sha256, 'Build bytes changed during binding');
const sourceLineage = lineage;
const publicationLineage = Object.freeze({scope: 'parent_provenance_only', path: publicationPath, sha256: publicationSha,
  parent_commit: publication.commit, parent_source_manifest_sha256: baselineSha, current_source_manifest_sha256: expectedManifestSha,
  private_parent_repository: true, current_publication_required: false, publication_is_acceptance: false});
function verify() {
  verifySource(); assert.deepEqual(verifySuite(), admittedSuite); assert.deepEqual(verifyBuild(), admittedBuild);
}
return {manifest, sourceRaw, sourceLineage, publicationLineage, suitePath, suiteRaw, suite,
  actualWorkspace: {passed: admittedSuite.passed, failed: 0, ignored: 0, required_source_names: names.length, required_renderer_gates: 4},
  buildRoot, buildRaw, build, binary: admittedBuild.binary_path, binarySHA: admittedBuild.binary_sha256, verify};
}
