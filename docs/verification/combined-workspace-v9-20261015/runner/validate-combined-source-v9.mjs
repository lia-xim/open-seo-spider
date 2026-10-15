// Direct source admission only. No compiler, network, Git, or recursive old validators.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
export const PARENT_SHA='58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720';
const PARENT_SUITE_SHA='7266c3087ca3b3998116f18d6ba05515c88bcf25f86daa5537ec3206201468f1';
const COMPOSITION_SHA='ade29eed3c3be14028e0b099e33e38b9020de394c0b3b6986485a5a4f1d77367';
const COMPOSITION_REVIEW_SHA='670119f340e26c4cd9ba94f0db42b3393f16e51b9d9686a0907b57a85a69fb51';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const normalized=p=>path.resolve(p).toLowerCase();
const HASH=/^[a-f0-9]{64}$/;
const profile=Object.freeze({opt_level:1,debug_assertions:true,overflow_checks:true,debug_info:0,environment:{CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0'}});
const gates=['live_browser_worker_executes_javascript_for_core_crawl','live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness','live_browser_worker_pause_drains_render_and_retains_rendered_discoveries','live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion'];
export function relativeFile(value){assert.equal(typeof value,'string');assert(value&&!value.includes('\\')&&!value.includes(':')&&!path.posix.isAbsolute(value));assert(value.split('/').every(p=>p&&p!=='.'&&p!=='..'));return value;}
function declaredTests(text){const rows=[];for(const m of text.matchAll(/((?:#\[[^\]]*\]\s*)+)(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)\b/g))if(/#\[(?:tokio::)?test(?:\(|\])/.test(m[1]))rows.push({name:m[2],platform:/#\[cfg\(unix\)\]/.test(m[1])?'unix':/#\[cfg\(windows\)\]/.test(m[1])?'windows':'all'});return rows;}
function rowsMap(rows,count){assert(Array.isArray(rows));assert.equal(rows.length,count);const map=new Map();for(const f of rows){relativeFile(f.path);assert.match(f.sha256,HASH);assert(Number.isSafeInteger(f.bytes)&&f.bytes>=0);assert(!map.has(f.path));map.set(f.path,f);}assert.equal(new Set([...map.keys()].map(p=>p.toLowerCase())).size,count);return map;}
function sameRef(a,b){assert.equal(normalized(a.path),normalized(b.path));assert.equal(a.sha256,b.sha256);}
const modules={
 'crates/app/src/broken_sources_tests.rs':['crates/app/src/main.rs','broken_sources','crates/app/src/broken_sources.rs','broken_sources::tests::'],
 'crates/app/src/resource_broken_sources_app_tests.rs':['crates/app/src/main.rs','resource_broken_sources_app','crates/app/src/resource_broken_sources_app.rs','resource_broken_sources_app::tests::'],
 'crates/app/src/server/broken_sources_tests.rs':['crates/app/src/server.rs','broken_sources','crates/app/src/server/broken_sources.rs','server::broken_sources::tests::'],
 'crates/app/src/server/resource_broken_sources_tests.rs':['crates/app/src/server.rs','resource_broken_sources','crates/app/src/server/resource_broken_sources.rs','server::resource_broken_sources::tests::'],
 'crates/core/src/resource_broken_sources_tests.rs':['crates/core/src/lib.rs','resource_broken_sources','crates/core/src/resource_broken_sources.rs','resource_broken_sources::tests::'],
 'crates/core/src/directory_tree_tests.rs':['crates/core/src/lib.rs','directory_tree','crates/core/src/directory_tree.rs','directory_tree::tests::'],
 'crates/app/src/server/directory_tree_tests.rs':['crates/app/src/server.rs','directory_tree','crates/app/src/server/directory_tree.rs','server::directory_tree::tests::'],
};
// Actual already-closed parent artifacts, not mutable task/work or live Root.
const parentArtifacts=[
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/proof.json",
    "sha256": "7266c3087ca3b3998116f18d6ba05515c88bcf25f86daa5537ec3206201468f1",
    "bytes": 118871
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/format.log",
    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "bytes": 0
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/format.json",
    "sha256": "64b1aed85b5498cc571cdbe03ddf3899d105fd869e69d6a21c091f5bb956533d",
    "bytes": 380
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/workspace-live.log",
    "sha256": "9da2744726818e84c9e53d4997aebea8c1fc922b158426281a4680573271f061",
    "bytes": 148953
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/workspace-live.json",
    "sha256": "1bf67b503f082f33edbc7413fa6ab7b0d7d485fae3320244f85217300b73d53e",
    "bytes": 2100
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/clippy.log",
    "sha256": "d118ca83af0d24b1461774424e92be017e9dbac69a7f6af2664f1f40395b159c",
    "bytes": 297
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/clippy.json",
    "sha256": "258163e02c706b01679463b1046c565df252e75f066baa46ba9b1a9714fcdf6e",
    "bytes": 450
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/required-test-results.json",
    "sha256": "75bd54f94edcaedb3e32581cfbe50a1fed4ccea853f6cac0b4a126dd2d340b1d",
    "bytes": 141351
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-suite-v8/owned-worker.json",
    "sha256": "43821ad2408961ae4d38b252df3b20765b52a5ce78d96a0672e03266521ad83e",
    "bytes": 2299
  },
  {
    "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/combined-v8-runner-preparation-v4/run-combined-suite-v8.mjs",
    "sha256": "921a8023a3354c42ef28583888734bbe6c4784794161ec5e5f5c8f53dd39f62b",
    "bytes": 16733
  }
];
const expectedParentLineage={
  "parent_source_manifest_sha256": "fd587ba252595d971e7557e5cca7f5ab90219ecda200a3f07fcd5c10c13b6397",
  "parent_suite_proof_sha256": "8c8cf67de2a2849ea2443f0a060f4812235a9c148f9e14e644fc64690d6fc9ea",
  "composition_proposal_sha256": "248231ea19203b85bb0b5a3af5d81037b17da710cfdda187b57524a72c734d25",
  "root_approval_sha256": "09dfa805ac19e6d8fe676bfa95c697a22c05f61a640a3706721621db227cc0aa",
  "parent_inputs": 326,
  "required_parent_names": 515,
  "unchanged_parent_inputs": 318,
  "changed_parent_inputs": 8,
  "new_input_files": 9,
  "delta_bindings": 17,
  "new_named_windows_functions": [
    "migration_cli_new_outputs_keep_mismatch_success_and_never_overwrite_inputs",
    "migration_workflow_complete_exports_preserve_unknown_and_formula_safe_originals",
    "migration_workflow_cursor_binds_actual_snapshot_csv_options_filter_and_limit",
    "migration_workflow_strict_request_and_independent_budgets_reject_before_evaluation",
    "migration_api_pages_exports_and_stale_reports_do_not_contact_observed_website",
    "migration_api_reuses_stable_readlock_authority_and_exposes_declared_import_scope",
    "response_channel_cancel_and_closed_before_enqueue_return_zero_progress_errors",
    "response_channel_default_write_all_preserves_exact_bytes_and_empty_writes_enqueue_nothing",
    "response_channel_write_returns_exact_one_chunk_prefix_and_leaves_tail_unaccepted",
    "migration_blocked_error_and_unfetched_proposed_redirect_target_never_pass",
    "migration_bom_quoted_csv_headers_non_ascii_and_input_hash_use_existing_importer",
    "migration_csv_mapping_remains_byte_bound_and_evaluation_has_no_crawl_side_effect",
    "migration_csv_order_duplicates_and_conflicting_expectations_are_preserved",
    "migration_exact_serialized_output_bound_and_repeated_projection_are_independent",
    "migration_fragments_are_retained_but_http_comparison_does_not_claim_anchor_navigation",
    "migration_import_row_field_input_and_observation_caps_fail_before_result_retention",
    "migration_incoherent_chain_status_continuity_tail_and_loop_are_not_success",
    "migration_invalid_mapping_fields_and_blank_records_keep_source_provenance",
    "migration_malformed_utf8_quoting_same_columns_and_unknown_options_are_rejected",
    "migration_matching_error_http_status_and_fetch_error_remain_failures",
    "migration_missing_and_duplicate_observations_are_unknown_without_latest_selection",
    "migration_multihop_chain_and_non_200_success_compare_actual_http_not_renderer",
    "migration_query_order_case_slash_scheme_and_encoded_spelling_remain_distinct",
    "migration_semicolon_tab_and_header_aliases_follow_existing_list_semantics",
    "migration_supplied_invalid_status_url_and_unexplained_final_url_are_explicit"
  ],
  "actual_required_names": 540,
  "actual_source_inputs": 335,
  "minimum_expected_workspace_tests": 1322
};
function verifyParentSuite(parent,pin){
 const task=TASK;
 const source=path.join(BASE,'combined-source-v8'),repo=path.join(BASE,'mcp-file-tools-v1');
 const suiteRoot=path.join(BASE,'results/combined-suite-v8');
 for(const r of parentArtifacts)assert.equal(pin(r.path,r.sha256).length,r.bytes);
 const namedReference=parentArtifacts.find(r=>r.path.endsWith('/required-test-results.json'));
 const expectedParentFullNames=JSON.parse(pin(namedReference.path,namedReference.sha256)).results.map(r=>r.actual).sort();
 const driverPath=path.join(BASE,'combined-v8-runner-preparation-v4/run-combined-suite-v8.mjs');
 const suiteDriverSha='921a8023a3354c42ef28583888734bbe6c4784794161ec5e5f5c8f53dd39f62b';
 const expectedSuiteProofSha=PARENT_SUITE_SHA,expectedManifestSha=PARENT_SHA;
 const manifest=parent,lineage=expectedParentLineage,names=parent.required_tests.flatMap(g=>g.names),minimumExpected=1322;
 const expectedTestProfile=profile,rendererGates=gates,expectedNewFullNames=expectedParentFullNames;
 const boundedRead=(p,max)=>{const s=fs.lstatSync(p);assert(s.isFile()&&!s.isSymbolicLink()&&s.size<=max);const b=fs.readFileSync(p);assert.equal(b.length,s.size);return b;};
 const parseSummaries=text=>[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
 const suiteCommands=[['format',['fmt','--all','--check']],['workspace-live',['test','--workspace','--locked','--','--include-ignored','--test-threads=1']],['clippy',['clippy','--workspace','--all-targets','--locked','--','-D','warnings']]];
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
    // Historical source is re-read only from immutable frozen Source8.
    const bytes = boundedRead(path.join(source, file.path), 64 * 1024 * 1024);
    assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256);
  }
  assert.equal(path.resolve(worker.chrome_path), path.resolve('C:/Program Files/Google/Chrome/Application/chrome.exe'));
  assert.match(worker.chrome_sha256, /^[a-f0-9]{64}$/);
  // The exact hashed historical worker receipt binds its observed Chrome SHA.
  // Current Chrome/runtime are rehashed by the fresh Source9 suite/build.
  // Re-reading mutable installed Chrome would invalidate an immutable old run.
  digests.push({path: 'owned-worker.json', sha256: sha(workerBytes), bytes: workerBytes.length});
  return {proof_sha256: sha(proofBytes), source_manifest_sha256: expectedManifestSha,
    suite_driver_sha256: suiteDriverSha, passed, failed, ignored, required_names: names.length, test_profile: expectedTestProfile, source_lineage: lineage,
    renderer_gates: 4, actual_three_closes: true, owned_resources_closed: true,
    new_authored_full_names: expectedNewFullNames, new_authored_full_names_verified: true,
    final_sources_verified: true, independently_reparsed_logs: true, digests};
}

 const verified=verifySuite();
 assert.equal(verified.passed,1322);assert.equal(verified.required_names,540);
 assert.equal(expectedNewFullNames.length,544);
 return {verified,full_names:expectedNewFullNames};
}
export function loadSource9Approval({approvalPath,approvalSha,expectedCommit,repo=path.join(BASE,'mcp-file-tools-v1')}){
 const references=new Map();
 function pin(file,hash){assert(path.isAbsolute(file));assert.match(hash,HASH);const s=fs.lstatSync(file);assert(s.isFile()&&!s.isSymbolicLink()&&s.size<=64*1024*1024,file);const raw=fs.readFileSync(file);assert.equal(raw.length,s.size);assert.equal(sha(raw),hash,file);const k=normalized(file),old=references.get(k);if(old)assert.equal(old.sha256,hash);else references.set(k,{path:file,sha256:hash,bytes:raw.length});return raw;}
 const data=r=>JSON.parse(pin(r.path,r.sha256));
 const baselinePath=path.join(BASE,'combined-source-v8/manifest.json'),baseline=data({path:baselinePath,sha256:PARENT_SHA});
 assert.equal(baseline.kind,'combined_source_v8');assert.equal(baseline.source_commit,'e6b1fa2f87d0b39ac7f83aba6cdecc2110de5a50');assert.equal(baseline.source_inputs,335);assert.equal(baseline.required_test_names,540);assert.equal(baseline.minimum_expected_workspace_tests,1322);assert.deepEqual(baseline.test_profile,{opt_level:1,debug_assertions:true,overflow_checks:true,debug_info:0});
 const parentFiles=rowsMap(baseline.files,335),parentRaw=new Map();for(const f of baseline.files){const raw=pin(path.join(path.dirname(baselinePath),f.path),f.sha256);assert.equal(raw.length,f.bytes);parentRaw.set(f.path,raw);}
 const parentSuiteVerified=verifyParentSuite(baseline,pin);
 const rootApprovalReference={path:approvalPath,sha256:approvalSha},rootApproval=data(rootApprovalReference);
 assert.equal(rootApproval.schema_version,1);assert.equal(rootApproval.kind,'root_approved_combined_source_v9');assert.equal(rootApproval.status,'REVIEWED_MERGED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED');assert.equal(rootApproval.parent_source_manifest_sha256,PARENT_SHA);assert.match(rootApproval.source_commit,/^[a-f0-9]{40}$/);if(expectedCommit)assert.equal(rootApproval.source_commit,expectedCommit);assert.notEqual(rootApproval.source_commit,baseline.source_commit);assert.equal(rootApproval.source_branch,baseline.source_branch);assert.equal(normalized(rootApproval.source_worktree),normalized(repo));assert.equal(rootApproval.private_repository_verified,true);
 const parentSuiteReference={path:path.join(BASE,'results/combined-suite-v8/proof.json'),sha256:PARENT_SUITE_SHA};sameRef(rootApproval.parent_suite,parentSuiteReference);
 assert.deepEqual(rootApproval.proposals.map(r=>r.role),['composition']);const proposal=rootApproval.proposals[0];assert.equal(proposal.sha256,COMPOSITION_SHA);assert.equal(proposal.review_sha256,COMPOSITION_REVIEW_SHA);assert.equal(normalized(proposal.path),normalized(path.join(BASE,'combined-source-v9-draft-v6/preformat.json')));
 const composition=data(proposal),compositionReview=data({path:proposal.review_path,sha256:proposal.review_sha256});
 assert.equal(composition.kind,'root_source9_complete_composition_before_format');assert.equal(composition.source_inputs,358);assert.equal(composition.changed_parent_inputs,7);assert.equal(composition.new_input_files,23);assert.equal(composition.unchanged_parent_inputs,328);assert.equal(composition.forecast_required_test_names,577);assert.equal(composition.forecast_minimum_workspace_tests,1359);sameRef(composition.parent_manifest,{path:baselinePath,sha256:PARENT_SHA});assert.equal(composition.parent_source_commit,baseline.source_commit);sameRef(composition.parent_suite,parentSuiteReference);assert.equal(compositionReview.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(compositionReview.findings,[]);sameRef(compositionReview.proposal_manifest,proposal);sameRef(compositionReview.composition_preformat,proposal);
 const roles=['resource','header','directory'];assert.deepEqual(rootApproval.source_reviews.map(r=>r.role),roles);
 const rawRoleFiles=new Map(),rawProposalBytes=new Map();
 for(const row of rootApproval.source_reviews){const ref=composition[row.role+'_proposal'];sameRef(row.proposal_manifest,ref);const value=data(ref),review=data(row.review);assert.equal(review.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(review.findings,[]);sameRef(review.proposal_manifest,ref);sameRef(compositionReview.proposal_manifests.find(r=>r.sha256===ref.sha256),ref);const reviewed=compositionReview.source_reviews.find(r=>r.sha256===row.review.sha256);assert(reviewed);sameRef(reviewed,row.review);if(composition[row.role+'_review'])sameRef(composition[row.role+'_review'],row.review);sameRef(value.parent_manifest,{path:baselinePath,sha256:PARENT_SHA});const payload=path.join(path.dirname(ref.path),'source');if(value.source_payload_directory)assert.equal(normalized(value.source_payload_directory),normalized(payload));for(const f of value.files){relativeFile(f.path);const hash=f.after_sha256??f.sha256,bytes=f.after_bytes??f.bytes;assert.equal(f.before_sha256??'ABSENT',parentFiles.get(f.path)?.sha256??'ABSENT');const raw=pin(path.join(payload,f.path),hash);assert.equal(raw.length,bytes);rawProposalBytes.set(hash,raw);if(!rawRoleFiles.has(f.path))rawRoleFiles.set(f.path,[]);rawRoleFiles.get(f.path).push({role:row.role,sha256:hash,bytes});}}
 const owned=[...composition.owned_paths];assert.equal(owned.length,30);assert.equal(new Set(owned).size,30);owned.forEach(relativeFile);assert.deepEqual([...rawRoleFiles.keys()].sort(),[...owned].sort());
 const shared=[...rawRoleFiles].filter(([,rows])=>rows.length>1).map(([p])=>p).sort();assert.deepEqual(shared,['crates/app/src/server.rs','crates/core/src/lib.rs','web/index.html']);
 const repairFile='crates/core/src/resource_broken_sources_tests.rs';
 assert.deepEqual(rootApproval.semantic_repair,composition.semantic_repair);
 assert.deepEqual(compositionReview.semantic_repair,composition.semantic_repair);
 const repairReference=composition.semantic_repair.proposal_manifest;
 assert.equal(repairReference.sha256,'bc8e4a9b7283e84eb3e2df23cbf321790a40bbe1fa029ed28b2a4b5e0ae03b67');
 const repair=data(repairReference),repairReview=data(composition.semantic_repair.review);
 assert.equal(composition.semantic_repair.review.sha256,'3fd9a96d6119ef6ce354346270dab6e4608b09f2304906a3d5d83058f39e0c06');
 assert.equal(repair.kind,'resource_broken_sources_test_identity_repair_proposal_v2');
 assert.equal(repairReview.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(repairReview.findings,[]);sameRef(repairReview.proposal_manifest,repairReference);
 assert.equal(repair.files.length,1);const repairRow=repair.files[0];assert.equal(repairRow.path,repairFile);assert.deepEqual(repair.new_authored_test_functions,[]);
 assert.equal(repair.actual_base_preliminary_clippy.sha256,'af5c3c6beb71a4797aff14ac90be9a928c48b8c4cd89c409ba529fdf4ed8548d');sameRef(composition.actual_base_clippy,repair.actual_base_preliminary_clippy);
 const beforeClippy=data(composition.actual_base_clippy);assert.equal(beforeClippy.source_inputs,358);assert.equal(beforeClippy.result,'PASS_PRELIMINARY_CLIPPY_ONLY');assert.equal(beforeClippy.actual_close_event,true);assert.deepEqual(beforeClippy.exit,{code:0,signal:null});assert.equal(beforeClippy.source_final_verified,true);
 const baseRows=rowsMap(beforeClippy.files,358);assert.equal(baseRows.get(repairFile).sha256,repairRow.before_sha256);
 const beforeRepair=pin(path.join(BASE,'combined-source-v9-draft-v5/source',repairFile),repairRow.before_sha256);
 const repairDirectory=path.join(path.dirname(repairReference.path),'source');assert.equal(normalized(repair.source_payload_directory),normalized(repairDirectory));
 const repaired=pin(path.join(repairDirectory,repairFile),repairRow.after_sha256);assert.equal(repaired.length,repairRow.after_bytes);
 const derivation=data(repair.derivation);assert.equal(derivation.operations.length,1);const op=derivation.operations[0];assert.equal(op.path,repairFile);assert.equal(op.before_sha256,repairRow.before_sha256);assert.equal(op.after_sha256,repairRow.after_sha256);assert(Number.isSafeInteger(op.offset)&&op.offset>=0);
 const oldFragment=Buffer.from(op.before_base64,'base64'),newFragment=Buffer.from(op.after_base64,'base64');assert(beforeRepair.subarray(op.offset,op.offset+oldFragment.length).equals(oldFragment));assert(Buffer.concat([beforeRepair.subarray(0,op.offset),newFragment,beforeRepair.subarray(op.offset+oldFragment.length)]).equals(repaired));assert(repaired.subarray(op.offset,op.offset+newFragment.length).equals(newFragment));assert(Buffer.concat([repaired.subarray(0,op.offset),oldFragment,repaired.subarray(op.offset+newFragment.length)]).equals(beforeRepair));
 assert.deepEqual(declaredTests(repaired.toString()),declaredTests(beforeRepair.toString()));
 rawRoleFiles.set(repairFile,[{role:'semantic_repair',sha256:repairRow.after_sha256,bytes:repairRow.after_bytes}]);rawProposalBytes.set(repairRow.after_sha256,repaired);
 const corpus=rowsMap(composition.files,358),authoredRaw=new Map();assert.equal(normalized(composition.source_directory),normalized(path.join(BASE,'combined-source-v9-draft-v6/source')));assert.equal(normalized(composition.preformat_directory),normalized(path.join(BASE,'combined-source-v9-draft-v6/preformat')));
 for(const f of corpus.values()){const raw=pin(path.join(composition.preformat_directory,f.path),f.sha256);assert.equal(raw.length,f.bytes);authoredRaw.set(f.path,raw);if(f.path===repairFile){assert.equal(f.sha256,repairRow.after_sha256);assert.equal(f.bytes,repairRow.after_bytes);}else assert.deepEqual(f,baseRows.get(f.path));if(!owned.includes(f.path))assert.deepEqual(f,parentFiles.get(f.path));}
 assert.deepEqual([...corpus.keys()].sort(),[...new Set([...parentFiles.keys(),...owned])].sort());assert.equal(owned.filter(p=>parentFiles.has(p)).length,7);assert.equal(owned.filter(p=>!parentFiles.has(p)).length,23);
 // Actual V5 formatting remains historical. The corrected test is semantic
 // authority from repairReference; only its subsequent actual rustfmt delta
 // may be recorded as formatting. All17 other prior tuples remain exact.
 const formatting=data(rootApproval.formatter_review);assert.equal(formatting.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(formatting.findings,[]);assert(Array.isArray(formatting.final_proposal_format_files));assert.equal(formatting.final_proposal_format_files.length,18);
 const sourceFiles=rowsMap(rootApproval.source_files,358),deltaPins=new Map();assert.deepEqual(rootApproval.files.map(r=>r.path),[...owned].sort());
 for(const row of rootApproval.files){const authored=corpus.get(row.path);assert.equal(row.before_sha256,parentFiles.get(row.path)?.sha256??'ABSENT');assert.equal(row.authored_sha256,authored.sha256);assert.match(row.after_sha256,HASH);assert.equal(row.format_only,row.authored_sha256!==row.after_sha256);assert.deepEqual(sourceFiles.get(row.path),{path:row.path,sha256:row.after_sha256,bytes:row.bytes});deltaPins.set(row.path,row);}
 for(const [p,f]of sourceFiles){assert(corpus.has(p));if(!deltaPins.has(p))assert.deepEqual(f,parentFiles.get(p));else assert.notEqual(f.sha256,parentFiles.get(p)?.sha256);}
 const formatted=rootApproval.files.filter(r=>r.format_only).map(r=>({path:r.path,before_sha256:r.authored_sha256,after_sha256:r.after_sha256,bytes:r.bytes}));assert.deepEqual(formatting.files,formatted);assert.equal(formatted.length,1);
 const historicalFormatting=data(composition.actual_base_formatter_review);assert.equal(historicalFormatting.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(historicalFormatting.findings,[]);assert.equal(historicalFormatting.final_proposal_format_files.length,18);assert.equal(formatted.length,1);assert.equal(formatted[0].path,repairFile);assert.equal(formatted[0].before_sha256,repairRow.after_sha256);const inheritedFormat=historicalFormatting.final_proposal_format_files.filter(f=>f.path!==repairFile);assert.equal(inheritedFormat.length,17);const actualFormatTuples=[...inheritedFormat,...formatted].sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0);assert.deepEqual(formatting.final_proposal_format_files,actualFormatTuples);
 for(const f of formatting.final_proposal_format_files){assert(owned.includes(f.path));assert.notEqual(f.before_sha256,f.after_sha256);assert.deepEqual(sourceFiles.get(f.path),{path:f.path,sha256:f.after_sha256,bytes:f.bytes});if(!shared.includes(f.path)){assert.equal(rawRoleFiles.get(f.path).length,1);assert.equal(f.before_sha256,rawRoleFiles.get(f.path)[0].sha256);}}
 const actualFormat=data(rootApproval.actual_format);assert.equal(actualFormat.kind,'actual_source9_v6_format_only');assert.equal(actualFormat.result,'PASS_FORMAT_COMMAND_ONLY');assert.equal(actualFormat.actual_close_event,true);assert.deepEqual(actualFormat.exit,{code:0,signal:null});assert.equal(actualFormat.error,null);assert.deepEqual(actualFormat.io_errors,[]);for(const stream of Object.values(actualFormat.streams)){assert.equal(stream.end,true);assert.equal(stream.close,true);}assert.equal(actualFormat.preformat_sha256,COMPOSITION_SHA);assert.deepEqual(actualFormat.changed_files,formatted);
 const preliminary=data(rootApproval.preliminary_clippy);assert.equal(preliminary.kind,'source9_actual_preliminary_clippy');assert.equal(preliminary.result,'PASS_PRELIMINARY_CLIPPY_ONLY');assert.equal(preliminary.executed_tests,0);assert.equal(preliminary.actual_close_event,true);assert(Number.isSafeInteger(preliminary.pid)&&preliminary.pid>0);assert.equal(preliminary.spawn_error,null);assert.deepEqual(preliminary.exit,{code:0,signal:null});assert.equal(preliminary.preformat_sha256,COMPOSITION_SHA);assert.equal(preliminary.format_sha256,rootApproval.actual_format.sha256);assert.equal(preliminary.source_final_verified,true);assert.equal(preliminary.source_inputs,358);assert.deepEqual(preliminary.files,rootApproval.source_files);assert.equal(rootApproval.preliminary_clippy_sha256,rootApproval.preliminary_clippy.sha256);assert.deepEqual(rootApproval.preliminary_clippy_args,['clippy','--workspace','--all-targets','--locked','--','-D','warnings']);assert.equal(rootApproval.preliminary_clippy_log.sha256,preliminary.log_sha256);const clippyLog=pin(rootApproval.preliminary_clippy_log.path,preliminary.log_sha256).toString('utf8');assert(/Finished\s+`dev`\s+profile/.test(clippyLog));assert(!/^error(?:\[|:)|^warning:/m.test(clippyLog));
 const publication=data(rootApproval.private_publication);assert.equal(publication.kind,'actual_private_source9_code_publication');assert.equal(publication.source_commit,rootApproval.source_commit);assert.equal(publication.branch,rootApproval.source_branch);assert.equal(publication.repository,'lia-xim/open-seo-spider');assert.equal(publication.is_private,true);assert.equal(publication.source_inputs,358);assert.equal(publication.all358_root_source_hashes_verified,true);assert.equal(publication.code_and_owned_raw_byte_attributes,true);assert.deepEqual(publication.owned_committed_paths,[...owned].sort());assert.deepEqual(publication.additional_tracked_files,['.gitattributes']);
 const actualNew=[];
 for(const [p,raw]of authoredRaw){if(!p.endsWith('.rs'))continue;const old=new Set(declaredTests((parentRaw.get(p)??Buffer.from('')).toString()).map(t=>t.name)),now=declaredTests(raw.toString());for(const name of old)assert(now.some(t=>t.name===name),'Inherited declaration removed: '+p+'::'+name);for(const t of now)if(!old.has(t.name)){assert.notEqual(t.platform,'unix');assert(modules[p],'Unmapped actual test module '+p);const [root,mod,module,prefix]=modules[p];assert(new RegExp('\\bmod\\s+'+mod+'\\s*;').test(authoredRaw.get(root).toString()));const moduleText=authoredRaw.get(module).toString();assert(moduleText.includes('#[path = "'+path.posix.basename(p)+'"]'));assert(/\bmod\s+tests\s*;/.test(moduleText));actualNew.push({path:p,name:t.name,full_name:prefix+t.name});}}
 assert.equal(actualNew.length,37);const key=t=>t.path+'::'+t.name;assert.equal(composition.new_authored_test_functions.length,37);assert.deepEqual(actualNew.map(key).sort(),composition.new_authored_test_functions.map(key).sort());for(const t of composition.new_authored_test_functions)if(t.full_name)assert.equal(t.full_name,actualNew.find(a=>key(a)===key(t)).full_name);
 const addedTests=[...new Set(actualNew.map(t=>t.path))].sort().map(p=>({path:p,names:actualNew.filter(t=>t.path===p).map(t=>t.name).sort()})),newFullNames=actualNew.map(t=>t.full_name).sort();assert.equal(new Set(newFullNames).size,37);assert.deepEqual(rootApproval.new_authored_tests,addedTests);assert.deepEqual(rootApproval.new_authored_full_test_names,newFullNames);
 const requiredTests=structuredClone(baseline.required_tests);for(const t of actualNew.sort((a,b)=>key(a)<key(b)?-1:key(a)>key(b)?1:0)){let g=requiredTests.find(g=>g.path===t.path);if(!g){g={path:t.path,names:[]};requiredTests.push(g);}assert(!g.names.includes(t.name));g.names.push(t.name);}const required=requiredTests.flatMap(g=>g.names);assert.equal(required.length,577);assert.equal(new Set(required).size,577);
 return {baselinePath,baselineSha:PARENT_SHA,baseline,parentFiles,parentRaw,rootApproval,rootApprovalReference,composition,compositionReference:{path:proposal.path,sha256:proposal.sha256},sourceFiles,deltaPins,paths:[...sourceFiles.keys()],addedTests,newFullNames,requiredTests,required,references,pin,parentSuiteVerified,parentSuiteReference,minimum:1359};
}
export function validateSource9Manifest({manifest,sourceDirectory,repo=path.join(BASE,'mcp-file-tools-v1')}){
 assert.equal(manifest.schema_version,1);assert.equal(manifest.kind,'combined_source_v9');assert.equal(manifest.result,'FROZEN_REVIEWED_SOURCE_FULL_SUITE_NOT_RUN');assert.equal(normalized(sourceDirectory),normalized(path.join(BASE,'combined-source-v9')));
 const c=loadSource9Approval({approvalPath:manifest.root_approval.path,approvalSha:manifest.root_approval.sha256,expectedCommit:manifest.source_commit,repo}),{sourceFiles,rootApproval}=c;
 assert.equal(manifest.parent_source_manifest_sha256,PARENT_SHA);assert.equal(manifest.parent_source_inputs,335);assert.equal(manifest.source_inputs,358);assert.equal(manifest.inherited_required_test_names,540);assert.equal(manifest.new_input_files,23);assert.equal(manifest.changed_parent_inputs,7);assert.equal(manifest.unchanged_parent_inputs,328);assert.equal(manifest.minimum_expected_workspace_tests,1359);assert.equal(manifest.required_test_names,577);assert.equal(manifest.new_authored_test_functions,37);assert.deepEqual(manifest.test_profile,profile);assert.deepEqual(manifest.files,rootApproval.source_files);assert.deepEqual(manifest.required_tests,c.requiredTests);assert.deepEqual(manifest.new_authored_tests,c.addedTests);assert.deepEqual(manifest.new_authored_full_test_names,c.newFullNames);assert.deepEqual(manifest.source_proposals,rootApproval.proposals);sameRef(manifest.parent_suite,c.parentSuiteReference);assert.equal(manifest.source_branch,rootApproval.source_branch);assert.equal(normalized(manifest.source_worktree),normalized(repo));
 const bindings=[...c.deltaPins.values()].map(r=>({path:r.path,before_sha256:r.before_sha256,after_sha256:r.after_sha256,bytes:r.bytes}));assert.deepEqual(manifest.approved_product_bindings,bindings);
 const validatorPath=fileURLToPath(import.meta.url);assert.equal(normalized(manifest.validator.path),normalized(validatorPath));c.pin(validatorPath,manifest.validator.sha256);
 const published=new Map();for(const ref of manifest.references){assert(path.isAbsolute(ref.path));assert.match(ref.sha256,HASH);assert(Number.isSafeInteger(ref.bytes)&&ref.bytes>=0);assert(!published.has(normalized(ref.path)));published.set(normalized(ref.path),ref);}assert.equal(published.size,c.references.size);for(const [k,r]of c.references)assert.deepEqual(published.get(k),r);
 for(const f of sourceFiles.values())for(const dir of [sourceDirectory,repo]){const raw=c.pin(path.join(dir,f.path),f.sha256);assert.equal(raw.length,f.bytes);}
 // Compile-time module aliases are checked again after final formatting.
 for(const [p,[root,mod,module]]of Object.entries(modules)){assert(new RegExp('\\bmod\\s+'+mod+'\\s*;').test(fs.readFileSync(path.join(sourceDirectory,root),'utf8')));assert(fs.readFileSync(path.join(sourceDirectory,module),'utf8').includes('#[path = "'+path.posix.basename(p)+'"]'));const actual=declaredTests(fs.readFileSync(path.join(sourceDirectory,p),'utf8')).filter(t=>t.platform!=='unix').map(t=>t.name).sort();assert.deepEqual(actual,c.addedTests.find(g=>g.path===p).names);}
 const files=sourceFiles,rendererFiles=[...files.values()].filter(f=>f.path.startsWith('renderer/'));assert.equal(rendererFiles.length,4);
 const lineage=Object.freeze({parent_source_manifest_sha256:PARENT_SHA,parent_suite_proof_sha256:PARENT_SUITE_SHA,composition_proposal_sha256:COMPOSITION_SHA,root_approval_sha256:manifest.root_approval.sha256,parent_inputs:335,required_parent_names:540,unchanged_parent_inputs:328,changed_parent_inputs:7,new_input_files:23,delta_bindings:30,new_named_windows_functions:c.addedTests.flatMap(g=>g.names),actual_required_names:577,actual_source_inputs:358,minimum_expected_workspace_tests:1359});
 const expectedNewFullNames=[...c.parentSuiteVerified.full_names,...c.newFullNames].sort();assert.equal(expectedNewFullNames.length,581);assert.equal(new Set(expectedNewFullNames).size,581);
 return {...c,files,rendererFiles,lineage,pinnedReferences:published,expectedNewFullNames};
}
