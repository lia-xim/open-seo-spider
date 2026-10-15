/** AUTHORED ONLY. No fixture, database, process, request or physical proof run. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import readline from 'node:readline';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';

const OWN = path.dirname(fileURLToPath(import.meta.url));
const ROOT = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const TASK = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const SOURCE = path.join(ROOT, 'sitemap-source-processing-ui-source-v1');
const REPO = path.join(ROOT, 'sitemap-source-processing-ui-v1');
const SOURCE_SHA = 'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';
const COMMIT = 'f4ed4b9bb7c760755ec7e448aac660cfb6063ade';
const PUBLICATION = path.join(TASK, 'work/sitemap-source-processing-ui-published-v1.json');
const PUBLICATION_SHA = '4316704b11e47d7f49bb541e9b1db9b1c31ac9c53d5b68dd2c47ab339b4bc24b';
const SUITE_DRIVER_SHA = 'b4c0ee42c03b6ade2f34f603bd501862568ed07960579d99cec4c14c1a07d43e';
const BUILD_DRIVER_SHA = '1c72e8b67ef818665b081d91689d91c737a9c95f69f9fde954b72a8323f0bded';
const PREPARATIONS_PUBLICATION = path.join(TASK, 'work/sitemap-native-preparations-published-v1.json');
const PREPARATIONS_PUBLICATION_SHA = 'ab9c37420bee6861f1317632dd8fbe90cd16dcf69f662d80cd00d76566c0b795';
const HISTORICAL_BINARY = 'ab1ac93c82905aa627d100d8622a50acf93dd7fe59c3f854eecdaac667110bcf';
const CSV_CAP = 64 * 1024 * 1024, JSON_CAP = 8 * 1024 * 1024, FIELD_CAP = 65_536;
const ENGINE = 'open-seo-spider/app-sitemap-findings-v1';
const SELECTION = {filter: 'urls-in-sitemap', assessment: 'all', requested_url: null, source_task_url: null, search: null};
const RENDERERS = ['live_browser_worker_executes_javascript_for_core_crawl', 'live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness', 'live_browser_worker_pause_drains_render_and_retains_rendered_discoveries', 'live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion'];
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const bytes = value => Buffer.byteLength(value, 'utf8');
const hashPattern = /^[a-f0-9]{64}$/;
const utf8 = value => Buffer.from(value, 'utf8');
async function within(promise, ms, reason) { let timer; try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(Error(reason)), ms); })]); } finally { clearTimeout(timer); } }
function childOf(base, file) { const r = path.relative(base, file); return r !== '' && r !== '..' && !r.startsWith('..' + path.sep) && !path.isAbsolute(r); }
function regular(file, maximum, allowEmpty = false) { const s = fs.lstatSync(file); assert(s.isFile() && !s.isSymbolicLink() && s.size >= (allowEmpty ? 0 : 1) && s.size <= maximum, 'bounded regular file: ' + file); return fs.readFileSync(file); }
async function hashFile(file) { let length = 0; const h = createHash('sha256'); for await (const chunk of fs.createReadStream(file)) { length += chunk.length; h.update(chunk); } return {bytes: length, sha256: h.digest('hex')}; }
function pairedArgs() {
  const names = ['binary', 'binary-sha256', 'suite-proof', 'suite-sha256', 'build-proof', 'build-sha256', 'compute-release', 'release-sha256', 'out'];
  const a = {};
  for (let i = 2; i < process.argv.length; i += 2) { const k = process.argv[i]?.slice(2); assert(process.argv[i]?.startsWith('--') && names.includes(k) && !Object.hasOwn(a, k) && process.argv[i + 1], 'exact unique CLI pairs only'); a[k] = process.argv[i + 1]; }
  for (const k of names) { assert(a[k], 'missing --' + k); if (k.endsWith('sha256')) assert.match(a[k], hashPattern); else a[k] = path.resolve(a[k]); }
  return a;
}
function summaries(raw) { return [...raw.toString('utf8').matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m => ({state: m[1], passed: +m[2], failed: +m[3], ignored: +m[4]})); }
function successfulRun(run, name, args) { assert.equal(run.name, name); assert.deepEqual(run.args, args); assert.equal(run.actual_close_event, true); assert.equal(run.spawn_error, null); assert.deepEqual(run.exit, {code: 0, signal: null}); assert.match(run.log_sha256, hashPattern); }

// No output, local module evaluation, SQLite import, fixture allocation, listen
// or child before the coordinated release and ALL immutable input guards.
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed', 'Coordinated compute HOLD remains active');
const a = pairedArgs();
assert(Number(process.versions.node.split('.')[0]) >= 24, 'Node24+ built-in SQLite required');
assert.equal(path.resolve(OWN), path.join(ROOT, 's6-finding-physical-ui-rebind-v1'));
const selfSha = sha(regular(fileURLToPath(import.meta.url), 512 * 1024));
const publicationRaw = regular(PUBLICATION, 1024 * 1024);
assert.equal(sha(publicationRaw), PUBLICATION_SHA);
const publication = JSON.parse(publicationRaw);
assert.equal(publication.commit, COMMIT); assert.equal(publication.source_manifest_sha256, SOURCE_SHA);
assert.equal(publication.repository.isPrivate, true); assert.equal(publication.source_inputs, 297);
assert.equal(sha(regular(path.join(TASK, 'work/run-sitemap-source-ui-suite-v1.mjs'), 512 * 1024)), SUITE_DRIVER_SHA);
assert.equal(sha(regular(path.join(TASK, 'work/build-sitemap-source-ui-v2.mjs'), 512 * 1024)), BUILD_DRIVER_SHA);
const releaseRaw = regular(a['compute-release'], 1024 * 1024); assert.equal(sha(releaseRaw), a['release-sha256']);
const release = JSON.parse(releaseRaw);
assert.equal(release.status, 'RELEASED'); assert.equal(release.kind, 'root_released_s6_finding_physical_ui_v1'); assert.equal(release.scope, 'native_s6_finding_csv_json_physical_ui_v1');
for (const [k, v] of Object.entries({source_manifest_sha256: SOURCE_SHA, source_commit: COMMIT, publication_receipt_sha256: PUBLICATION_SHA, preparations_publication_sha256: PREPARATIONS_PUBLICATION_SHA, harness_sha256: selfSha, suite_proof_sha256: a['suite-sha256'], build_proof_sha256: a['build-sha256'], binary_sha256: a['binary-sha256']})) assert.equal(release[k], v, 'Root operative release pin: ' + k);
const notBefore = Date.parse(release.not_before), expires = Date.parse(release.expires_at);
assert(Number.isFinite(notBefore) && Number.isFinite(expires) && notBefore <= Date.now() && Date.now() < expires && expires - notBefore <= 4 * 3600_000, 'bounded active Root compute release');
const sourceRaw = regular(path.join(SOURCE, 'manifest.json'), 16 * 1024 * 1024); assert.equal(sha(sourceRaw), SOURCE_SHA);
const source = JSON.parse(sourceRaw), required = source.required_tests.flatMap(g => g.names);
// Fixed historical publication lineage; later metadata commits may change HEAD.
// Actual current compiler bytes in repo/source, not live Git metadata, admit IO.
function verifyUiLineage() {
  const baseSha = '69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751';
  const uiSha = 'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';
  const baseCommit = 'f43efc17d1393645a9eb8c8b8510743f3dfedb5c';
  const uiCommit = 'f4ed4b9bb7c760755ec7e448aac660cfb6063ade';
  const basePublicationSha = 'b71188d145a8e720bd9d6c9aaf4720c03c20cced7f406839264e5432e85e84ff';
  const uiPublicationSha = '4316704b11e47d7f49bb541e9b1db9b1c31ac9c53d5b68dd2c47ab339b4bc24b';
  const baseDirectory = path.join(ROOT, 'sitemap-findings-surfaces-source-v1');
  const baseRaw = fs.readFileSync(path.join(baseDirectory, 'manifest.json'));
  assert.equal(sha(baseRaw), baseSha, 'Genuine base694 manifest required');
  assert.equal(sha(fs.readFileSync(path.join(SOURCE, 'manifest.json'))), uiSha, 'Current UI frozen manifest');
  const base = JSON.parse(baseRaw);
  const basePublicationRaw = fs.readFileSync(path.join(TASK, 'work/sitemap-findings-surfaces-published-v1.json'));
  const uiPublicationRaw = fs.readFileSync(path.join(TASK, 'work/sitemap-source-processing-ui-published-v1.json'));
  assert.equal(sha(basePublicationRaw), basePublicationSha, 'Fixed base publication receipt');
  assert.equal(sha(uiPublicationRaw), uiPublicationSha, 'Fixed UI publication receipt');
  const basePublication = JSON.parse(basePublicationRaw), uiPublication = JSON.parse(uiPublicationRaw);
  assert.equal(basePublication.commit, baseCommit); assert.equal(basePublication.source_manifest_sha256, baseSha); assert.equal(basePublication.repository.isPrivate, true);
  assert.equal(uiPublication.commit, uiCommit); assert.equal(uiPublication.source_manifest_sha256, uiSha); assert.equal(uiPublication.repository.isPrivate, true);
  assert.equal(uiPublication.base_source_commit, baseCommit); assert.equal(uiPublication.base_source_manifest_sha256, baseSha);
  assert.equal(uiPublication.source_inputs, 297); assert.equal(uiPublication.unchanged_base_inputs, 295); assert.equal(uiPublication.changed_ui_inputs, 2);
  assert.equal(uiPublication.new_authored_rust_tests, 0); assert.equal(uiPublication.inherited_source_authored_test_expectations, 376);
  assert.equal(uiPublication.required_test_names, 388); assert.equal(uiPublication.minimum_expected_workspace_tests, 1170);
  assert.equal(uiPublication.source_preparation_proof_sha256, 'b71c0b855945f41958a68bf168f7a2a00fd51cb15d17b5c7157045fcefe2f458');
  assert.equal(base.files.length, 297); assert.equal(base.new_authored_test_functions, 47); assert.equal(base.approved_product_bindings.length, 38);
  assert.equal(base.required_test_names, 388); assert.equal(base.minimum_expected_workspace_tests, 1170);
  assert.equal(source.base_source_commit, baseCommit); assert.equal(source.base_source_manifest_sha256, baseSha);
  assert.equal(source.base_inputs, 297); assert.equal(source.source_inputs, 297); assert.equal(source.unchanged_inputs, 295);
  assert.equal(source.new_authored_test_functions, 0); assert.equal(source.inherited_source_authored_test_expectations, 376);
  assert.equal(path.resolve(source.source_worktree), path.resolve(REPO));
  assert.deepEqual(source.required_tests, base.required_tests, 'All inherited grouped names/order remain exact');
  assert.equal(source.review.sha256, '3e67b838a3f90a0381b274b85d9fd51cc41fcc435b71f15377b5e53d40fbbb5f');
  assert.equal(sha(fs.readFileSync(path.join(ROOT, 's4-source-processing-ui-review-v1/receipt.json'))), source.review.sha256);
  const expectedUI = [
    {path: 'web/app.js', before_sha256: '84df9b724802d86d69262d98dcee10e22d465c7a6583fc034b438df452bb0bb9', after_sha256: 'b1ec57080fec11529e89b21ce07471ea3ddc2141d4f64ae79ffc236c9b5af3f0', bytes: 379587},
    {path: 'web/index.html', before_sha256: 'f5f494781f91029f1b64ace3c0f1ddcba742014690e0d4ee0dbbef53ebf79cff', after_sha256: 'ed8b3d0a624d02b428a3a7430845b417eca73a34bd3ecd6b0d5899d46bd15e77', bytes: 82970},
  ];
  assert.deepEqual(source.approved_product_bindings, expectedUI, 'Only the exact two reviewed UI payloads');
  const baseFiles = new Map(base.files.map(file => [file.path, file]));
  const uiFiles = new Map(source.files.map(file => [file.path, file]));
  assert.equal(baseFiles.size, 297); assert.equal(uiFiles.size, 297);
  assert.deepEqual([...baseFiles.keys()].sort(), [...uiFiles.keys()].sort(), 'Identical297 input path set');
  let unchanged = 0, changed = 0;
  const verifyFile = (directory, file) => {
    const current = fs.readFileSync(path.join(directory, file.path));
    assert.equal(current.length, file.bytes, file.path + ' exact bytes');
    assert.equal(sha(current), file.sha256, file.path + ' exact SHA');
  };
  for (const [filePath, previous] of baseFiles) {
    const current = uiFiles.get(filePath), binding = expectedUI.find(item => item.path === filePath);
    if (binding) {
      assert.equal(previous.sha256, binding.before_sha256); assert.equal(current.sha256, binding.after_sha256); assert.equal(current.bytes, binding.bytes);
      assert.notEqual(current.sha256, previous.sha256); changed++;
    } else { assert.deepEqual(current, previous, filePath + ' inherited input unchanged'); unchanged++; }
    verifyFile(baseDirectory, previous); verifyFile(SOURCE, current); verifyFile(REPO, current);
  }
  assert.equal(unchanged, 295); assert.equal(changed, 2);
  return {base_source_manifest_sha256: baseSha, base_source_commit: baseCommit, base_publication_receipt_sha256: basePublicationSha, ui_source_manifest_sha256: uiSha, ui_source_commit: uiCommit, ui_publication_receipt_sha256: uiPublicationSha, base_inputs: 297, unchanged_inputs: unchanged, changed_ui_inputs: changed, inherited_base_new_rust_tests: 47, inherited_base_approved_bindings: 38, new_ui_rust_tests: 0, approved_ui_bindings: 2, inherited_source_authored_expectations: 376, inherited_required_names: 388, minimum_expected_workspace_tests: 1170};
}
const sourceUILineage = verifyUiLineage();
// Historical preparation publication only. This is separate from the exact
// source_lineage object returned by the current UI suite and builder.
function verifyPreparationPublication() {
  const raw = regular(PREPARATIONS_PUBLICATION, 1024 * 1024);
  assert.equal(sha(raw), PREPARATIONS_PUBLICATION_SHA);
  const record = JSON.parse(raw);
  assert.equal(record.result, 'PUSHED_PRIVATE_SOURCE_PREPARATIONS_ONLY_GOAL_ACTIVE');
  assert.equal(record.commit, '70f2f45f92a896a12d81b3a46558c2b50f0e8db7');
  assert.equal(record.repository.isPrivate, true);
  assert.equal(record.current_ui_source_commit, COMMIT); assert.equal(record.current_ui_source_manifest_sha256, SOURCE_SHA);
  assert.equal(record.source_inputs, 297); assert.equal(record.unchanged_base_inputs, 295); assert.equal(record.new_ui_bindings, 2); assert.equal(record.new_ui_rust_tests, 0);
  assert.equal(record.native_basis_source_commit, 'f43efc17d1393645a9eb8c8b8510743f3dfedb5c');
  assert.equal(record.native_basis_source_manifest_sha256, '69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751');
  assert.equal(record.native_acceptance_transferred_to_changed_ui, false); assert.equal(record.native_runs, 0);
  assert.equal(record.physical_cases_authored, 12); assert.equal(record.physical_cases_executed, 0);
  assert.equal(record.required_test_names, 388); assert.equal(record.minimum_expected_workspace_tests, 1170);
  const previous = record.selected.physical, ui = record.selected.separate_ui_suite_builder;
  assert.equal(previous.sha256, '95c014077ff72565eb48e243aa48e31da79efec1e82b662812108179a48f1f67');
  assert.equal(previous.selected_manifest_sha256, '50f070b0342d90cc378d360b888e1e5862929ab942762a27709f7567245806b5');
  assert.equal(previous.closure_sha256, 'c48bd77696ea8ebd760d66649145523edae38bc442b17057f06469ba10c1368d'); assert.equal(previous.execution, 0);
  assert.equal(ui.suite_sha256, SUITE_DRIVER_SHA); assert.equal(ui.builder_sha256, BUILD_DRIVER_SHA);
  assert.equal(ui.selected_manifest_sha256, '7f956d0ade7ed0d70ced3ec92766df5d390c0da9abe674e0665a37d49dd6c2aa');
  assert.equal(ui.closure_sha256, 'ee0e25b61b2795b85cbf7ed5abe2dc73c09860bf9b2aa9c73db813db22c8d0a6');
  assert.equal(ui.source_manifest_sha256, SOURCE_SHA); assert.equal(ui.execution, 0); assert.equal(ui.native_acceptance_transferred, false);
  return {metadata_publication_sha256: PREPARATIONS_PUBLICATION_SHA, historical_metadata_commit: record.commit, current_ui_source_commit_at_publication: record.current_ui_source_commit, historical_physical_template_sha256: previous.sha256, historical_physical_manifest_sha256: previous.selected_manifest_sha256, historical_physical_closure_sha256: previous.closure_sha256, ui_suite_driver_sha256: ui.suite_sha256, ui_builder_driver_sha256: ui.builder_sha256, ui_suite_builder_manifest_sha256: ui.selected_manifest_sha256, ui_suite_builder_closure_sha256: ui.closure_sha256, native_acceptance_transferred_at_publication: false, native_runs_at_publication: 0};
}
const preparationsPublicationLineage = verifyPreparationPublication();
assert.equal(source.files.length, 297); assert.equal(source.minimum_expected_workspace_tests, 1170); assert.equal(required.length, 388); assert.equal(new Set(required).size, 388);
const suiteRaw = regular(a['suite-proof'], 4 * 1024 * 1024); assert.equal(sha(suiteRaw), a['suite-sha256']);
const suite = JSON.parse(suiteRaw);
assert.equal(a['suite-proof'], path.join(ROOT, 'results/sitemap-source-ui-suite-v1/proof.json'));
assert.equal(suite.result, 'PASS_LOCAL_CURRENT_WORKSPACE_REAL_RENDERER_CLIPPY_FMT'); assert.equal(suite.source_manifest_sha256, SOURCE_SHA); assert.equal(suite.files, 297); assert.equal(suite.required_test_names, 388);
assert.deepEqual(suite.source_lineage, sourceUILineage, 'Exact current UI suite returned lineage');
assert.equal(suite.owned_worker_closed, true); assert.equal(suite.owned_renderer_gates_verified, true); assert.equal(suite.owned_renderer_gate_count, 4); assert.deepEqual(suite.owned_renderer_gates, RENDERERS); assert.deepEqual(suite.failures, []);
const suiteRuns = [['format', ['fmt', '--all', '--check']], ['workspace-live', ['test', '--workspace', '--locked', '--', '--include-ignored', '--test-threads=1']], ['clippy', ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']]];
assert.deepEqual(suite.runs.map(r => r.name), suiteRuns.map(r => r[0]));
for (const [i, [name, args]] of suiteRuns.entries()) {
  const run = suite.runs[i]; successfulRun(run, name, args);
  const raw = regular(path.join(path.dirname(a['suite-proof']), name + '.log'), 64 * 1024 * 1024, true); assert.equal(sha(raw), run.log_sha256);
  const parsed = summaries(raw); assert.deepEqual(parsed, run.summaries);
  for (const r of parsed) { assert.equal(r.state, 'ok'); assert.equal(r.failed, 0); assert.equal(r.ignored, 0); assert(Number.isSafeInteger(r.passed) && r.passed >= 0); }
  if (name === 'workspace-live') { const total = parsed.reduce((n, r) => n + r.passed, 0); assert(parsed.length && Number.isSafeInteger(total) && total >= 1170); assert.equal(suite.passed, total); assert.equal(suite.ignored, 0); for (const test of required) assert(raw.includes(utf8('::' + test + ' ... ok')), test); for (const test of RENDERERS) assert(raw.includes(utf8('test ' + test + ' ... ok')), test); }
}
const buildRaw = regular(a['build-proof'], 4 * 1024 * 1024); assert.equal(sha(buildRaw), a['build-sha256']); const build = JSON.parse(buildRaw);
assert(['debug', 'optimized'].includes(build.mode)); assert.equal(build.result, 'PASS_LOCAL_CURRENT_' + build.mode.toUpperCase() + '_BUILD');
assert.equal(a['build-proof'], path.join(ROOT, 'results/sitemap-source-ui-' + build.mode + '-build-v2/proof.json'));
assert.equal(build.source_manifest_sha256, SOURCE_SHA); assert.equal(build.input_files, 297); assert.equal(build.source_and_checkout_identity_verified, true); assert.deepEqual(build.failures, []);
assert.deepEqual(build.source_lineage, sourceUILineage, 'Exact current UI builder returned lineage');
assert.equal(build.required_workspace.sha256, a['suite-sha256']); assert.equal(path.resolve(build.required_workspace.path), a['suite-proof']);
assert.equal(build.required_workspace.actual_workspace.passed, suite.passed); assert.equal(build.required_workspace.actual_workspace.required_source_names, 388); assert.equal(build.required_workspace.actual_workspace.required_renderer_gates, 4);
assert.deepEqual(build.required_workspace.verified_runs.map(r => ({name: r.name, log_sha256: r.log_sha256})), suite.runs.map(r => ({name: r.name, log_sha256: r.log_sha256})));
const buildRuns = [['cargo-version', ['--version', '--verbose']], ['rustc-version', ['--version', '--verbose']], ['build', ['build', '--locked', '-p', 'open-seo-spider', ...(build.mode === 'optimized' ? ['--release'] : [])]]];
assert.deepEqual(build.runs.map(r => r.name), buildRuns.map(r => r[0]));
for (const [i, [name, args]] of buildRuns.entries()) { successfulRun(build.runs[i], name, args); assert.equal(sha(regular(path.join(path.dirname(a['build-proof']), name + '.log'), 64 * 1024 * 1024, true)), build.runs[i].log_sha256); }
assert.match(a['binary-sha256'], hashPattern); assert.notEqual(a['binary-sha256'], HISTORICAL_BINARY);
assert.equal(path.resolve(build.binary.path), a.binary); assert.equal(build.binary.sha256, a['binary-sha256']);
assert.equal(a.binary, path.join(path.dirname(a['build-proof']), 'open-seo-spider.exe'));
assert.equal(path.dirname(a.out), path.join(OWN, 'runs')); assert.match(path.basename(a.out), /^[A-Za-z0-9][A-Za-z0-9_-]{0,95}$/); assert(!fs.existsSync(a.out), 'fresh own result directory');
for (const ancestor of [OWN, path.dirname(a.out)]) if (fs.existsSync(ancestor)) { const s = fs.lstatSync(ancestor); assert(s.isDirectory() && !s.isSymbolicLink()); }
async function verifySource() { assert.deepEqual(verifyUiLineage(), sourceUILineage); assert.deepEqual(verifyPreparationPublication(), preparationsPublicationLineage); assert.equal(sha(regular(path.join(SOURCE, 'manifest.json'), 16 * 1024 * 1024)), SOURCE_SHA); const seen = new Set(); for (const f of source.files) { assert(!seen.has(f.path)); seen.add(f.path); for (const base of [SOURCE, REPO]) { const p = path.resolve(base, f.path); assert(childOf(base, p)); const stat = fs.lstatSync(p); assert(stat.isFile() && !stat.isSymbolicLink()); assert.deepEqual(await hashFile(p), {bytes: f.bytes, sha256: f.sha256}, f.path); } } }
await verifySource(); assert.deepEqual(await hashFile(a.binary), {bytes: build.binary.bytes, sha256: a['binary-sha256']}); assert.deepEqual([...regular(a.binary, 512 * 1024 * 1024).subarray(0, 2)], [0x4d, 0x5a]);
const disk = fs.statfsSync(OWN); assert(disk.bavail * disk.bsize >= 8 * 1024 ** 3, '8GiB owned scratch required');
const {DatabaseSync} = await import('node:sqlite');
fs.mkdirSync(a.out, {recursive: true});
const runtimeRoot = path.join(a.out, 'runtime'); fs.mkdirSync(runtimeRoot);
function save(name, value) { assert(!name.includes('..') && !path.isAbsolute(name)); fs.writeFileSync(path.join(a.out, name), JSON.stringify(value, null, 2) + '\n', {flag: 'wx'}); }
save('inputs.json', {status: 'RUNNING_NOT_ACCEPTED', source_manifest_sha256: SOURCE_SHA, source_commit: COMMIT, publication_receipt_sha256: PUBLICATION_SHA, source_lineage: sourceUILineage, preparation_publication_lineage: preparationsPublicationLineage, harness_sha256: selfSha, args: a, controlled_fact_qualification: 'Only real public Crawler baselines are genuine captures; OFFLINE robots_warning substitutions are synthetic test facts, not website observations or authentic native-warning writer proof.', limits: {csv: CSV_CAP, whole_app_json: JSON_CAP, warning_field: FIELD_CAP, csv_pages: 405, json_pages: 160}});

const appRuns = [], baselines = [], cases = [], failures = [], hits = [];
const siteSockets = new Set(), activeSite = new Set(); let app = null, site = null, siteClosed = false, origin = null;
function stillReleased() { assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed'); assert(Date.now() < expires, 'operative release expired'); assert.equal(sha(regular(a['compute-release'], 1024 * 1024)), a['release-sha256'], 'operative release changed/revoked'); }
async function request(route, {body, maximum = JSON_CAP, timeout = 180_000, saveTo = null} = {}) {
  assert(app?.origin && !app.run.closed); const url = new URL(route, app.origin); assert.equal(url.origin, app.origin); assert(![8787, 8790].includes(Number(url.port)));
  const payload = body === undefined ? null : utf8(JSON.stringify(body)); assert(!payload || payload.length <= 64 * 1024 * 1024);
  const response = await fetch(url, {redirect: 'error', signal: AbortSignal.timeout(timeout), ...(payload ? {method: 'POST', headers: {'content-type': 'application/json'}, body: payload} : {})});
  let length = 0; const h = createHash('sha256'), parts = [], fd = saveTo ? fs.openSync(saveTo, 'wx') : null;
  try { for await (const chunk of response.body) { length += chunk.byteLength; assert(length <= maximum, 'actual response exceeded observation sentinel'); h.update(chunk); if (fd !== null) fs.writeSync(fd, chunk); else parts.push(Buffer.from(chunk)); } } finally { if (fd !== null) fs.closeSync(fd); }
  const digest = h.digest('hex');
  if (saveTo) assert.deepEqual(await hashFile(saveTo), {bytes: length, sha256: digest}, 'saved body must equal all observed network bytes');
  return {status: response.status, headers: Object.fromEntries(response.headers), length, sha256: digest, normal_eof: true, data: saveTo ? null : Buffer.concat(parts)};
}
const decode = response => JSON.parse(response.data);
async function start(label, data, provenance) {
  assert(!app && childOf(runtimeRoot, data)); stillReleased(); assert.deepEqual(await hashFile(a.binary), {bytes: build.binary.bytes, sha256: a['binary-sha256']});
  if (!fs.existsSync(data)) fs.mkdirSync(data);
  const stdout = fs.openSync(path.join(a.out, label + '.stdout.log'), 'wx'), stderr = fs.openSync(path.join(a.out, label + '.stderr.log'), 'wx');
  const run = {label, data, provenance, pid: null, closed: false, forced: false, exit: null, startup_origin: null}; appRuns.push(run);
  let pending = '', logBytes = 0, fatal = null;
  const child = spawn(a.binary, ['serve', '--listen', '127.0.0.1:0', '--data-dir', data], {cwd: SOURCE, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env: {...process.env, TEMP: runtimeRoot, TMP: runtimeRoot}});
  run.pid = child.pid ?? null;
  const close = new Promise(resolve => child.once('close', (code, signal) => { run.closed = true; run.exit = {code, signal}; fs.closeSync(stdout); fs.closeSync(stderr); resolve(); }));
  app = {child, run, close, data, origin: null}; child.once('error', error => { fatal = error; });
  for (const [stream, fd, isOut] of [[child.stdout, stdout, true], [child.stderr, stderr, false]]) stream?.on('data', chunk => {
    logBytes += chunk.length; if (logBytes > 8 * 1024 * 1024) { fatal = Error('bounded child log exceeded'); child.kill('SIGTERM'); return; } fs.writeSync(fd, chunk);
    if (isOut) { pending += chunk.toString('utf8'); if (pending.length > 65_536) { fatal = Error('oversized startup line'); child.kill('SIGTERM'); return; } let end; while ((end = pending.indexOf('\n')) >= 0) { const line = pending.slice(0, end); pending = pending.slice(end + 1); const m = /^Open SEO Spider: (http:\/\/127\.0\.0\.1:([1-9][0-9]*))\r?$/.exec(line); if (m) { if (app?.origin || [8787, 8790].includes(+m[2])) { fatal = Error('invalid own app bind'); child.kill('SIGTERM'); return; } app.origin = m[1]; run.startup_origin = m[1]; } } }
  });
  const until = Date.now() + 180_000; while (!app.origin && Date.now() < until) { if (fatal) throw fatal; assert(!run.closed, 'own app exited during startup'); await delay(50); } assert(app.origin, 'actual owned ephemeral startup receipt'); assert.equal((await request('/healthz', {maximum: 1024 * 1024})).status, 200);
}
async function stop() { if (!app) return; const current = app; app = null; if (!current.run.closed) current.child.kill('SIGTERM'); const force = setTimeout(() => { current.run.forced = true; current.child.kill('SIGKILL'); }, 10_000); try { await within(current.close, 15_000, 'owned native child close missing'); } finally { clearTimeout(force); } assert(current.run.closed && !current.run.forced, 'actual owned child close required'); }
async function files(directory) { const out = []; async function walk(p, relative = '') { for (const name of fs.readdirSync(p).sort()) { const f = path.join(p, name), r = path.join(relative, name); const s = fs.lstatSync(f); assert(!s.isSymbolicLink()); if (s.isDirectory()) await walk(f, r); else { assert(s.isFile()); out.push({path: r.replaceAll('\\', '/'), ...await hashFile(f)}); } } } await walk(directory); return out; }
function quoteIdentifier(value) { assert(/^[A-Za-z_][A-Za-z0-9_]*$/.test(value)); return '"' + value + '"'; }
function fingerprint(db) {
  const h = createHash('sha256'); let rows = 0;
  for (const schema of db.prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name COLLATE BINARY").iterate()) {
    h.update(JSON.stringify(schema) + '\n');
    const pk = db.prepare('PRAGMA table_info(' + quoteIdentifier(schema.name) + ')').all().filter(c => c.pk).sort((a, b) => a.pk - b.pk).map(c => quoteIdentifier(c.name));
    const where = schema.name === 'metadata' ? " WHERE key NOT IN ('report','checkpoint')" : '';
    const order = pk.length ? pk.join(',') : 'rowid';
    for (const row of db.prepare('SELECT * FROM ' + quoteIdentifier(schema.name) + where + ' ORDER BY ' + order).iterate()) { const s = JSON.stringify(row); h.update(String(bytes(s)) + ':' + s + '\n'); rows++; }
  }
  return {rows, sha256: h.digest('hex'), exclusions: ['metadata.report', 'metadata.checkpoint']};
}
function closedBaseline(database, count) {
  const db = new DatabaseSync(database, {readOnly: true}); try {
    const get = key => db.prepare('SELECT value FROM metadata WHERE key=?').get(key).value;
    const report = get('report'), checkpoint = get('checkpoint'), config = get('config');
    assert.equal(JSON.parse(checkpoint).report_sha256, sha(report)); assert.equal(JSON.parse(checkpoint).config_sha256, sha(config)); assert.equal(db.prepare('SELECT count(*) n FROM pages').get().n, count);
    const sourceSummary = JSON.parse(get('sitemap_source_store')); assert.equal(sourceSummary.record_count, 1); assert.equal(sourceSummary.commit_count, 1); assert.equal(sourceSummary.omission, null); assert.equal(sourceSummary.legacy_history_unknown, false);
    assert.equal((report.match(/"robots_warning":null/g) ?? []).length, count, 'only real baseline null warnings are eligible');
    assert.equal(db.prepare("SELECT json_array_length(value,'$.pages') n FROM metadata WHERE key='report'").get().n, count);
    return {count, report_sha256: sha(report), report_bytes: bytes(report), checkpoint_sha256: sha(checkpoint), config_sha256: sha(config), generation: JSON.parse(checkpoint).generation, untouched: fingerprint(db), source_summary: sourceSummary};
  } finally { db.close(); }
}
function warning(p, ordinal, count) { return '"'.repeat(p.base_quotes + (ordinal === count - 1 ? p.last_quotes : 0)) + 'a'.repeat(p.base_ascii + (ordinal === count - 1 ? p.last_ascii : 0)); }
function mutateClosedFork(database, baseline, p) {
  assert(!app, 'app MUST be actually closed before fixture writes');
  for (const n of Object.values(p)) assert(Number.isInteger(n) && n >= 0 && n <= FIELD_CAP);
  assert(bytes(warning(p, baseline.count - 1, baseline.count)) <= FIELD_CAP);
  const db = new DatabaseSync(database); try {
    db.exec('BEGIN IMMEDIATE');
    const get = k => db.prepare('SELECT value FROM metadata WHERE key=?').get(k).value;
    const original = get('report'), checkpoint = get('checkpoint'); assert.equal(sha(original), baseline.report_sha256); assert.equal(sha(checkpoint), baseline.checkpoint_sha256); assert.deepEqual(fingerprint(db), baseline.untouched);
    let ordinal = 0;
    // Byte-fragment-only replacement retains every other original serialization
    // byte, including floats/order/unknown optional fields. No full JSON roundtrip.
    const report = original.replace(/"robots_warning":null/g, () => '"robots_warning":' + JSON.stringify(warning(p, ordinal++, baseline.count)));
    assert.equal(ordinal, baseline.count); assert(bytes(report) <= 256 * 1024 * 1024);
    let inverseCells = 0;
    const inverse = report.replace(/"robots_warning":"(?:[^"\\]|\\.)*"/g, () => { inverseCells++; return '"robots_warning":null'; });
    assert.equal(inverseCells, baseline.count); assert.equal(inverse, original, 'all non-warning report bytes exactly preserved');
    const oldCell = '"report_sha256":"' + baseline.report_sha256 + '"', newCell = '"report_sha256":"' + sha(report) + '"';
    assert.equal(checkpoint.split(oldCell).length - 1, 1, 'actual canonical checkpoint cell required');
    const updatedCheckpoint = checkpoint.replace(oldCell, newCell); assert.equal(updatedCheckpoint.replace(newCell, oldCell), checkpoint);
    db.prepare("UPDATE metadata SET value=? WHERE key='report'").run(report); db.prepare("UPDATE metadata SET value=? WHERE key='checkpoint'").run(updatedCheckpoint);
    assert.equal(db.prepare("SELECT json_valid(value) n FROM metadata WHERE key='report'").get().n, 1);
    assert.deepEqual(fingerprint(db), baseline.untouched, 'raw/A/B/source/config/gen/all other SQL rows unchanged');
    assert.equal(sha(get('config')), baseline.config_sha256); assert.equal(JSON.parse(get('checkpoint')).generation, baseline.generation);
    db.exec('COMMIT');
    return {qualification: 'SYNTHETIC_OFFLINE_WARNING_FACTS_NOT_WEBSITE_OBSERVATIONS', parameters: p, original_report_sha256: baseline.report_sha256, report_sha256: sha(report), report_bytes: bytes(report), original_checkpoint_sha256: baseline.checkpoint_sha256, checkpoint_sha256: sha(updatedCheckpoint), changed_sql_rows: ['metadata.report', 'metadata.checkpoint'], changed_warning_values: ordinal, changed_warning_ordinals: {first: 0, last: ordinal - 1, count: ordinal}, warning_fragment_inverse_exact: true, checkpoint_fragment_inverse_exact: true, unchanged_sql_fingerprint: baseline.untouched, warning_field_bytes_max: bytes(warning(p, baseline.count - 1, baseline.count)), raw_source_config_generation_retained: true};
  } catch (error) { try { db.exec('ROLLBACK'); } catch {} throw error; } finally { db.close(); }
}
async function createBaseline(family, count) {
  const data = path.join(runtimeRoot, 'actual-baseline-' + family); await start('baseline-' + family, data, 'REAL_PUBLIC_CRAWLER_BASELINE');
  let id;
  try {
    const urls = Array.from({length: count}, (_, i) => origin + '/' + family + '/p' + String(i).padStart(3, '0'));
    const input = {seed_urls: urls, sitemap_urls: [origin + '/' + family + '/task.xml'], crawl_links: false, discover_sitemaps: false, obey_robots: false, renderer: null, concurrency: 1, delay_ms: 0, max_pages: count + 1, request_timeout_secs: 10, max_body_bytes: 8 * 1024 * 1024};
    const created = await request('/api/crawls', {body: input, maximum: 1024 * 1024}); assert.equal(created.status, 200); id = decode(created).id; assert.match(id, /^[a-f0-9-]{36}$/);
    let state; const until = Date.now() + 900_000;
    while (Date.now() < until) { await delay(500); const r = await request('/api/crawls/' + id, {maximum: 4 * 1024 * 1024}); assert.equal(r.status, 200); state = decode(r); if (state.status === 'failed') throw Error('real baseline crawl failed: ' + state.error); if (state.status === 'completed') break; }
    assert.equal(state?.status, 'completed'); assert.equal(state.stats.fetched, count);
    const page = await request('/api/crawls/' + id + '/sitemaps/findings', {body: {...SELECTION, limit: 1, candidate_limit: 5000}}); assert.equal(page.status, 200); const value = decode(page); assert.equal(value.engine, ENGINE); assert.equal(value.findings.context.availability, 'available'); assert.equal(value.findings.rows[0].assessment.verdict, 'match'); assert(value.export_token);
  } finally { await stop(); }
  const db = path.join(data, id + '.sqlite'), proof = closedBaseline(db, count);
  const baseline = {family, count, id, data, database: db, ...proof};
  const actualHits = hits.filter(h => h.path.startsWith('/' + family + '/'));
  assert.equal(actualHits.length, count + 1); assert.equal(actualHits.filter(h => h.path.endsWith('/task.xml')).length, 1); assert(actualHits.every(h => h.finished));
  baselines.push(baseline); save('baseline-' + family + '.json', {...baseline, files: await files(data), provenance: 'Actual fresh same-candidate public Crawler and terminal checkpoint before any synthetic fork; no historical proof reused'}); return baseline;
}

async function collectNativeRows(baseline, p, caseDir) {
  const file = path.join(caseDir, 'actual-native-rows.ndjson'), fd = fs.openSync(file, 'wx'); let cursor = null, first = null, count = 0, requests = 0; const tokens = new Set();
  try { do {
    const r = await request('/api/crawls/' + baseline.id + '/sitemaps/findings', {body: {...SELECTION, limit: 10, candidate_limit: 5000, ...(cursor ? {cursor} : {})}}); assert.equal(r.status, 200);
    const page = decode(r); assert.equal(page.website_requests, 0); assert.equal(page.engine, ENGINE); assert.deepEqual(utf8(JSON.stringify(page)), r.data, 'actual ASCII native serialization must match oracle JSON profile');
    assert.equal(page.access_target.job_id, baseline.id); assert.equal(page.access_target.generation, baseline.generation); assert.equal(page.access_target.config_sha256, baseline.config_sha256);
    assert.equal(page.findings.context.availability, 'available'); assert.equal(page.findings.context.basis.analysis_context.available, true); assert.equal(page.findings.context.basis.analysis_context.saved_page_count, baseline.count);
    assert.equal(page.findings.context.basis.sitemap_source_measurement_summary.record_count, 1); assert.equal(page.findings.context.basis.sitemap_source_measurement_summary.commit_count, 1);
    if (!first) first = page; else { assert.deepEqual(page.findings.context, first.findings.context); assert.equal(page.export_token, first.export_token); }
    assert(Array.isArray(page.findings.rows));
    for (const row of page.findings.rows) { assert.equal(row.position.kind, 'saved_page'); assert.equal(row.position.ordinal, count); const candidate = row.evidence.page; assert.equal(row.evidence.domain, 'saved_page'); assert.equal(candidate.finding.observation.report_page_ordinal, count); assert.equal(candidate.finding.membership, 'listed_original'); assert.equal(candidate.finding.original_membership_occurrences, 1); assert.equal(candidate.finding.original_source_document_count, 1); assert.equal(candidate.robots_warning_field_observed, true); assert.equal(candidate.robots_warning, warning(p, count, baseline.count)); assert.equal(row.assessment.verdict, 'match'); assert.deepEqual(row.assessment.reasons, ['listed_original']); fs.writeSync(fd, JSON.stringify(row) + '\n'); count++; }
    cursor = page.next_cursor; if (cursor) { assert(!tokens.has(cursor)); tokens.add(cursor); assert.equal(page.findings.scan.exhausted, false); } else { assert.equal(page.findings.scan.exhausted, true); assert.equal(page.findings.scan.stop, 'exhausted'); }
    requests++; assert(requests <= baseline.count + 1);
  } while (cursor); } finally { fs.closeSync(fd); }
  assert.equal(count, baseline.count); return {first, file, count, requests, rows_file: await hashFile(file)};
}
async function visitRows(file, visit) { const stream = fs.createReadStream(file), lines = readline.createInterface({input: stream, crlfDelay: Infinity}); try { for await (const line of lines) { assert(bytes(line) <= 2 * 1024 * 1024); await visit(JSON.parse(line), line); } } finally { lines.close(); stream.destroy(); } }
async function csvOracle(native) {
  const h = createHash('sha256'); let length = 0, count = 0; const feed = values => { for (const [i, v] of values.entries()) { if (i) { h.update(','); length++; } const cell = '"' + v.replaceAll('"', '""') + '"'; h.update(cell); length += bytes(cell); } h.update('\r\n'); length += 2; };
  feed(['record_type', 'schema_version', 'domain', 'filter', 'assessment', 'original_identity', 'position_json', 'reasons_json', 'typed_finding_json', 'context_json']);
  feed(['context', '1', 'saved_page', SELECTION.filter, SELECTION.assessment, '', '', '', '', JSON.stringify({engine: ENGINE, selection: SELECTION, context: native.first.findings.context})]);
  await visitRows(native.file, (row, canonical) => { const identity = row.evidence.page.finding.observation.requested_url; assert(identity.startsWith(origin + '/'), 'canonical HTTP identities do not need formula prefix in this fixture'); feed(['finding', '1', 'saved_page', SELECTION.filter, row.assessment.verdict, identity, JSON.stringify(row.position), JSON.stringify(row.assessment.reasons), canonical, '']); count++; });
  assert.equal(count, native.count); return {length, sha256: h.digest('hex'), finding_rows: count, context_rows: 1, header_rows: 1, columns: 10, independence: 'Independent uncapped CSV quoting/count/SHA over actual bounded native Finding DTOs and actual metadata context, rowwise; predicate truth and native-warning-writer provenance are not invented'};
}
function templateSegments(value) { const text = JSON.stringify(value), needle = '"rows":[],"scan":', start = text.indexOf(needle); assert(start >= 0 && text.indexOf(needle, start + 1) < 0); const bracket = start + '"rows":'.length; return [text.slice(0, bracket + 1), text.slice(bracket + 1)]; }
async function wholeJsonOracle(native) {
  assert(native.count <= 200, 'whole-native page limit unchanged');
  const envelope = JSON.parse(JSON.stringify(native.first)); envelope.findings.rows = []; envelope.findings.scan = {evaluated_candidates: native.count, exhausted: true, stop: 'exhausted', more_matches: false}; envelope.findings.continuation = null; envelope.next_cursor = null;
  const [prefix, suffix] = templateSegments(envelope), [corePrefix, coreSuffix] = templateSegments(envelope.findings);
  const h = createHash('sha256'), core = createHash('sha256'); let length = bytes(prefix) + bytes(suffix), coreBytes = bytes(corePrefix) + bytes(coreSuffix), count = 0; h.update(prefix); core.update(corePrefix);
  await visitRows(native.file, (_row, canonical) => { if (count) { h.update(','); core.update(','); length++; coreBytes++; } h.update(canonical); core.update(canonical); length += bytes(canonical); coreBytes += bytes(canonical); count++; }); h.update(suffix); core.update(coreSuffix);
  assert.equal(count, native.count); assert(coreBytes < JSON_CAP, 'separate Core page must remain below8MiB; this gate targets whole App wrapper');
  return {length, sha256: h.digest('hex'), core_page_bytes: coreBytes, core_page_sha256: core.digest('hex'), wrapper_bytes: length - coreBytes, rows: count, next_cursor: null, export_token_bytes: bytes(envelope.export_token), complete_envelope_including_tokens: true, oracle: 'Whole actual App serialization shape plus actual native context/token/rows, all stored candidates exhausted, streamed independently; exact native200 is required to validate construction'};
}
async function physicalCase(kind, label, baseline, p, target = null) {
  stillReleased(); assert(!app); assert.match(label, /^[a-z0-9-]+$/); const caseDir = path.join(a.out, label); assert(childOf(a.out, caseDir)); fs.mkdirSync(caseDir); const data = path.join(runtimeRoot, label); assert(childOf(runtimeRoot, data) && !fs.existsSync(data));
  // Source is a CLOSED current-run real baseline, never a user directory.
  const originalFiles = await files(baseline.data); fs.cpSync(baseline.data, data, {recursive: true, errorOnExist: true, force: false}); assert.deepEqual(await files(data), originalFiles);
  const mutation = mutateClosedFork(path.join(data, baseline.id + '.sqlite'), baseline, p); save(label + '.fixture.json', mutation); const beforeHits = hits.length;
  await start(label, data, 'CONTROLLED_OFFLINE_FORK_OF_REAL_BASELINE');
  try {
    const native = await collectNativeRows(baseline, p, caseDir), expected = kind === 'csv' ? await csvOracle(native) : await wholeJsonOracle(native);
    if (target !== null) assert.equal(expected.length, target, 'actual fresh oracle must hit physical size exactly; never hide calibration drift');
    const cap = kind === 'csv' ? CSV_CAP : JSON_CAP; assert(expected.length <= cap + 1); const primedFiles = await files(data), route = '/api/crawls/' + baseline.id + '/sitemaps/findings';
    const physical = await request(kind === 'csv' ? route + '/export.csv' : route, {body: kind === 'csv' ? {...SELECTION, snapshot: native.first.export_token} : {...SELECTION, limit: 200, candidate_limit: 5000}, maximum: cap + 1, timeout: 450_000, saveTo: path.join(caseDir, 'wire-body.bin')});
    if (expected.length <= cap) {
      assert.equal(physical.status, 200); assert.equal(physical.headers['content-length'], String(expected.length)); assert.equal(physical.length, expected.length); assert.equal(physical.sha256, expected.sha256); assert.equal(physical.normal_eof, true);
      if (kind === 'csv') { assert.match(physical.headers['content-type'], /^text\/csv/); assert.equal(physical.headers['x-sitemap-csv-sha256'], expected.sha256); assert.equal(physical.headers['x-sitemap-findings-context-sha256'], native.first.findings.context.context_sha256); assert.equal(physical.headers['x-sitemap-findings-selection-sha256'], native.first.findings.context.selection_sha256); assert.equal(physical.headers['content-disposition'], 'attachment; filename="' + baseline.id + '-sitemap-findings.csv"'); }
      else assert.match(physical.headers['content-type'], /^application\/json/);
    } else {
      assert.equal(expected.length, cap + 1); assert.equal(physical.status, 413); assert(physical.length <= 1024 * 1024);
      for (const header of ['content-disposition', 'x-sitemap-csv-sha256', 'x-sitemap-findings-context-sha256']) assert.equal(physical.headers[header], undefined);
      const error = JSON.parse(regular(path.join(caseDir, 'wire-body.bin'), 1024 * 1024)); assert.equal(error.code, kind === 'csv' ? 'sitemap_findings_csv_byte_budget' : 'sitemap_findings_response_byte_budget'); assert.equal(error.website_requests, 0); assert(!Object.hasOwn(error, 'rows') && !Object.hasOwn(error, 'export_token'));
    }
    const recovery = await request(route, {body: {...SELECTION, limit: 10, candidate_limit: 5000}}); assert.equal(recovery.status, 200); assert.deepEqual(decode(recovery), native.first); assert.equal(hits.length, beforeHits, 'zero website requests during startup/read/export/recovery'); assert.deepEqual(await files(data), primedFiles, 'strict primed read interval leaves saved bytes unchanged');
    const result = {kind, label, target, expected, physical, mutation, native_rows: {count: native.count, requests: native.requests, ...native.rows_file}, native_context_sha256: native.first.findings.context.context_sha256, zero_website_requests: true, saved_bytes_unchanged_in_primed_read_interval: true, public_baseline_id: baseline.id, controlled_offline_test_facts: true}; delete result.physical.data;
    cases.push(result); save(label + '.case.json', result); return result;
  } finally { await stop(); }
}
async function calibrate(kind, baseline) {
  const base = kind === 'csv' ? {base_quotes: 52_000, base_ascii: 0, last_quotes: 0, last_ascii: 0} : {base_quotes: 0, base_ascii: 45_000, last_quotes: 0, last_ascii: 0};
  const first = await physicalCase(kind, kind + '-01-base', baseline, base);
  const quote = await physicalCase(kind, kind + '-02-quote-probe', baseline, {...base, last_quotes: 1});
  const ascii = await physicalCase(kind, kind + '-03-ascii-probe', baseline, {...base, last_ascii: 1});
  const dq = quote.expected.length - first.expected.length, da = ascii.expected.length - first.expected.length, cap = kind === 'csv' ? CSV_CAP : JSON_CAP;
  // Newly observed per-format derivatives only. Old RawCSV5/+2 is never used.
  assert.equal(dq, kind === 'csv' ? 3 : 2, 'actual current quote escaping derivative'); assert.equal(da, 1, 'actual current ASCII derivative');
  const field = kind === 'csv' ? 'base_quotes' : 'base_ascii', step = kind === 'csv' ? dq : da;
  const coarseP = {...base, [field]: base[field] + Math.floor((cap - first.expected.length) / (baseline.count * step))}; assert(coarseP[field] >= base[field]);
  const coarse = await physicalCase(kind, kind + '-04-coarse', baseline, coarseP);
  const remainder = cap - coarse.expected.length; assert(Number.isInteger(remainder) && remainder >= 0 && remainder < baseline.count * step + 128, 'newly measured coarse step; unexpected representation change stops');
  const exactP = {...coarseP, last_ascii: remainder};
  const exact = await physicalCase(kind, kind + '-05-exact', baseline, exactP, cap);
  const plus = await physicalCase(kind, kind + '-06-plus-one', baseline, {...exactP, last_ascii: exactP.last_ascii + 1}, cap + 1);
  save(kind + '-calibration.json', {quote_derivative_newly_observed: dq, ascii_derivative_newly_observed: da, baseline_length: first.expected.length, coarse_length: coarse.expected.length, exact_parameters: exactP, exact_length: exact.expected.length, rejected_plus_one_length: plus.expected.length, native_bodies_measured: true, old_raw_coefficients_reused: false});
}

try {
  site = http.createServer((req, res) => {
    const hit = {path: req.url, method: req.method, finished: false}; hits.push(hit); activeSite.add(hit); req.resume(); req.on('error', () => activeSite.delete(hit)); res.on('error', () => activeSite.delete(hit)); res.once('finish', () => { hit.finished = true; activeSite.delete(hit); }); res.once('close', () => activeSite.delete(hit));
    const match = /^\/(csv|json)\/(task\.xml|p\d{3})$/.exec(req.url ?? '');
    if (!match || req.method !== 'GET' || hits.length > 580) { res.writeHead(404); res.end('unexpected owned fixture request'); return; }
    const count = match[1] === 'csv' ? 405 : 160;
    const body = match[2] === 'task.xml' ? '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">' + Array.from({length: count}, (_, i) => '<url><loc>' + origin + '/' + match[1] + '/p' + String(i).padStart(3, '0') + '</loc></url>').join('') + '</urlset>' : '<html><head><title>Actual current finding boundary fixture</title></head><body>genuine small saved page</body></html>';
    res.writeHead(200, {'content-type': match[2] === 'task.xml' ? 'application/xml' : 'text/html', 'content-length': bytes(body)}); res.end(body);
  });
  site.on('connection', socket => { siteSockets.add(socket); socket.on('error', () => {}); socket.once('close', () => siteSockets.delete(socket)); });
  await new Promise((resolve, reject) => { site.once('error', reject); site.listen(0, '127.0.0.1', resolve); }); const port = site.address().port; assert(![8787, 8790].includes(port)); origin = 'http://127.0.0.1:' + port;
  const csvBaseline = await createBaseline('csv', 405); await calibrate('csv', csvBaseline);
  const jsonBaseline = await createBaseline('json', 160); await calibrate('json', jsonBaseline);
  assert.equal(hits.length, 567, '405+160 actual page requests plus2 actual sitemap requests only');
} catch (error) { failures.push({phase: 'actual-native-fixture/oracle/transport', message: error.message, stack: error.stack}); process.exitCode = 1; }
finally {
  try { await stop(); } catch (error) { failures.push({phase: 'owned-native-close', message: error.message}); process.exitCode = 1; }
  if (site) { try { site.closeIdleConnections(); const close = new Promise(resolve => site.close(() => { siteClosed = true; resolve(); })); const end = Date.now() + 10_000; while ((activeSite.size || siteSockets.size) && Date.now() < end) { site.closeIdleConnections(); await delay(25); } if (activeSite.size || siteSockets.size) { site.closeAllConnections(); throw Error('fixture forced-close required'); } await within(close, 5000, 'fixture close callback missing'); assert(hits.every(h => h.finished)); } catch (error) { failures.push({phase: 'owned-fixture-drain', message: error.message}); process.exitCode = 1; } }
  try { await verifySource(); assert.deepEqual(await hashFile(a.binary), {bytes: build.binary.bytes, sha256: a['binary-sha256']}); for (const b of baselines) assert.deepEqual(closedBaseline(b.database, b.count), {count: b.count, report_sha256: b.report_sha256, report_bytes: b.report_bytes, checkpoint_sha256: b.checkpoint_sha256, config_sha256: b.config_sha256, generation: b.generation, untouched: b.untouched, source_summary: b.source_summary}); } catch (error) { failures.push({phase: 'final-input-and-baseline-preservation', message: error.message}); process.exitCode = 1; }
  const pass = failures.length === 0 && baselines.length === 2 && cases.length === 12 && appRuns.every(r => r.closed && !r.forced) && siteClosed;
  save('proof.json', {schema_version: 1, result: pass ? 'PASS_CURRENT_UI_NATIVE_CONTROLLED_FINDING_CSV64M_APPJSON8M_EXACT_PLUS1' : 'FAIL_CURRENT_UI_NATIVE_FINDING_PHYSICAL_GATE', source_manifest_sha256: SOURCE_SHA, source_commit: COMMIT, publication_receipt_sha256: PUBLICATION_SHA, source_lineage: sourceUILineage, preparation_publication_lineage: preparationsPublicationLineage, harness_sha256: selfSha, binary_sha256: a['binary-sha256'], suite_proof_sha256: a['suite-sha256'], build_proof_sha256: a['build-sha256'], compute_release_sha256: a['release-sha256'], baselines, cases: cases.map(c => ({kind: c.kind, label: c.label, expected_bytes: c.expected.length, actual_status: c.physical.status, actual_body_bytes: c.physical.length, actual_body_sha256: c.physical.sha256, normal_eof: c.physical.normal_eof})), app_runs: appRuns, actual_website_requests: hits, owned_children_closed: appRuns.every(r => r.closed && !r.forced), owned_fixture_drained: siteClosed && !activeSite.size && !siteSockets.size, failures, qualification: 'Latest e9 UI-source candidate only; fixed70f2/ab9 metadata is historical preparation lineage, not acceptance transfer or a live Git HEAD requirement. Physical native transport over genuine same-candidate public Crawler baselines with EXPLICIT SYNTHETIC OFFLINE WARNING FACTS only. Complete HTTP body length/SHA/normal EOF and atomic413 require actual observations. No fake Core session/header/proof, native-warning-writer provenance, historical proof transfer, test-count increase, internal worker Join/TX instrumentation, late-fault/cancel timing, global RSS, rawCSV regression, source50k/50MiB matrix, UI/SDK/platform or manufacturer parity is claimed. Source-bound existing internal barrier tests and separate native gates remain required.'}); if (!pass) process.exitCode = 1;
}
