import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const task = "C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https";
const repo = "E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-source-processing-ui-v1";
const root = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const source = path.join(root, 'sitemap-source-processing-ui-source-v1');
const expectedSourceSha = 'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW, 'confirmed', 'Compute hold must be released before invoking this build');
assert.equal(path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),path.resolve(task),'Promote this guarded driver into the pinned task/work directory before execution');
assert.equal(process.argv.length, 4, 'Expected --mode debug|optimized');
assert.equal(process.argv[2], '--mode');
const mode = process.argv[3];
assert(['debug', 'optimized'].includes(mode));
for (const name of ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']) {
  assert(!process.env[name], 'Unexpected externally configured compiler setting: ' + name);
}
const sourceRaw = fs.readFileSync(path.join(source, 'manifest.json'));
assert.equal(sha(sourceRaw), expectedSourceSha);
const manifest = JSON.parse(sourceRaw);
assert.equal(manifest.files.length, 297);
assert.equal(manifest.required_test_names, 388);
assert.equal(manifest.required_tests.reduce((n,x)=>n+x.names.length,0), 388);
assert.equal(manifest.minimum_expected_workspace_tests, 1170);
assert.equal(manifest.new_authored_test_functions, 0);
assert.equal(manifest.approved_product_bindings.length, 2);
// Fixed historical publication lineage; later metadata commits may change HEAD.
// Actual current compiler bytes in repo/source, not live Git metadata, admit IO.
function verifyUiLineage() {
  const baseSha = '69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751';
  const uiSha = 'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';
  const baseCommit = 'f43efc17d1393645a9eb8c8b8510743f3dfedb5c';
  const uiCommit = 'f4ed4b9bb7c760755ec7e448aac660cfb6063ade';
  const basePublicationSha = 'b71188d145a8e720bd9d6c9aaf4720c03c20cced7f406839264e5432e85e84ff';
  const uiPublicationSha = '4316704b11e47d7f49bb541e9b1db9b1c31ac9c53d5b68dd2c47ab339b4bc24b';
  const baseDirectory = path.join(root, 'sitemap-findings-surfaces-source-v1');
  const baseRaw = fs.readFileSync(path.join(baseDirectory, 'manifest.json'));
  assert.equal(sha(baseRaw), baseSha, 'Genuine base694 manifest required');
  assert.equal(sha(fs.readFileSync(path.join(source, 'manifest.json'))), uiSha, 'Current UI frozen manifest');
  const base = JSON.parse(baseRaw);
  const basePublicationRaw = fs.readFileSync(path.join(task, 'work/sitemap-findings-surfaces-published-v1.json'));
  const uiPublicationRaw = fs.readFileSync(path.join(task, 'work/sitemap-source-processing-ui-published-v1.json'));
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
  assert.equal(manifest.base_source_commit, baseCommit); assert.equal(manifest.base_source_manifest_sha256, baseSha);
  assert.equal(manifest.base_inputs, 297); assert.equal(manifest.source_inputs, 297); assert.equal(manifest.unchanged_inputs, 295);
  assert.equal(manifest.new_authored_test_functions, 0); assert.equal(manifest.inherited_source_authored_test_expectations, 376);
  assert.equal(path.resolve(manifest.source_worktree), path.resolve(repo));
  assert.deepEqual(manifest.required_tests, base.required_tests, 'All inherited grouped names/order remain exact');
  assert.equal(manifest.review.sha256, '3e67b838a3f90a0381b274b85d9fd51cc41fcc435b71f15377b5e53d40fbbb5f');
  assert.equal(sha(fs.readFileSync(path.join(root, 's4-source-processing-ui-review-v1/receipt.json'))), manifest.review.sha256);
  const expectedUI = [
    {path: 'web/app.js', before_sha256: '84df9b724802d86d69262d98dcee10e22d465c7a6583fc034b438df452bb0bb9', after_sha256: 'b1ec57080fec11529e89b21ce07471ea3ddc2141d4f64ae79ffc236c9b5af3f0', bytes: 379587},
    {path: 'web/index.html', before_sha256: 'f5f494781f91029f1b64ace3c0f1ddcba742014690e0d4ee0dbbef53ebf79cff', after_sha256: 'ed8b3d0a624d02b428a3a7430845b417eca73a34bd3ecd6b0d5899d46bd15e77', bytes: 82970},
  ];
  assert.deepEqual(manifest.approved_product_bindings, expectedUI, 'Only the exact two reviewed UI payloads');
  const baseFiles = new Map(base.files.map(file => [file.path, file]));
  const uiFiles = new Map(manifest.files.map(file => [file.path, file]));
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
    verifyFile(baseDirectory, previous); verifyFile(source, current); verifyFile(repo, current);
  }
  assert.equal(unchanged, 295); assert.equal(changed, 2);
  return {base_source_manifest_sha256: baseSha, base_source_commit: baseCommit, base_publication_receipt_sha256: basePublicationSha, ui_source_manifest_sha256: uiSha, ui_source_commit: uiCommit, ui_publication_receipt_sha256: uiPublicationSha, base_inputs: 297, unchanged_inputs: unchanged, changed_ui_inputs: changed, inherited_base_new_rust_tests: 47, inherited_base_approved_bindings: 38, new_ui_rust_tests: 0, approved_ui_bindings: 2, inherited_source_authored_expectations: 376, inherited_required_names: 388, minimum_expected_workspace_tests: 1170};
}
const sourceUILineage = verifyUiLineage();
const ownedRendererGates = ["live_browser_worker_executes_javascript_for_core_crawl","live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness","live_browser_worker_pause_drains_render_and_retains_rendered_discoveries","live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion"];
const suitePath = path.join(root, 'results/sitemap-source-ui-suite-v1/proof.json');
const suiteRaw = fs.readFileSync(suitePath);
const suite = JSON.parse(suiteRaw);
assert.equal(suite.result, 'PASS_LOCAL_CURRENT_WORKSPACE_REAL_RENDERER_CLIPPY_FMT');
assert.equal(suite.source_manifest_sha256, expectedSourceSha);
assert.deepEqual(suite.source_lineage, sourceUILineage, 'Exact new UI suite lineage required');
assert(suite.passed >= 1170 && suite.ignored === 0 && suite.required_test_names === 388);
assert.equal(suite.owned_worker_closed, true);
assert.equal(suite.files, 297);
assert.equal(suite.owned_renderer_gate_count, 4);
assert.equal(suite.owned_renderer_gates_verified, true);
assert.deepEqual(suite.owned_renderer_gates, ownedRendererGates);
// Verify the actual three producer runs and captured logs, not a summary-only
// PASS string. All admission checks finish before any output/backup/child write.
assert(Array.isArray(suite.failures) && suite.failures.length === 0, 'Workspace proof must have no failures');
const expectedSuiteRuns = [
  {name: 'format', args: ['fmt', '--all', '--check']},
  {name: 'workspace-live', args: ['test', '--workspace', '--locked', '--', '--include-ignored', '--test-threads=1']},
  {name: 'clippy', args: ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']},
];
assert(Array.isArray(suite.runs), 'Actual workspace run records required');
assert.deepEqual(suite.runs.map(run => run.name), expectedSuiteRuns.map(run => run.name), 'Exactly format/workspace-live/clippy in producer order required');
const verifiedSuiteRuns = [];
let actualWorkspace = null;
for (const [index, expected] of expectedSuiteRuns.entries()) {
  const run = suite.runs[index];
  assert.deepEqual(run.args, expected.args, expected.name + ' exact command');
  assert.equal(run.actual_close_event, true, expected.name + ' actual child close');
  assert.equal(run.spawn_error, null, expected.name + ' spawn error');
  assert.deepEqual(run.exit, {code: 0, signal: null}, expected.name + ' successful close');
  assert.equal(typeof run.log_sha256, 'string', expected.name + ' log SHA type');
  assert.match(run.log_sha256, /^[0-9a-f]{64}$/, expected.name + ' log SHA profile');
  // Fixed names from the expected command set, never untrusted path components.
  const logPath = path.join(path.dirname(suitePath), expected.name + '.log');
  const logBytes = fs.readFileSync(logPath);
  assert.equal(sha(logBytes), run.log_sha256, expected.name + ' actual log SHA');
  const logText = logBytes.toString('utf8');
  const summaries = [...logText.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)]
    .map(match => ({state: match[1], passed: +match[2], failed: +match[3], ignored: +match[4]}));
  assert(Array.isArray(run.summaries), expected.name + ' actual summary rows required');
  assert.deepEqual(run.summaries, summaries, expected.name + ' summary rows must match its actual log');
  for (const summary of summaries) {
    assert.equal(summary.state, 'ok', expected.name + ' summary state');
    for (const key of ['passed', 'failed', 'ignored']) assert(Number.isSafeInteger(summary[key]) && summary[key] >= 0, expected.name + ' bounded summary count');
    assert.equal(summary.failed, 0, expected.name + ' failed tests');
    assert.equal(summary.ignored, 0, expected.name + ' ignored tests');
  }
  if (expected.name === 'workspace-live') {
    assert(summaries.length > 0, 'Actual workspace test summaries required');
    const total = summaries.reduce((sum, row) => ({passed: sum.passed + row.passed, failed: sum.failed + row.failed, ignored: sum.ignored + row.ignored}), {passed: 0, failed: 0, ignored: 0});
    for (const value of Object.values(total)) assert(Number.isSafeInteger(value), 'Actual workspace aggregate must be exact');
    assert(total.passed >= 1170 && total.failed === 0 && total.ignored === 0, 'Actual workspace minimum/failed/ignored');
    assert.equal(suite.passed, total.passed, 'Proof passed total must equal actual workspace log');
    assert.equal(suite.ignored, total.ignored, 'Proof ignored total must equal actual workspace log');
    for (const group of manifest.required_tests) for (const name of group.names) assert(logText.includes('::' + name + ' ... ok'), 'Required actual source case not green: ' + group.path + '::' + name);
    for (const name of ownedRendererGates) assert(logText.includes('test ' + name + ' ... ok'), 'Required actual owned browser gate not green: ' + name);
    actualWorkspace = {...total, summary_rows: summaries.length, required_source_names: 388, required_renderer_gates: 4};
  }
  verifiedSuiteRuns.push({name: expected.name, args: expected.args, log_path: logPath, log_sha256: sha(logBytes), actual_close_event: run.actual_close_event, spawn_error: run.spawn_error, exit: run.exit, summaries});
}
assert(actualWorkspace !== null, 'Actual workspace run not verified');
const out = path.join(root, 'results/sitemap-source-ui-' + mode + '-build-v2');
assert(!fs.existsSync(out), 'Build result output must be new');
function verify() {
  assert.deepEqual(verifyUiLineage(), sourceUILineage);
  for (const file of manifest.files) {
    for (const base of [source, repo]) {
      const bytes = fs.readFileSync(path.join(base, file.path));
      assert.equal(bytes.length, file.bytes, file.path + ' byte length');
      assert.equal(sha(bytes), file.sha256, file.path + ' SHA');
    }
  }
}
verify();
fs.mkdirSync(out);
const temp = path.join(out, 'temp');
fs.mkdirSync(temp);
const env = {
  ...process.env,
  CARGO_TARGET_DIR: path.join(root, 'cargo-target'),
  CARGO_INCREMENTAL: '0',
  CARGO_PROFILE_DEV_DEBUG: '0',
  CARGO_PROFILE_TEST_DEBUG: '0',
  CARGO_TERM_COLOR: 'never',
  TEMP: temp,
  TMP: temp,
};
const cargo = 'C:/Users/matth/.cargo/bin/cargo.exe';
const target = path.join(root, 'cargo-target', mode === 'debug' ? 'debug' : 'release', 'open-seo-spider.exe');
const runs = [];
const failures = [];
let binary = null;
let previous = null;
function write(name, object) {
  fs.writeFileSync(path.join(out, name), JSON.stringify(object, null, 2) + '\n', {flag: 'wx'});
}
async function command(name, executable, args) {
  const logPath = path.join(out, name + '.log');
  const fd = fs.openSync(logPath, 'wx');
  const started = Date.now();
  let child;
  let spawnError = null;
  let exit;
  try {
    child = spawn(executable, args, {cwd: source, env, windowsHide: true, stdio: ['ignore', fd, fd]});
    child.once('error', error => { spawnError = error.message; });
    exit = await new Promise(resolve => child.once('close', (code, signal) => resolve({code, signal})));
  } finally {
    fs.closeSync(fd);
  }
  const bytes = fs.readFileSync(logPath);
  const receipt = {name, executable, args, pid: child?.pid ?? null, actual_close_event: true, exit, spawn_error: spawnError, elapsed_ms: Date.now() - started, log_sha256: sha(bytes)};
  runs.push(receipt);
  write(name + '.json', receipt);
  console.log(JSON.stringify({name, exit, spawn_error: spawnError}));
  assert.equal(spawnError, null, spawnError ?? 'spawn');
  assert.equal(exit.code, 0, bytes.toString('utf8').slice(-3500));
  assert.equal(exit.signal, null);
}
try {
  await command('cargo-version', cargo, ['--version', '--verbose']);
  await command('rustc-version', 'C:/Users/matth/.cargo/bin/rustc.exe', ['--version', '--verbose']);
  if (fs.existsSync(target)) {
    const bytes = fs.readFileSync(target);
    const preserved = path.join(out, 'previous-target.exe');
    fs.writeFileSync(preserved, bytes, {flag: 'wx'});
    assert.equal(sha(fs.readFileSync(preserved)), sha(bytes));
    previous = {path: preserved, bytes: bytes.length, sha256: sha(bytes)};
  }
  const args = ['build', '--locked', '-p', 'open-seo-spider'];
  if (mode === 'optimized') args.push('--release');
  await command('build', cargo, args);
  verify();
  const bytes = fs.readFileSync(target);
  assert(bytes.length > 0 && bytes[0] === 0x4d && bytes[1] === 0x5a, 'Actual Windows executable required');
  const published = path.join(out, 'open-seo-spider.exe');
  fs.writeFileSync(published, bytes, {flag: 'wx'});
  assert.equal(sha(fs.readFileSync(published)), sha(bytes));
  binary = {path: published, bytes: bytes.length, sha256: sha(bytes)};
} catch (error) {
  failures.push({message: error.message, stack: error.stack});
  process.exitCode = 1;
} finally {
  try { verify(); } catch (error) { failures.push({phase: 'final source identity', message: error.message}); process.exitCode = 1; }
  const pass = failures.length === 0 && binary !== null;
  const result = (pass ? 'PASS_' : 'FAIL_') + 'LOCAL_CURRENT_' + mode.toUpperCase() + '_BUILD';
  write('proof.json', {
    schema_version: 1, result, recorded_at: new Date().toISOString(), mode,
    source_manifest_sha256: expectedSourceSha, input_files: manifest.files.length, source_lineage: sourceUILineage,
    required_workspace: {path: suitePath, sha256: sha(suiteRaw), passed: suite.passed, ignored: suite.ignored, required_test_names: suite.required_test_names, owned_renderer_gate_count: suite.owned_renderer_gate_count, owned_renderer_gates_verified: suite.owned_renderer_gates_verified, verified_runs: verifiedSuiteRuns, actual_workspace: actualWorkspace},
    compiler_profile: mode === 'optimized' ? {lto: 'thin', codegen_units: 1, strip: true, declared_in: 'Cargo.toml'} : {debug: 0, incremental: false},
    source_and_checkout_identity_verified: failures.every(x => x.phase !== 'final source identity'),
    runs, previous, binary, failures,
    qualification: "Actual local Windows build over297 new UI-profile frozen inputs with genuine base694 lineage,295 unchanged inputs and only2 exact UI changes. Base47 newly authored Rust functions/38 bindings are historical; this UI layer adds0 Rust tests/2 bindings, inheriting all388 named source expectations and minimum1170 including4 browser gates. Actual same-UI suite proof plus exact3 producer logs/close/exit/spawn/reparsed summaries are required. This build does not prove UI form/native payload/browser/large-source behavior,13 UI mocks, SDK/native/physical/source matrix, RSS/latency/recovery/selected/platform/install or manufacturer parity. Old694 acceptance drivers are not automatically rebound. Installed user executable and shared runtimes are never overwritten by this builder."
  });
  console.log(JSON.stringify({result, out, binary, failures: failures.map(x => x.message)}));
}
