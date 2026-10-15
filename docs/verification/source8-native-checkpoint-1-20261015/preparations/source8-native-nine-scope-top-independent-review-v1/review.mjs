import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const B='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const P=B+'/source8-native-nine-scope-top-preparation-v1/manifest.json';
const norm=p=>path.resolve(p).replaceAll('\\','/').toLowerCase();
const sha=b=>createHash('sha256').update(b).digest('hex');
const checked=new Map();
function pin(p){const b=fs.readFileSync(p);return {path:p,sha256:sha(b),bytes:b.length};}
function check(r){assert(r&&typeof r.path==='string');assert.match(r.sha256,/^[a-f0-9]{64}$/);const p=pin(r.path);assert.equal(p.sha256,r.sha256,r.path);if(r.bytes!==undefined)assert.equal(p.bytes,r.bytes,r.path);checked.set(norm(r.path),p);return p;}
function read(r){check(r);return JSON.parse(fs.readFileSync(r.path,'utf8'));}
function same(a,b){assert.equal(norm(a.path),norm(b.path));assert.equal(a.sha256,b.sha256);}
const proposal=pin(P);assert.equal(proposal.sha256,'bba97c5aadff40d438f584cbc101cbf1b14d38669cc78e3822fd66f8d6cf743b');
const t=read(proposal);assert.equal(t.kind,'source8_native_nine_scope_top_preparation');
for(const r of t.source_only_references)check(r);
for(const key of ['actual_source','binder','binder_manifest','binder_review','suite_producer','builder','handoff','actual_suite_proof'])check(t[key]);
assert.equal(t.runtime_imports,0);assert.equal(t.runtime_payload_copies,0);
for(const key of ['actual_build_proof','actual_candidate','actual_nine_native_results','actual_release_files','actual_root_promotion'])assert.equal(t[key],null);
assert.equal(t.manufacturer_parity,'NOT_PROVEN');
assert.equal(t.actual_source_commit,'e6b1fa2f87d0b39ac7f83aba6cdecc2110de5a50');
assert.deepEqual(t.source_counts,{inputs:335,required_names:540,min_workspace_tests:1322,owned:17,modified:8,new:9,unchanged:318});
const suite=read(t.actual_suite_proof);assert.equal(suite.result,'PASS_COMBINED_WORKSPACE_V8_REAL_RENDERER_FMT_CLIPPY');assert.equal(suite.source_manifest_sha256,t.actual_source.sha256);assert.equal(suite.driver_sha256,t.suite_producer.sha256);assert.equal(suite.source_inputs,335);
const child=t.children.map(x=>({role:x.role,pin:check(x.preparation),value:read(x.preparation)}));assert.deepEqual(child.map(x=>x.role),['small6','eof_large_body']);
for(const c of child){same(c.value.actual_source??c.value.source_manifest,t.actual_source);assert.equal(c.value.binder.sha256,t.binder.sha256);}
const late={
large_eof_body_package:pin(B+'/source8-native-large-eof-body-independent-review-v2/receipt.json'),
body_direct:pin(B+'/source8-native-large-eof-body-independent-review-v2/body-receipt.json'),
body_outer_direct:pin(B+'/source8-native-large-eof-body-independent-review-v2/outer-receipt.json')};
assert.equal(late.large_eof_body_package.sha256,'d900849548cae2c9746ab88451d01eb5d6147e3973226db7d47f18d7ec74dfee');
assert.equal(late.body_direct.sha256,'8119b719ee3bca266b2b75f93695a5b95ea9c42bae71e8aabaf636d001f4a34a');
assert.equal(late.body_outer_direct.sha256,'eceef1942aabb69bb69c6fee1a6dfea722cf6cf42190fbfe1603d833237c386b');
const directReviews={};
for(const [role,c] of Object.entries(t.review_contracts)){
  if(role==='top_integration'){assert.equal(c.review,null);continue;}
  const r=c.review??late[role];assert(r,'missing actual review '+role);const actual=read(r);assert.equal(actual.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(actual.findings,[]);
  if(c.proposal_manifest)same(actual.proposal_manifest,c.proposal_manifest);
  if(c.harness)same(actual.harness,c.harness);if(c.outer)same(actual.outer,c.outer);if(c.kind)assert.equal(actual.kind,c.kind);
  directReviews[role]=check(r);
}
const scopes=['native','ui-mock','browser-native','physical-findings','file-sdk','root-junction','eof','large-source-v8','source-body-v8'];
assert.equal(t.required_scope_count,9);assert.deepEqual(t.scopes.map(x=>x.scope),scopes);assert.deepEqual(t.scopes.map(x=>x.required_groups),[3,13,27,12,175,2,2,13,13]);
assert.equal(new Set(t.scopes.map(x=>norm(x.expected_inner_result))).size,9);
const manifestRows=[];
for(const c of child){for(const s of c.value.scopes??[])manifestRows.push(s.manifest??s.preparation);}
const parentSmall=child[0].value;
for(const s of t.scopes){
  check(s.child_preparation);if(s.outer_preparation)check(s.outer_preparation);assert.equal(s.actual_native_proof,null);
  const cm=read(s.child_preparation);const local=norm(s.harness.path).includes('/work/')?t.root_promotion_list.find(p=>norm(p.target)===norm(s.harness.path))?.source:s.harness;assert(local,s.scope);check({...local,sha256:s.harness.sha256});
  if(scopes.indexOf(s.scope)>=6)assert(manifestRows.some(x=>norm(x.path)===norm(s.child_preparation.path)&&x.sha256===s.child_preparation.sha256),s.scope+' must use actual child manifest path');
  if(s.scope==='eof'){assert.equal(s.root_wrapper,null);assert.equal(s.expected_outer_proof,null);assert.match(s.execution_lock_owner,/harness itself/);}
  else assert(s.root_wrapper&&s.expected_outer_proof);
}
assert.equal(t.root_promotion_list.length,7);assert.equal(new Set(t.root_promotion_list.map(x=>norm(x.target))).size,7);
for(const p of t.root_promotion_list){check(p.source);assert.equal(path.basename(p.target),path.basename(p.source.path));assert(norm(p.target).startsWith(norm('C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/work')+'/'));}
const small=t.scopes.slice(0,6);assert.deepEqual(t.release_contracts.small6_issuer.grants,small.map(x=>x.release_path));assert.equal(new Set(small.map(x=>x.root_wrapper)).size,1);
for(const s of small){assert.deepEqual(s.root_wrapper_cli,['--gate',s.scope,'--bindings-sha','<actual issued receipt SHA>']);assert.equal(s.root_issued_receipt,t.release_contracts.small6_issuer.issued_receipt);}
assert.equal(t.release_contracts.small6_issuer.cli.length,10);assert.equal(t.release_contracts.large_issuer.cli.length,16);assert.equal(t.release_contracts.large_issuer.lifetime_max_hours,2);
const large=t.scopes[7],body=t.scopes[8],eof=t.scopes[6];
assert.equal(t.release_contracts.large_issuer.release,large.release_path);assert.equal(t.release_contracts.large_issuer.cli[11],large.outer_preparation.sha256);
assert.deepEqual(large.root_wrapper_cli,['--release-sha','<actual Large grant SHA>','--preparation-sha',large.outer_preparation.sha256]);
assert.deepEqual(body.root_wrapper_cli,['--release-sha','<actual BODY grant SHA>','--preparation-sha',body.outer_preparation.sha256]);
assert.equal(t.release_contracts.body.output,path.dirname(body.expected_inner_result).replaceAll('\\','/'));assert.equal(t.release_contracts.eof.output,path.dirname(eof.expected_inner_result).replaceAll('\\','/'));
const contract=read(t.release_contracts.body.contract);assert.equal(contract.actual_root_release_required.kind,t.release_contracts.body.kind);assert.equal(contract.actual_root_release_required.scope,t.release_contracts.body.scope);assert.equal(contract.actual_root_release_required.deadline_ms,t.release_contracts.body.deadline_ms);assert.equal(norm(contract.actual_root_release_required.output),norm(t.release_contracts.body.output));
const lock=B+'/results/combined-native-outer-v4/active.lock';assert.equal(t.shared_global_lock,lock);assert.equal(contract.outer_lock_required.path,lock);assert.equal(contract.outer_lock_required.gate,body.scope);
assert.equal(t.additional_migration_acceptance.included_in_nine_scope_coverage,false);assert.equal(t.additional_migration_acceptance.groups,8);assert.equal(t.additional_migration_acceptance.actual_runtime,null);
for(const key of ['preparation','review','outer_preparation','outer_review'])check(t.additional_migration_acceptance[key]);
const summary={schema_version:1,proposal_manifest:proposal,checks:{actual_references:checked.size,scope_count:9,promoted_script_count:7,child_review_binding:'actual Small6 and current package/BODY/outer BODY direct reviews, inherited unchanged EOF/Large/outer Large',source_manifest_and_suite_checked:true,future_build_candidate_and_native_not_invented:true,metadata_only_no_payload_copies:true,root_cli_release_outputs_and_lock_crosschecked:true,full_child_reviews_not_duplicated:true},current_review_bindings:directReviews,source_only_references:[...checked.values()],runtime_imports:0,runtime_calls:0,compiler_calls:0,root_writes:0};
if(process.argv.includes('--freeze')){
 const out=B+'/source8-native-nine-scope-top-independent-review-v1';assert(!fs.existsSync(out));fs.mkdirSync(out);const checks=Buffer.from(JSON.stringify(summary,null,2)+'\n');fs.writeFileSync(out+'/checks.json',checks,{flag:'wx'});
 const receipt={schema_version:1,kind:'source8_native_nine_scope_top_independent_review',result:'NO_FINDINGS_BOUNDED_SOURCE_REVIEW',findings:[],proposal_manifest:proposal,actual_source:t.actual_source,actual_source_commit:t.actual_source_commit,actual_suite_proof:t.actual_suite_proof,child_preparations:t.children,current_child_review_bindings:directReviews,checks:pin(out+'/checks.json'),scope:'Metadata integration only: all nine scopes, seven exact Root script promotions, actual final child paths/reviews, current CLI/grants/outputs and single shared native lock; no duplicated child source review.',qualification:'The three null current package/BODY/BODY-outer receipt slots in the immutable Top are completed by explicit actual review references in this receipt. Actual optimized build/candidate, grants and native proofs remain required Root runtime inputs. Source7 failed evidence is not acceptance. Migration is an additional separate eight-group scope.',actual_build_proof:null,actual_candidate:null,actual_native_acceptance:'NOT_RUN',runtime_imports:0,runtime_calls:0,compiler_calls:0,root_writes:0};fs.writeFileSync(out+'/receipt.json',JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});fs.copyFileSync(import.meta.filename,out+'/review.mjs',fs.constants.COPYFILE_EXCL);console.log(JSON.stringify({receipt:pin(out+'/receipt.json'),checks:summary.checks}));
}else console.log(JSON.stringify(summary.checks));
