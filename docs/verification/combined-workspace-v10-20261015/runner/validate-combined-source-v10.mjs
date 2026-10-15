// Direct source admission only. No compiler, network, Git, or recursive old validators.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
export const PARENT_SHA='4e22e195cb918f3fcc820135da9f9c858c0a531e59bb502e15bf0eb8711bb3a9';
const PARENT_SUITE_SHA='dd4accbd9edfcf523264c9f70f255bff14270427fbb4d118a2f51c06290ad280';
const COMPOSITION_SHA='27425d98d26bf51b518fd3048a265c497d525c1204ea77442d9f07d9a84c7a24';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const normalized=p=>path.resolve(p).toLowerCase();
const HASH=/^[a-f0-9]{64}$/;
const profile=Object.freeze({opt_level:1,debug_assertions:true,overflow_checks:true,debug_info:0,environment:{CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0'}});
const gates=['live_browser_worker_executes_javascript_for_core_crawl','live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness','live_browser_worker_pause_drains_render_and_retains_rendered_discoveries','live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion'];
export function relativeFile(value){assert.equal(typeof value,'string');assert(value&&!value.includes('\\')&&!value.includes(':')&&!path.posix.isAbsolute(value));assert(value.split('/').every(p=>p&&p!=='.'&&p!=='..'));return value;}
function declaredTests(text){const rows=[];for(const m of text.matchAll(/((?:#\[[^\]]*\]\s*)+)(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)\b/g))if(/#\[(?:tokio::)?test(?:\(|\])/.test(m[1]))rows.push({name:m[2],platform:/#\[cfg\(unix\)\]/.test(m[1])?'unix':/#\[cfg\(windows\)\]/.test(m[1])?'windows':'all'});return rows;}
function rowsMap(rows,count){assert(Array.isArray(rows));assert.equal(rows.length,count);const map=new Map();for(const f of rows){relativeFile(f.path);assert.match(f.sha256,HASH);assert(Number.isSafeInteger(f.bytes)&&f.bytes>=0);assert(!map.has(f.path));map.set(f.path,f);}assert.equal(new Set([...map.keys()].map(p=>p.toLowerCase())).size,count);return map;}
function sameRef(a,b){assert.equal(normalized(a.path),normalized(b.path));assert.equal(a.sha256,b.sha256);}
const PARENT_BUILD_SHA='75bd448a9feea334a6670c56c01b7ebcef2e91196d92cff9a69b20c50e9cb4a5';
const COMPOSITION_PATH="E:/CodexWork/open-seo-spider-01a12ac3-20261013/combined-source-v10-draft-v2/preformat.json";
const ACTUAL_FORMAT_KIND="actual_source10_v2_format_only";
const expectedSourceRoles=[
  {
    "role": "ngram_corpus",
    "manifest": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/saved-ngram-corpus-proposal-v3/manifest.json",
      "sha256": "9610252b3573c849f4dfccc68841679155deac0ea018eed5c3025e5b16d32b1c",
      "bytes": 14158
    },
    "review": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/saved-ngram-corpus-independent-review-v3/receipt.json",
      "sha256": "4fa91ba83d4ac7281fda5555526890e5655e4fd1d8d6da6148d5b5a073d24666",
      "bytes": 14038
    }
  },
  {
    "role": "report_projection",
    "manifest": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-report-projection-proposal-v2/manifest.json",
      "sha256": "8a813833ecbe45149adb1049f49f664421e176cfeff8d56f77682146bbe06c2e",
      "bytes": 5481
    },
    "review": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-report-projection-independent-review-v2/receipt.json",
      "sha256": "3a30fb3c395f44e159ef8d4de637d00e1be357f3460e4869a8ece32013696b5c",
      "bytes": 6276
    }
  },
  {
    "role": "body_selection_refresh",
    "manifest": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/source-body-selection-refresh-proposal-v1/manifest.json",
      "sha256": "f8181168e2c67aaab053eb1257fa27d119a79760d0c4c11ecd7adeb7bfc93606",
      "bytes": 2590
    },
    "review": {
      "path": "E:/CodexWork/open-seo-spider-01a12ac3-20261013/source-body-selection-refresh-independent-review-v1/receipt.json",
      "sha256": "73a2fb41187125ae72c22fe88322da9833b5d0e2aedfb1a96cd3ff09720ac1fb",
      "bytes": 5874
    }
  }
];
const modules={
  "crates/app/src/ngram_app_tests.rs": [
    [
      "crates/app/src/main.rs",
      "ngram_app"
    ],
    "crates/app/src/ngram_app.rs",
    "ngram_app::tests::"
  ],
  "crates/app/src/server/ngram_corpus_tests.rs": [
    [
      "crates/app/src/server.rs",
      "ngram_corpus"
    ],
    "crates/app/src/server/ngram_corpus.rs",
    "server::ngram_corpus::tests::"
  ],
  "crates/core/src/ngram_corpus_tests.rs": [
    [
      "crates/core/src/lib.rs",
      "ngram_corpus"
    ],
    "crates/core/src/ngram_corpus.rs",
    "ngram_corpus::tests::"
  ],
  "crates/core/src/sitemap_membership_analysis/report_projection_tests.rs": [
    [
      "crates/core/src/lib.rs",
      "sitemap_membership_analysis"
    ],
    "crates/core/src/sitemap_membership_analysis/report_projection.rs",
    "sitemap_membership_analysis::report_projection::tests::",
    [
      "crates/core/src/sitemap_membership_analysis.rs",
      "report_projection"
    ]
  ]
};
// Actual closed immutable Source9 producer/raw artifacts; no target imports.
const parentArtifacts=[
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\proof.json",
    "sha256": "dd4accbd9edfcf523264c9f70f255bff14270427fbb4d118a2f51c06290ad280",
    "bytes": 232941
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\format.log",
    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "bytes": 0
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\format.json",
    "sha256": "8f61b9895880f4444d5e68b139b5c508a45fe400cae8a3c2c1c8e6889b0d9cd1",
    "bytes": 380
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\workspace-live.log",
    "sha256": "c59bf7d73fbcc5e01671cd7346dfe466678284943faece5044ad18e91919de69",
    "bytes": 153489
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\workspace-live.json",
    "sha256": "8e9c6c29a601f2fd581751815d2400d31e1012af5a1df37682ed85ef68ef2f20",
    "bytes": 1838
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\clippy.log",
    "sha256": "847868a5db3562208b6c3fb25881d015c46b5a1ec34a567b49746619d72c1da5",
    "bytes": 60
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\clippy.json",
    "sha256": "aad2609e059f0b17935813bfded6a8596a1b9ceac0d846f450138edcf99e350a",
    "bytes": 448
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\required-test-results.json",
    "sha256": "1409ee99e73fe345302a08e8a19f7718f5e8d12c3131ed9e384ae429e6c7ddc2",
    "bytes": 151696
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-suite-v9\\owned-worker.json",
    "sha256": "1197af46f7e225f8dae6067126d4ee3db0a3c3b258c8fd906624b1f3d230bc84",
    "bytes": 2299
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v9-driver-preparation-v4\\run-combined-suite-v9.mjs",
    "sha256": "5c1406fee168c971907480add0c9ad613441aa9dacf809ba188711344e3afc1f",
    "bytes": 16853
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-optimized-build-v9\\proof.json",
    "sha256": "75bd448a9feea334a6670c56c01b7ebcef2e91196d92cff9a69b20c50e9cb4a5",
    "bytes": 84023
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-optimized-build-v9\\cargo-version.log",
    "sha256": "99ea60353d8536d359444ca9554a806459bdffc33a35cfb49e75cb66c1139bd0",
    "bytes": 321
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-optimized-build-v9\\cargo-version.json",
    "sha256": "b75e05e6fffc024e1a2a539385d76a8d518191a7325ccfdc401af4c0e5ddbbcf",
    "bytes": 383
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-optimized-build-v9\\rustc-version.log",
    "sha256": "185aa337caf524a621d7b1ce7b21a4d6f92f3ea592d009e481fea0e00e0d0434",
    "bytes": 194
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-optimized-build-v9\\rustc-version.json",
    "sha256": "1042e751fd1631e0b83f5e79d3eaa5d4cd2256b839bb76d05087bb4552341a29",
    "bytes": 384
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-optimized-build-v9\\build.log",
    "sha256": "cfaa1f74c0f735887891ae4f3c4376a5e10a33da045bafb3c943b82d34383477",
    "bytes": 299
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\results\\combined-optimized-build-v9\\build.json",
    "sha256": "3349530e60d320360512f733617336a43fe2fc500c984855e499c5adaab2c9d7",
    "bytes": 425
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v9-driver-preparation-v4\\build-combined-optimized-v9.mjs",
    "sha256": "0c8ddba45e6b3731d349005d699cc6ff665975b7896848d80ae3f3ddefade0fe",
    "bytes": 18939
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v9-driver-preparation-v4\\manifest.json",
    "sha256": "2479d2bc4f9c18dc03a7d30c2ec9e5d0e73b0d887c3fad6afbc2ee5277bd577c",
    "bytes": 5469
  },
  {
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\combined-v9-driver-independent-review-v4\\receipt.json",
    "sha256": "a01311ecea22aa45af6e9e92a37719c3e6bf895f93c163f1061e082a12c7b257",
    "bytes": 13082
  }
];
const expectedParentLineage={
  "parent_source_manifest_sha256": "58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720",
  "parent_suite_proof_sha256": "7266c3087ca3b3998116f18d6ba05515c88bcf25f86daa5537ec3206201468f1",
  "composition_proposal_sha256": "ade29eed3c3be14028e0b099e33e38b9020de394c0b3b6986485a5a4f1d77367",
  "root_approval_sha256": "663d4741f3a2ba6044cdc7638342563bc1ebc8679abb90753fc1497550c39c44",
  "parent_inputs": 335,
  "required_parent_names": 540,
  "unchanged_parent_inputs": 328,
  "changed_parent_inputs": 7,
  "new_input_files": 23,
  "delta_bindings": 30,
  "new_named_windows_functions": [
    "broken_sources_caps_reject_rows_and_ambiguous_selected_join_expansion_without_truncation",
    "broken_sources_csv_exports_every_filtered_occurrence_formula_safe_exact_utf8_and_refuses_overcap",
    "broken_sources_cursor_and_export_require_exact_snapshot_filter_generation_and_page_limit",
    "broken_sources_preserves_repeated_occurrences_dom_absence_and_exact_requested_redirect_identity",
    "broken_sources_selected_inherited_and_imported_age_cannot_turn_old_404_into_a_current_observed_failure",
    "broken_sources_unknowns_keep_ambiguous_robots_errors_proposed_locations_old_and_incoherent_observations",
    "resource_sources_app_json_rejects_envelope_over_budget_and_retains_unknown_null",
    "resource_sources_cli_cannot_overwrite_database_request_sidecars_or_existing_outputs",
    "resource_sources_request_rejects_paths_observations_unknown_fields_and_wrong_job_generation",
    "broken_sources_api_full_csv_keeps_duplicate_sources_and_exact_length_sha_without_website_requests",
    "broken_sources_api_refuses_active_and_generation_changes_and_marks_imported_age_unknown",
    "directory_api_full_csv_binds_actual_length_sha_token_and_never_contacts_website",
    "directory_api_generation_report_config_and_status_mutation_stale_all_old_tokens",
    "directory_api_real_oversized_saved_report_refuses_before_authority_clone_and_keeps_job",
    "directory_api_real_tree_navigation_keeps_synthetic_nodes_and_token_bound_paging",
    "directory_api_reuses_unmodified_saved_authority_and_declares_imported_scope",
    "directory_api_strict_requests_refuse_forged_paths_missing_snapshot_and_bad_windows",
    "resource_sources_api_real_saved_css_over200_full_csv_stale_and_cli_reopen_do_no_new_website_io",
    "resource_sources_api_unavailable_active_config_and_snapshot_generation_authority_are_explicit",
    "directory_tree_csv_is_complete_formula_safe_and_exact_byte_caps_are_independent",
    "directory_tree_duplicates_conflicting_observations_are_counted_without_latest_selection",
    "directory_tree_encoded_slash_query_order_empty_query_and_fragment_identity",
    "directory_tree_initial_status_and_unknown_final_indexability_never_use_landing_200",
    "directory_tree_invalid_unsupported_and_credentials_are_explicit_exclusions",
    "directory_tree_protocol_host_effective_port_ipv6_and_root_are_separate",
    "directory_tree_real_node_text_url_segment_and_page_boundaries_refuse_without_partial_tree",
    "directory_tree_rendered_document_identity_and_incomplete_observations_stay_unknown",
    "directory_tree_saved_nonstandard_http_codes_are_known_other_not_unobserved",
    "directory_tree_synthetic_paths_never_become_fetched_url_observations",
    "directory_tree_trailing_slash_empty_segments_and_query_directory_leaf_stay_distinct",
    "resource_broken_sources_complete_transport_set_stales_same_generation_even_for_unselected_healthy_target",
    "resource_broken_sources_evaluation_revisions_do_not_inflate_physical_ids_or_upgrade_old_no_grant",
    "resource_broken_sources_full_csv_over200_keeps_tail_formula_safe_and_exact_snapshot_filter",
    "resource_broken_sources_missing_history_is_unavailable_and_caps_hash_config_generation_and_parent_corruption_refuse",
    "resource_broken_sources_policy_fetch_alias_is_separate_from_original_resolved_value_and_status_chain",
    "resource_broken_sources_real_css_duplicates_private_404_and_cold_read_only_reopen_preserve_source_units",
    "resource_broken_sources_same_read_transaction_cannot_mix_transport_changes_and_new_continuation"
  ],
  "actual_required_names": 577,
  "actual_source_inputs": 358,
  "minimum_expected_workspace_tests": 1359
};
function verifyParentSuite(parent,pin){
 const task=TASK;
 const source=path.join(BASE,'combined-source-v9'),repo=path.join(BASE,'mcp-file-tools-v1');
 const suiteRoot=path.join(BASE,'results/combined-suite-v9');
 for(const r of parentArtifacts)assert.equal(pin(r.path,r.sha256).length,r.bytes);
 const namedReference=parentArtifacts.find(r=>path.basename(r.path)==='required-test-results.json');
 const expectedParentFullNames=JSON.parse(pin(namedReference.path,namedReference.sha256)).results.map(r=>r.actual).sort();
 const driverPath=path.join(BASE,'combined-v9-driver-preparation-v4/run-combined-suite-v9.mjs');
 const suiteDriverSha='5c1406fee168c971907480add0c9ad613441aa9dacf809ba188711344e3afc1f';
 const expectedSuiteProofSha=PARENT_SUITE_SHA,expectedManifestSha=PARENT_SHA;
 const manifest=parent,lineage=expectedParentLineage,names=parent.required_tests.flatMap(g=>g.names),minimumExpected=1359;
 const expectedTestProfile=profile,rendererGates=gates,expectedNewFullNames=expectedParentFullNames;
 const boundedRead=(p,max)=>{const s=fs.lstatSync(p);assert(s.isFile()&&!s.isSymbolicLink()&&s.size<=max);const b=fs.readFileSync(p);assert.equal(b.length,s.size);return b;};
 const parseSummaries=text=>[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
 const suiteCommands=[['format',['fmt','--all','--check']],['workspace-live',['test','--workspace','--locked','--','--include-ignored','--test-threads=1']],['clippy',['clippy','--workspace','--all-targets','--locked','--','-D','warnings']]];
function verifySuite() {
  assert.equal(sha(fs.readFileSync(driverPath)), suiteDriverSha, 'Exact actual suite producer required');
  const proofBytes = boundedRead(path.join(suiteRoot, 'proof.json'), 8 * 1024 * 1024);
  assert.equal(sha(proofBytes), expectedSuiteProofSha, 'Explicit actual suite proof hash required');
  const proof = JSON.parse(proofBytes);
  assert.equal(proof.kind, 'combined_full_suite_v9');
  assert.equal(proof.result, 'PASS_COMBINED_WORKSPACE_V9_REAL_RENDERER_FMT_CLIPPY');
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
    // Historical source is re-read only from immutable frozen Source9.
    const bytes = boundedRead(path.join(source, file.path), 64 * 1024 * 1024);
    assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256);
  }
  assert.equal(path.resolve(worker.chrome_path), path.resolve('C:/Program Files/Google/Chrome/Application/chrome.exe'));
  assert.match(worker.chrome_sha256, /^[a-f0-9]{64}$/);
  // The exact hashed historical worker receipt binds its observed Chrome SHA.
  // Current Chrome/runtime are rehashed by the fresh Source10 suite/build.
  // Re-reading mutable installed Chrome would invalidate an immutable old run.
  digests.push({path: 'owned-worker.json', sha256: sha(workerBytes), bytes: workerBytes.length});
  return {proof_sha256: sha(proofBytes), source_manifest_sha256: expectedManifestSha,
    suite_driver_sha256: suiteDriverSha, passed, failed, ignored, required_names: names.length, test_profile: expectedTestProfile, source_lineage: lineage,
    renderer_gates: 4, actual_three_closes: true, owned_resources_closed: true,
    new_authored_full_names: expectedNewFullNames, new_authored_full_names_verified: true,
    final_sources_verified: true, independently_reparsed_logs: true, digests};
}

 const verified=verifySuite();
 assert.equal(verified.passed,1359);assert.equal(verified.required_names,577);
 assert.equal(expectedNewFullNames.length,581);
 return {verified,full_names:expectedNewFullNames};
}
function verifyParentBuild(parent,admittedSuite,pin){
 const manifest=parent,lineage=expectedParentLineage,source=path.join(BASE,'combined-source-v9'),repo=path.join(BASE,'mcp-file-tools-v1');
 const buildRoot=path.join(BASE,'results/combined-optimized-build-v9'),suitePath=path.join(BASE,'results/combined-suite-v9/proof.json');
 const builderPath=path.join(BASE,'combined-v9-driver-preparation-v4/build-combined-optimized-v9.mjs'),builderSha='0c8ddba45e6b3731d349005d699cc6ff665975b7896848d80ae3f3ddefade0fe';
 const expectedSuiteProofSha=PARENT_SUITE_SHA,expectedManifestSha=PARENT_SHA;
 const release={build_proof_sha256:PARENT_BUILD_SHA,candidate_binary_sha256:'d3aea3a5a43bf39c3ac1f4584268077f81c89b8ef7f46378592138def038a8fe'};
 const boundedRead=(p,max)=>{const st=fs.lstatSync(p);assert(st.isFile()&&!st.isSymbolicLink()&&st.size<=max);const raw=pin(p,sha(fs.readFileSync(p)));assert.equal(raw.length,st.size);return raw;};
function verifyBuild() {
  assert.equal(sha(boundedRead(builderPath, 1024 * 1024)), builderSha);
  const raw = boundedRead(path.join(buildRoot, 'proof.json'), 8 * 1024 * 1024), build = JSON.parse(raw);
  assert.equal(sha(raw), release.build_proof_sha256);
  assert.equal(build.kind, 'combined_optimized_build_v9');
  assert.equal(build.result, 'PASS_COMBINED_OPTIMIZED_BUILD_V9');
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

 const verified=verifyBuild();assert.equal(verified.binary_bytes,42530304);return verified;
}

function verifyModule(raws,p){const [root,module,prefix,nested]=modules[p];for(const [file,name]of [root,...(nested?[nested]:[])])assert(new RegExp('\\bmod\\s+'+name+'\\s*;').test(raws.get(file).toString()),file+' must declare '+name);const t=raws.get(module).toString();assert(new RegExp('#\\[path\\s*=\\s*"'+path.posix.basename(p).replaceAll('.','\\.')+'"\\]').test(t));assert(/\bmod\s+tests\s*;/.test(t));return prefix;}
function closedStreams(record){assert.equal(record.actual_close_event,true);assert.deepEqual(record.exit,{code:0,signal:null});assert(Number.isSafeInteger(record.pid)&&record.pid>0);assert.deepEqual(record.io_errors,[]);assert.deepEqual(Object.keys(record.streams).sort(),['stderr','stdout']);for(const stream of Object.values(record.streams)){assert.equal(stream.end,true);assert.equal(stream.close,true);}}
function verifyUi(composition,rootApproval,parentFiles,corpus,pin,data){
 const expected={path:path.join(BASE,'results/source-body-selection-regressions-v2/proof.json'),sha256:'6de71d0da80b7626546b3e026f13cb0e55270489fff29c7adc14325d5252f590'};
 sameRef(rootApproval.ui_regressions,expected);sameRef(composition.separate_node_verification.existing_actual_root_proof,expected);
 const ui=data(expected);assert.equal(ui.kind,'source_body_selection_actual_regressions_v1');assert.equal(ui.status,'PASS_BASELINE_DETECTED_AND_REVIEWED_FIX_PASSES_ALL_THREE');assert.equal(ui.cargo_tests_added,0);assert.equal(ui.compiled_browser_rerun,null);
 const row=composition.source_proposals.find(r=>r.role==='body_selection_refresh');sameRef(ui.proposal,row.manifest);sameRef(ui.independent_review,row.review);
 assert.equal(composition.separate_node_verification.product_input,false);assert.equal(composition.separate_node_verification.cargo_test_count,0);assert.equal(composition.separate_node_verification.names.length,3);
 assert.equal(ui.harness.sha256,composition.separate_node_verification.harness.sha256);assert.equal(ui.harness.bytes,composition.separate_node_verification.harness.bytes);const harness=pin(ui.harness.path,ui.harness.sha256);assert.equal(harness.length,ui.harness.bytes);assert(pin(composition.separate_node_verification.harness.path,ui.harness.sha256).equals(harness));
 assert.equal(ui.results.length,2);const pids=[];
 for(const [index,r]of ui.results.entries()){
  assert.equal(r.role,['actual_source9_baseline','reviewed_one_line_proposal'][index]);assert(Number.isSafeInteger(r.pid)&&r.pid>0);pids.push(r.pid);
  assert.equal(r.close.code,[1,0][index]);assert.equal(r.close.signal,null);assert(Number.isFinite(Date.parse(r.started_at)));assert(Number.isFinite(Date.parse(r.close.closed_at)));assert(Date.parse(r.close.closed_at)>=Date.parse(r.started_at));
  assert.equal(r.passed,[1,3][index]);assert.equal(r.failed,[2,0][index]);assert.equal(r.source.sha256,(index===0?parentFiles:corpus).get('web/app.js').sha256);
  const log=pin(r.log.path,r.log.sha256);assert.equal(log.length,r.log.bytes);const text=log.toString();assert(/^# tests 3$/m.test(text));assert(new RegExp('^# pass '+r.passed+'$','m').test(text));assert(new RegExp('^# fail '+r.failed+'$','m').test(text));
  assert(/^# cancelled 0$/m.test(text)&&/^# skipped 0$/m.test(text)&&/^# todo 0$/m.test(text));
  const cases=[...text.matchAll(/^(ok|not ok) \d+ - (source_body_selection_[^\r\n]+)$/gm)];assert.deepEqual(cases.map(m=>m[2]),composition.separate_node_verification.names);assert.equal(cases.filter(m=>m[1]==='ok').length,r.passed);assert.equal(cases.filter(m=>m[1]==='not ok').length,r.failed);
 }
 assert.equal(new Set(pids).size,2);assert.equal(ui.product_root_unchanged.sha256,parentFiles.get('web/app.js').sha256);
 return expected;
}
export function loadSource10Approval({approvalPath,approvalSha,expectedCommit,repo=path.join(BASE,'mcp-file-tools-v1')}){
 const references=new Map();
 function pin(file,hash){assert(path.isAbsolute(file));assert.match(hash,HASH);const s=fs.lstatSync(file);assert(s.isFile()&&!s.isSymbolicLink()&&s.size<=64*1024*1024,file);const raw=fs.readFileSync(file);assert.equal(raw.length,s.size);assert.equal(sha(raw),hash,file);const k=normalized(file),old=references.get(k);if(old)assert.equal(old.sha256,hash);else references.set(k,{path:file,sha256:hash,bytes:raw.length});return raw;}
 const data=r=>{const raw=pin(r.path,r.sha256);if(r.bytes!==undefined)assert.equal(raw.length,r.bytes);return JSON.parse(raw);};
 const baselinePath=path.join(BASE,'combined-source-v9/manifest.json'),baseline=data({path:baselinePath,sha256:PARENT_SHA});
 assert.equal(baseline.kind,'combined_source_v9');assert.equal(baseline.source_commit,'374229117c6fcc98d942976aa55444517d40524a');assert.equal(baseline.source_inputs,358);assert.equal(baseline.required_test_names,577);assert.equal(baseline.minimum_expected_workspace_tests,1359);assert.deepEqual(baseline.test_profile,profile);
 // All parent hashes are checked ONLY in immutable E/frozen Source9. Current Root may already contain369 inputs.
 const parentFiles=rowsMap(baseline.files,358),parentRaw=new Map();for(const f of baseline.files){const raw=pin(path.join(path.dirname(baselinePath),f.path),f.sha256);assert.equal(raw.length,f.bytes);parentRaw.set(f.path,raw);}
 const parentSuiteVerified=verifyParentSuite(baseline,pin),parentBuildVerified=verifyParentBuild(baseline,parentSuiteVerified.verified,pin);
 const parentSuiteReference={path:path.join(BASE,'results/combined-suite-v9/proof.json'),sha256:PARENT_SUITE_SHA},parentBuildReference={path:path.join(BASE,'results/combined-optimized-build-v9/proof.json'),sha256:PARENT_BUILD_SHA};
 const rootApprovalReference={path:approvalPath,sha256:approvalSha},rootApproval=data(rootApprovalReference);
 assert.equal(rootApproval.schema_version,1);assert.equal(rootApproval.kind,'root_approved_combined_source_v10');assert.equal(rootApproval.status,'REVIEWED_MERGED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED');assert.equal(rootApproval.parent_source_manifest_sha256,PARENT_SHA);assert.match(rootApproval.source_commit,/^[a-f0-9]{40}$/);if(expectedCommit)assert.equal(rootApproval.source_commit,expectedCommit);assert.notEqual(rootApproval.source_commit,baseline.source_commit);assert.equal(rootApproval.source_branch,baseline.source_branch);assert.equal(normalized(rootApproval.source_worktree),normalized(repo));assert.equal(rootApproval.private_repository_verified,true);sameRef(rootApproval.parent_suite,parentSuiteReference);sameRef(rootApproval.parent_build,parentBuildReference);
 assert.deepEqual(rootApproval.proposals.map(r=>r.role),['composition']);const proposal=rootApproval.proposals[0];assert.equal(proposal.sha256,COMPOSITION_SHA);assert.equal(normalized(proposal.path),normalized(COMPOSITION_PATH));assert.match(proposal.review_sha256,HASH);
 const composition=data(proposal),compositionReview=data({path:proposal.review_path,sha256:proposal.review_sha256});assert.equal(compositionReview.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(compositionReview.findings,[]);sameRef(compositionReview.proposal_manifest,proposal);
 assert.equal(composition.kind,'combined_source_v10_draft_composition');sameRef(composition.parent_manifest,{path:baselinePath,sha256:PARENT_SHA});sameRef(composition.parent_suite,parentSuiteReference);assert.equal(composition.parent_source_commit,baseline.source_commit);assert.equal(composition.parent_source_inputs,358);assert.equal(composition.source_inputs,369);assert.equal(composition.owned_paths,18);assert.equal(composition.changed_parent_inputs,7);assert.equal(composition.new_input_files,11);assert.equal(composition.unchanged_parent_inputs,351);assert.equal(composition.required_test_names,598);assert.equal(composition.minimum_expected_workspace_tests,1380);assert.equal(composition.all_required_full_test_name_count,602);
 assert.deepEqual(composition.source_proposals,expectedSourceRoles);assert.deepEqual(rootApproval.source_reviews,expectedSourceRoles);
 const rolePayloads=new Map();for(const row of composition.source_proposals){const value=data(row.manifest),review=data(row.review);assert.equal(review.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(review.findings,[]);sameRef(review.proposal_manifest,row.manifest);const payload=path.join(path.dirname(row.manifest.path),'source');const files=new Map();for(const f of value.files){relativeFile(f.path);const hash=f.after_sha256??f.sha256,bytes=f.after_bytes??f.bytes;const raw=pin(path.join(payload,f.path),hash);assert.equal(raw.length,bytes);files.set(f.path,{sha256:hash,bytes,raw});}rolePayloads.set(row.role,files);}
 const corpus=rowsMap(composition.files,369),authoredRaw=new Map();assert.equal(normalized(composition.source_directory),normalized(path.join(path.dirname(COMPOSITION_PATH),'source')));assert.equal(normalized(composition.preformat_directory),normalized(path.join(path.dirname(COMPOSITION_PATH),'preformat')));
 const bindings=composition.approved_product_bindings;assert.equal(bindings.length,18);const owned=bindings.map(f=>relativeFile(f.path));assert.equal(new Set(owned).size,18);assert.deepEqual(owned,[...owned].sort());assert.equal(owned.filter(p=>parentFiles.has(p)).length,7);assert.equal(owned.filter(p=>!parentFiles.has(p)).length,11);
 for(const f of corpus.values()){const raw=pin(path.join(composition.preformat_directory,f.path),f.sha256);assert.equal(raw.length,f.bytes);authoredRaw.set(f.path,raw);if(!owned.includes(f.path))assert.deepEqual(f,parentFiles.get(f.path));}
 assert.deepEqual([...corpus.keys()].sort(),[...new Set([...parentFiles.keys(),...owned])].sort());for(const row of bindings){assert.equal(row.before_sha256,parentFiles.get(row.path)?.sha256??'ABSENT');assert.equal(row.before_bytes,parentFiles.get(row.path)?.bytes??0);assert.deepEqual(corpus.get(row.path),{path:row.path,sha256:row.after_sha256,bytes:row.bytes});assert.equal(row.roles.length,1);assert(rolePayloads.get(row.roles[0]).has(row.path));}
 const derivation=data(composition.composition_derivation);assert.equal(derivation.kind,'source10_exact_reviewed_hunk_composition');sameRef(derivation.parent_manifest,composition.parent_manifest);assert.deepEqual(derivation.source_proposals,composition.source_proposals);assert.equal(derivation.operations.length,15);assert.equal(derivation.whole_modified_file_inverses.length,7);assert.equal(derivation.new_files.length,11);assert.equal(derivation.format_heuristics,0);assert.equal(derivation.whole_historical_shared_file_overwrites,0);
 const running=new Map([...parentRaw].map(([p,b])=>[p,Buffer.from(b)]));
 for(const op of derivation.operations){assert(owned.includes(op.path));assert(bindings.find(f=>f.path===op.path).roles.includes(op.role));const b=running.get(op.path);assert.equal(sha(b),op.before_sha256);assert(Number.isSafeInteger(op.offset)&&op.offset>=0);const before=Buffer.from(op.before_base64,'base64'),after=Buffer.from(op.after_base64,'base64');assert(before.length&&after.length);assert.equal(b.indexOf(before),op.offset);assert.equal(b.indexOf(before,op.offset+1),-1);assert(b.subarray(op.offset,op.offset+before.length).equals(before));const next=Buffer.concat([b.subarray(0,op.offset),after,b.subarray(op.offset+before.length)]);assert.equal(sha(next),op.after_sha256);running.set(op.path,next);}
 for(const row of derivation.whole_modified_file_inverses){assert.equal(row.before_sha256,parentFiles.get(row.path).sha256);assert.equal(row.after_sha256,corpus.get(row.path).sha256);assert(running.get(row.path).equals(authoredRaw.get(row.path)));let inverse=running.get(row.path);for(const op of derivation.operations.filter(o=>o.path===row.path).reverse()){const before=Buffer.from(op.before_base64,'base64'),after=Buffer.from(op.after_base64,'base64');assert(inverse.subarray(op.offset,op.offset+after.length).equals(after));inverse=Buffer.concat([inverse.subarray(0,op.offset),before,inverse.subarray(op.offset+after.length)]);assert.equal(sha(inverse),op.before_sha256);}assert(inverse.equals(parentRaw.get(row.path)));}
 for(const row of derivation.new_files){assert(!parentFiles.has(row.path));const payload=rolePayloads.get(row.role).get(row.path);assert.equal(payload.sha256,corpus.get(row.path).sha256);assert(payload.raw.equals(authoredRaw.get(row.path)));assert.equal(sha(pin(row.reviewed_payload.path,row.reviewed_payload.sha256)),payload.sha256);}
 const uiReference=verifyUi(composition,rootApproval,parentFiles,corpus,pin,data);
 const sourceFiles=rowsMap(rootApproval.source_files,369),deltaPins=new Map();assert.deepEqual(rootApproval.files.map(f=>f.path),owned);
 for(const row of rootApproval.files){const authored=corpus.get(row.path);assert.equal(row.before_sha256,parentFiles.get(row.path)?.sha256??'ABSENT');assert.equal(row.authored_sha256,authored.sha256);assert.match(row.after_sha256,HASH);assert.equal(row.format_only,row.authored_sha256!==row.after_sha256);assert.deepEqual(sourceFiles.get(row.path),{path:row.path,sha256:row.after_sha256,bytes:row.bytes});deltaPins.set(row.path,row);}
 for(const [p,f]of sourceFiles){assert(corpus.has(p));if(!deltaPins.has(p))assert.deepEqual(f,parentFiles.get(p));else assert.notEqual(f.sha256,parentFiles.get(p)?.sha256);}
 const formatted=rootApproval.files.filter(r=>r.format_only).map(r=>({path:r.path,before_sha256:r.authored_sha256,after_sha256:r.after_sha256,bytes:r.bytes}));
 const formatting=data(rootApproval.formatter_review);assert.equal(formatting.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(formatting.findings,[]);assert.deepEqual(formatting.files,formatted);for(const r of formatted)assert.notEqual(r.before_sha256,r.after_sha256);
 const actualFormat=data(rootApproval.actual_format);assert.equal(actualFormat.kind,ACTUAL_FORMAT_KIND);assert.equal(actualFormat.result,'PASS_FORMAT_COMMAND_ONLY');closedStreams(actualFormat);assert.equal(actualFormat.error,null);assert.equal(actualFormat.preformat_sha256,COMPOSITION_SHA);assert.equal(actualFormat.source_inputs,369);assert.deepEqual(actualFormat.changed_files,formatted);
 const preliminary=data(rootApproval.preliminary_clippy);assert.equal(preliminary.kind,'source10_actual_preliminary_clippy');assert.equal(preliminary.result,'PASS_PRELIMINARY_CLIPPY_ONLY');assert.equal(preliminary.executed_tests,0);assert.equal(preliminary.actual_close_event,true);assert.deepEqual(preliminary.exit,{code:0,signal:null});assert(Number.isSafeInteger(preliminary.pid)&&preliminary.pid>0);assert.equal(preliminary.spawn_error,null);assert.equal(preliminary.preformat_sha256,COMPOSITION_SHA);assert.equal(preliminary.format_sha256,rootApproval.actual_format.sha256);assert.equal(preliminary.source_final_verified,true);assert.equal(preliminary.source_inputs,369);assert.deepEqual(preliminary.files,rootApproval.source_files);assert.equal(rootApproval.preliminary_clippy_sha256,rootApproval.preliminary_clippy.sha256);assert.deepEqual(rootApproval.preliminary_clippy_args,['clippy','--workspace','--all-targets','--locked','--','-D','warnings']);assert.equal(rootApproval.preliminary_clippy_log.sha256,preliminary.log_sha256);const clippyLog=pin(rootApproval.preliminary_clippy_log.path,preliminary.log_sha256).toString();assert(/Finished\s+\x60dev\x60\s+profile/.test(clippyLog));assert(!/^error(?:\[|:)|^warning:/m.test(clippyLog));
 const publication=data(rootApproval.private_publication);assert.equal(publication.kind,'actual_private_source10_code_publication');assert.equal(publication.source_commit,rootApproval.source_commit);assert.equal(publication.branch,rootApproval.source_branch);assert.equal(publication.repository,'lia-xim/open-seo-spider');assert.equal(publication.is_private,true);assert.equal(publication.source_inputs,369);assert.equal(publication.all369_root_source_hashes_verified,true);assert.equal(publication.code_and_owned_raw_byte_attributes,true);assert.deepEqual(publication.owned_committed_paths,owned);assert.deepEqual(publication.additional_tracked_files,['.gitattributes']);
 const actualNew=[];for(const [p,raw]of authoredRaw){if(!p.endsWith('.rs'))continue;const old=new Set(declaredTests((parentRaw.get(p)??Buffer.from('')).toString()).map(t=>t.name)),now=declaredTests(raw.toString());for(const name of old)assert(now.some(t=>t.name===name),'Inherited declaration removed: '+p+'::'+name);for(const t of now)if(!old.has(t.name)){assert.notEqual(t.platform,'unix');assert(modules[p],'Unmapped actual test module '+p);actualNew.push({path:p,name:t.name,full_name:verifyModule(authoredRaw,p)+t.name});}}
 const key=t=>t.path+'::'+t.name;assert.equal(actualNew.length,21);assert.equal(composition.new_authored_test_functions.length,21);assert.deepEqual(actualNew.map(key).sort(),composition.new_authored_test_functions.map(key).sort());for(const t of composition.new_authored_test_functions)assert.equal(t.full_name,actualNew.find(a=>key(a)===key(t)).full_name);
 const addedTests=[...new Set(actualNew.map(t=>t.path))].sort().map(p=>({path:p,names:actualNew.filter(t=>t.path===p).map(t=>t.name).sort()})),newFullNames=actualNew.map(t=>t.full_name).sort();assert.equal(new Set(newFullNames).size,21);assert.deepEqual(rootApproval.new_authored_tests,addedTests);assert.deepEqual(rootApproval.new_authored_full_test_names,newFullNames);assert.deepEqual(composition.new_authored_tests,addedTests);assert.deepEqual(composition.new_authored_full_test_names,newFullNames);
 const requiredTests=structuredClone(baseline.required_tests);for(const t of actualNew.sort((a,b)=>key(a)<key(b)?-1:key(a)>key(b)?1:0)){let g=requiredTests.find(g=>g.path===t.path);if(!g){g={path:t.path,names:[]};requiredTests.push(g);}assert(!g.names.includes(t.name));g.names.push(t.name);}const required=requiredTests.flatMap(g=>g.names);assert.equal(required.length,598);assert.equal(new Set(required).size,598);assert.deepEqual(composition.required_tests,requiredTests);
 const expectedFull=[...parentSuiteVerified.full_names,...newFullNames].sort();assert.equal(expectedFull.length,602);assert.equal(new Set(expectedFull).size,602);assert.equal(composition.inherited_full_test_names,581);assert.deepEqual(composition.all_required_full_test_names,expectedFull);const inventory=data(composition.test_inventory);assert.deepEqual(inventory.required_tests,requiredTests);assert.deepEqual(inventory.all_inherited_and_new_full_test_names,expectedFull);assert.deepEqual(inventory.new_authored_test_functions,composition.new_authored_test_functions);assert.equal(inventory.minimum_expected_workspace_tests,1380);assert.equal(inventory.required_test_names,598);
 return {baselinePath,baselineSha:PARENT_SHA,baseline,parentFiles,parentRaw,rootApproval,rootApprovalReference,composition,compositionReference:{path:proposal.path,sha256:proposal.sha256},sourceFiles,deltaPins,paths:[...sourceFiles.keys()],addedTests,newFullNames,requiredTests,required,references,pin,parentSuiteVerified,parentBuildVerified,parentSuiteReference,parentBuildReference,uiReference,minimum:1380};
}
export function validateSource10Manifest({manifest,sourceDirectory,repo=path.join(BASE,'mcp-file-tools-v1')}){
 assert.equal(manifest.schema_version,1);assert.equal(manifest.kind,'combined_source_v10');assert.equal(manifest.result,'FROZEN_REVIEWED_SOURCE_FULL_SUITE_NOT_RUN');assert.equal(normalized(sourceDirectory),normalized(path.join(BASE,'combined-source-v10')));
 const c=loadSource10Approval({approvalPath:manifest.root_approval.path,approvalSha:manifest.root_approval.sha256,expectedCommit:manifest.source_commit,repo}),{sourceFiles,rootApproval}=c;
 assert.equal(manifest.parent_source_manifest_sha256,PARENT_SHA);assert.equal(manifest.parent_source_inputs,358);assert.equal(manifest.source_inputs,369);assert.equal(manifest.inherited_required_test_names,577);assert.equal(manifest.new_input_files,11);assert.equal(manifest.changed_parent_inputs,7);assert.equal(manifest.unchanged_parent_inputs,351);assert.equal(manifest.minimum_expected_workspace_tests,1380);assert.equal(manifest.required_test_names,598);assert.equal(manifest.new_authored_test_functions,21);assert.deepEqual(manifest.test_profile,profile);assert.deepEqual(manifest.files,rootApproval.source_files);assert.deepEqual(manifest.required_tests,c.requiredTests);assert.deepEqual(manifest.new_authored_tests,c.addedTests);assert.deepEqual(manifest.new_authored_full_test_names,c.newFullNames);assert.deepEqual(manifest.source_proposals,rootApproval.proposals);sameRef(manifest.parent_suite,c.parentSuiteReference);sameRef(manifest.parent_build,c.parentBuildReference);sameRef(manifest.ui_regressions,c.uiReference);assert.equal(manifest.source_branch,rootApproval.source_branch);assert.equal(normalized(manifest.source_worktree),normalized(repo));
 const bindings=[...c.deltaPins.values()].map(r=>({path:r.path,before_sha256:r.before_sha256,after_sha256:r.after_sha256,bytes:r.bytes}));assert.deepEqual(manifest.approved_product_bindings,bindings);
 const validatorPath=fileURLToPath(import.meta.url);assert.equal(normalized(manifest.validator.path),normalized(validatorPath));c.pin(validatorPath,manifest.validator.sha256);
 const published=new Map();for(const ref of manifest.references){assert(path.isAbsolute(ref.path));assert.match(ref.sha256,HASH);assert(Number.isSafeInteger(ref.bytes)&&ref.bytes>=0);assert(!published.has(normalized(ref.path)));published.set(normalized(ref.path),ref);}assert.equal(published.size,c.references.size);for(const [k,r]of c.references)assert.deepEqual(published.get(k),r);
 for(const f of sourceFiles.values())for(const dir of [sourceDirectory,repo]){const raw=c.pin(path.join(dir,f.path),f.sha256);assert.equal(raw.length,f.bytes);}
 const finalRaw=new Map([...sourceFiles.keys()].map(p=>[p,fs.readFileSync(path.join(sourceDirectory,p))]));for(const p of Object.keys(modules)){verifyModule(finalRaw,p);const actual=declaredTests(finalRaw.get(p).toString()).filter(t=>t.platform!=='unix').map(t=>t.name).sort();assert.deepEqual(actual,c.addedTests.find(g=>g.path===p).names);}
 const files=sourceFiles,rendererFiles=[...files.values()].filter(f=>f.path.startsWith('renderer/'));assert.equal(rendererFiles.length,4);
 const lineage=Object.freeze({parent_source_manifest_sha256:PARENT_SHA,parent_suite_proof_sha256:PARENT_SUITE_SHA,parent_build_proof_sha256:PARENT_BUILD_SHA,composition_proposal_sha256:COMPOSITION_SHA,root_approval_sha256:manifest.root_approval.sha256,ui_regressions_sha256:c.uiReference.sha256,parent_inputs:358,required_parent_names:577,unchanged_parent_inputs:351,changed_parent_inputs:7,new_input_files:11,delta_bindings:18,new_named_windows_functions:c.addedTests.flatMap(g=>g.names),actual_required_names:598,actual_source_inputs:369,minimum_expected_workspace_tests:1380});
 const expectedNewFullNames=[...c.parentSuiteVerified.full_names,...c.newFullNames].sort();assert.equal(expectedNewFullNames.length,602);assert.equal(new Set(expectedNewFullNames).size,602);
 return {...c,files,rendererFiles,lineage,pinnedReferences:published,expectedNewFullNames};
}
