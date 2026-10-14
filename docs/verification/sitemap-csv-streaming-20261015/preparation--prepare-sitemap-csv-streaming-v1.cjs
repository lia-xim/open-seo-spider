const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const sha=b=>createHash('sha256').update(b).digest('hex');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',repo=path.join(base,'sitemap-csv-streaming-integration-v1');
const previous=path.join(base,'sitemap-rich-read-source-v1'),source=path.join(base,'sitemap-csv-streaming-source-v1');
const git=args=>execFileSync('git',args,{cwd:repo});
assert.equal(git(['rev-parse','HEAD']).toString().trim(),'d105fb8a0c527ffd99164e44ec644bd0e49f61a5');
assert(!fs.existsSync(source),'Fresh compiler source required');
const wirePin=process.argv[2],wireReviewPin=process.argv[3];assert(/^[a-f0-9]{64}$/.test(wirePin??''));assert(/^[a-f0-9]{64}$/.test(wireReviewPin??''));
const previousRaw=fs.readFileSync(path.join(previous,'manifest.json'));assert.equal(sha(previousRaw),'6d0e319f2800a7d32fb8509c7120ccb45d6d6724c3495c9cf3911a96dd5b26d9');
const prior=JSON.parse(previousRaw);assert.equal(prior.files.length,255);
for(const f of prior.files)assert.equal(sha(fs.readFileSync(path.join(previous,f.path))),f.sha256,f.path);
const provenance=[],manifests=new Map(),approved=new Map();
for(const [dir,file,pin] of [
 ['s4-csv-root-session-v1','manifest.json','115531f52ad1df716de592f68e317c1e3897470615a9ddc36c2833805358481a'],
 ['s4-csv-streaming-app-v1','manifest.json','3dac7530629186a60d9da074f12a765167dc5e470110d4c63469f6d9a90ed99e'],
 ['s4-csv-control-core-v1','manifest.json','aaceebb8167634bf0c748e92340948ed8b84ca3c1540e874b8c951f04782bebb'],
 ['s4-csv-reviewed-postfix-v1','manifest.json','5f97d60596b1ba05549b81a124ada1f2680341696f4aaf569343a0bfbb744932'],
 ['s4-csv-compat-wrappers-v1','manifest.json','c1da9605e574e302edeaece94e7f220b73c7df9df76fdbc12502b0f7e4035024'],
 ['s4-csv-streaming-tests-v1','manifest.json',wirePin],
 ['s4-csv-root-session-review-v1','receipt.json','d82396cf8c4d2bc5b80b4c2ab2bc16d145158b6d3733ca453a66ec3d7dbbd7c7'],
 ['s4-csv-app-independent-review-v1','receipt.json','199aea85c56fd8d81b77fbaaff34580c59ba33a9a5743144cd7706a0d5260a52'],
 ['s4-csv-control-independent-review-v1','receipt.json','1259eeb385f566f7f3819caf7bcfa99124894a214b9e8558d6a7dd1fbe534cc2'],
 ['s4-csv-postfix-review-v1','receipt.json','2701440146e06b824d2ce9b3bd0239a835c3759e89472b711899282c1f62e667'],
 ['s4-csv-compat-wrappers-review-v1','receipt.json','84ff96f4cb633bee4b32492f0d907ae8005bb55039d02a712aa5f9b277c9880f'],
 ['s4-csv-streaming-tests-review-v1','receipt.json',wireReviewPin],
]){
 const raw=fs.readFileSync(path.join(base,dir,file));assert.equal(sha(raw),pin,dir);const m=JSON.parse(raw);manifests.set(dir,m);
 if(file==='manifest.json')for(const f of m.entries??m.payloads??m.files){const p=f.payload??f.path,b=fs.readFileSync(path.join(base,dir,p));assert.equal(sha(b),f.sha256??f.after_sha256,p);approved.set(f.path,b);}
 provenance.push({path:dir+'/'+file,sha256:pin});
}
const owned=[...new Set(['s4-csv-root-session-v1','s4-csv-streaming-app-v1','s4-csv-control-core-v1','s4-csv-streaming-tests-v1'].flatMap(dir=>{const m=manifests.get(dir);return (m.entries??m.payloads??m.files).map(f=>f.path);}))].sort();
assert.equal(owned.length,26,'Reviewed exact product scope');
const actual=git(['status','--porcelain=v1','-uall']).toString().trimEnd().split(/\r?\n/).filter(Boolean).map(x=>x.slice(3)).filter(x=>x.startsWith('crates/')).sort();assert.deepEqual(actual,owned);
// Derive the final approved bytes from pinned payloads, their exact allowed
// parent append, and only repository-required CRLF -> LF. Current owned bytes
// must not silently become their own new source-review authority.
const hook=manifests.get('s4-csv-streaming-tests-v1').parent_hook,parent=approved.get(hook.path);assert(parent);assert.equal(sha(parent),hook.before_sha256);
approved.set(hook.path,Buffer.concat([parent,Buffer.from('\n'+hook.append_lines.join('\n')+'\n')]));
assert.equal(approved.size,owned.length);
const approved_product_bindings=owned.map(p=>{const raw=approved.get(p);assert(raw);const text=raw.toString('utf8');assert(!/\r(?!\n)/.test(text),'No bare CR '+p);const bytes=Buffer.from(text.replaceAll('\r\n','\n'));assert.equal(sha(fs.readFileSync(path.join(repo,p))),sha(bytes),'Reviewed final payload/hook/LF bytes '+p);return {path:p,bytes:bytes.length,sha256:sha(bytes)};});
const namesIn=b=>[...b.toString().matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)/g)].map(x=>x[1]);
const groups=[];
for(const p of owned){const old=prior.files.some(f=>f.path===p)?namesIn(fs.readFileSync(path.join(previous,p))):[],current=namesIn(fs.readFileSync(path.join(repo,p)));for(const n of old)assert(current.includes(n),'Previous test removed '+p+' '+n);const names=current.filter(n=>!old.includes(n));if(names.length)groups.push({path:p,names});}
const newCount=groups.reduce((n,g)=>n+g.names.length,0);assert.equal(newCount,35,'9 Core session +10 Control +3 App sinks +13 App wire');
const required_tests=prior.required_tests.map(g=>({path:g.path,names:[...g.names]}));for(const g of groups){const old=required_tests.find(x=>x.path===g.path);if(old)old.names.push(...g.names);else required_tests.push(g);}
const names=required_tests.flatMap(g=>g.names);assert.equal(new Set(names).size,names.length);const minimum=prior.minimum_expected_workspace_tests+newCount;
git(['add','--',...owned]);
const staged_product_bindings=owned.map(p=>{const b=fs.readFileSync(path.join(repo,p));assert.equal(sha(git(['show',':'+p])),sha(b),'Exact staged bytes '+p);return {path:p,bytes:b.length,sha256:sha(b)};});
const paths=[...new Set([...prior.files.map(f=>f.path),...owned])].sort();
const files=paths.map(p=>{const b=fs.readFileSync(path.join(repo,p)),old=prior.files.find(f=>f.path===p);if(old&&!owned.includes(p))assert.equal(sha(b),old.sha256,'Unowned compiler input drift '+p);return {path:p,bytes:b.length,sha256:sha(b)};});
const runtimeRoot=path.resolve(__dirname,'../outputs/open-seo-spider');for(const f of files.filter(f=>f.path.startsWith('renderer/')))assert.equal(sha(fs.readFileSync(path.join(runtimeRoot,f.path))),f.sha256);
fs.mkdirSync(source);for(const f of files){const dest=path.join(source,f.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(path.join(repo,f.path),dest);assert.equal(sha(fs.readFileSync(dest)),f.sha256);}
const manifest={schema_version:1,result:'FROZEN_CSV_STREAMING_SOURCE_COMPILE_TEST_NATIVE_NOT_RUN',recorded_at:new Date().toISOString(),baseline_commit:'d105fb8a0c527ffd99164e44ec644bd0e49f61a5',previous_manifest_sha256:sha(previousRaw),minimum_expected_workspace_tests:minimum,required_test_names:names.length,new_authored_csv_tests:newCount,new_tests_executed:0,approved_product_bindings,staged_product_bindings,files,required_tests,provenance,scope:'Complete previous255-input cohort plus shared actual-TX repeated allstored visitor, controlled actual readers and two-pass bounded CSV transport with terminal tail held through Core return and actual blocking-worker join. All new behavior is authored only. Selected materialization, physical CSV64MiB/+1, RSS/latency, source protocol matrix, native/recovery/platforms and seven full finding filters remain open.',heavy_execution:'HELD_COORDINATED_LOCAL_COMPUTE_WINDOW',installed_runtime_modified:false};
const manifestRaw=Buffer.from(JSON.stringify(manifest,null,2)+'\n');fs.writeFileSync(path.join(source,'manifest.json'),manifestRaw,{flag:'wx'});const manifestSha=sha(manifestRaw),runners=[];
for(const [oldName,newName,pin] of [
 ['run-sitemap-rich-read-suite-v1.mjs','run-sitemap-csv-streaming-suite-v1.mjs','f49d32a8b2a584f38bd3474e6f85be636cdecd097a62cd2ece148ca51378fd8b'],
 ['build-sitemap-rich-read-v1.mjs','build-sitemap-csv-streaming-v1.mjs','455f66230c81d9ad22f457660927f5826be575055ba74c2d520701a8546c2983'],
]){
 const old=fs.readFileSync(path.join(__dirname,oldName));assert.equal(sha(old),pin);let text=old.toString().replace(/^const repo\s*=\s*[^;\r\n]+;/m,'const repo = '+JSON.stringify(repo.replaceAll('\\','/'))+';');
 text=text.replaceAll('sitemap-rich-read','sitemap-csv-streaming').replaceAll(sha(previousRaw),manifestSha).replace(/\b255\b/g,String(files.length)).replace(/\b253\b/g,String(names.length)).replace(/\b1035\b/g,String(minimum)).replaceAll('combined255-source','combined'+files.length+'-source').replaceAll('all253 named','all'+names.length+' named').replaceAll('adds241 tests','adds'+(minimum-794)+' tests').replaceAll('minimum1035','minimum'+minimum);
 if(oldName.startsWith('run-')){
  const guard="for (const name of ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']) { assert(!process.env[name], 'Unexpected externally configured compiler setting: ' + name); }\n";
  text=text.replace("const source=path.join(root,'sitemap-csv-streaming-source-v1');",guard+"const source=path.join(root,'sitemap-csv-streaming-source-v1');");
  text=text.replace('Rich App inventory and ordinary coverage/analysis are included as source; selected materializing compatibility, buffered CSV transport, full finding filters, physical source matrix, RSS and native proof remain open.','Rich inventory, controlled same-TX allstored visitor and bounded two-pass CSV are included as authored source. Selected materialization, physical CSV64MiB/+1, seven full finding filters, source matrix, RSS/latency, recovery and native/platform acceptance remain open.');
 }
 const expression=text.match(/^const repo\s*=\s*([^;\r\n]+);/m)[1],value=vm.runInNewContext(expression,Object.create(null),{timeout:1000});assert(path.win32.isAbsolute(value));assert.equal(path.win32.normalize(value),path.win32.normalize(repo));assert(fs.statSync(value).isDirectory());
 assert(!/sitemap-rich-read|\b255\b|\b253\b|\b1035\b|combined255-source|adds241 tests|minimum1035|buffered CSV transport/.test(text),'No stale suite/build literals');
 assert(text.indexOf('OPEN_SEO_COORDINATED_EXECUTION_WINDOW')<text.indexOf('fs.mkdirSync('),'Actual compute guard before writes');
 for(const k of ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER'])assert(text.includes(k));
 const dest=path.join(__dirname,newName);fs.writeFileSync(dest,text,{flag:'wx'});execFileSync(process.execPath,['--check',dest]);runners.push({path:dest,sha256:sha(fs.readFileSync(dest)),node_syntax_exit:0,isolated_actual_repo_literal_verified:true,execution:'NOT_RUN'});
}
const receipt={schema_version:1,result:'SOURCE_FROZEN_FUTURE_SUITE_PREPARED_NOT_EXECUTED',recorded_at:new Date().toISOString(),source,manifest_sha256:manifestSha,files:files.length,minimum_expected:minimum,required_names:names.length,new_authored_tests:newCount,new_test_groups:groups,staged_product_bindings,runners,previous_source_preserved:true,actual_checks:'Exact SHA inputs/staged bytes and ownership; unique actual authored test names; existing absolute JS repo expression; Node --check; no runner imported/executed',compiler_tests_native:'NOT_RUN_COORDINATED_COMPUTE_HOLD',installed_runtime_modified:false};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({source,manifest_sha256:manifestSha,files:files.length,minimum_expected:minimum,required_names:names.length,new_authored_tests:newCount,runners}));
