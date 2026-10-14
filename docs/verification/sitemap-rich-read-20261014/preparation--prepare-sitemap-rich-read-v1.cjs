const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const sha=b=>createHash('sha256').update(b).digest('hex');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const repo=path.join(base,'sitemap-rich-read-integration-v1');
const previous=path.join(base,'sitemap-streaming-read-source-v1');
const source=path.join(base,'sitemap-rich-read-source-v1');
const git=args=>execFileSync('git',args,{cwd:repo});
const expectedNewTests=Number(process.argv[2]);
assert(Number.isSafeInteger(expectedNewTests)&&expectedNewTests>=42&&expectedNewTests<=45,'Reviewed new-test count argument required');
assert.equal(git(['rev-parse','HEAD']).toString().trim(),'5091eb941c6740bfb2a6bf610388bc9ca6fb081b');
assert(!fs.existsSync(source),'Fresh immutable source destination required');
const previousRaw=fs.readFileSync(path.join(previous,'manifest.json'));
assert.equal(sha(previousRaw),'1fc7c77677c82e7c9a9460aa30a8b261a54a2f7567e41a1b04638c965c93f85c');
const prior=JSON.parse(previousRaw);
for(const f of prior.files)assert.equal(sha(fs.readFileSync(path.join(previous,f.path))),f.sha256,f.path);
const owned=[
 'crates/app/src/server/sitemaps.rs','crates/app/src/server/sitemaps_tests.rs',
 'crates/app/src/server/sitemaps_rich_read_tests.rs','crates/app/src/sitemap_membership.rs',
 'crates/app/src/sitemap_membership_tests.rs','crates/app/src/sitemap_membership_rich_read_tests.rs',
 'crates/core/src/lib.rs','crates/core/src/sitemap_coverage.rs',
 'crates/core/src/sitemap_coverage/verified_read.rs','crates/core/src/sitemap_coverage/verified_read_tests.rs',
 'crates/core/src/sitemap_initial_roots.rs','crates/core/src/sitemap_ledger/verified_read.rs',
 'crates/core/src/sitemap_ledger/verified_read_tests.rs','crates/core/src/sitemap_membership_analysis.rs',
 'crates/core/src/sitemap_membership_analysis/window.rs','crates/core/src/sitemap_membership_analysis/window_tests.rs',
 'crates/core/src/sitemap_membership_analysis_tests.rs','crates/core/src/sitemap_source_store.rs',
 'crates/core/src/sitemap_source_store/verified_report.rs','crates/core/src/store-sitemap-source-tests.rs',
 'crates/core/src/store.rs','crates/core/src/store/sitemap_raw_read.rs',
 'crates/core/src/store/sitemap_read_window.rs','crates/core/src/store/sitemap_read_window_tests.rs',
].sort();
const actual=git(['status','--porcelain=v1','-uall']).toString().trimEnd().split(/\r?\n/).filter(Boolean).map(x=>x.slice(3)).filter(x=>x.startsWith('crates/')).sort();
assert.deepEqual(actual,owned,'Only explicit owned product files');
const provenance=[];
for(const [dir,pin] of [
 ['s4-coverage-read-core-v1','5d1c6b6a35d3864417a4711a099f84ef9820d682c766266e33388efc4e4fcece'],
 ['s4-analysis-read-core-v1','c702fb80a5abf5ad3e1f1b7afd8d513ee07baf07020b3d79501ab0e5da31e35c'],
 ['s4-app-rich-read-v1','b02737c28cc72bbe081f1030a03a74dce1a7e129b8fa89b7dc40f5aa90e1d3f8'],
 ['s4-analysis-report-key-postfix-v1','8b9d6db457f74f71b8380b05bae8a98d9f6ce6067582a5b022171ebee8fde854'],
]){
 const raw=fs.readFileSync(path.join(base,dir,'manifest.json'));assert.equal(sha(raw),pin);
 const m=JSON.parse(raw);for(const f of m.entries??m.payloads)assert.equal(sha(fs.readFileSync(path.join(base,dir,f.payload))),f.sha256??f.after_sha256,f.path);
 provenance.push({path:dir+'/manifest.json',sha256:pin});
}
const namesIn=b=>[...b.toString('utf8').matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)/g)].map(m=>m[1]);
const groups=[];
for(const p of owned){
 const old=prior.files.some(f=>f.path===p)?namesIn(fs.readFileSync(path.join(previous,p))):[];
 const current=namesIn(fs.readFileSync(path.join(repo,p)));
 for(const name of old)assert(current.includes(name),'Prior test name removed '+p+' '+name);
 const added=current.filter(n=>!old.includes(n));if(added.length)groups.push({path:p,names:added});
}
const newCount=groups.reduce((n,g)=>n+g.names.length,0);assert.equal(newCount,expectedNewTests);
const required_tests=prior.required_tests.map(g=>({path:g.path,names:[...g.names]}));
for(const g of groups){const old=required_tests.find(x=>x.path===g.path);if(old)old.names.push(...g.names);else required_tests.push(g);}
const names=required_tests.flatMap(g=>g.names);assert.equal(new Set(names).size,names.length);
const minimum=prior.minimum_expected_workspace_tests+newCount;
git(['add','--',...owned]);
const staged_product_bindings=owned.map(p=>{const b=fs.readFileSync(path.join(repo,p));assert.equal(sha(git(['show',':'+p])),sha(b),'Exact staged compiler bytes '+p);return {path:p,bytes:b.length,sha256:sha(b)};});
const paths=[...new Set([...prior.files.map(f=>f.path),...owned])].sort();assert.equal(paths.length,255);
const files=paths.map(p=>{const b=fs.readFileSync(path.join(repo,p)),old=prior.files.find(f=>f.path===p);if(old&&!owned.includes(p))assert.equal(sha(b),old.sha256,'Unowned input drift '+p);return {path:p,bytes:b.length,sha256:sha(b)};});
const runtimeRoot=path.resolve(__dirname,'../outputs/open-seo-spider');
for(const f of files.filter(f=>f.path.startsWith('renderer/')))assert.equal(sha(fs.readFileSync(path.join(runtimeRoot,f.path))),f.sha256);
fs.mkdirSync(source);
for(const f of files){const dest=path.join(source,f.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(path.join(repo,f.path),dest);assert.equal(sha(fs.readFileSync(dest)),f.sha256);}
const manifest={schema_version:1,result:'FROZEN255_RICH_READ_SOURCE_COMPILE_TEST_NATIVE_NOT_RUN',recorded_at:new Date().toISOString(),
 baseline_commit:'5091eb941c6740bfb2a6bf610388bc9ca6fb081b',previous_manifest_sha256:sha(previousRaw),
 minimum_expected_workspace_tests:minimum,required_test_names:names.length,new_authored_rich_read_tests:newCount,new_tests_executed:0,
 staged_product_bindings,files,required_tests,provenance,
 scope:'Complete prior source cohort plus compact same-TX rich coverage, terminal source/report analysis, Store facade and actual App inventory route. CSV is still buffered/materializing; selected compatibility, full finding predicates, physical source/RSS/native/platform proof remain open.',
 heavy_execution:'HELD_COORDINATED_LOCAL_COMPUTE_WINDOW',installed_runtime_modified:false};
const manifestRaw=Buffer.from(JSON.stringify(manifest,null,2)+'\n');fs.writeFileSync(path.join(source,'manifest.json'),manifestRaw,{flag:'wx'});
const manifestSha=sha(manifestRaw),runners=[];
for(const [oldName,newName,pin] of [
 ['run-sitemap-streaming-read-suite-v1.mjs','run-sitemap-rich-read-suite-v1.mjs','ef3920ec29f3cf77dc62788b79fea8636698359cba1b104ccfc0e1b81fdce444'],
 ['build-sitemap-streaming-read-v1.mjs','build-sitemap-rich-read-v1.mjs','1e7068826646cc939b407d69158eed9a4e111bc935a5ea905a075655a78ecf14'],
]){
 const old=fs.readFileSync(path.join(__dirname,oldName));assert.equal(sha(old),pin);
 let text=old.toString().replace(/^const repo\s*=\s*[^;\r\n]+;/m,'const repo = '+JSON.stringify(repo.replaceAll('\\','/'))+';');
 text=text.replaceAll('sitemap-streaming-read','sitemap-rich-read').replaceAll(sha(previousRaw),manifestSha)
 .replace(/\b246\b/g,'255').replace(/\b211\b/g,String(names.length)).replace(/\b993\b/g,String(minimum))
 .replaceAll('combined246-source','combined255-source').replaceAll('all211 named','all'+names.length+' named').replaceAll('adds199 tests','adds'+(minimum-794)+' tests').replaceAll('minimum993','minimum'+minimum)
 .replaceAll('Rich App/coverage/analysis/selected/CSV streaming, full stored finding filters, physical source matrix and RSS remain open.','Rich App inventory and ordinary coverage/analysis are included as source; selected materializing compatibility, buffered CSV transport, full finding filters, physical source matrix, RSS and native proof remain open.');
 const expression=text.match(/^const repo\s*=\s*([^;\r\n]+);/m)[1];assert(/^"[^"\r\n]*"$/.test(expression));
 const value=vm.runInNewContext(expression,Object.create(null),{timeout:1000});assert(path.win32.isAbsolute(value));assert.equal(path.win32.normalize(value),path.win32.normalize(repo));assert(fs.statSync(value).isDirectory());
 assert(!/sitemap-streaming-read|\b246\b|\b211\b|\b993\b|combined246-source|adds199 tests|minimum993/.test(text),'No stale route literals');
 const dest=path.join(__dirname,newName);fs.writeFileSync(dest,text,{flag:'wx'});execFileSync(process.execPath,['--check',dest]);
 runners.push({path:dest,sha256:sha(fs.readFileSync(dest)),node_syntax_exit:0,isolated_actual_repo_literal_verified:true,execution:'NOT_RUN'});
}
const receipt={schema_version:1,result:'SOURCE_FROZEN_FUTURE_SUITE_PREPARED_NOT_EXECUTED',recorded_at:new Date().toISOString(),source,manifest_sha256:manifestSha,files:files.length,
 minimum_expected:minimum,required_names:names.length,new_authored_tests:newCount,new_test_groups:groups,staged_product_bindings,runners,previous_source_preserved:true,
 actual_checks:'SHA bytes, explicit staged ownership, unique actual authored names, absolute existing JS repo literal, node --check only',compiler_tests_native:'NOT_RUN_COORDINATED_COMPUTE_HOLD',installed_runtime_modified:false};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({source,manifest_sha256:manifestSha,files:files.length,minimum_expected:minimum,required_names:names.length,new_authored_tests:newCount,runners}));
