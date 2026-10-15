// Source author only. Built-in file/crypto/assert and Node syntax checks; no target imports.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
const B='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const T='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
const OLD=path.join(B,'source7-native-preparation-v2'),MOCK=path.join(B,'source7-ui-mock-repair-preparation-v3-r2');
const O=path.join(B,'source8-native-small-preparation-v1');
const SHA='58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720';
const PARENT='fd587ba252595d971e7557e5cca7f5ab90219ecda200a3f07fcd5c10c13b6397';
const sha=b=>createHash('sha256').update(b).digest('hex'),read=p=>fs.readFileSync(p),json=p=>JSON.parse(read(p));
const pin=p=>{const b=read(p);return {path:p,sha256:sha(b),bytes:b.length};};
const requiredPin=(p,h)=>{const r=pin(p);assert.equal(r.sha256,h,p);return r;};
assert(!fs.existsSync(path.join(O,'manifest.json')),'Final immutable handoff already exists');
const sourcePin=requiredPin(path.join(B,'combined-source-v8/manifest.json'),SHA),source=json(sourcePin.path);
assert.equal(source.kind,'combined_source_v8');assert.equal(source.source_inputs,335);assert.equal(source.files.length,335);assert.equal(source.required_test_names,540);assert.equal(source.minimum_expected_workspace_tests,1322);assert.equal(source.source_commit,'e6b1fa2f87d0b39ac7f83aba6cdecc2110de5a50');assert.equal(source.parent_source_manifest_sha256,PARENT);
const parentPin=requiredPin(path.join(OLD,'manifest.json'),'e8d22c28244c97e857071aa4b369f83c203c9decbc74e6354ba92f5e8e3c6e85');
const parentReview=requiredPin(path.join(B,'source7-native-independent-review-v2/receipt.json'),'9b1ffbc1e3f83f780361f8ac56086b5ed6f95dc60b07d317224bd13aa52a448a');
const mockPin=requiredPin(path.join(MOCK,'manifest.json'),'5fb462a528b04ea24b0efb9e4baaa7b048ba3660f8fc71438270ea37adff14a7');
const mockReview=requiredPin(path.join(B,'source7-ui-mock-independent-review-v3-r2/receipt.json'),'6e3577af554c3e470628c55d779a1b25c61db52386c3af8d81ab42cc00ca00de');
const runner=requiredPin(path.join(B,'combined-v8-runner-preparation-v4/manifest.json'),'d4adece778ec4a2b3ef713be7319840457bbf5119b48809fe07493af65ca1c80');
const binder=requiredPin(path.join(B,'combined-v8-runner-preparation-v4/combined-bindings-v8.mjs'),'c58f1984c6b6593d384915157afdcdae206acd1bb8af72279a9ee82702e1b7e7');
const binderReview=requiredPin(path.join(B,'combined-v8-runner-independent-review-v4/receipt.json'),'3b6240aca4bec56afb058d83b8552ef1c36b28ba465169a31052c5df0e28ae78');
const builder=requiredPin(path.join(B,'combined-v8-runner-preparation-v4/build-combined-optimized-v8.mjs'),'e48e2cadc361a100eecbbed5465bf4436d28f7fa19068682550932f320e1c547');
const suite=requiredPin(path.join(B,'combined-v8-runner-preparation-v4/run-combined-suite-v8.mjs'),'921a8023a3354c42ef28583888734bbe6c4784794161ec5e5f5c8f53dd39f62b');
for(const p of [parentReview,mockReview,binderReview]){const v=json(p.path);assert.equal(v.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(v.findings,[]);}
assert.equal(json(binderReview.path).binder.sha256,binder.sha256);
const pending=new Map(),derivations=[],sourcePins=[],scopes=[];
const row=p=>{const b=pending.get(p)??read(p);return {path:p,sha256:sha(b),bytes:b.length};};
function save(p,b){assert(!pending.has(p)&&!fs.existsSync(p),p);pending.set(p,Buffer.isBuffer(b)?b:Buffer.from(b));return row(p);}
const put=(p,v)=>save(p,JSON.stringify(v,null,2)+'\n');
function doc(from,to){const raw=read(from);return {from,to,raw,buffer:raw,operations:[]};}
function literal(d,label,oldText,newText,required=false){
  if(oldText===newText)return;const a=Buffer.from(oldText),b=Buffer.from(newText);let at=0,n=0;
  while((at=d.buffer.indexOf(a,at))>=0){const before=d.buffer;d.buffer=Buffer.concat([before.subarray(0,at),b,before.subarray(at+a.length)]);d.operations.push({label,offset:at,before_base64:a.toString('base64'),after_base64:b.toString('base64'),before_sha256:sha(before),after_sha256:sha(d.buffer)});at+=b.length;n++;}
  assert(!required||n>0,label+' absent in '+d.from);
}
function finish(d){
  let inverse=d.buffer;for(const op of [...d.operations].reverse()){assert.equal(sha(inverse),op.after_sha256);const a=Buffer.from(op.after_base64,'base64'),b=Buffer.from(op.before_base64,'base64');assert(inverse.subarray(op.offset,op.offset+a.length).equals(a));inverse=Buffer.concat([inverse.subarray(0,op.offset),b,inverse.subarray(op.offset+a.length)]);assert.equal(sha(inverse),op.before_sha256);}assert(inverse.equals(d.raw));
  const p=save(d.to,d.buffer);derivations.push({before:pin(d.from),after:p,operations:d.operations,inverse_restored_parent_exact:true});return p;
}
const hashes=new Map([
 ['c2bf39d4e6c177374a1f9331f1bcb06b04993bb1000777dff80319532b5e9978',binder.sha256],
 ['7bdbf4bce7cc47bb2744a2d38b8cf173d202a6ed04b371fd3b61a6309696cc05',runner.sha256],
 ['3edc970c19808fe008d4fd46e74cc72be25a7dfbd718977c79419166dcb26048',binderReview.sha256],
 ['1704796d260e46bb7f9bedd28e2f951a0d4bda0c1d70bef655cf4cf58042dee6',builder.sha256],
 ['576bc7a91d44281593c4f61a48d72714c7aead0279bfe50202afcf673f82029c',suite.sha256],
]);
function common(d){
  literal(d,'fresh Small6 package path','source7-native-preparation-v2','source8-native-small-preparation-v1');
  literal(d,'reviewed corrected mock package path','source7-ui-mock-repair-preparation-v3-r2','source8-native-small-preparation-v1');
  literal(d,'actual V4 Source8 producer directory','combined-v7-runner-preparation-v2','combined-v8-runner-preparation-v4');
  literal(d,'actual V4 Source8 runner review','combined-v7-runner-independent-review-v2','combined-v8-runner-independent-review-v4');
  for(const [old,value]of hashes)literal(d,'current exact dependency hash',old,value);
  literal(d,'actual frozen Source8 authority',PARENT,SHA);
  literal(d,'versioned fresh protocol/output/scope names','v7','v8');
  literal(d,'versioned source names','source7','source8');literal(d,'versioned result names','V7','V8');literal(d,'versioned prose','Source7','Source8');literal(d,'versioned uppercase source prose','SOURCE7','SOURCE8');
}
function script(from,relative,custom=()=>{}){const d=doc(from,path.join(O,relative));common(d);custom(d);return finish(d);}
function metadata(from,relative,fn){const d=doc(from,path.join(O,relative));const v=fn(json(from));literal(d,'explicit actual Source8 metadata',d.raw.toString(),JSON.stringify(v,null,2)+'\n',true);return finish(d);}
function actualFile(p){const v=source.files.find(f=>f.path===p);assert(v,p);const actual=pin(path.join(B,'combined-source-v8',p));assert.equal(actual.sha256,v.sha256);assert.equal(actual.bytes,v.bytes);if(!sourcePins.some(r=>r.path===actual.path))sourcePins.push(actual);return v;}
const sourceMeta={path:sourcePin.path,sha256:SHA,source_commit:source.source_commit,inputs:335,required_names:540,minimum_workspace_tests:1322,status:'ACTUAL_FROZEN_SOURCE_SUITE_BUILD_CANDIDATE_STILL_REQUIRED'};
const relativeRows=rows=>rows.map(p=>({path:path.basename(p.path),sha256:p.sha256,bytes:p.bytes}));
function manifest(scope,parent,fields,rows){const p=put(path.join(O,scope,'manifest.json'),{schema_version:1,...fields,source_manifest:sourceMeta,parent_preparation:pin(parent),source_only_lineage_not_runtime_acceptance:true,...rows});scopes.push({scope,manifest:p});return p;}

// Exact original six UI paths, rebuilt from actual frozen Source8; no fixture assertion weakened.
const asset=metadata(path.join(MOCK,'adapters/UI-ASSETS.json'),'adapters/UI-ASSETS.json',v=>{
  const oldSource=json(path.join(B,'combined-source-v7/manifest.json'));
  const files=v.files.map(r=>{const f=actualFile(r.path);return {...r,bytes:f.bytes,after_sha256:f.sha256};});
  const changed=files.filter(r=>oldSource.files.find(f=>f.path===r.path).sha256!==r.after_sha256).length;
  return {...v,kind:'ui_combined_v8_mock_asset_manifest_v1',supported_source_kind:'combined_source_v8',compiler_source_parent_manifest_sha256:PARENT,compiler_source_manifest_sha256:SHA,changed_parent_assets:changed,unchanged_parent_assets:files.length-changed,files,executed_mock_groups:0,scope:'Actual frozen Source8 six UI assets; same thirteen original mock groups. Migration workflow and real browser acceptance remain separate.',compiler_source_binding:'REQUIRE_ACTUAL_ROOT_SOURCE8_HASH_AND_ALL_SIX_PINNED_ASSETS'};
});
hashes.set('631bf3ce6dcb9505ffdc3054c4358f654f4278981d09b4885b73e76b861ac886',asset.sha256);
hashes.set('78eafd37c831795190aafea0bbb09f2af1898b379360db0058b93e7c5ece8f5b',asset.sha256);
const mock=script(path.join(MOCK,'adapters/findings-ui-mock-combined-owned-adapter.cjs'),'adapters/findings-ui-mock-combined-owned-adapter.cjs',d=>literal(d,'actual direct Source8 parent','4572dc39ee478b8b3d55d06b5983779d6a42416c307256780050dc5950c49b77',PARENT,true));
hashes.set('a0daa9d26251e3e87177344bcd79164571ac0d968cf21778e6c42c8563707a5e',mock.sha256);hashes.set('524ef313f63da23f774854fbb425016acfba43ce82d729c1d38ab24d820e4c2b',mock.sha256);
const nativeDriver=script(path.join(OLD,'adapters/run-combined-findings-acceptance-v7.mjs'),'adapters/run-combined-findings-acceptance-v8.mjs',d=>literal(d,'actual direct Source8 parent','27a8b74c4cd0c394906f3fe3c0a6a0327569294b25056cf6c6f0aabcffc42788',PARENT,true));
const mockDriver=script(path.join(MOCK,'adapters/run-combined-findings-ui-mock-acceptance-v7-r2.mjs'),'adapters/run-combined-findings-ui-mock-acceptance-v8.mjs',d=>{
  literal(d,'fresh first Source8 mock driver/protocol','v8-r2','v8',true);literal(d,'fresh first Source8 mock release kind','v8_r2','v8',true);
  literal(d,'actual direct Source8 parent','4572dc39ee478b8b3d55d06b5983779d6a42416c307256780050dc5950c49b77',PARENT,true);
});
const adapterManifest=manifest('adapters',path.join(MOCK,'adapters/manifest.json'),{kind:'native_adapters_combined_v8',status:'AUTHORED_NOT_RUN',native_parent:pin(path.join(OLD,'adapters/manifest.json')),corrected_mock_parent:mockPin},{files:relativeRows([nativeDriver,mockDriver,mock,asset])});

const physDriver=script(path.join(OLD,'physical/native-finding-physical-combined-v7.mjs'),'physical/native-finding-physical-combined-v8.mjs');
const physBinding=save(path.join(O,'physical/combined-bindings.mjs'),read(binder.path));
const physicalManifest=manifest('physical',path.join(OLD,'physical/manifest.json'),{kind:'physical_combined_rebind_v8',status:'AUTHORED_NOT_RUN'},{files:relativeRows([physDriver,physBinding])});

const browserParent=json(path.join(OLD,'browser/manifest.json')),browserRows=[];
for(const r of browserParent.files){
  if(r.path==='combined-bindings.mjs')browserRows.push(save(path.join(O,'browser',r.path),read(binder.path)));
  else browserRows.push(script(path.join(OLD,'browser',r.path),'browser/'+r.path.replaceAll('v7','v8')));
}
const browserManifest=manifest('browser',path.join(OLD,'browser/manifest.json'),{kind:'browser_combined_rebind_v8',status:'AUTHORED_NOT_RUN'},{files:relativeRows(browserRows)});
const browserDriver=browserRows.find(r=>r.path.endsWith('run-sitemap-findings-browser-combined-v8.mjs'));assert(browserDriver);
assert.equal(browserRows.find(r=>r.path.endsWith('browser-original-body-responsive.mjs')).sha256,'539c67c2af6cc34022327a6c201df41fbda600b35684d9b0455a8051efc00981','Reviewed original BODY helper remains byte exact');
const sdkData={};
for(const scope of ['file-sdk','root-junction']){
  const pm=json(path.join(OLD,scope,'manifest.json')),name=pm.payloads.find(f=>f.path.endsWith('.mjs')).path;
  const driver=script(path.join(OLD,scope,name),scope+'/'+name);
  const contract=metadata(path.join(OLD,scope,'contract.json'),scope+'/contract.json',v=>{
    const walk=x=>{if(Array.isArray(x))return x.map(walk);if(!x||typeof x!=='object')return x;if(x.path&&x.sha256&&source.files.some(r=>r.path===x.path)){const f=actualFile(x.path);return {...x,sha256:f.sha256,...('bytes'in x?{bytes:f.bytes}:{})};}return Object.fromEntries(Object.entries(x).map(([k,y])=>[k,walk(y)]));};
    const z=walk(v);z.version=8;z.sourceInputs=335;z.sourceManifestSha256=SHA;return z;
  });
  const prep=manifest(scope,path.join(OLD,scope,'manifest.json'),{kind:scope==='file-sdk'?'mcp_file_tools_sdk_preparation_v8':'mcp_root_junction_cli_preparation_v8',status:'AUTHORED_SOURCE_ONLY_NOT_RUN'},{payloads:relativeRows([driver,contract])});sdkData[scope]={driver,contract,prep};
}

// The standard six-row issuer/outer is preserved. Only the ui-mock row gets the corrected dedicated driver.
const preparations=[
  {gate:'native',driver:path.join(T,'work/run-combined-findings-acceptance-v8.mjs'),sha256:nativeDriver.sha256,path:adapterManifest.path,manifest_sha256:adapterManifest.sha256},
  {gate:'ui-mock',driver:path.join(T,'work/run-combined-findings-ui-mock-acceptance-v8.mjs'),sha256:mockDriver.sha256,path:adapterManifest.path,manifest_sha256:adapterManifest.sha256},
  {gate:'browser-native',driver:browserDriver.path,sha256:browserDriver.sha256,path:browserManifest.path,manifest_sha256:browserManifest.sha256},
  {gate:'physical-findings',driver:physDriver.path,sha256:physDriver.sha256,path:physicalManifest.path,manifest_sha256:physicalManifest.sha256},
  ...['file-sdk','root-junction'].map(scope=>({gate:scope,driver:sdkData[scope].driver.path,sha256:sdkData[scope].driver.sha256,path:sdkData[scope].prep.path,manifest_sha256:sdkData[scope].prep.sha256})),
];
const replacements=[
 ['1bc4bc60513119cde70803bfd00720e64ef1cc9b6da8dce07fcb0a8c5afabaf8',nativeDriver.sha256],
 ['e94f24449c2a1b705501bab1b80fcf8b57088888d4f266f316face8d44c397db',browserDriver.sha256],
 ['ff10d9a7b6b5258faa486ac73cc38de5b89236300658de3f39ab69b2801a6ac3',physDriver.sha256],
 ['b593002399fc8439fd6764011691e9c8579a71838195f811d666d861f2bdffe3',adapterManifest.sha256],
 ['0f17a452745e9fae45a8d350a13c13249fce93595406f375c108f25497657f16',browserManifest.sha256],
 ['9bed04bbb1f8b2aea4c3d7cdde9b93d3510fdfa34b673a97ae2e639b0e3efac8',physicalManifest.sha256],
 ...['file-sdk','root-junction'].flatMap(scope=>{const old=json(path.join(OLD,scope,'manifest.json'));return [[old.payloads.find(r=>r.path.endsWith('.mjs')).sha256,sdkData[scope].driver.sha256],[pin(path.join(OLD,scope,'manifest.json')).sha256,sdkData[scope].prep.sha256]];}),
];
for(const [a,b]of replacements)hashes.set(a,b);
const writer=script(path.join(OLD,'outer-small/prepare-v7-native-bindings.mjs'),'outer-small/prepare-v8-native-bindings.mjs',d=>{
  literal(d,'actual complete Source8 input count',"assert.equal(binding.manifest.source_inputs,326);","assert.equal(binding.manifest.source_inputs,335);",true);
  const text=d.buffer.toString(),start=text.indexOf('const preparations=['),end=text.indexOf('\nfor(const p of preparations){',start);assert(start>=0&&end>start);
  literal(d,'exact six current inner drivers and child manifests',text.slice(start,end),'const preparations='+JSON.stringify(preparations,null,2)+';',true);
  const before="for(const gate of ['native','ui-mock'])add('v8-'+gate+'-release.json',{...common,kind:'root_released_combined_findings_acceptance_v8',gate,driver_sha256:'"+nativeDriver.sha256+"',output:path.join(base,'results/combined-findings-'+gate+'-optimized-acceptance-v8'),...(gate==='ui-mock'?{chrome_executable:chrome,chrome_sha256:chromeSha}:{})});";
  const after="add('v8-native-release.json',{...common,kind:'root_released_combined_findings_acceptance_v8',gate:'native',driver_sha256:'"+nativeDriver.sha256+"',output:path.join(base,'results/combined-findings-native-optimized-acceptance-v8')});\nadd('v8-ui-mock-release.json',{...common,kind:'root_released_combined_findings_ui_mock_acceptance_v8',gate:'ui-mock',driver_sha256:'"+mockDriver.sha256+"',output:path.join(base,'results/combined-findings-ui-mock-optimized-acceptance-v8'),chrome_executable:chrome,chrome_sha256:chromeSha});";
  literal(d,'corrected dedicated mock release instead of legacy guardian',before,after,true);
});
const grants={native:'v8-native-release.json','ui-mock':'v8-ui-mock-release.json','browser-native':'v8-browser-native-release.json','physical-findings':'v8-physical-release.json','file-sdk':'v8-mcp-file-sdk-binding.json','root-junction':'v8-mcp-root-junction-binding.json'};
const gates=Object.fromEntries(preparations.map(r=>[r.gate,[grants[r.gate],r.driver,r.sha256,path.relative(B,path.dirname(r.path)).replaceAll('\\','/'),r.manifest_sha256]]));
const outer=script(path.join(OLD,'outer-small/run-combined-native-outer-v7.mjs'),'outer-small/run-combined-native-outer-v8.mjs',d=>{
  const text=d.buffer.toString(),start=text.indexOf('const gates = {'),end=text.indexOf('\n\nasync function main()',start);assert(start>=0&&end>start);
  literal(d,'exact six current Root grants and inner drivers',text.slice(start,end),'const gates = '+JSON.stringify(gates,null,2)+';',true);
});
const outerManifest=manifest('outer-small',path.join(OLD,'outer-small/manifest.json'),{kind:'native_outer_v8_lease_preparation',status:'AUTHORED_NOT_RUN',binder_manifest:runner,binder,binder_review:binderReview,source_inputs:335,required_names:540,minimum_workspace_tests:1322},{files:relativeRows([writer,outer])});

const whole=[];
function region(from,to,bytes,label){const b=pending.get(to);assert(b);const at=b.indexOf(bytes);assert(at>=0,'whole old oracle missing '+label);whole.push({label,before:from,after:to,bytes:bytes.length,sha256:sha(bytes),after_offset:at,byte_exact:true});}
const oldRegions=json(path.join(OLD,'DERIVATION.json')).whole_functional_regions;
for(const r of oldRegions){const rel=path.relative(OLD,r.after).replaceAll('\\','/');if(!['physical/','browser/','adapters/','file-sdk/','root-junction/','outer-small/'].some(p=>rel.startsWith(p)))continue;
  if(rel==='adapters/findings-ui-mock-combined-owned-adapter.cjs')continue;
  region(r.after,path.join(O,rel.replaceAll('v7','v8')),read(r.after).subarray(r.after_offset,r.after_offset+r.bytes),r.label);
}
// Mock behavior comes from the corrected reviewed adapter, not the obsolete parent header.
{const from=path.join(MOCK,'adapters/findings-ui-mock-combined-owned-adapter.cjs'),b=read(from),at=b.indexOf(Buffer.from('const playwright='));assert(at>0);region(from,mock.path,b.subarray(at),'all corrected mock thirteen cases and real browser/server cleanup');}
for(const name of ['browser-flow.mjs','profile-form-flow.mjs','browser-membership-flow.mjs','browser-original-body-responsive.mjs']){
  const from=path.join(OLD,'browser',name),to=path.join(O,'browser',name);assert(pending.get(to).equals(read(from)),name+' complete retained helper');region(from,to,read(from),'complete retained '+name);
}
// Preserve the corrected mock guardian's full child/process, dependency and installed-runtime guards.
{const from=path.join(MOCK,'adapters/run-combined-findings-ui-mock-acceptance-v7-r2.mjs'),b=read(from),i=b.indexOf(Buffer.from('function treeIdentity(')),j=b.indexOf(Buffer.from('const installed='),i);assert(i>=0&&j>i);region(from,mockDriver.path,b.subarray(i,j),'corrected mock complete dependency tree and SDK version guards');}
assert(pending.get(physDriver.path).includes(Buffer.from('CSV64M_APPJSON8M_EXACT_PLUS1')));
const bodyFlow=pending.get(path.join(O,'browser/run-sitemap-findings-browser-combined-v8.mjs')).toString();assert(bodyFlow.includes('assert.equal(records.length,26)')&&bodyFlow.includes('assert.equal(records.length,27)'));assert.equal((bodyFlow.match(/await runOriginalBodyFormResponsive\(/g)||[]).length,1);
for(const scope of ['file-sdk','root-junction']){const c=JSON.parse(pending.get(sdkData[scope].contract.path));assert.equal(c.sourceInputs,335);assert.equal(c.sourceManifestSha256,SHA);}
// The new migration API does not change ownership/statistics schemas used by the retained small scopes.
const ownership=[];for(const p of ['crates/app/src/server/lifecycle.rs','crates/app/src/server/transport.rs','crates/app/src/server/source_body_stream.rs','crates/app/src/server/response_stream.rs']){
  const f=actualFile(p),parent=json(path.join(B,'combined-source-v7/manifest.json')).files.find(r=>r.path===p);ownership.push({path:p,parent_sha256:parent.sha256,source8_sha256:f.sha256,byte_exact:parent.sha256===f.sha256});
}
assert(ownership.find(r=>r.path.endsWith('/lifecycle.rs')).byte_exact);assert(ownership.find(r=>r.path.endsWith('/transport.rs')).byte_exact);assert(ownership.find(r=>r.path.endsWith('/source_body_stream.rs')).byte_exact);
for(const [p,b]of pending){fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});}
const syntax=[];for(const [p]of pending){if(!/\.(mjs|cjs)$/.test(p))continue;const r=spawnSync(process.execPath,['--check',p],{encoding:'utf8',windowsHide:true});assert.equal(r.status,0,p+' '+r.stderr);assert.equal(r.signal,null);syntax.push({...pin(p),exitcode:r.status,signal:r.signal});}
const writeFinal=(p,v)=>{fs.writeFileSync(p,JSON.stringify(v,null,2)+'\n',{flag:'wx'});return pin(p);};
const derivation=writeFinal(path.join(O,'DERIVATION.json'),{kind:'source8_small_native_exact_source_derivations',operations:derivations.reduce((n,d)=>n+d.operations.length,0),files:derivations,inverse_restored_all_parents_exact:true,whole_functional_regions:whole});
const checks=writeFinal(path.join(O,'SOURCE-CHECKS.json'),{kind:'source8_small_native_source_checks_only',syntax,actual_relevant_frozen_source_pins:sourcePins,ownership_schema:ownership,whole_regions:whole.length,exact_inverses:derivations.length,compiler:0,fixtures:0,target_imports:0,native:0,root_writes:0,git:0,qualification:'Only source derivations and Node syntax checks. Original Source7 PASS/FAIL history is not a Source8 acceptance. Response writer partial-write implementation differs but lifecycle/transport/body schemas remain source-pinned; runtime outcomes require fresh actual Source8.'});
const handoff=`# Source8: sechs kleine native Prüfbereiche\n\nDieses Paket ist ausschließlich vorbereitet. Es wurde kein Harness importiert oder ausgeführt. Tatsächlicher Source8 ist ${SHA}, Commit ${source.source_commit}, 335 Quellen, 540 Pflichtnamen, Mindestzahl 1322. Suite, optimierter Build und Kandidat sind weiterhin zwingende zukünftige Belege.\n\nDie sechs Bereiche sind Physical12 (CSV 64 MiB / App-JSON 8 MiB / jeweilige +1-Kontrollen), SDK/API/CLI3, FileSDK175, Rootjunction2, Mock13 und echter Browser27. Alle vollständigen übernommenen Orakelkörper stehen mit Hash und inversen Operationen in DERIVATION.json. Die vier Browser-Helfer bleiben vollständig bytegleich; Gruppe27 und die echten 1440/320-Orakel werden nicht gelockert. Der aktuelle Source8 enthält die separat geprüfte responsive CSS-Korrektur. Die Migration-UI ist dadurch noch nicht nativ geprüft.\n\nMock13 stammt aus dem tatsächlich korrigierten Paket5fb / Review6e357. Das neue UI-ASSETS.json bindet die sechs tatsächlichen Source8-Dateien und den direkten Parent fd587. Sowohl der native Guardian als auch der eigene Mock-Guardian pinnen dieselben aktuellen Assets und den korrigierten Mockadapter. Der normale sechsfach Issuer/Outer behält sechs eindeutige Grants; die historische Einzelfreigabe der Mock-Reparatur wird nicht als sechsfache Freigabe wiederverwendet.\n\nRoot muss vier exakte Dateien nach Task/work promoten: adapters/run-combined-findings-acceptance-v8.mjs, adapters/run-combined-findings-ui-mock-acceptance-v8.mjs, outer-small/prepare-v8-native-bindings.mjs und outer-small/run-combined-native-outer-v8.mjs. Der Issuer akzeptiert unverändert --manifest-sha ACTUAL64 --suite-proof-sha ACTUAL64 --build-proof-sha ACTUAL64 --body-helper-review ABS --body-helper-review-sha ACTUAL64. Der Helper-Review bleibt 5f02396aad562cfc155032f020c11dd6c89779c494fb5f84df8a46f7ed23e3d7 und bindet den bytegleichen Helper539c.\n\nNach tatsächlicher Source8-Suite und tatsächlichem optimiertem Build erzeugt Root frische v8-Grants plus work/v8-native-bindings-issued.json. Jede Ausführung benutzt genau --gate NAME --bindings-sha ACTUAL_ISSUED_SHA. Der Outer hält weiterhin einmal results/combined-native-outer-v4/active.lock; kein zweiter Lock im Inneren, kein paralleler Lauf, keine Installation. Die bestehenden Prozess-/Pipe-/Fsync-/Close-/Identitäts-/Ablaufzeit-Guards bleiben erhalten. FileSDK/Junction verlangen SourceManifestSha256 intern und außen identisch. Die installierte EXE wird nur mit vorher/nachher beobachteter Identität bewacht, ohne alten ab1-Pin.\n\nEOF2, Large13, BODY13 und deren Outer werden getrennt von Large vorbereitet. Ein gemeinsames Topmanifest darf später nur die tatsächlich finalen Kinder referenzieren. Frühere Source7-Erfolge und -Fehler bleiben historische Belege; insbesondere sind weder der fehlgeschlagene Browser320 noch BODY13 durch dieses Paket abgenommen. Herstellerparität bleibt nicht bewiesen.\n`;
fs.writeFileSync(path.join(O,'HANDOFF.md'),handoff,{flag:'wx'});
const final={schema_version:1,kind:'source8_native_small_preparation',status:'AUTHORED_NOT_RUN_INDEPENDENT_REVIEW_REQUIRED',immutable:true,actual_source:sourcePin,actual_source_commit:source.source_commit,source_inputs:335,required_names:540,minimum_workspace_tests:1322,binder,binder_manifest:runner,binder_review:binderReview,builder,suite,actual_suite_proof:null,actual_build_proof:null,actual_candidate:null,parent_preparations:[parentPin,mockPin],parent_reviews:[parentReview,mockReview],scopes,derivation,checks,handoff:pin(path.join(O,'HANDOFF.md')),mandatory_scopes:['physical12','native3','FileSDK175','rootjunction2','mockUI13','actualBrowser27'],global_lock:path.join(B,'results/combined-native-outer-v4/active.lock'),lock_policy:'Six small gates serialize under one Root outer lock; no duplicate inner acquire.',installed_runtime:'READ_ONLY_DYNAMIC_IDENTITY_GUARDS_NO_INSTALL',runtime_acceptance:'NOT_RUN_NO_OLD_PASS_TRANSFER',manufacturer_parity:'NOT_PROVEN'};
const manifestPin=writeFinal(path.join(O,'manifest.json'),final);
console.log(JSON.stringify({manifest:manifestPin,derivation,syntax:syntax.length,whole_regions:whole.length,scopes,execution:'AUTHOR_AND_NODE_CHECK_ONLY'}));
