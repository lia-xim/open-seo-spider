const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const { createHash } = require('node:crypto');
const { execFileSync } = require('node:child_process');
const sha = b => createHash('sha256').update(b).digest('hex');
const base = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const repo = path.join(base, 'sitemap-large-source-integration-v1');
const source = path.join(base, 'sitemap-large-processing-source-v1');
const previous = path.join(base, 'sitemap-source-durable-source-v1');
const git = args => execFileSync('git', args, { cwd: repo });
assert.equal(git(['rev-parse', 'HEAD']).toString().trim(), '2a01db34f4c38b31701305584ed2bdaffab92a72');
assert(!fs.existsSync(source));
const previousRaw = fs.readFileSync(path.join(previous, 'manifest.json'));
assert.equal(sha(previousRaw), '684ef01afb3f20ac9614296c21ac874262fde84596abbb1585b9623decebe34b');
const prior = JSON.parse(previousRaw);
for (const f of prior.files) assert.equal(sha(fs.readFileSync(path.join(previous, f.path))), f.sha256);
const core = JSON.parse(fs.readFileSync(path.join(base, 'sitemap-source-processing-core-v1/manifest.json')));
const fetch = JSON.parse(fs.readFileSync(path.join(base, 'sitemap-large-source-fetch-store-v1/manifest.json')));
const owned = [...new Set([...core.payloads.map(f => f.path), ...fetch.payloads.map(f => f.path),
  'crates/core/src/models.rs', 'crates/core/src/lib.rs', 'crates/core/src/sitemap/entity.rs',
  'crates/core/src/crawler/sitemap_capture_tests.rs', 'crates/app/src/mcp.rs', 'crates/app/src/server.rs',
  'crates/app/src/server/sitemap_processing_config_tests.rs'])].sort();
assert.equal(owned.length, 18);
const actual = git(['status', '--porcelain=v1', '-uall']).toString().trimEnd().split(/\r?\n/).filter(Boolean).map(x => x.slice(3)).filter(p => p.startsWith('crates/')).sort();
assert.deepEqual(actual, owned, 'Only explicit owned product paths');
git(['add', '--', ...owned]);
const staged_product_bindings = owned.map(p => { const b = fs.readFileSync(path.join(repo, p)); assert.equal(sha(git(['show', ':' + p])), sha(b), p); return { path: p, bytes: b.length, sha256: sha(b) }; });
const newPaths = owned.filter(p => !prior.files.some(f => f.path === p));
assert.equal(newPaths.length, 4);
const paths = [...new Set([...prior.files.map(f => f.path), ...newPaths])].sort();
assert.equal(paths.length, 238);
const newline_variants = [];
const files = paths.map(p => {
  const bytes = fs.readFileSync(path.join(repo, p));
  const old = prior.files.find(f => f.path === p);
  if (old && !owned.includes(p) && sha(bytes) !== old.sha256) {
    const before = fs.readFileSync(path.join(previous, p));
    assert.equal(before.toString('utf8').replaceAll('\r\n', '\n'), bytes.toString('utf8').replaceAll('\r\n', '\n'), 'Unowned semantic drift: ' + p);
    newline_variants.push({ path: p, previous_sha256: old.sha256, current_sha256: sha(bytes), qualification: 'Fresh Git worktree line-ending normalization only; no acceptance transfer' });
  }
  return { path: p, bytes: bytes.length, sha256: sha(bytes) };
});
const newGroups = [
  ['crates/core/src/sitemap_source_processing_tests.rs', 10],
  ['crates/core/src/crawler/sitemap_large_capture_tests.rs', 4],
  ['crates/app/src/server/sitemap_processing_config_tests.rs', 3],
].map(([p, n]) => { const text = fs.readFileSync(path.join(repo, p), 'utf8'); const names = [...text.matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)/g)].map(m => m[1]); assert.equal(names.length, n); return { path: p, names }; });
const required_tests = [...prior.required_tests, ...newGroups];
assert.equal(required_tests.reduce((n, g) => n + g.names.length, 0), 175);
for (const f of files) { const target = path.join(source, f.path); fs.mkdirSync(path.dirname(target), { recursive: true }); fs.writeFileSync(target, fs.readFileSync(path.join(repo, f.path)), { flag: 'wx' }); assert.equal(sha(fs.readFileSync(target)), f.sha256); }
const runtimeRoot = path.resolve(__dirname, '../outputs/open-seo-spider');
for (const f of files.filter(f => f.path.startsWith('renderer/'))) assert.equal(sha(fs.readFileSync(path.join(runtimeRoot, f.path))), f.sha256);
const manifest = { schema_version: 1, result: 'FROZEN238_SOURCE_CANDIDATE_COMPILE_TEST_NATIVE_NOT_RUN', recorded_at: new Date().toISOString(), baseline_commit: '2a01db34f4c38b31701305584ed2bdaffab92a72', previous_manifest_sha256: sha(previousRaw), minimum_expected_workspace_tests: 957, required_test_names: 175, new_authored_s4_tests: 17, new_tests_executed: 0, staged_product_bindings, newline_variants, files, required_tests, scope: 'Previous generation/target/S3 plus explicit bounded large-source configuration and actual fetch/capture/durable bound propagation. SQL-streamed paging, full stored filters and genuine physical protocol matrix remain open.', heavy_execution: 'HELD_COORDINATED_LOCAL_COMPUTE_WINDOW', installed_runtime_modified: false };
fs.writeFileSync(path.join(source, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
const manifestSha = sha(fs.readFileSync(path.join(source, 'manifest.json')));
const runners = [];
for (const [oldName, newName, expected] of [
  ['run-sitemap-source-durable-suite-v2.mjs', 'run-sitemap-large-processing-suite-v1.mjs', 'f8ca2eb37fcd9737d5f2b442baf827736158005c34b5f767934c2864ae34b04f'],
  ['build-sitemap-source-durable-v2.mjs', 'build-sitemap-large-processing-v1.mjs', '9e884ee44cbfd7cf9e9be0e51ccba3c3f685ccf2c10c76fc307f863bfc5e3652'],
]) {
  const old = fs.readFileSync(path.join(__dirname, oldName)); assert.equal(sha(old), expected);
  let text = old.toString('utf8').replace(/^const repo\s*=\s*[^;\r\n]+;/m, "const repo = '" + repo + "';");
  for (const [a, b] of [
    ['sitemap-source-durable-source-v1', 'sitemap-large-processing-source-v1'], [sha(previousRaw), manifestSha],
    ['results/sitemap-source-durable-suite-v2', 'results/sitemap-large-processing-suite-v1'],
    ["'results/sitemap-source-durable-'", "'results/sitemap-large-processing-'"], ["mode + '-build-v2'", "mode + '-build-v1'"],
    ['manifest.files.length,234', 'manifest.files.length,238'], ['manifest.files.length, 234', 'manifest.files.length, 238'],
    ['n+x.names.length,0),158', 'n+x.names.length,0),175'], ['suite.required_test_names === 158', 'suite.required_test_names === 175'],
    ['all158 named', 'all175 named'], ['combined234-source', 'combined238-source'],
    ['passed>=940', 'passed>=957'], ['minimum_expected:940', 'minimum_expected:957'], ['suite.passed >= 940', 'suite.passed >= 957'],
    ['minimum940', 'minimum957'], ['adds146 tests', 'adds163 tests'],
    ['S3 durable/source binding is included; complete public filters and protocol-sized physical matrix remain open.', 'S3 binding and explicit S4 large-source work budgets are included; streamed SQL paging, complete public filters and genuine physical protocol matrix remain open.'],
  ]) text = text.replaceAll(a, b);
  const expression = text.match(/^const repo\s*=\s*([^;\r\n]+);/m)[1];
  assert(/^'[^'\r\n]*'$/.test(expression));
  const actualRepo = vm.runInNewContext(expression, Object.create(null), { timeout: 1000 });
  assert.equal(actualRepo, repo); assert(path.win32.isAbsolute(actualRepo)); assert(fs.statSync(actualRepo).isDirectory());
  const target = path.join(__dirname, newName); fs.writeFileSync(target, text, { flag: 'wx' }); execFileSync(process.execPath, ['--check', target]);
  runners.push({ path: target, sha256: sha(fs.readFileSync(target)), node_syntax_exit: 0, isolated_actual_repo_literal_verified: true, execution: 'NOT_RUN' });
}
const receipt = { result: 'FROZEN238_FUTURE957_NAMED175_PREPARED_NOT_EXECUTED', recorded_at: new Date().toISOString(), source, manifest_sha256: manifestSha, files: 238, minimum_expected: 957, required_names: 175, new_authored_s4_tests: 17, staged_product_bindings, newline_variants, runners, compiler_tests_native: 'NOT_RUN_COORDINATED_COMPUTE_HOLD', installed_runtime_modified: false };
fs.writeFileSync(__filename.replace(/\.cjs$/, '.json'), JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ result: receipt.result, manifest_sha256: manifestSha, files: 238, minimum_expected: 957, required_names: 175, newline_variants: newline_variants.length, runners }));
