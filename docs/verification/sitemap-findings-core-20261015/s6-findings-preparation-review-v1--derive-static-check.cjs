// Independent read-only derivation/syntax/literal checks. Never loads a runner.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const {createHash} = require('node:crypto');
const {execFileSync} = require('node:child_process');
const sha = value => createHash('sha256').update(value).digest('hex');
const task = 'C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const work = path.join(task, 'work');
const preparer = fs.readFileSync(path.join(work, 'prepare-sitemap-findings-core-v1.cjs'));
assert.equal(sha(preparer), '3d159f184201a36d364b1a47f13ffe9993aad1c60a391013477bf4fc142d8981');
const preparation = preparer.toString();
assert(preparation.indexOf("git(['add','--',...products])") > preparation.indexOf('runnerPlans.push'));
assert(preparation.indexOf("git(['add','--',...products])") > preparation.indexOf("['--input-type=module','--check']"));
assert(preparation.indexOf("git(['add','--',...products])") < preparation.indexOf('fs.mkdirSync(source)'));
const counts = JSON.parse(fs.readFileSync(path.join(__dirname, 'validation.json')));
assert.deepEqual([counts.compiler_files, counts.new_authored_tests, counts.required_names, counts.minimum_expected, counts.owned_products], [275, 53, 341, 1123, 18]);
const repo = path.join('E:/CodexWork/open-seo-spider-01a12ac3-20261013', 'sitemap-findings-core-integration-v1');
const dummyManifest = 'f'.repeat(64); // Future actual recorded_at/manifest hash does not yet exist.
const priorManifest = 'a31936510381cca40901eb547f15090a77c833f617f84f51f67b057fe7a5714e';
const results = [];
for (const [file, pin, kind] of [
  ['run-sitemap-csv-streaming-suite-v1.mjs', '613aff8e49df6a50115922ff3aa61e8cacce2f6c14066a7a002d0c7f80010332', 'suite'],
  ['build-sitemap-csv-streaming-v1.mjs', '73c5f7dfc0d279025dd84bce629635378c1d8eff6efff08632406174f03ed0e5', 'build'],
]) {
  const before = fs.readFileSync(path.join(work, file));
  assert.equal(sha(before), pin);
  const text = before.toString()
    .replace(/^const repo\s*=\s*[^;\r\n]+;/m, 'const repo = ' + JSON.stringify(repo) + ';')
    .replaceAll('sitemap-csv-streaming', 'sitemap-findings-core')
    .replaceAll(priorManifest, dummyManifest)
    .replaceAll('combined265-source', 'combined275-source')
    .replaceAll('all288 named', 'all341 named')
    .replaceAll('adds276 tests', 'adds329 tests')
    .replaceAll('minimum1070', 'minimum1123')
    .replace(/\b265\b/g, '275').replace(/\b288\b/g, '341').replace(/\b1070\b/g, '1123')
    .replace('Rich inventory, controlled same-TX allstored visitor and bounded two-pass CSV are included as authored source. Selected materialization, physical CSV64MiB/+1, seven full finding filters, source matrix, RSS/latency, recovery and native/platform acceptance remain open.', 'Rich inventory, controlled same-TX allstored raw/CSV and seven findings Core predicates/readers/session are authored source. Complete finding API/CLI/MCP/UI/export adapters, selected materialization, physical CSV64MiB/+1, source matrix, RSS/latency, recovery and native/platform acceptance remain open.');
  assert(!/sitemap-csv-streaming|\b265\b|\b288\b|\b1070\b/.test(text));
  const expression = text.match(/^const repo\s*=\s*([^;\r\n]+);/m)[1];
  const actualRepo = vm.runInNewContext(expression, Object.create(null), {timeout:1000});
  assert(path.win32.isAbsolute(actualRepo));
  assert.equal(path.win32.normalize(actualRepo), path.win32.normalize(repo));
  assert(fs.statSync(actualRepo).isDirectory());
  const guard = text.indexOf('OPEN_SEO_COORDINATED_EXECUTION_WINDOW');
  assert(guard >= 0 && guard < text.indexOf('fs.mkdirSync('));
  assert(guard < text.indexOf('spawn('));
  if (kind === 'suite') assert(guard < text.indexOf('await import('));
  for (const setting of ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']) {
    assert(text.indexOf(setting) > guard && text.indexOf(setting) < text.indexOf('fs.mkdirSync('));
  }
  assert(text.includes('sitemap-findings-core-source-v1'));
  assert(text.includes("'results/sitemap-findings-core-"));
  assert(text.includes(dummyManifest));
  assert(text.includes('1123'));
  if (kind === 'suite') {
    assert(text.includes("'outputs/open-seo-spider'"));
    assert(text.includes('341'));
    assert(text.includes('Complete finding API/CLI/MCP/UI/export adapters'));
  } else {
    assert(text.includes('results/sitemap-findings-core-suite-v1/proof.json'));
    assert(text.includes('suite.required_test_names === 341'));
    assert(text.indexOf("'previous-target.exe'") < text.indexOf("await command('build'"));
  }
  // Parse-only stdin. Do not import/evaluate the derived runner body.
  execFileSync(process.execPath, ['--input-type=module', '--check'], {input:text});
  results.push({kind, input_sha256:pin, derived_node_syntax_exit:0, actual_repo_literal:actualRepo,
    hold_and_compiler_guards_before_mkdir_and_execution:true,
    manifest_hash_is_placeholder_not_final_route_proof:true, runner_imported_or_executed:false});
}
console.log(JSON.stringify({preparer_sha256:sha(preparer), static_derivation:results, source_or_runner_outputs_written:false}));
