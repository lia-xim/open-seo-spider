import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013',R=E+'/mcp-file-tools-v1',D=E+'/combined-source-v11-draft-v1';
const P=E+'/combined-v11-exact-length-driver-preparation-v2',A=E+'/combined-source-v11-approval-v1',O=E+'/combined-source-v11';
const H=b=>createHash('sha256').update(b).digest('hex'),ref=p=>{const b=fs.readFileSync(p);return {path:p,sha256:H(b),bytes:b.length};};
const [reviewPath,reviewSHA]=process.argv.slice(2);assert.equal(process.argv.length,4);assert(path.isAbsolute(reviewPath));assert.match(reviewSHA,/^[a-f0-9]{64}$/);
assert(!fs.existsSync(A)&&!fs.existsSync(O));assert(!fs.existsSync(E+'/results/combined-native-outer-v4/active.lock'));
const pr=ref(P+'/manifest.json');assert.equal(pr.sha256,'f24fce6c565664bac372ab01aaeefb89f8880883b0967cab883fd72c1d4a055e');const prep=JSON.parse(fs.readFileSync(pr.path));
const rr=ref(reviewPath);assert.equal(rr.sha256,reviewSHA);const review=JSON.parse(fs.readFileSync(rr.path));assert.equal(review.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(review.findings,[]);assert.equal(review.proposal_manifest.sha256,pr.sha256);
const promoted=[];
for(const f of prep.files){const raw=fs.readFileSync(P+'/'+f.path);assert.equal(H(raw),f.sha256);assert.equal(raw.length,f.bytes);if(f.path!=='combined-bindings.mjs'){const to=path.resolve('work/'+f.path);assert(!fs.existsSync(to));fs.writeFileSync(to,raw,{flag:'wx'});assert.equal(H(fs.readFileSync(to)),f.sha256);promoted.push(ref(to));}}
const pub=ref(D+'/private-publication.json');assert.equal(pub.sha256,'acfb858bc07e47de8ea49f718439c46c3369e36066d132da32d6bda07769201c');const publication=JSON.parse(fs.readFileSync(pub.path));assert.equal(publication.source_commit,'61b0e3bca5386a17b449915b2915222e245084c5');
assert.equal(execFileSync('C:/Program Files/Git/cmd/git.exe',['rev-parse','HEAD'],{cwd:R,encoding:'utf8',windowsHide:true}).trim(),publication.source_commit);
const approval=JSON.parse(fs.readFileSync(P+'/ROOT-APPROVAL-TEMPLATE.NOT-ACTUAL.json'));approval.source_commit=publication.source_commit;approval.private_repository_verified=true;approval.private_publication=pub;approval.recorded_at=new Date().toISOString();
for(const [key,filename] of Object.entries({directed_tests:'proof.json',directed_tests_log:'response-stream.log',new_test_names:'actual-new-test-names.json',response_stream_test_names:'actual-response-stream-test-names.json'}))approval[key]=ref(E+'/results/source11-response-stream-tests-v1/'+filename);
assert.equal(approval.directed_tests.sha256,'7a3a4527f1f3372337a4d8b556088fd535521e75af4e6dbd46d9dc388e030a96');assert.equal(approval.source_files.length,369);
for(const f of approval.source_files){const raw=fs.readFileSync(R+'/'+f.path);assert.equal(H(raw),f.sha256,f.path);assert.equal(raw.length,f.bytes);}
fs.mkdirSync(A);const approvalPath=A+'/root-approval.json';fs.writeFileSync(approvalPath,JSON.stringify(approval,null,2)+'\n',{flag:'wx'});const ar=ref(approvalPath);
const validatorPath=path.resolve('work/validate-combined-source-v11-r2.mjs'),validator=prep.files.find(f=>f.path===path.basename(validatorPath));assert.equal(H(fs.readFileSync(validatorPath)),validator.sha256);
const {createSource11Manifest,validateSource11Manifest}=await import(pathToFileURL(validatorPath));assert.equal(H(fs.readFileSync(validatorPath)),validator.sha256);
const manifest=createSource11Manifest({approvalPath,approvalSha:ar.sha256,expectedCommit:publication.source_commit,repo:R});
assert.equal(manifest.files.length,369);assert.equal(manifest.changed_parent_inputs,5);assert.equal(manifest.new_input_files,0);assert.equal(manifest.unchanged_parent_inputs,364);assert.equal(manifest.required_test_names,602);assert.equal(manifest.minimum_expected_workspace_tests,1384);assert.equal(manifest.all_required_full_test_names.length,606);
fs.mkdirSync(O);for(const f of manifest.files){const raw=fs.readFileSync(R+'/'+f.path);assert.equal(H(raw),f.sha256);const to=O+'/'+f.path;fs.mkdirSync(path.dirname(to),{recursive:true});fs.writeFileSync(to,raw,{flag:'wx'});assert.equal(H(fs.readFileSync(to)),f.sha256);}
fs.writeFileSync(O+'/manifest.json',JSON.stringify(manifest,null,2)+'\n',{flag:'wx'});const validated=validateSource11Manifest({manifest,sourceDirectory:O,repo:R});assert.equal(validated.files.size,369);
const receipt={kind:'actual_root_response_css_source11_freeze',recorded_at:new Date().toISOString(),result:'FROZEN_ACTUAL_369_INPUTS_CURRENT_SUITE_BUILD_NATIVE_NOT_RUN',source_commit:publication.source_commit,manifest:ref(O+'/manifest.json'),approval:ar,publication:pub,preparation:pr,review:rr,source_inputs:369,changed_parent_inputs:5,unchanged_parent_inputs:364,new_input_files:0,new_rust_tests:4,required_test_names:602,qualified_names:606,minimum_expected_workspace_tests:1384,all_root_and_frozen_source_hashes_verified:true,promoted_runtime_drivers:promoted,source11_suite:null,source11_build:null,source11_native:null,installed:'UNCHANGED_SOURCE5_R3'};
fs.writeFileSync(A+'/freeze-receipt.json',JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(receipt));
