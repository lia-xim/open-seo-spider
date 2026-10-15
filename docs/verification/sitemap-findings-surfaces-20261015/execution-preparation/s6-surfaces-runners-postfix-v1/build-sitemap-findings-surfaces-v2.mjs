import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const task = "C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https";
const repo = "E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-findings-surfaces-integration-v1";
const root = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const source = path.join(root, 'sitemap-findings-surfaces-source-v1');
const expectedSourceSha = '69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751';
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
assert.equal(manifest.new_authored_test_functions, 47);
assert.equal(manifest.approved_product_bindings.length, 38);
const ownedRendererGates = ["live_browser_worker_executes_javascript_for_core_crawl","live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness","live_browser_worker_pause_drains_render_and_retains_rendered_discoveries","live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion"];
const suitePath = path.join(root, 'results/sitemap-findings-surfaces-suite-v1/proof.json');
const suiteRaw = fs.readFileSync(suitePath);
const suite = JSON.parse(suiteRaw);
assert.equal(suite.result, 'PASS_LOCAL_CURRENT_WORKSPACE_REAL_RENDERER_CLIPPY_FMT');
assert.equal(suite.source_manifest_sha256, expectedSourceSha);
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
const out = path.join(root, 'results/sitemap-findings-surfaces-' + mode + '-build-v2');
assert(!fs.existsSync(out), 'Build result output must be new');
function verify() {
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
    source_manifest_sha256: expectedSourceSha, input_files: manifest.files.length,
    required_workspace: {path: suitePath, sha256: sha(suiteRaw), passed: suite.passed, ignored: suite.ignored, required_test_names: suite.required_test_names, owned_renderer_gate_count: suite.owned_renderer_gate_count, owned_renderer_gates_verified: suite.owned_renderer_gates_verified, verified_runs: verifiedSuiteRuns, actual_workspace: actualWorkspace},
    compiler_profile: mode === 'optimized' ? {lto: 'thin', codegen_units: 1, strip: true, declared_in: 'Cargo.toml'} : {debug: 0, incremental: false},
    source_and_checkout_identity_verified: failures.every(x => x.phase !== 'final source identity'),
    runs, previous, binary, failures,
    qualification: "Actual local Windows build bound to 297 source inputs only. All S6 Core, HTTP API, CLI, MCP, UI and export surfaces are AUTHORED in source; 47 new plus 329 prior source-authored expectations extend the accepted baseline of 794. Native SDK acceptance and the 13 UI browser cases, physical/source matrix, RSS/latency, recovery, selected materialization, optimized installation and other platforms remain OPEN separate gates. The installed user executable and shared runtimes are never touched by this builder."
  });
  console.log(JSON.stringify({result, out, binary, failures: failures.map(x => x.message)}));
}
