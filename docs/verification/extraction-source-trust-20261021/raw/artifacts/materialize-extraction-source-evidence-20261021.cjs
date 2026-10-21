'use strict';
// Proposal only. Root runs --validate, reviews its exact output plan, then may
// --apply that SHA. This program has no Git, network, compiler or runtime calls.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {createHash} = require('node:crypto');
const {gzipSync, gunzipSync} = require('node:zlib');

const ROOT = path.resolve(__dirname, '..');
const SELF = 'artifacts/materialize-extraction-source-evidence-20261021.cjs';
const DEST = 'docs/verification/extraction-source-trust-20261021';
const PURE = 'artifacts/extraction-source-pure-20261021-r2';
const SEMANTIC = 'artifacts/extraction-source-semantic-20261021-r1.json';
const APP_AUTHOR = 'artifacts/extraction-source-trust-next-slice-20261021';
const RAW_RULE = '/docs/verification/extraction-source-trust-20261021/** -text whitespace=-blank-at-eol,-blank-at-eof,-space-before-tab,cr-at-eol';
const GZIP_AT = 1024 * 1024;
const SHA = bytes => createHash('sha256').update(bytes).digest('hex');
const jsonBytes = value => Buffer.from(JSON.stringify(value, null, 2) + '\n');

function relative(value) {
  assert.equal(typeof value, 'string');
  assert(value && value === value.replaceAll('\\', '/') && !path.isAbsolute(value) && !/[\u0000-\u001f\u007f:]/u.test(value), 'Unsafe relative path: ' + value);
  assert(value.split('/').every(part => part && part !== '.' && part !== '..'), 'Unsafe relative segment: ' + value);
  return value;
}
function absolute(value) {
  const result = path.resolve(ROOT, relative(value));
  assert(result.startsWith(ROOT + path.sep), 'Outside repository: ' + value);
  return result;
}
function noLinks(value, allowMissing = false) {
  let current = ROOT;
  const parts = relative(value).split('/');
  for (let index = 0; index < parts.length; index++) {
    current = path.join(current, parts[index]);
    let stat;
    try { stat = fs.lstatSync(current); }
    catch (error) { if (allowMissing && error.code === 'ENOENT') return; throw error; }
    assert(!stat.isSymbolicLink(), 'Link/junction refused: ' + value);
    assert(index === parts.length - 1 || stat.isDirectory(), 'Non-directory parent: ' + value);
  }
}
function cli() {
  const args = process.argv.slice(2);
  let mode = '--validate', expected;
  if (args[0] === '--validate') args.shift();
  else if (args[0] === '--apply') { mode = args.shift(); expected = args.shift(); assert(/^[a-f0-9]{64}$/.test(expected || ''), 'Exact reviewed plan SHA required'); }
  assert.equal(args.length, 4, 'Use --validate|--apply SHA --rust artifacts/extraction-source-rust-20261021-rN --native artifacts/extraction-source-native-20261021-rN');
  assert.equal(args[0], '--rust'); assert.equal(args[2], '--native');
  const rust = relative(args[1]), native = relative(args[3]);
  const rustMatch = /^artifacts\/extraction-source-rust-20261021-r([1-9][0-9]*)$/.exec(rust);
  const nativeMatch = /^artifacts\/extraction-source-native-20261021-r([1-9][0-9]*)$/.exec(native);
  assert(rustMatch && nativeMatch, 'Explicit scoped final attempt roots required; no automatic latest selection');
  assert(Number(rustMatch[1]) <= 50 && Number(nativeMatch[1]) <= 50, 'Review an unexpectedly large attempt set explicitly');
  return {apply: mode === '--apply', expected, rust, native, rustAttempt: Number(rustMatch[1]), nativeAttempt: Number(nativeMatch[1])};
}
function main() {
  const config = cli();
  assert.equal(path.resolve(process.cwd()), ROOT, 'Run from owned repository');
  assert(fs.lstatSync(ROOT).isDirectory() && !fs.lstatSync(ROOT).isSymbolicLink());
  noLinks(DEST, true); assert(!fs.existsSync(absolute(DEST)), 'Fresh destination only; reruns/overwrites are refused');
  const parent = path.posix.dirname(DEST); noLinks(parent); assert(fs.lstatSync(absolute(parent)).isDirectory());
  noLinks('.gitattributes');
  const attributes = fs.readFileSync(absolute('.gitattributes'));
  assert(attributes.toString('utf8').split(/\r?\n/).includes(RAW_RULE), 'Exact raw-preservation attributes rule required');

  const inputs = new Map(), outputs = new Map(), copies = new Map(), excluded = [];
  function read(file, expected) {
    relative(file); noLinks(file);
    const stat = fs.lstatSync(absolute(file)); assert(stat.isFile(), 'Regular source file required: ' + file);
    const bytes = fs.readFileSync(absolute(file)); assert.equal(bytes.length, stat.size, 'Input size changed while reading: ' + file);
    const sha256 = SHA(bytes); if (expected) assert.equal(sha256, expected, 'Pinned source changed: ' + file);
    const prior = inputs.get(file);
    if (prior) assert.deepEqual(bytes, prior.bytes, 'Input changed during validation: ' + file);
    else inputs.set(file, {bytes, sha256});
    return bytes;
  }
  const parsed = file => JSON.parse(read(file).toString('utf8').replace(/^\uFEFF/u, ''));
  function put(name, bytes) {
    relative(name); assert(!outputs.has(name), 'Duplicate destination: ' + name);
    outputs.set(name, Buffer.isBuffer(bytes) ? bytes : Buffer.from(bytes));
  }
  function copy(file, category, expected) {
    const bytes = read(file, expected);
    if (copies.has(file)) { const prior = copies.get(file); if (!prior.categories.includes(category)) prior.categories.push(category); return bytes; }
    const gzip = bytes.length >= GZIP_AT;
    const stored = gzip ? gzipSync(bytes, {level: 9, mtime: 0}) : bytes;
    if (gzip) assert.deepEqual(gunzipSync(stored), bytes, 'Lossless gzip verification: ' + file);
    const destination = 'raw/' + file + (gzip ? '.gz' : '');
    put(destination, stored);
    copies.set(file, {source: file, path: destination, categories: [category], raw_bytes: bytes.length, raw_sha256: SHA(bytes), storage: gzip ? 'gzip' : 'identity', stored_bytes: stored.length, stored_sha256: SHA(stored)});
    return bytes;
  }
  function omit(file, reason) { excluded.push({source: file, reason}); }
  const runtimeDirectory = name => /^(?:store|data|node_modules|target|browser[-_]?profiles?|chrome[-_]?profiles?|profiles?|cache|runtime[-_]?cache|user[-_]?data|\.git)$/i.test(name);
  const runtimeFile = name => /(?:\.(?:sqlite|sqlite3|db)(?:-wal|-shm|-journal)?|\.(?:exe|dll|pdb|so|dylib|node|lock))$/i.test(name) || /^(?:LOCK|Cookies|History|Preferences|Local State|Singleton.*)$/i.test(name);
  function collect(directory, category) {
    relative(directory); noLinks(directory); assert(fs.lstatSync(absolute(directory)).isDirectory(), 'Explicit root directory required');
    function visit(current) {
      for (const entry of fs.readdirSync(absolute(current), {withFileTypes: true}).sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0)) {
        const file = relative(current + '/' + entry.name), stat = fs.lstatSync(absolute(file));
        if (stat.isSymbolicLink()) { omit(file, 'source link/junction excluded without traversal'); continue; }
        if (stat.isDirectory()) { if (runtimeDirectory(entry.name)) omit(file, 'owned runtime store/profile/cache directory excluded'); else visit(file); continue; }
        assert(stat.isFile(), 'Unsupported filesystem object: ' + file);
        if (runtimeFile(entry.name)) { omit(file, 'native binary/SQLite/runtime lock excluded'); continue; }
        const bytes = read(file);
        if (bytes.subarray(0, 16).toString('binary') === 'SQLite format 3\0' || bytes.subarray(0, 4).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46])) || (bytes.length >= 64 && bytes.subarray(0, 2).toString() === 'MZ' && bytes.readUInt32LE(60) + 4 <= bytes.length && bytes.subarray(bytes.readUInt32LE(60), bytes.readUInt32LE(60) + 4).equals(Buffer.from([0x50, 0x45, 0, 0])))) {
          omit(file, 'recognized SQLite/native executable magic excluded'); continue;
        }
        copy(file, category);
      }
    }
    visit(directory);
  }
  function maybeCopy(file, category) { noLinks(file, true); if (fs.existsSync(absolute(file))) copy(file, category); }
  function unchangedPins(rows, category, archive = false) {
    assert(Array.isArray(rows) && rows.length > 0, 'Nonempty explicit source input manifest required');
    const seen = new Set();
    for (const row of rows) {
      const file = relative(row.file || row.path); assert(!seen.has(file), 'Duplicate source input pin'); seen.add(file);
      const bytes = read(file, row.sha256); if (row.bytes != null) assert.equal(bytes.length, row.bytes);
      if (archive) copy(file, category, row.sha256);
    }
    return seen;
  }
  function tap(file) {
    const text = read(file).toString('utf8').replace(/\x1b\[[0-9;]*m/g, '');
    const count = name => { const all = [...text.matchAll(new RegExp('^(?:#|ℹ) ' + name + ' (\\d+)\\r?$', 'gm'))]; assert(all.length, 'Missing Node test counter ' + name + ': ' + file); return Number(all.at(-1)[1]); };
    return {tests: count('tests'), passed: count('pass'), failed: count('fail')};
  }
  const rust = parsed(config.rust + '/proof.json');
  assert.equal(rust.complete, true, 'Selected final Rust gate must be terminal PASS');
  assert(!rust.failure && Array.isArray(rust.changed) && rust.changed.length === 0);
  assert.deepEqual(rust.results.map(row => row.name), ['format', 'workspace-tests', 'build', 'clippy']);
  for (const row of rust.results) { assert.equal(row.code, 0); assert(!row.signal && !row.launchError); }
  assert.deepEqual(rust.results[0].args, ['fmt', '--all', '--check']);
  assert.deepEqual(rust.results[1].args, ['test', '--workspace', '--locked', '--', '--test-threads=2'], 'Exact ordinary workspace test command; optional feature-dependent runtime tests are separate');
  assert.deepEqual(rust.results[2].args, ['build', '--locked', '-p', 'open-seo-spider']);
  assert.deepEqual(rust.results[3].args, ['clippy', '--workspace', '--all-targets', '--all-features', '--locked', '--', '-D', 'warnings']);
  const rustInputs = parsed(config.rust + '/inputs.json'); unchangedPins(rustInputs, 'selected-final-compiler-source');
  assert(rust.binary && /^[a-f0-9]{64}$/.test(rust.binary.sha256));
  const rustAttempts = [];
  for (let attempt = 1; attempt <= config.rustAttempt; attempt++) {
    const directory = 'artifacts/extraction-source-rust-20261021-r' + attempt, original = parsed(directory + '/proof.json');
    assert(Array.isArray(original.results) && original.results.length > 0, 'Original completed Rust attempt receipt required');
    collect(directory, 'original-Rust-attempt-' + attempt);
    maybeCopy('artifacts/run-extraction-source-rust-20261021-r' + attempt + '.cjs', 'original-gate-runner');
    rustAttempts.push({attempt, source: directory + '/proof.json', complete: original.complete, failure: original.failure || null, results: original.results});
  }
  const pure = parsed(PURE + '/proof.json');
  assert.equal(pure.complete, true); assert.equal(pure.passed, 535); assert(!pure.failure); assert.deepEqual(pure.changed, []);
  assert(pure.results.every(row => row.code === 0 && !row.signal && !row.launchError && row.failed === 0 && row.tests === row.passed));
  assert.equal(pure.results.reduce((sum, row) => sum + row.passed, 0), pure.passed);
  unchangedPins(pure.sources, 'selected-pure-source');
  for (let attempt = 1; attempt <= 2; attempt++) { collect('artifacts/extraction-source-pure-20261021-r' + attempt, 'original-pure-attempt-' + attempt); copy('artifacts/run-extraction-source-pure-20261021-r' + attempt + '.cjs', 'original-gate-runner'); }
  const semantic = parsed(SEMANTIC); assert.equal(semantic.passed, true); assert.equal(semantic.groups, 40); assert.equal(semantic.actual_browser, false); assert.equal(semantic.actual_engine, false);
  for (const [file, pin] of Object.entries(semantic.reviewed_current_targets)) { const bytes = read(file, pin.sha256); assert.equal(bytes.length, pin.bytes); }
  copy(SEMANTIC, 'current-saved-fixture-semantic-40');

  const rendererPrefix = 'artifacts/extraction-source-renderer-optins-20261021';
  let rendererOptins = null;
  noLinks(rendererPrefix + '.proof.json', true);
  if (fs.existsSync(absolute(rendererPrefix + '.proof.json'))) {
    const proof = parsed(rendererPrefix + '.proof.json');
    const expected = [
      'live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness',
      'live_browser_worker_pause_drains_render_and_retains_rendered_discoveries',
      'live_browser_worker_executes_javascript_for_core_crawl',
      'live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion',
    ];
    assert.equal(proof.complete, true); assert.equal(proof.code, 0); assert(!proof.signal && !proof.launchError);
    assert.deepEqual(proof.changed, []); assert.equal(proof.binary_unchanged, true); assert.equal(proof.binary_sha256, rust.binary.sha256);
    assert.deepEqual([...proof.passed].sort(), [...expected].sort()); assert.equal(new Set(proof.passed).size, 4);
    const stdout = read(rendererPrefix + '.stdout.log').toString('utf8');
    const observed = [...stdout.matchAll(/^test (live_browser_worker_\S+) \.\.\. ok\r?$/gm)].map(match => match[1]);
    assert.deepEqual(observed, proof.passed, 'Exact four opt-in PASS records in original stdout');
    assert(proof.scope.includes('App native_enrichment_browser_fixture remains separately unexecuted'));
    for (const suffix of ['.proof.json', '.stdout.log', '.stderr.log']) copy(rendererPrefix + suffix, 'separate-four-Core-renderer-optins');
    copy('artifacts/run-extraction-renderer-optins-20261021.cjs', 'original-renderer-optins-runner');
    rendererOptins = {source: rendererPrefix + '.proof.json', complete: true, code: proof.code, binary_sha256: proof.binary_sha256, executed: proof.passed, scope: proof.scope, App_native_enrichment_browser_fixture: 'NOT_RUN'};
  }

  const appAuthor = parsed(APP_AUTHOR + '/authoring-proof.json');
  assert.equal(appAuthor.pre_fix_exit_code, 1); assert.equal(appAuthor.current_exit_code, 0); assert.equal(appAuthor.native_runs, 0); assert.equal(appAuthor.browser_runs, 0);
  read('web/app.js', appAuthor.current_app_sha256); read('web/verification/extraction-source-trust.test.cjs', appAuthor.same_final_test_sha256);
  read(APP_AUTHOR + '/pre-fix-app-aef8cd.js', appAuthor.pre_fix_app_sha256);
  const appRed = tap(APP_AUTHOR + '/final-regressions-pre-fix.log'), appGreen = tap(APP_AUTHOR + '/final-regressions-current.log');
  assert.deepEqual(appRed, {tests: 23, passed: 0, failed: 23}); assert.deepEqual(appGreen, {tests: 23, passed: 23, failed: 0});
  collect(APP_AUTHOR, 'actual-App-declared-state-and-final-red-green');
  collect('artifacts/extraction-source-trust-core-20261021', 'historical-Core-authoring-not-runtime');
  const libraryRedFile = 'artifacts/extraction-source-view-library-red-20261021.log', libraryGreenFile = 'artifacts/extraction-source-view-library-green-20261021.log';
  const libraryRed = tap(libraryRedFile), libraryGreen = tap(libraryGreenFile);
  assert.deepEqual(libraryRed, {tests: 2, passed: 0, failed: 2}); assert.equal(libraryGreen.failed, 0); assert.equal(libraryGreen.tests, libraryGreen.passed);
  copy(libraryRedFile, 'two-new-library-regressions-red'); copy(libraryGreenFile, 'complete-library-suite-green-including-two-new-regressions');

  const native = parsed(config.native + '/proof.json'), wrapper = parsed(config.native + '.wrapper.json');
  assert.equal(wrapper.code, 0); assert(!wrapper.signal && !wrapper.launchError); assert.equal(wrapper.binary_unchanged, true);
  assert.equal(native.complete, true, 'Selected coordinator must be terminal PASS'); assert(!native.failure); assert.equal(native.binary_unchanged, true); assert.equal(native.tracked_sources_unchanged, true); assert.deepEqual(native.changed, []);
  assert.deepEqual(native.shutdown, {code: 0, signal: null}); assert.equal(native.binary_sha256, rust.binary.sha256);
  assert.equal(native.results.length, 1, 'This supplement covers one explicitly selected suite, not a combined full registry run');
  assert.equal(native.results[0].name, 'extraction-source-trust'); assert.equal(native.results[0].kind, 'owned-native');
  assert.equal(native.results[0].code, 0); assert(!native.results[0].signal && !native.results[0].error && !native.results[0].timed_out);
  const nativeAttempts = [];
  for (let attempt = 1; attempt <= config.nativeAttempt; attempt++) {
    const directory = 'artifacts/extraction-source-native-20261021-r' + attempt, originalWrapper = parsed(directory + '.wrapper.json');
    assert(Number.isInteger(originalWrapper.code) || (originalWrapper.code === null && typeof originalWrapper.signal === 'string'), 'Prior native wrapper must be terminal');
    collect(directory, 'original-native-attempt-' + attempt);
    copy(directory + '.stdout.log', 'original-native-wrapper-log'); copy(directory + '.stderr.log', 'original-native-wrapper-log'); copy(directory + '.wrapper.json', 'original-native-wrapper-receipt');
    for (const suffix of ['.driver-source.cjs', '.app-source.js', '.view-library-source.js']) maybeCopy(directory + suffix, 'original-pre-execution-native-source-snapshot');
    maybeCopy('artifacts/run-extraction-native-20261021-r' + attempt + '.cjs', 'original-native-runner');
    const proofPath = directory + '/proof.json', original = fs.existsSync(absolute(proofPath)) ? parsed(proofPath) : null;
    nativeAttempts.push({attempt, wrapper: directory + '.wrapper.json', wrapper_code: originalWrapper.code, wrapper_signal: originalWrapper.signal ?? null, complete: original?.complete ?? null, failure: original?.failure ?? null, missing_inner_receipt: !original});
  }
  const liveReceipts = [...copies.keys()].filter(file => file.startsWith(config.native + '/') && file.endsWith('/ui-extraction-source-trust-live.json'));
  assert.equal(liveReceipts.length, 1, 'Exactly one original child acceptance receipt required');
  const liveFile = liveReceipts[0], liveRoot = path.posix.dirname(liveFile), live = parsed(liveFile);
  assert.equal(live.result, 'PASS'); assert(!live.failure); assert.deepEqual(live.errors, []); assert.deepEqual(live.external_browser_requests, []); assert.deepEqual(live.offline_product_requests, []);
  assert.equal(live.native.length, 2); assert(live.native.every(row => row.natural_eof === true && row.forced === false && row.close?.code === 0 && row.close?.signal == null));
  assert.equal(live.positive_410_witnesses.length, 4); assert(live.positive_410_witnesses.every(row => row.status === 410));
  assert(Array.isArray(live.files) && live.files.length > 0); assert.equal(new Set(live.files.map(row => row.label)).size, live.files.length);
  for (const row of live.files) {
    relative(row.label); assert(!row.label.includes('/'), 'Download label must be a filename stem');
    for (const kind of ['xlsx', 'csv']) { const file = liveRoot + '/' + row.label + '.' + kind, bytes = read(file, row[kind + '_sha256']); assert.equal(bytes.length, row[kind + '_bytes']); assert(copies.has(file), 'Original download must be archived'); }
    assert(copies.has(liveRoot + '/' + row.label + '.expected.json')); assert(copies.has(liveRoot + '/' + row.label + '.reader.json'));
  }
  for (const source of live.sources) {
    relative(source.label); assert(!source.label.includes('/'));
    const projectFile = liveRoot + '/' + source.label + '-project.json', bytes = read(projectFile, source.project_sha256);
    assert.equal(JSON.parse(bytes).pages.length, source.pages); assert(copies.has(projectFile));
  }
  for (const row of live.worker_responses) { const bytes = read(liveRoot + '/' + relative(row.file), row.sha256); assert.equal(bytes.length, row.bytes); assert(copies.has(liveRoot + '/' + row.file)); }
  // Pin source text only. Native/Chrome executables and runtime stores remain
  // declared in untouched original receipts but are not copied or normalized.
  for (const pin of live.pins) {
    const full = path.resolve(pin.file), relativePath = path.relative(ROOT, full).replaceAll('\\', '/');
    if (!relativePath || relativePath.startsWith('../') || path.isAbsolute(relativePath) || runtimeFile(path.basename(relativePath))) continue;
    const bytes = read(relative(relativePath), pin.sha256); assert.equal(bytes.length, pin.bytes); copy(relativePath, 'native-selected-text-source', pin.sha256);
  }
  for (const file of [SELF, 'artifacts/implement-extraction-source-app-20261021.py', 'artifacts/prepare-extraction-gates-20261021.cjs', 'scripts/ui-e2e.mjs', 'web/verification/ui-extraction-source-trust-live.cjs', 'web/verification/xlsx-package-oracle.cjs', 'web/verification/extraction-source-trust.test.cjs', 'web/verification/table-view-library.test.cjs', '.gitattributes']) copy(file, 'selected-authoring-and-reproduction-source');
  assert.deepEqual(read('.gitattributes'), attributes, 'Raw-preservation attributes changed during planning');

  const summary = {
    schema_version: 1, kind: 'bounded_extraction_source_trust_local_evidence', result: 'PASS_SELECTED_LOCAL_GATES_NOT_FULL_BACKLOG_OR_MANUFACTURER_PARITY',
    selected_final_rust: config.rust, selected_final_native: config.native, binary_sha256: native.binary_sha256,
    rust_attempts: rustAttempts, native_attempts: nativeAttempts,
    selected_compiler_inputs: {file: config.rust + '/inputs.json', entries: rustInputs.length, scope: rust.scope},
    pure: {tests: pure.passed, source: PURE + '/proof.json', scope: 'Selected Node tests. App23 and the library suite below are subsets, not additional counts.'},
    saved_semantic_fixture_groups: semantic.groups,
    separate_Core_renderer_optins: rendererOptins || {result: 'NOT_PRESENT_NOT_CLAIMED'},
    actual_App_red: appRed, actual_App_green: appGreen, two_new_library_regressions_red: libraryRed, library_suite_green: libraryGreen,
    native: {coordinator: config.native + '/proof.json', original_child_receipt: liveFile, selected_suites: native.results.map(row => ({name: row.name, kind: row.kind})), downloaded_pairs: live.files.length, downloaded_rows: live.files.reduce((sum, row) => sum + row.rows, 0), downloads: live.downloads.length, original_projects: live.sources, natural_child_EOF0: live.native.length, coordinator_EOF0: native.shutdown.code === 0, positive_410_witnesses: live.positive_410_witnesses.length, offline_product_requests: live.offline_product_requests.length, worker_response_files: live.worker_responses.length, negative_controls: live.negative_controls, declared_scope: live.scope, declared_not_proven: live.not_proven},
    preserved_failures: 'Every original earlier selected Rust/native attempt is archived without changing its complete/failure/code declarations. R1 compile failure and R2 all-features LanguageTool endpoint failure are historical FAILs; later PASS does not relabel them.',
    scope_limits: ['Source input manifests are selected explicit files, not a transitive dependency/toolchain closure.', 'Hashes bind declarations and exact archived bytes; they do not authenticate arbitrary imported HTML or values.', 'App and library tests are pure declared-state regressions; native acceptance is separately bound to the original coordinator and child receipts.', 'Only the selected extraction-source-trust native suite is claimed. No combined full49 run, public CI or manufacturer parity is inferred.', 'SQLite/WAL/SHM, native executables, browser profiles, private runtime caches and links are excluded. Original receipt declarations are never rewritten.', 'The materializer verifies raw byte bindings and retains original readers/expectations; it does not independently rerun extraction or claim Excel GUI conformance.'],
  };
  put('local-result.json', jsonBytes(summary));
  const archived = source => { const entry = copies.get(source); assert(entry); return entry.path; };
  put('README.md', `# Extraction source trust: bounded local acceptance\n\nThe selected final Rust attempt is \`${config.rust}\`; the selected native invocation is \`${config.native}\`. Their original terminal receipts pass, with the same binary SHA256 \`${native.binary_sha256}\`. The final compiler source manifest contains ${rustInputs.length} explicitly selected inputs. Original earlier failures remain failures in their unchanged receipts, including the initial compile error and the all-features LanguageTool endpoint precondition failure.\n\nThe selected Node gate passes **${pure.passed} tests**. Its actual-App subset has 23 final regressions: 23 failed on the captured pre-fix App and 23 pass on the corrected App. Two additional library regressions failed before the library change; the final complete library log passes ${libraryGreen.passed} tests. These subsets are already included in the selected Node total. The separate saved-fixture semantic check passes ${semantic.groups} groups and is not browser or engine execution.\n\nOne explicitly selected native suite passes: \`extraction-source-trust\`. Its original child receipt records **${live.files.length} downloaded XLSX/CSV pairs**, ${summary.native.downloaded_rows} total rows across those downloads, ${live.downloads.length} download events and ${live.sources.length} saved project reports. The child receipt records ${live.native.length} natural native EOF0 closures, four qualified offline HTTP410 witnesses and zero product origin requests in its declared offline windows; the outer coordinator independently records EOF0. Real worker response files, source receipts, project reports, CSV/XLSX downloads, expected matrices, reader outputs and logs are preserved as available under the explicit native root. Their exact original declarations remain intact.\n\nSee [local-result.json](local-result.json), [index.json](index.json), the [original final Rust receipt](${archived(config.rust + '/proof.json')}), [original native coordinator receipt](${archived(config.native + '/proof.json')}) and [original child receipt](${archived(liveFile)}). Archived files at least 1 MiB are stored with deterministic lossless gzip; index entries carry both original byte count/SHA256 and stored byte count/SHA256. Decompress those files before using the archived helpers or report paths. Helpers are archived source evidence; relocation is not a promise that they execute directly from this folder.\n\nThe collection verifies selected file bindings and preserves all raw readers and their expectations. It does not rerun extraction or establish full spreadsheet schema/Excel GUI interoperability. Retained hashes bind declarations; imported JSON is not authenticated capture evidence. Full table coverage, arbitrary capture/race/error cases, native portable table workspaces, broader backlog completion, a combined full49 invocation, exact successor public CI and manufacturer parity remain separate open checks. SQLite files, native binaries, browser profiles, private runtime caches and links are deliberately excluded; index exclusions identify the bounded raw scope.\n`);
  if (rendererOptins) outputs.set('README.md', Buffer.concat([outputs.get('README.md'), Buffer.from('\nFour separately executed actual Core renderer opt-ins also passed on the same binary, with unchanged selected source inputs. Their exact test names, original stdout/stderr, proof and runner are archived and listed in local-result.json. This is limited to those four Core tests; App native_enrichment_browser_fixture remains unexecuted. These separate opt-ins are not relabelled as part of the ordinary workspace command.\n')]));
  const raw = [...copies.values()].sort((a, b) => a.source < b.source ? -1 : a.source > b.source ? 1 : 0);
  const index = {schema_version: 1, kind: 'raw_extraction_source_evidence_copy_index', selected_final_rust: config.rust, selected_final_native: config.native, raw_files: raw, copy_count: raw.length, raw_bytes: raw.reduce((sum, row) => sum + row.raw_bytes, 0), stored_raw_bytes: raw.reduce((sum, row) => sum + row.stored_bytes, 0), generated_files: [...outputs].filter(([name]) => !name.startsWith('raw/')).map(([name, bytes]) => ({path: name, bytes: bytes.length, sha256: SHA(bytes)})), excluded, index_self_excluded: true, scope: 'Raw original bytes and declarations are immutable. Gzip is a reversible storage encoding only; plan SHA binds index.json too. Excluded live stores/caches/binaries are not claimed as public raw evidence.'};
  put('index.json', jsonBytes(index));
  const plan = [...outputs].map(([name, bytes]) => ({path: DEST + '/' + name, bytes: bytes.length, sha256: SHA(bytes)})).sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  const planSHA = SHA(Buffer.from(JSON.stringify(plan)));
  const result = {result: config.apply ? 'MATERIALIZED_SELECTED_EXTRACTION_SOURCE_EVIDENCE' : 'READY_VALIDATED_NO_WRITES', destination: DEST, selected_final_rust: config.rust, selected_final_native: config.native, raw_files: raw.length, generated_files: 3, output_files: outputs.size, raw_bytes: index.raw_bytes, stored_bytes: plan.reduce((sum, row) => sum + row.bytes, 0), plan_sha256: planSHA, index_sha256: SHA(outputs.get('index.json')), materializer_sha256: inputs.get(SELF).sha256};
  if (!config.apply) { console.log(JSON.stringify(result, null, 2)); return; }
  assert.equal(config.expected, planSHA, 'Exact reviewed full output plan required');
  for (const [file, item] of inputs) { noLinks(file); assert.deepEqual(fs.readFileSync(absolute(file)), item.bytes, 'Input changed after preflight: ' + file); }
  noLinks(DEST, true); assert(!fs.existsSync(absolute(DEST)), 'Fresh destination required immediately before apply'); fs.mkdirSync(absolute(DEST));
  for (const [name, bytes] of outputs) {
    const file = DEST + '/' + name, directory = path.posix.dirname(file);
    noLinks(directory, true); fs.mkdirSync(absolute(directory), {recursive: true}); noLinks(directory);
    assert(!fs.existsSync(absolute(file))); fs.writeFileSync(absolute(file), bytes, {flag: 'wx'});
    assert.equal(SHA(fs.readFileSync(absolute(file))), SHA(bytes), 'Stored output verification: ' + file);
  }
  console.log(JSON.stringify(result, null, 2));
}
try { main(); } catch (error) { console.error(error.stack); process.exitCode = 1; }
