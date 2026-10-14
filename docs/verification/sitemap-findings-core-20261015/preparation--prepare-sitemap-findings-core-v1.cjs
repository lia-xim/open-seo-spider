const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),vm=require('node:vm');
const {createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const sha=b=>createHash('sha256').update(b).digest('hex');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',repo=path.join(base,'sitemap-findings-core-integration-v1');
const previous=path.join(base,'sitemap-csv-streaming-source-v1'),source=path.join(base,'sitemap-findings-core-source-v1');
const git=args=>execFileSync('git',args,{cwd:repo});
assert.equal(git(['rev-parse','HEAD']).toString().trim(),'c8f539be979ac5436dcc962c5ce7677aab9f3b0d');
assert(!fs.existsSync(source),'New immutable full compiler input required');
const [pagePostfixPin,sourceReviewPin,rootPostfixReviewPin,pageReviewPin]=process.argv.slice(2);
for(const p of [pagePostfixPin,sourceReviewPin,rootPostfixReviewPin,pageReviewPin])assert(/^[a-f0-9]{64}$/.test(p??''));
const priorRaw=fs.readFileSync(path.join(previous,'manifest.json'));
assert.equal(sha(priorRaw),'a31936510381cca40901eb547f15090a77c833f617f84f51f67b057fe7a5714e');
const prior=JSON.parse(priorRaw);assert.equal(prior.files.length,265);
const approved=new Map(),owned=new Set(),provenance=[];
for(const f of prior.files){const b=fs.readFileSync(path.join(previous,f.path));assert.equal(sha(b),f.sha256);approved.set(f.path,b);}
for(const [dir,pin] of [
 ['s6-root-session-core-v1','952b832dbec985155b8aaf7189f97fad6cdd7032688d68a1bfba492cf31ac0a0'],
 ['s6-source-reader-core-v1','53f224a96d6e54fec03a5d3057abcb5d7382f1c6cd4d4d0ea813a967878f669c'],
 ['s6-page-reader-core-v1','abe095c0e5eb6cd4666fec5ec41dbd4af95a6598978ad9aa0d1747120c12604d'],
 ['s6-predicate-core-v1','9ecf9b4b8dbd2989de904e609657a8e9cad74deaa5e8a743ceb689f6bc7e4749'],
 ['s6-root-session-postfix-v1','3e07da49570db1666d87b7b4f4d183dd38aabdabe1f30c0485a26e79eb40a7e7'],
 ['s6-page-reader-postfix-v1',pagePostfixPin],
]){
 const raw=fs.readFileSync(path.join(base,dir,'manifest.json'));assert.equal(sha(raw),pin,dir);
 const m=JSON.parse(raw);provenance.push({path:dir+'/manifest.json',sha256:pin});
 for(const f of m.entries??m.payloads){
  assert(f.path.startsWith('crates/core/src/')&&!f.path.split('/').includes('..'));
  const b=fs.readFileSync(path.join(base,dir,f.payload??f.path));assert.equal(sha(b),f.after_sha256??f.sha256);assert.equal(b.length,f.bytes);assert(!b.includes(13),'LF only source');
  if(f.before_sha256===null)assert(!approved.has(f.path),'Absent at prior stage '+f.path);
  else{assert(approved.has(f.path));assert.equal(sha(approved.get(f.path)),f.before_sha256,'Exact reviewed lineage '+f.path);}
  approved.set(f.path,b);owned.add(f.path);
 }
}
for(const [dir,file,pin] of [
 ['s6-source-audit-v1','receipt.json','4550c5e0d92ffcd67957168a21d55ced0de78a8e42b522cb524c41a8827b32e3'],
 ['s6-pages-audit-v1','receipt.json','7fea4e9b8ad3683c44cfcef00d88112d5a62d330640dbdf0ee2d33a2adf42c12'],
 ['s6-acceptance-audit-v1','receipt.json','2285c36f702fa5a648a60e4ea5cd0695705acb178b712f8d060392a14dc3af03'],
 ['s6-root-session-review-v1','receipt.json','b90abca74d5d2f2e24e5a41a204b3d601e54ab8f06369d2836426f5490085933'],
 ['s6-predicate-review-v1','receipt.json','cdcccb83df19cc70708a81cc62d263730f95d5af15f01296d80b4b605c16074b'],
 ['s6-source-reader-review-v1','receipt.json',sourceReviewPin],
 ['s6-root-session-postfix-review-v1','receipt.json',rootPostfixReviewPin],
 ['s6-page-reader-review-v1','receipt.json',pageReviewPin],
]){const raw=fs.readFileSync(path.join(base,dir,file));assert.equal(sha(raw),pin,dir);provenance.push({path:dir+'/'+file,sha256:pin});}
const products=[...owned].sort(),actual=git(['status','--porcelain=v1','-uall']).toString().trimEnd().split(/\r?\n/).filter(Boolean).map(x=>x.slice(3)).filter(x=>x.startsWith('crates/')).sort();
assert.deepEqual(actual,products,'Exact owned product scope');
const approved_product_bindings=products.map(p=>{const b=approved.get(p);assert.equal(sha(fs.readFileSync(path.join(repo,p))),sha(b),'Approved final payload '+p);return {path:p,bytes:b.length,sha256:sha(b)};});
const namesIn=b=>[...b.toString().matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)/g)].map(x=>x[1]);
const groups=[];
for(const p of products){const old=prior.files.some(f=>f.path===p)?namesIn(fs.readFileSync(path.join(previous,p))):[],current=namesIn(approved.get(p));for(const n of old)assert(current.includes(n),'Previous authored test preserved '+n);const names=current.filter(n=>!old.includes(n));if(names.length)groups.push({path:p,names});}
const newCount=groups.reduce((n,g)=>n+g.names.length,0);assert.equal(newCount,53,'Root18/Source11/Page6/Predicate18');
const required_tests=prior.required_tests.map(g=>({path:g.path,names:[...g.names]}));for(const g of groups){const old=required_tests.find(x=>x.path===g.path);if(old)old.names.push(...g.names);else required_tests.push(g);}
const names=required_tests.flatMap(g=>g.names);assert.equal(new Set(names).size,names.length);const minimum=prior.minimum_expected_workspace_tests+newCount;
const paths=[...new Set([...prior.files.map(f=>f.path),...products])].sort();
const files=paths.map(p=>{const b=fs.readFileSync(path.join(repo,p));assert.equal(sha(b),sha(approved.get(p)),'Unreviewed compiler drift '+p);return {path:p,bytes:b.length,sha256:sha(b)};});
const runtimeRoot=path.resolve(__dirname,'../outputs/open-seo-spider');for(const f of files.filter(f=>f.path.startsWith('renderer/')))assert.equal(sha(fs.readFileSync(path.join(runtimeRoot,f.path))),f.sha256);
for(const f of ['run-sitemap-findings-core-suite-v1.mjs','build-sitemap-findings-core-v1.mjs'])assert(!fs.existsSync(path.join(__dirname,f)));
assert(!fs.existsSync(__filename.replace(/\.cjs$/,'.json')));
// Derive the exact expected index bytes now; their actual stage is deferred
// until both future runner plans have passed all deterministic checks below.
const staged_product_bindings=approved_product_bindings.map(f=>({...f}));
const manifest={schema_version:1,result:'FROZEN_FINDINGS_CORE_SOURCE_COMPILE_TEST_NATIVE_NOT_RUN',recorded_at:new Date().toISOString(),baseline_commit:'c8f539be979ac5436dcc962c5ce7677aab9f3b0d',previous_manifest_sha256:sha(priorRaw),minimum_expected_workspace_tests:minimum,required_test_names:names.length,new_authored_findings_tests:newCount,new_tests_executed:0,approved_product_bindings,staged_product_bindings,files,required_tests,provenance,scope:'Previous265-input cohort preserved plus independent seven findings Core predicates, actual typed page and exact source evidence traversals, same-TX opaque full visitor, context-bound sparse pagination, new global typed-tail validation. Adapters/API/CLI/MCP/UI/distinct finding export, actual performance/physical/native/selected/recovery/platform acceptance remain open.',heavy_execution:'HELD_COORDINATED_LOCAL_COMPUTE_WINDOW',installed_runtime_modified:false};
const raw=Buffer.from(JSON.stringify(manifest,null,2)+'\n');const manifestSha=sha(raw),runners=[],runnerPlans=[];
for(const [oldName,newName,pin] of [
 ['run-sitemap-csv-streaming-suite-v1.mjs','run-sitemap-findings-core-suite-v1.mjs','613aff8e49df6a50115922ff3aa61e8cacce2f6c14066a7a002d0c7f80010332'],
 ['build-sitemap-csv-streaming-v1.mjs','build-sitemap-findings-core-v1.mjs','73c5f7dfc0d279025dd84bce629635378c1d8eff6efff08632406174f03ed0e5'],
]){
 const old=fs.readFileSync(path.join(__dirname,oldName));assert.equal(sha(old),pin);
 let text=old.toString().replace(/^const repo\s*=\s*[^;\r\n]+;/m,'const repo = '+JSON.stringify(repo)+';')
  .replaceAll('sitemap-csv-streaming','sitemap-findings-core').replaceAll(sha(priorRaw),manifestSha)
  .replaceAll('combined265-source','combined'+files.length+'-source').replaceAll('all288 named','all'+names.length+' named')
  .replaceAll('adds276 tests','adds'+(minimum-794)+' tests').replaceAll('minimum1070','minimum'+minimum)
  .replace(/\b265\b/g,String(files.length)).replace(/\b288\b/g,String(names.length)).replace(/\b1070\b/g,String(minimum));
 text=text.replace('Rich inventory, controlled same-TX allstored visitor and bounded two-pass CSV are included as authored source. Selected materialization, physical CSV64MiB/+1, seven full finding filters, source matrix, RSS/latency, recovery and native/platform acceptance remain open.','Rich inventory, controlled same-TX allstored raw/CSV and seven findings Core predicates/readers/session are authored source. Complete finding API/CLI/MCP/UI/export adapters, selected materialization, physical CSV64MiB/+1, source matrix, RSS/latency, recovery and native/platform acceptance remain open.');
 const expression=text.match(/^const repo\s*=\s*([^;\r\n]+);/m)[1],value=vm.runInNewContext(expression,Object.create(null),{timeout:1000});assert(path.win32.isAbsolute(value));assert.equal(path.win32.normalize(value),path.win32.normalize(repo));assert(fs.statSync(value).isDirectory());
 assert(!/sitemap-csv-streaming|\b265\b|\b288\b|\b1070\b|combined265-source|all288 named|adds276 tests|minimum1070/.test(text),'Stale previous runner literals');
 assert(text.indexOf('OPEN_SEO_COORDINATED_EXECUTION_WINDOW')<text.indexOf('fs.mkdirSync('));for(const k of ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER'])assert(text.includes(k));
 execFileSync(process.execPath,['--input-type=module','--check'],{input:text});
 const dest=path.join(__dirname,newName);runnerPlans.push({dest,text});runners.push({path:dest,sha256:sha(Buffer.from(text)),node_syntax_exit:0,isolated_actual_repo_literal_verified:true,execution:'NOT_RUN'});
}
// Every derived runner has parsed without evaluation and every deterministic
// prerequisite passed before the first source/runner output is created.
git(['add','--',...products]);
for(const f of staged_product_bindings){const b=git(['show',':'+f.path]);assert.equal(b.length,f.bytes);assert.equal(sha(b),f.sha256,'Actual exact staged bytes '+f.path);}
fs.mkdirSync(source);for(const f of files){const dest=path.join(source,f.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(path.join(repo,f.path),dest);assert.equal(sha(fs.readFileSync(dest)),f.sha256);}
fs.writeFileSync(path.join(source,'manifest.json'),raw,{flag:'wx'});
for(const {dest,text} of runnerPlans){fs.writeFileSync(dest,text,{flag:'wx'});execFileSync(process.execPath,['--check',dest]);assert.equal(sha(fs.readFileSync(dest)),sha(Buffer.from(text)));}
const receipt={schema_version:1,result:'SOURCE_FROZEN_FUTURE_SUITE_PREPARED_NOT_EXECUTED',recorded_at:new Date().toISOString(),source,manifest_sha256:manifestSha,files:files.length,minimum_expected:minimum,required_names:names.length,new_authored_tests:newCount,new_test_groups:groups,approved_product_bindings,staged_product_bindings,runners,previous_source_preserved:true,actual_checks:'Exact pinned payload lineage/staged bytes and ownership, preserved compiler inputs, unique authored test names, Node --check and isolated repo literal only; no runner imported or executed',compiler_tests_native:'NOT_RUN_COORDINATED_COMPUTE_HOLD',installed_runtime_modified:false};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({source,manifest_sha256:manifestSha,files:files.length,minimum_expected:minimum,required_names:names.length,new_authored_tests:newCount,runners}));
