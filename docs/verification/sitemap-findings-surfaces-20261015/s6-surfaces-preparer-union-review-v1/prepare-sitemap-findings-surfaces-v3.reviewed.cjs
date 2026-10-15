const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),cp=require('node:child_process');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',repo=path.join(base,'sitemap-findings-surfaces-integration-v1'),previous=path.join(base,'sitemap-findings-core-source-v1'),out=path.join(base,'sitemap-findings-surfaces-source-v1');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex'),insist=(c,m)=>{if(!c)throw new Error(m);};
const cliPostfixPin='ca05854c67af70e45b2c0bc92bf0fda0c6379656c20bbcea1b26eaae828b825c';
const cliReviewPath=path.join(base,'s6-cli-mcp-postfix-review-v1','receipt.json');
const cliReviewPin='f4b98a23dc14f6b75bf6c3b9a2c735bce9e6fa0b084d4c3893eb0ffe0b4ab29f';
const receiptPath=path.join(__dirname,'prepare-sitemap-findings-surfaces-v3.json');
insist(process.argv.length===2,'No caller-supplied authorization pin is accepted');
insist(!fs.existsSync(receiptPath),'fresh preparation receipt required before any source output');
insist(sha(fs.readFileSync(cliReviewPath))===cliReviewPin,'independent CLI postfix approval receipt changed');
const git=(args,optional=false)=>{const r=cp.spawnSync('git',args,{cwd:repo,encoding:null,maxBuffer:16*1024*1024});if(optional&&r.status!==0)return null;insist(r.status===0,'git read failed: '+args.join(' '));return r.stdout;};
insist(!fs.existsSync(out),'immutable source output exists');
insist(git(['rev-parse','HEAD']).toString().trim()==='898c404b596cdb812fd76c19a0a38de5d155371c','integration baseline changed');
const previousRaw=fs.readFileSync(path.join(previous,'manifest.json'));insist(sha(previousRaw)==='72dd26c3108c3e3432932e5bfb02d7f75eb8aeed9e7d71b24629511e81289f8d','previous275 manifest changed');const old=JSON.parse(previousRaw);
insist(old.files.length===275&&old.minimum_expected_workspace_tests===1123,'previous compiler/test basis changed');
const approved=new Map();
for(const f of old.files){const bytes=fs.readFileSync(path.join(previous,f.path));insist(bytes.length===f.bytes&&sha(bytes)===f.sha256,'previous frozen input changed '+f.path);approved.set(f.path,{sha256:f.sha256,bytes:f.bytes,basis:'previous275'});}
const specs=[
 ['s6-session-page-core-v1','8ca736921c66be73a0624294df3bdee9b30e0cddc13a2688c4eec66e1766d5e5'],
 ['s6-app-adapter-core-v1','5a0a2e7c22b345b689339deb0bd18a7ecb4292c127bf0c9e69b0f389f99101bb'],
 ['s6-app-adapter-postfix-v1','4cdc827ff07ea9a39019ff377a8be06b36e045e72f184450e0c40a4356a692dd'],
 ['s6-server-surfaces-core-v1','72fea685224b77ed175344a244cbf69f9531aaf4626de0fd3298fc03a4bc6a32'],
 ['s6-server-surfaces-postfix-v1','286425d05d43812dda0e0382c9cc13b2ea6325f15a07f7d975b2d3d36c8eb257'],
 ['s6-owned-read-registry-v1','3f449bb7f11455ed34dacbe127b2c995248d5d76e5fcc1a24e847a547ab6676a'],
 ['s6-owned-read-registry-postfix-v1','fa7f67a0e93cfe7fdd00e60f45fbd57946e4a669cb22929fd602a5a8833b8fb6'],
 ['s6-ui-surface-core-v1','c9e4cb2821182d8a357ce8f8082668a6892d96cf7929e959187e31321c0ee707'],
 ['s6-cli-mcp-core-v1','ebaebf161dbdc7defbd3e96ff7c24af9cadf6535d801d28714961558783492cf'],
 ['s6-parent-hooks-integration-v1','a74484cf3b4028fe98b218e58d02da0495f9ad7470cdf6cf5911142dfcf9f0d8'],
 ['s6-cli-mcp-postfix-v1',cliPostfixPin],
];
const lineage=[];const changed=new Set();
for(const [name,pin] of specs){
 const dir=path.join(base,name),raw=fs.readFileSync(path.join(dir,'manifest.json'));insist(sha(raw)===pin,'handoff manifest changed '+name);const m=JSON.parse(raw);
 const entries=m.entries??m.files;insist(Array.isArray(entries),'missing entries '+name);
 for(const e of entries){
  const p=e.path;insist(!path.isAbsolute(p)&&!p.split('/').includes('..'),'invalid input path');const bytes=fs.readFileSync(path.join(dir,p));insist(bytes.length===e.bytes&&sha(bytes)===e.after_sha256,'handoff payload changed '+name+':'+p);
  let before=approved.get(p);
  if(!before&&e.before_sha256!=null){
   const blob=git(['show','HEAD:'+p],true);insist(blob,'existing extra input not in baseline HEAD '+p);
   const lf=blob.toString('utf8').replace(/\r\n/g,'\n');const variants=[blob,Buffer.from(lf),Buffer.from(lf.replace(/\n/g,'\r\n'))];
   const exact=variants.find(b=>sha(b)===e.before_sha256);insist(exact,'extra baseline before image not reproduced from HEAD '+p);before={sha256:sha(exact),bytes:exact.length,basis:'HEAD exact LF/CRLF before image'};
  }
  if(e.before_sha256==null){insist(!before&&!git(['cat-file','-e','HEAD:'+p],true),'new input existed at baseline '+p);}else insist(before?.sha256===e.before_sha256,'unapproved lineage predecessor '+name+':'+p);
  approved.set(p,{sha256:e.after_sha256,bytes:e.bytes,basis:name});if(e.after_sha256!==e.before_sha256)changed.add(p);
 }
 lineage.push({name,manifest_sha256:pin,entries:entries.length});
}
const dirty=git(['ls-files','--modified','--others','--exclude-standard']).toString().trim().split(/\r?\n/).filter(Boolean);
for(const p of dirty.filter(p=>/^(crates|web|scripts|renderer)\//.test(p)||/^Cargo\.(toml|lock)$/.test(p)))insist(changed.has(p),'unexpected dirty product input '+p);
const actual=[...approved].sort(([a],[b])=>a.localeCompare(b)).map(([p,e])=>{const bytes=fs.readFileSync(path.join(repo,p));insist(bytes.length===e.bytes&&sha(bytes)===e.sha256,'actual integration input differs from approved lineage '+p);return {path:p,bytes:bytes.length,sha256:e.sha256,content:bytes};});
// Derive NEW test functions against the actual baseline Git source, rather than
// counting every pre-existing test in a changed file as newly authored.
function testNames(bytes){return new Set([...bytes.toString('utf8').matchAll(/#\[(?:test|tokio::test)(?:\([^\]]*\))?\][\s\S]*?(?:async\s+)?fn\s+(\w+)\s*\(/g)].map(m=>m[1]));}
const newTests=[];
for(const f of actual.filter(f=>changed.has(f.path)&&/^crates\/.+\.rs$/.test(f.path))){const blob=git(['show','HEAD:'+f.path],true),before=blob?testNames(blob):new Set(),after=testNames(f.content);for(const name of before)insist(after.has(name),'existing test removed '+f.path+'::'+name);const names=[...after].filter(n=>!before.has(n)).sort();if(names.length)newTests.push({path:f.path,names});}
const newCount=newTests.reduce((n,g)=>n+g.names.length,0);insist(newCount>=45,'new authored test inventory unexpectedly shrank');
const groups=new Map();for(const group of old.required_tests){if(!groups.has(group.path))groups.set(group.path,new Set());for(const name of group.names)groups.get(group.path).add(name);}for(const g of newTests){if(!groups.has(g.path))groups.set(g.path,new Set());for(const n of g.names)groups.get(g.path).add(n);}
const required=[...groups].map(([p,names])=>({path:p,names:[...names].sort()}));const requiredCount=required.reduce((n,g)=>n+g.names.length,0);
insist(requiredCount===341+newCount,'required-name overlap/recount must be reviewed');
const manifest={schema_version:1,result:'FROZEN_FINDINGS_SURFACES_SOURCE_COMPILE_TEST_NATIVE_NOT_RUN',recorded_at:new Date().toISOString(),baseline_commit:'898c404b596cdb812fd76c19a0a38de5d155371c',previous_manifest_sha256:sha(previousRaw),previous_inputs:275,previous_minimum_workspace_tests:1123,new_authored_test_functions:newCount,new_tests_executed:0,minimum_expected_workspace_tests:1123+newCount,required_test_names:requiredCount,lineage,cli_postfix_review:{path:cliReviewPath,sha256:cliReviewPin,scope:"INDEPENDENT_BOUNDED_SOURCE_CLOSURE_NOT_RUNTIME_ACCEPTANCE"},approved_product_bindings:actual.filter(f=>changed.has(f.path)).map(({content,...f})=>f),files:actual.map(({content,...f})=>f),required_tests:required,new_required_tests:newTests,ui_mock_groups_authored:13,ui_mock_groups_executed:0,compiler:'NOT_RUN',clippy:'NOT_RUN',native_sdk_browser_cli:'NOT_RUN',physical_boundaries:'NOT_RUN',installed_runtime_modified:false,manufacturer_parity:'NOT_PROVEN',qualification:'Complete source freeze with all prior275 inputs and exact approved additive/replacement lineage. New test counts are authored expectations, never executed results. Old installed ab1 and old raw physical proof are unchanged; new same-candidate compile/workspace/native/physical/renderer/browser/platform acceptance remains open.'};
// No source/artifact output exists until ALL lineage and actual inputs passed.
for(const f of actual){const p=path.join(out,f.path);fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,f.content,{flag:'wx'});}
const bytes=Buffer.from(JSON.stringify(manifest,null,2)+'\n');fs.writeFileSync(path.join(out,'manifest.json'),bytes,{flag:'wx'});
const result={schema_version:1,result:manifest.result,source:out,manifest_sha256:sha(bytes),files:actual.length,new_authored_tests:newCount,minimum_expected:manifest.minimum_expected_workspace_tests,required_test_names:requiredCount,actual_test_runs:0,approved_product_bindings:manifest.approved_product_bindings.length,lineage};fs.writeFileSync(receiptPath,JSON.stringify(result,null,2)+'\n',{flag:'wx'});process.stdout.write(JSON.stringify(result)+'\n');
