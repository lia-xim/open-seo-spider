import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',task=process.cwd(),repo=path.join(base,'mcp-file-tools-v1');
const sha=b=>createHash('sha256').update(b).digest('hex');
const sourceSha='01bfa8efcb6745c5521be5e533ae63f2f15679318b095a29eb3ab925fef95bdf';
const binarySha='22f8c602f0203c1be946882c3f83cee370dc3f354f5bf678cbfff4c40ecd7abd';
const buildSha='32f7d2bb3edeb5a6d27d34369bf7b7fb909187ab5ac5981a0c0a4d4ab1187e4e';
const binderPath=path.join(base,'native-combined-rebind-v3/combined-bindings.mjs');
assert.equal(sha(fs.readFileSync(binderPath)),'69501b4ad1d2b0b7bef044ff8eafd6e047541f6e4088544d9f63a8f8d275a45a');
const {bindCombined}=await import(pathToFileURL(binderPath).href);
const bound=bindCombined({build_proof_sha256:buildSha,candidate_binary_sha256:binarySha},sourceSha);bound.verify();
const rows=new Map();
function add(source,target,expected=null,bytes=null){
 assert(!path.isAbsolute(target)&&!target.split(/[\\/]/).includes('..'));
 const stat=fs.lstatSync(source);assert(stat.isFile()&&!stat.isSymbolicLink()&&stat.size<=8*1024*1024);
 const raw=fs.readFileSync(source),hash=sha(raw);if(expected)assert.equal(hash,expected,source);if(bytes!==null)assert.equal(raw.length,bytes,source);
 if(rows.has(target)){const old=rows.get(target);assert.equal(old.sha256,hash);assert.equal(old.bytes,raw.length);return;}
 rows.set(target,{source,path:target,sha256:hash,bytes:raw.length,raw});
}
for(const [dir,pin]of [['completed-native-first-three-review-v1','0eb17265e47428b561b06e24ded19a3f6bb5d9204357ef21640d3a1a6a19cc89'],['completed-ui-browser-review-v1','aa4055de9915a6927e40ebede1fbb75bea2c8eab1d990c2f66c938ec402a33b7']]){
 const file=path.join(base,dir,'archive-sources.json'),raw=fs.readFileSync(file);assert.equal(sha(raw),pin);const record=JSON.parse(raw);assert.equal(record.source_manifest_sha256,sourceSha);
 for(const r of record.records)add(r.source,r.path,r.sha256,r.bytes);
 add(file,'reviews/'+dir+'-archive-sources.json',pin);add(path.join(base,dir,'receipt.json'),'reviews/'+dir+'-receipt.json');
}
const physicalRoot=path.join(base,'physical-combined-rebind-v3/runs/optimized-v3-acceptance');
const physicalRaw=fs.readFileSync(path.join(physicalRoot,'proof.json'));assert.equal(sha(physicalRaw),'c7a7b216e0ddbf781cf69431069655a07732147ddde120af1d29f60b29437ca2');
const physical=JSON.parse(physicalRaw);assert.equal(physical.result,'FAIL_COMBINED_V3_NATIVE_FINDING_PHYSICAL_GATE');assert.equal(physical.cases.length,0);assert.equal(physical.baselines.length,1);assert(physical.owned_children_closed&&physical.owned_fixture_drained);
for(const name of ['proof.json','inputs.json','baseline-csv.json','baseline-csv.stdout.log','baseline-csv.stderr.log','csv-01-base.fixture.json','csv-01-base.stdout.log','csv-01-base.stderr.log'])add(path.join(physicalRoot,name),'physical-failed/'+name);
const partialRowsPath=path.join(physicalRoot,'csv-01-base/actual-native-rows.ndjson'),partialRowsRaw=fs.readFileSync(partialRowsPath);let partialRows=0;for(const value of partialRowsRaw)if(value===10)partialRows++;assert.equal(partialRows,140);
add(path.join(task,'work/physical-v3-root-cancellation.json'),'physical-failed/external-root-abort.json');
for(const gate of ['native','file-sdk','root-junction','ui-mock','browser-native','physical-findings']){
 const dir=path.join(base,'results/combined-native-outer-v3-r2',gate),raw=fs.readFileSync(path.join(dir,'proof.json')),p=JSON.parse(raw);
 assert(p.actual_close_event&&p.actual_exit_event&&p.spawn_event&&p.lock_released);assert.equal(p.spawn_error,null);assert(p.stdout.drained&&p.stderr.drained);assert.equal(p.exit.signal,null);
 assert.equal(p.driver_sha256,'3774cdda8ae2f9f13c2603d23d4d8296d79f66f861867b0723d699831398b78f');
 if(gate==='physical-findings'){assert.equal(p.result,'FAIL_OUTER_CHILD');assert.equal(p.exit.code,1);}else{assert.equal(p.result,'PASS_OUTER_CHILD_EXIT_NOT_INNER_ACCEPTANCE');assert.equal(p.exit.code,0);assert.equal(p.failures.length,0);}
 for(const [name,expected]of [['proof.json',null],['stdout.log',p.stdout.sha256],['stderr.log',p.stderr.sha256]])add(path.join(dir,name),'outer/'+gate+'/'+name,expected);
}
for(const name of ['proof.json','cargo-version.json','cargo-version.log','rustc-version.json','rustc-version.log','build.json','build.log'])add(path.join(base,'results/combined-optimized-build-v3',name),'build/'+name,name==='proof.json'?buildSha:null);
for(const name of ['prepare-v3-native-bindings-r2.mjs','v3-native-renewal-r2-derivation.json','run-combined-native-outer-v3-r2.mjs','v3-native-outer-r2-derivation.json'])add(path.join(task,'work',name),'inputs/'+name);
add(binderPath,'inputs/combined-bindings.mjs','69501b4ad1d2b0b7bef044ff8eafd6e047541f6e4088544d9f63a8f8d275a45a');
add(path.join(base,'native-operational-renewal-r2-review-v1/receipt.json'),'reviews/renewal-r2-review.json','d60a43c6afb37cf025f86be9dad2dfd608586a884beee3f597dd296d7c99548f');
add(path.join(task,'work/build-combined-optimized-v3.mjs'),'inputs/build-combined-optimized-v3.mjs','878c981a1645205f2ba958775ca318b95abbde62b6dda226ff55dcf2de82f983');
const relative='docs/verification/combined-optimized-native-v3-20261015',out=path.join(repo,relative);assert(!fs.existsSync(out));
for(const critical of ['docs/feature-parity.json','docs/FEATURE_TODO.md','docs/ROADMAP.md','.gitattributes'])assert(!bound.manifest.files.some(f=>f.path===critical));
fs.mkdirSync(out);let total=0;for(const row of rows.values()){total+=row.bytes;assert(total<=16*1024*1024);const dest=path.join(out,row.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.writeFileSync(dest,row.raw,{flag:'wx'});assert.equal(sha(fs.readFileSync(dest)),row.sha256);}
bound.verify();
const installed=path.join(task,'outputs/open-seo-spider/target/release/open-seo-spider.exe');assert.equal(sha(fs.readFileSync(installed)),'ab1ac93c82905aa627d100d8622a50acf93dd7fe59c3f854eecdaac667110bcf');
const index={kind:'root_archived_combined_optimized_and_native_v3',recorded_at:new Date().toISOString(),result:'FIVE_NATIVE_SCOPES_PASS_PHYSICAL_FAILED_EXTERNAL_ROOT_ABORT_NOT_INSTALL_ELIGIBLE',source_commit:bound.manifest.source_commit,source_manifest_sha256:sourceSha,suite:{result:bound.suite.result,passed:1203,required_names:421,renderer_gates:4},build:{result:bound.build.result,proof_sha256:buildSha,binary:bound.build.binary},scopes:{native_api_cli_sdk:'PASS_DIRECTED_SMALL_REAL_FIXTURE',file_sdk:'PASS_175_OPERATION_ASSERTIONS_60_SUCCESS_115_EXPECTED_REFUSALS',root_junction:'PASS_2_NATURAL_REJECTIONS',ui_mock:'PASS_13_MOCK_GROUPS',browser_native:'PASS_25_REAL_GROUPS',physical_findings:'NOT_PASSED_0_OF_12_CASES_EXTERNAL_ROOT_ABORT'},physical_observation:{raw_failure:'fetch failed',external_root_abort:true,reason:'Large finding collection remained running after about524seconds;140of405rows were actually preserved. Open-writer NTFS directory metadata had misleadingly shown zero length; the no-first-row inference is withdrawn. Source-confirmed per-page whole-report JSON parsing requires correction. No natural timeout or graceful-drain attribution is asserted.',retained_partial_rows:{path:partialRowsPath,bytes:partialRowsRaw.length,sha256:sha(partialRowsRaw),complete_rows:partialRows,expected_rows:405},owned_apps_closed:true,fixture_drained:true},installed_runtime:'UNCHANGED_AB1',installation:'NOT_RUN_NOT_ELIGIBLE',feature_counts:{implemented:30,partial:68,planned:46},remaining:['Repair quadratic report-wide per-page SQL JSON projection and rerun identical physical oracles on fresh accepted source','Genuine50000URL complete durable membership positive; current32MiB serialized ledger capacity gap','Actual50MiB source processing/retention/unknown matrix','HTTP in-flight graceful drain,Unix/Linux/container,remaining corpus/recovery/resource gates','Data-preserving installation after all matching candidate gates pass','All remaining catalog/filter/workflow acceptance and manufacturer parity'],manufacturer_parity:'NOT_PROVEN',qualification:'Private immutable receipt/log archive, not vendor equivalence. Old frozen NOT_RUN metadata and failed historical attempts remain unchanged. Controlled physical warning facts are synthetic, never original website observations. No candidate executable, user data, SQL databases, wire bodies or screenshots copied. Archive hashes establish copied-byte integrity; they do not retroactively authenticate an unpinned producer.',records:[...rows.values()].map(({raw,...r})=>r)};
const raw=Buffer.from(JSON.stringify(index,null,2)+'\n');fs.writeFileSync(path.join(out,'index.json'),raw,{flag:'wx'});
const attr=path.join(repo,'.gitattributes'),line=relative+'/** -text',old=fs.readFileSync(attr,'utf8');assert(!old.includes(line));fs.writeFileSync(attr,old.trimEnd()+'\n'+line+'\n');
const receipt={kind:'root_archive_combined_native_v3',path:relative+'/index.json',sha256:sha(raw),records:rows.size,bytes:total,result:index.result};fs.writeFileSync(path.join(task,'work/combined-native-v3-archive.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(receipt));
