// Exact reviewed R2 builder verifySuite body, parent-only frozen reads.
// Neither the failed predecessor nor any R2 optimized/native run is acceptance.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';
export function verifyParentSource7Suite(expectedSuiteProofSha){
assert.match(expectedSuiteProofSha,/^[a-f0-9]{64}$/);
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',task='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const source=path.join(base,'combined-source-v7'),repo=path.join(base,'mcp-file-tools-v1'),suiteRoot=path.join(base,'results/combined-suite-v7');
const expectedManifestSha='fd587ba252595d971e7557e5cca7f5ab90219ecda200a3f07fcd5c10c13b6397',driverPath=path.join(task,'work/run-combined-suite-v7.mjs'),suiteDriverSha='576bc7a91d44281593c4f61a48d72714c7aead0279bfe50202afcf673f82029c';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex'),reads=new Map();
function boundedRead(file,max){const s=fs.lstatSync(file);assert(s.isFile()&&!s.isSymbolicLink()&&s.size<=max,file);const raw=fs.readFileSync(file);assert.equal(raw.length,s.size);const key=path.resolve(file).toLowerCase(),row={path:file,sha256:sha(raw),bytes:raw.length};if(reads.has(key))assert.deepEqual(reads.get(key),row);else reads.set(key,row);return raw;}
const sourceRaw=boundedRead(path.join(source,'manifest.json'),8*1024*1024);assert.equal(sha(sourceRaw),expectedManifestSha);const manifest=JSON.parse(sourceRaw);assert.equal(manifest.kind,'combined_source_v7');assert.equal(manifest.source_commit,'e96d1d17575896e351141fa51d296fede9908c2f');assert.equal(manifest.files.length,326);assert.equal(manifest.required_test_names,515);assert.equal(manifest.minimum_expected_workspace_tests,1297);
for(const row of manifest.files){const raw=boundedRead(path.join(source,row.path),64*1024*1024);assert.equal(sha(raw),row.sha256);assert.equal(raw.length,row.bytes);}
assert.equal(sha(boundedRead(driverPath,1024*1024)),suiteDriverSha);
const previousRaw=boundedRead(path.join(base,'combined-source-v6/manifest.json'),8*1024*1024);assert.equal(sha(previousRaw),'1210c227ac90c1936008d7667c877fb6e18d661a2925be5d4cf36d0e447d194e');const previous=JSON.parse(previousRaw);
const repairRaw=boundedRead(path.join(base,'combined-source-v6-r2/manifest.json'),8*1024*1024);assert.equal(sha(repairRaw),'4572dc39ee478b8b3d55d06b5983779d6a42416c307256780050dc5950c49b77');const repair=JSON.parse(repairRaw);
const names=manifest.required_tests.flatMap(g=>g.names),minimumExpected=1297,expectedNewFullNames=[...previous.new_authored_full_test_names,...repair.new_authored_full_test_names,...manifest.new_authored_full_test_names].sort();assert.equal(names.length,515);assert.equal(expectedNewFullNames.length,41);assert.equal(new Set(expectedNewFullNames).size,41);
const lineage=Object.freeze({parent_source_manifest_sha256:manifest.parent_source_manifest_sha256,parent_suite_proof_sha256:manifest.parent_suite.sha256,incoming_transport_proposal_sha256:manifest.source_proposals[0].sha256,root_approval_sha256:manifest.root_approval.sha256,parent_inputs:324,required_parent_names:502,unchanged_parent_inputs:317,changed_parent_inputs:7,new_input_files:2,delta_bindings:9,new_named_windows_functions:manifest.new_authored_tests.flatMap(g=>g.names),actual_required_names:515,actual_source_inputs:326,minimum_expected_workspace_tests:1297});
const expectedTestProfile = Object.freeze({opt_level: 1, debug_assertions: true,
  overflow_checks: true, debug_info: 0, environment: {
    CARGO_PROFILE_TEST_OPT_LEVEL: '1', CARGO_PROFILE_TEST_DEBUG_ASSERTIONS: 'true',
    CARGO_PROFILE_TEST_OVERFLOW_CHECKS: 'true', CARGO_PROFILE_TEST_DEBUG: '0',
    CARGO_PROFILE_DEV_DEBUG: '0',
  }});
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
  assert.equal(proof.kind, 'combined_full_suite_v7');
  assert.equal(proof.result, 'PASS_COMBINED_WORKSPACE_V7_REAL_RENDERER_FMT_CLIPPY');
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
const verified=verifySuite();return {verified,references:[...reads.values()]};
}
