const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const {createHash} = require('node:crypto');
const {execFileSync} = require('node:child_process');
const sha = b => createHash('sha256').update(b).digest('hex');
const base = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const repo = path.join(base, 'sitemap-streaming-integration-v1');
const source = path.join(base, 'sitemap-streaming-read-source-v1');
const previous = path.join(base, 'sitemap-large-processing-source-v1');
const git = args => execFileSync('git', args, {cwd: repo});
assert.equal(git(['rev-parse', 'HEAD']).toString().trim(), '380beb1ccacf73659d0ac8dd7fdb39c543b88883');
assert(!fs.existsSync(source), 'Fresh immutable source destination required');
const previousRaw = fs.readFileSync(path.join(previous, 'manifest.json'));
assert.equal(sha(previousRaw), '1fee9f1be33549cfb00ca3b0abd411601272933e6ed4c34abe4e2914a7480329');
const prior = JSON.parse(previousRaw);
for (const f of prior.files) assert.equal(sha(fs.readFileSync(path.join(previous, f.path))), f.sha256, f.path);
const owned = [
  'crates/app/src/main.rs', 'crates/app/src/sitemap_membership.rs',
  'crates/app/src/sitemap_raw_cli.rs', 'crates/app/src/sitemap_raw_cli_tests.rs',
  'crates/core/src/lib.rs', 'crates/core/src/store.rs',
  'crates/core/src/store/sitemap_raw_read.rs', 'crates/core/src/store/sitemap_raw_read_tests.rs',
  'crates/core/src/store-sitemap-source-tests.rs',
  'crates/core/src/sitemap_ledger.rs', 'crates/core/src/sitemap_ledger/verified_read.rs',
  'crates/core/src/sitemap_ledger/verified_read_tests.rs',
  'crates/core/src/sitemap_source_store.rs', 'crates/core/src/sitemap_source_store/verified_read.rs',
  'crates/core/src/sitemap_source_store/verified_read_tests.rs', 'crates/core/src/sitemap_source_store_tests.rs',
].sort();
const actual = git(['status', '--porcelain=v1', '-uall']).toString().trimEnd().split(/\r?\n/)
  .filter(Boolean).map(line => line.slice(3)).filter(p => p.startsWith('crates/')).sort();
assert.deepEqual(actual, owned, 'Only explicit owned product paths');
for (const [dir, expected] of [
  ['s4-raw-read-core-v1', '14221c867291188534c0f5d7e9630ebc1250cbeccd3653266bdc02214441095c'],
  ['s4-source-read-core-v1', '06ea077f06f4233c47575b0cfeafcf7fa732b6952307b2958c775e43e81c7073'],
]) assert.equal(sha(fs.readFileSync(path.join(base, dir, 'manifest.json'))), expected);
const rootPins = JSON.parse(fs.readFileSync(path.join(base, 's4-root-raw-window-cli-review-v2/SOURCE-PINS.json')));
for (const f of rootPins) assert.equal(sha(fs.readFileSync(f.root_path)), f.sha256, f.root_path);
git(['add', '--', ...owned]);
const staged_product_bindings = owned.map(p => {
  const b = fs.readFileSync(path.join(repo,p));
  assert.equal(sha(git(['show', ':'+p])),sha(b), 'Exact staged compiler bytes: '+p);
  return {path:p,bytes:b.length,sha256:sha(b)};
});
const newPaths = owned.filter(p => !prior.files.some(f => f.path === p));
assert.equal(newPaths.length,8);
const paths = [...new Set([...prior.files.map(f => f.path), ...newPaths])].sort();
assert.equal(paths.length,246);
const files = paths.map(p => {
  const b=fs.readFileSync(path.join(repo,p)), old=prior.files.find(f=>f.path===p);
  if(old&&!owned.includes(p)) assert.equal(sha(b),old.sha256,'No unowned input drift: '+p);
  return {path:p,bytes:b.length,sha256:sha(b)};
});
const namesIn = p => [...fs.readFileSync(path.join(repo,p),'utf8').matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)/g)].map(m=>m[1]);
const groups = [
  ['crates/core/src/sitemap_ledger/verified_read_tests.rs',15],
  ['crates/core/src/sitemap_source_store/verified_read_tests.rs',9],
  ['crates/core/src/store/sitemap_raw_read_tests.rs',6],
  ['crates/app/src/sitemap_raw_cli_tests.rs',4],
].map(([p,n])=>{const names=namesIn(p);assert.equal(names.length,n,p);return {path:p,names};});
const sourceTestPath='crates/core/src/store-sitemap-source-tests.rs';
const extraSourceNames=namesIn(sourceTestPath).filter(name=>!prior.required_tests.some(g=>g.path===sourceTestPath&&g.names.includes(name)));
assert.equal(extraSourceNames.length,2);
groups.push({path:sourceTestPath,names:extraSourceNames});
const newCount=groups.reduce((n,g)=>n+g.names.length,0);
assert.equal(newCount,36);
const required_tests=[...prior.required_tests.map(g=>({path:g.path,names:[...g.names]}))];
for(const group of groups){const existing=required_tests.find(g=>g.path===group.path);if(existing)existing.names.push(...group.names);else required_tests.push(group);}
const allNames=required_tests.flatMap(g=>g.names);
assert.equal(allNames.length,211);assert.equal(new Set(allNames).size,allNames.length,'Required names unique');
const minimum=prior.minimum_expected_workspace_tests+newCount;assert.equal(minimum,993);
const runtimeRoot=path.resolve(__dirname,'../outputs/open-seo-spider');
for(const f of files.filter(f=>f.path.startsWith('renderer/')))assert.equal(sha(fs.readFileSync(path.join(runtimeRoot,f.path))),f.sha256);
fs.mkdirSync(source);
for(const f of files){const dest=path.join(source,f.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(path.join(repo,f.path),dest);assert.equal(sha(fs.readFileSync(dest)),f.sha256);}
const manifest={schema_version:1,result:'FROZEN246_SOURCE_CANDIDATE_COMPILE_TEST_NATIVE_NOT_RUN',recorded_at:new Date().toISOString(),
  baseline_commit:'380beb1ccacf73659d0ac8dd7fdb39c543b88883',previous_manifest_sha256:sha(previousRaw),
  minimum_expected_workspace_tests:minimum,required_test_names:allNames.length,new_authored_streaming_tests:newCount,new_tests_executed:0,
  staged_product_bindings,files,required_tests,
  scope:'Previous generation/target/S3/S4 profile plus ordinary verified raw/source SQL window, actual Store facade and separate raw CLI; rich App/coverage/analysis/selected/CSV transport and physical memory/large-source proof remain open.',
  heavy_execution:'HELD_COORDINATED_LOCAL_COMPUTE_WINDOW',installed_runtime_modified:false};
const manifestRaw=Buffer.from(JSON.stringify(manifest,null,2)+'\n');
fs.writeFileSync(path.join(source,'manifest.json'),manifestRaw,{flag:'wx'});
const manifestSha=sha(manifestRaw),runners=[];
for(const [oldName,newName,expected] of [
  ['run-sitemap-large-processing-suite-v2.mjs','run-sitemap-streaming-read-suite-v1.mjs','1fc0b9219a04a407f412235fda4051b08d69edd3b45318bec7407ff4d36b443f'],
  ['build-sitemap-large-processing-v2.mjs','build-sitemap-streaming-read-v1.mjs','ae2f5c40117c3218a703cf9514234c9df75782e53013f6bb6a29312ffe425c3c'],
]){
  const old=fs.readFileSync(path.join(__dirname,oldName));assert.equal(sha(old),expected);
  let text=old.toString('utf8').replace(/^const repo\s*=\s*[^;\r\n]+;/m,'const repo = '+JSON.stringify(repo.replaceAll('\\','/'))+';');
  for(const [a,b] of [
    ['sitemap-large-processing','sitemap-streaming-read'],[sha(previousRaw),manifestSha],
    ['sitemap-streaming-read-suite-v2','sitemap-streaming-read-suite-v1'],['-build-v2','-build-v1'],
    ['combined238-source','combined246-source'],['all175 named','all211 named'],['adds163 tests','adds199 tests'],['minimum957','minimum993'],
    ['S3 binding and explicit S4 large-source work budgets are included; streamed SQL paging, complete public filters and genuine physical protocol matrix remain open.',
      'S3/S4 profiles and ordinary verified raw/source windows plus raw CLI are included. Rich App/coverage/analysis/selected/CSV streaming, full stored finding filters, physical source matrix and RSS remain open.'],
  ])text=text.replaceAll(a,b);
  text=text.replace(/\b238\b/g,'246').replace(/\b175\b/g,'211').replace(/\b957\b/g,'993');
  const expression=text.match(/^const repo\s*=\s*([^;\r\n]+);/m)[1];
  assert(/^"[^"\r\n]*"$/.test(expression));
  const value=vm.runInNewContext(expression,Object.create(null),{timeout:1000});
  assert(path.win32.isAbsolute(value));assert.equal(path.win32.normalize(value),path.win32.normalize(repo));assert(fs.statSync(value).isDirectory());
  assert(!/sitemap-large-processing|\b238\b|\b175\b|\b957\b/.test(text),'No stale route counts/paths');
  const dest=path.join(__dirname,newName);fs.writeFileSync(dest,text,{flag:'wx'});
  execFileSync(process.execPath,['--check',dest]);
  runners.push({path:dest,sha256:sha(fs.readFileSync(dest)),node_syntax_exit:0,isolated_actual_repo_literal_verified:true,execution:'NOT_RUN'});
}
const receipt={schema_version:1,result:'FROZEN246_FUTURE993_NAMED211_PREPARED_NOT_EXECUTED',recorded_at:new Date().toISOString(),
  source,manifest_sha256:manifestSha,files:files.length,minimum_expected:minimum,required_names:allNames.length,new_authored_tests:newCount,
  new_test_groups:groups,staged_product_bindings,runners,previous_source_preserved:true,actual_checks:'SHA bytes, explicit staged ownership, unique required names, absolute existing JS path literal, node --check only',
  compiler_tests_native:'NOT_RUN_COORDINATED_COMPUTE_HOLD',installed_runtime_modified:false};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({result:receipt.result,source,manifest_sha256:manifestSha,files:files.length,minimum_expected:minimum,required_names:allNames.length,runners}));
