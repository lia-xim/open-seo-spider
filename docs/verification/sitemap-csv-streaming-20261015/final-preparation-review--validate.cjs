// Independent read/hash/index/syntax/literal validation ONLY. Never import or run routes.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),vm=require('node:vm');
const {createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const task='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const repo=base+'/sitemap-csv-streaming-integration-v1',source=base+'/sitemap-csv-streaming-source-v1',priorRoot=base+'/sitemap-rich-read-source-v1';
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const read=(root,p)=>fs.readFileSync(path.join(root,p));
const pin=(root,p,sha)=>{const raw=read(root,p);assert.equal(digest(raw),sha,p);return raw;};
const sourceSha='a31936510381cca40901eb547f15090a77c833f617f84f51f67b057fe7a5714e';
const priorSha='6d0e319f2800a7d32fb8509c7120ccb45d6d6724c3495c9cf3911a96dd5b26d9';
const sourceRaw=pin(source,'manifest.json',sourceSha),manifest=JSON.parse(sourceRaw);
const priorRaw=pin(priorRoot,'manifest.json',priorSha),prior=JSON.parse(priorRaw);
const preparer=pin(task,'work/prepare-sitemap-csv-streaming-v1.cjs','cbce067286097a6d6d35f0ecb8b0cee5268c07556887bbfbb2c5ef1a8e867bf8');
const preparationRaw=read(task,'work/prepare-sitemap-csv-streaming-v1.json'),preparation=JSON.parse(preparationRaw);
assert.equal(preparation.manifest_sha256,sourceSha);
assert.equal(manifest.previous_manifest_sha256,priorSha);
assert.equal(manifest.files.length,265);assert.equal(prior.files.length,255);
assert.equal(manifest.new_authored_csv_tests,35);assert.equal(manifest.new_tests_executed,0);
assert.equal(manifest.required_test_names,288);assert.equal(manifest.minimum_expected_workspace_tests,1070);
assert.equal(preparation.files,265);assert.equal(preparation.required_names,288);assert.equal(preparation.minimum_expected,1070);assert.equal(preparation.new_authored_tests,35);
assert.equal(manifest.baseline_commit,'d105fb8a0c527ffd99164e44ec644bd0e49f61a5');
const safe=p=>{assert(!path.isAbsolute(p)&&!p.includes('\\')&&!p.split('/').includes('..'),p);return p;};
const maps=new Map(),approved=new Map();
for(const [dir,sha] of [
 ['s4-csv-root-session-v1','115531f52ad1df716de592f68e317c1e3897470615a9ddc36c2833805358481a'],
 ['s4-csv-streaming-app-v1','3dac7530629186a60d9da074f12a765167dc5e470110d4c63469f6d9a90ed99e'],
 ['s4-csv-control-core-v1','aaceebb8167634bf0c748e92340948ed8b84ca3c1540e874b8c951f04782bebb'],
 ['s4-csv-reviewed-postfix-v1','5f97d60596b1ba05549b81a124ada1f2680341696f4aaf569343a0bfbb744932'],
 ['s4-csv-compat-wrappers-v1','c1da9605e574e302edeaece94e7f220b73c7df9df76fdbc12502b0f7e4035024'],
 ['s4-csv-streaming-tests-v1','83707b519f05b0b7e964ac25c4fdfacfbf998791409b7149218b0ae45fc891f0'],
]){
 const root=base+'/'+dir,m=JSON.parse(pin(root,'manifest.json',sha));maps.set(dir,m);
 for(const entry of m.entries??m.payloads??m.files){const p=safe(entry.path),b=pin(root,entry.payload??p,entry.sha256??entry.after_sha256);approved.set(p,b);}
 assert(manifest.provenance.some(p=>p.path===dir+'/manifest.json'&&p.sha256===sha));
}
for(const p of manifest.provenance){safe(p.path);pin(base,p.path,p.sha256);}
assert(manifest.provenance.some(p=>p.path==='s4-csv-streaming-tests-review-v1/receipt.json'&&p.sha256==='052c3b6774b4abbb3c55c282bedbb3f110bda8fdd42911f16cf531c447bce31a'));
const hook=maps.get('s4-csv-streaming-tests-v1').parent_hook;
assert.equal(digest(approved.get(hook.path)),hook.before_sha256);
assert.deepEqual(hook.append_lines,['#[path = "sitemaps_streaming_tests.rs"]','mod streaming_tests;']);
approved.set(hook.path,Buffer.concat([approved.get(hook.path),Buffer.from('\n'+hook.append_lines.join('\n')+'\n')]));
assert.equal(approved.size,26);
const derived=[],normalized=[];
for(const [p,raw] of approved){const text=raw.toString('utf8');assert(!/\r(?!\n)/.test(text));const bytes=Buffer.from(text.replaceAll('\r\n','\n'));if(!bytes.equals(raw))normalized.push(p);derived.push({path:p,bytes:bytes.length,sha256:digest(bytes)});}
derived.sort((a,b)=>a.path.localeCompare(b.path,'en'));
const ordered=a=>[...a].sort((a,b)=>a.path.localeCompare(b.path,'en'));
assert.deepEqual(ordered(manifest.approved_product_bindings),derived);
assert.deepEqual(ordered(manifest.staged_product_bindings),derived);
assert.deepEqual(ordered(preparation.staged_product_bindings),derived);
assert.equal(normalized.length,2);
const git=args=>execFileSync('git',args,{cwd:repo,maxBuffer:8*1024*1024,windowsHide:true});
const indexPins=[];
for(const expected of derived){const p=expected.path,b=read(repo,p),index=git(['show',':'+p]);assert.equal(b.length,expected.bytes,p);assert.equal(digest(b),expected.sha256,p);assert.equal(digest(index),expected.sha256,p+' actual index');indexPins.push({path:p,bytes:index.length,sha256:digest(index)});}
const staged=git(['diff','--cached','--name-only']).toString().trim().split(/\r?\n/).filter(p=>p.startsWith('crates/')).sort();
assert.deepEqual(staged,[...approved.keys()].sort());
const byPath=new Map();for(const f of manifest.files){safe(f.path);assert(!byPath.has(f.path));byPath.set(f.path,f);for(const root of [repo,source]){const b=read(root,f.path);assert.equal(b.length,f.bytes,f.path);assert.equal(digest(b),f.sha256,f.path);}}
let unownedPreserved=0;
for(const f of prior.files){safe(f.path);const old=read(priorRoot,f.path);assert.equal(digest(old),f.sha256,f.path+' previous');assert.equal(old.length,f.bytes,f.path+' previous bytes');assert(byPath.has(f.path),'Prior compiler input missing '+f.path);if(!approved.has(f.path)){assert.equal(byPath.get(f.path).sha256,f.sha256,f.path+' unowned drift');unownedPreserved++;}}
assert.equal(unownedPreserved,239);
const namesIn=b=>[...b.toString().matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)/g)].map(m=>m[1]);
const added=[];
for(const p of [...approved.keys()].sort()){const old=prior.files.some(f=>f.path===p)?namesIn(read(priorRoot,p)):[],now=namesIn(read(source,p));for(const name of old)assert(now.includes(name),'Removed prior test '+name);const names=now.filter(name=>!old.includes(name));if(names.length)added.push({path:p,names});}
assert.equal(added.reduce((n,g)=>n+g.names.length,0),35);assert.deepEqual(added,preparation.new_test_groups);
const required=prior.required_tests.map(g=>({path:g.path,names:[...g.names]}));
for(const group of added){const existing=required.find(g=>g.path===group.path);if(existing)existing.names.push(...group.names);else required.push(group);}
assert.deepEqual(required,manifest.required_tests);
const requiredNames=required.flatMap(g=>g.names);assert.equal(requiredNames.length,288);assert.equal(new Set(requiredNames).size,288);
for(const group of required){const actual=namesIn(read(source,group.path));for(const name of group.names)assert(actual.includes(name),group.path+'::'+name);}
const runtimeRoot=path.join(task,'outputs/open-seo-spider');let rendererAssets=0;
for(const f of manifest.files.filter(f=>f.path.startsWith('renderer/'))){assert.equal(digest(read(runtimeRoot,f.path)),f.sha256,f.path);rendererAssets++;}
const runnerResults=[];
for(const [oldName,oldSha,name,sha] of [
 ['run-sitemap-rich-read-suite-v1.mjs','f49d32a8b2a584f38bd3474e6f85be636cdecd097a62cd2ece148ca51378fd8b','run-sitemap-csv-streaming-suite-v1.mjs','613aff8e49df6a50115922ff3aa61e8cacce2f6c14066a7a002d0c7f80010332'],
 ['build-sitemap-rich-read-v1.mjs','455f66230c81d9ad22f457660927f5826be575055ba74c2d520701a8546c2983','build-sitemap-csv-streaming-v1.mjs','73c5f7dfc0d279025dd84bce629635378c1d8eff6efff08632406174f03ed0e5'],
]){
 const old=pin(task,'work/'+oldName,oldSha).toString(),actual=pin(task,'work/'+name,sha).toString();
 let expected=old.replace(/^const repo\s*=\s*[^;\r\n]+;/m,'const repo = '+JSON.stringify(repo)+';').replaceAll('sitemap-rich-read','sitemap-csv-streaming').replaceAll(priorSha,sourceSha).replace(/\b255\b/g,'265').replace(/\b253\b/g,'288').replace(/\b1035\b/g,'1070').replaceAll('combined255-source','combined265-source').replaceAll('all253 named','all288 named').replaceAll('adds241 tests','adds276 tests').replaceAll('minimum1035','minimum1070');
 if(name.startsWith('run-')){const guard="for (const name of ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']) { assert(!process.env[name], 'Unexpected externally configured compiler setting: ' + name); }\n";expected=expected.replace("const source=path.join(root,'sitemap-csv-streaming-source-v1');",guard+"const source=path.join(root,'sitemap-csv-streaming-source-v1');").replace('Rich App inventory and ordinary coverage/analysis are included as source; selected materializing compatibility, buffered CSV transport, full finding filters, physical source matrix, RSS and native proof remain open.','Rich inventory, controlled same-TX allstored visitor and bounded two-pass CSV are included as authored source. Selected materialization, physical CSV64MiB/+1, seven full finding filters, source matrix, RSS/latency, recovery and native/platform acceptance remain open.');}
 assert.equal(actual,expected,'Exact complete template derivation '+name);
 assert(!/sitemap-rich-read|\b255\b|\b253\b|\b1035\b|adds241 tests|buffered CSV transport/.test(actual));
 const literal=actual.match(/^const repo\s*=\s*([^;\r\n]+);/m)[1];assert(/^"[^"\r\n]*"$/.test(literal));
 const value=vm.runInNewContext(literal,Object.create(null),{timeout:1000});assert.equal(value,repo);assert(path.win32.isAbsolute(value));assert(fs.statSync(value).isDirectory());
 const hold=actual.indexOf('assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW');assert(hold>0);
 for(const operation of ['fs.mkdirSync(','await import(','spawn(']){const position=actual.indexOf(operation);if(position>=0)assert(hold<position,'HOLD before '+operation);}
 const compilerGuard=actual.indexOf("for (const name of ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'])");assert(compilerGuard>hold&&compilerGuard<actual.indexOf('fs.mkdirSync('));assert(actual.includes('assert(!process.env[name]'));
 execFileSync(process.execPath,['--check',path.join(task,'work',name)],{windowsHide:true});
 const reported=preparation.runners.find(r=>path.basename(r.path)===name);assert(reported&&reported.sha256===sha);
 runnerResults.push({path:'work/'+name,sha256:sha,node_syntax_exit:0,isolated_repo_literal:value,complete_template_derivation:true,hold_and_compiler_guards_before_writes:true,route_execution:'NOT_RUN'});
}
const output={status:'PASS_BOUNDED_SOURCE_HASH_INDEX_SYNTAX_LITERAL_CONTROLS_ONLY',source_manifest_sha256:sourceSha,preparer_sha256:digest(preparer),preparation_receipt_sha256:digest(preparationRaw),files:265,prior_files:255,prior_paths_preserved:255,unowned_prior_bytes_preserved:unownedPreserved,new_files:10,approved_and_index_bound_products:26,normalized_paths:normalized,new_authored_named_tests:35,required_names:288,minimum_expected:1070,new_tests_executed:0,renderer_source_assets_hash_checked:rendererAssets,installed_executable_verified_or_executed:false,derived_product_bindings:indexPins,runners:runnerResults,git_writes:false,source_or_runtime_writes:false,preparer_execution:'NOT_RUN',compiler_tests_renderer_native:'NOT_RUN_COMPUTE_HOLD'};
fs.writeFileSync(path.join(__dirname,'validation.json'),JSON.stringify(output,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({result:output.status,files:265,products:26,new_named:35,required:288,minimum:1070,normalized,rendererAssets,runners:runnerResults.map(r=>r.sha256)}));
