import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';

// Caller pins this module before import. Its one reviewed source-only dependency
// is hashed before and after ESM import. bindCombined remains synchronous after
// the caller has awaited this module's import; no fixture/compiler is started.
const validatorPath='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/work/validate-combined-source-v8.mjs';
const validatorSHA='a6ac87b82d367e269f6d021ff0570d50a793d4243511e550f294a471fd3b344f';
const validatorHash=()=>createHash('sha256').update(fs.readFileSync(validatorPath)).digest('hex');
assert.equal(validatorHash(),validatorSHA,'Exact reviewed Source8 validator required before import');
const {validateSource8Manifest}=await import(pathToFileURL(validatorPath));
assert.equal(validatorHash(),validatorSHA,'Source8 validator changed during import');

// This module performs only bounded local reads and hashes. Its caller validates
// the exact operational release and module SHA before importing/calling it.
export function bindCombined(release, expectedManifestSha) {
assert.match(expectedManifestSha, /^[a-f0-9]{64}$/);
assert.notEqual(expectedManifestSha,'27a8b74c4cd0c394906f3fe3c0a6a0327569294b25056cf6c6f0aabcffc42788','Source5 R3 is parent history only; explicit actual Source8 manifest required');
for (const key of ['source_manifest_sha256','suite_proof_sha256','build_proof_sha256','candidate_binary_sha256','build_driver_sha256']) assert.match(release[key],/^[a-f0-9]{64}$/);
assert.equal(release.source_manifest_sha256,expectedManifestSha);
const base = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const source = path.join(base, 'combined-source-v8'), repo = path.join(base, 'mcp-file-tools-v1');
const suiteRoot = path.join(base, 'results/combined-suite-v8');
const buildRoot = path.join(base, 'results/combined-optimized-build-v8');
const driverPath = path.join(task, 'work/run-combined-suite-v8.mjs');
const suiteDriverSha = '921a8023a3354c42ef28583888734bbe6c4784794161ec5e5f5c8f53dd39f62b';
const builderPath = path.join(task, 'work/build-combined-optimized-v8.mjs');
const builderSha = release.build_driver_sha256;
assert.equal(builderSha,'e48e2cadc361a100eecbbed5465bf4436d28f7fa19068682550932f320e1c547');
assert.notEqual(release.candidate_binary_sha256,'c38f5743685353ab54331e074d2a20ac38543dfc4750c6896157218821b65348','Source4 candidate is historical only');
const expectedSuiteProofSha=release.suite_proof_sha256;
// Current source publication is bound below from the immutable Root approval.
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
assert.equal(manifest.kind, 'combined_source_v8');
assert.equal(manifest.result,'FROZEN_REVIEWED_SOURCE_FULL_SUITE_NOT_RUN');
assert.match(manifest.source_commit,/^[a-f0-9]{40}$/,'Actual Source8 commit must be bound by its reviewed Root approval');
assert.equal(manifest.freezer_sha256,'b86e687e8b12050fdf772ac996e95a9e7103a6b17f068083a716861901d9945a');
assert.equal(sha(boundedRead(path.join(task,'work/freeze-combined-source-v8.mjs'),1024*1024)),manifest.freezer_sha256);
assert.equal(validatorHash(),validatorSHA);
const validation=validateSource8Manifest({manifest,sourceDirectory:source,repo});
const {files,required,baselinePath,baselineSha,rootApprovalReference,pinnedReferences,
  expectedNewFullNames,lineage}=validation;
assert.equal(files.size,335);
assert.equal(manifest.source_inputs,335);
assert.equal(required.length,validation.baseline.required_test_names+validation.newFullNames.length);
assert.equal(expectedNewFullNames.length,41+validation.newFullNames.length);
assert.equal(manifest.minimum_expected_workspace_tests,validation.minimum);
const rootApproval=validation.rootApproval;
const baseline=validation.baseline;
const publicationPath=rootApprovalReference.path,publicationSha=rootApprovalReference.sha256;
assert.equal(manifest.actual_private_publication.isPrivate,true);
assert.equal(manifest.actual_private_publication.nameWithOwner,'lia-xim/open-seo-spider');
assert.equal(manifest.private_repository,'lia-xim/open-seo-spider');
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
  assert.equal(validatorHash(),validatorSHA,'Source8 validator changed');
  assert.equal(sha(boundedRead(path.join(task,'work/freeze-combined-source-v8.mjs'),1024*1024)),manifest.freezer_sha256,'Actual Source8 freezer source changed');
  assert.equal(sha(boundedRead(path.join(source, 'manifest.json'), 8 * 1024 * 1024)), expectedManifestSha);
  assert.equal(sha(boundedRead(baselinePath, 8 * 1024 * 1024)), baselineSha);
  assert.equal(sha(boundedRead(rootApprovalReference.path,8*1024*1024)),rootApprovalReference.sha256);
  for (const reference of pinnedReferences.values()) {
    const bytes=boundedRead(reference.path,8*1024*1024);
    assert.equal(bytes.length,reference.bytes);assert.equal(sha(bytes),reference.sha256);
  }
  for (const file of manifest.files) for (const directory of [source, repo]) {
    const bytes = fs.readFileSync(path.join(directory, file.path));
    assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256);
  }
  assert.equal(sha(boundedRead(publicationPath, 1024 * 1024)), publicationSha);
  assert.equal(sha(boundedRead(builderPath, 1024 * 1024)), builderSha);
}
verifySource();
const publication=JSON.parse(boundedRead(publicationPath,1024*1024));
assert.deepEqual(publication,rootApproval);
assert.equal(publication.kind,'root_approved_combined_source_v8');
assert.equal(publication.status,'REVIEWED_MERGED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED');
assert.equal(publication.source_commit,manifest.source_commit);
// The source manifest captures actual private publication; this is provenance,
// never borrowed V3 suite/build/native acceptance or a live Git HEAD comparison.
const suitePath = path.join(suiteRoot, 'proof.json');
const suiteRaw = boundedRead(suitePath, 8 * 1024 * 1024), suite = JSON.parse(suiteRaw);
const admittedSuite = verifySuite();
assert.equal(sha(suiteRaw), admittedSuite.proof_sha256, 'Suite bytes changed during binding');
function verifyBuild() {
  assert.equal(sha(boundedRead(builderPath, 1024 * 1024)), builderSha);
  const raw = boundedRead(path.join(buildRoot, 'proof.json'), 8 * 1024 * 1024), build = JSON.parse(raw);
  assert.equal(sha(raw), release.build_proof_sha256);
  assert.equal(build.kind, 'combined_optimized_build_v8');
  assert.equal(build.result, 'PASS_COMBINED_OPTIMIZED_BUILD_V8');
  assert.equal(build.builder_sha256, builderSha);
  assert.equal(build.explicit_suite_proof_sha256,expectedSuiteProofSha);
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
const publicationLineage = Object.freeze({scope:'current_private_source_approval_provenance_only',path:publicationPath,sha256:publicationSha,
  current_commit:manifest.source_commit,parent_source_commit:baseline.source_commit,parent_source_manifest_sha256:baselineSha,current_source_manifest_sha256:expectedManifestSha,
  private_current_repository:true,current_source_inputs:files.size,current_publication_required:true,publication_is_acceptance:false});
function verify() {
  verifySource(); assert.deepEqual(verifySuite(), admittedSuite); assert.deepEqual(verifyBuild(), admittedBuild);
}
return {manifest, sourceRaw, sourceLineage, publicationLineage, suitePath, suiteRaw, suite,
  actualWorkspace: {passed: admittedSuite.passed, failed: 0, ignored: 0, required_source_names: names.length, required_renderer_gates: 4},
  buildRoot, buildRaw, build, binary: admittedBuild.binary_path, binarySHA: admittedBuild.binary_sha256, verify};
}
