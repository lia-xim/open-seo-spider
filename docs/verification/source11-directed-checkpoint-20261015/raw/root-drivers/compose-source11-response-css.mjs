import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

// Root-owned isolated composition. This does not write the product checkout.
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const D=E+'/combined-source-v11-draft-v1', R=E+'/mcp-file-tools-v1';
const H=b=>createHash('sha256').update(b).digest('hex');
const ref=p=>{const b=fs.readFileSync(p);return {path:p,sha256:H(b),bytes:b.length};};
const load=(p,s)=>{const r=ref(p);assert.equal(r.sha256,s,p);return {ref:r,value:JSON.parse(fs.readFileSync(p))};};
const [reviewPath,reviewSHA]=process.argv.slice(2);
assert.equal(process.argv.length,4);assert(path.isAbsolute(reviewPath));assert.match(reviewSHA,/^[a-f0-9]{64}$/);
assert(!fs.existsSync(D));assert(!fs.existsSync(E+'/results/combined-native-outer-v4/active.lock'));
const parent=load(E+'/combined-source-v10/manifest.json','12b238c43bd504b71e4948a8e11d4622f34589a35290ca8ef923b833464206c7');
const suite=load(E+'/results/combined-suite-v10/proof.json','3bb753190aaf64aea45367c3afa9507772fc765d19e31bbe93d7cba9c63440f5');
const fix=load(E+'/http-response-exact-length-proposal-v3/manifest.json','9f8048eadf8ff548892b2d634f6245280fee718e88d25317ba8ac7527d504d27');
const review=load(reviewPath,reviewSHA);
assert.equal(review.value.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(review.value.findings,[]);
assert.equal(review.value.proposal_manifest.sha256,fix.ref.sha256);
const css=load(E+'/settings-hint-wrap-proposal-v2/manifest.json','d29f3dfabd3a1fef20f9b877d2358ccf97d624f2360f9ebcc88e1a9a88a27b97');
const cssReview=load(E+'/settings-hint-wrap-independent-review-v2/receipt.json','dedb0e420992f60d506252f9678355950c75605deada834250704745eca785a8');
assert.equal(cssReview.value.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(cssReview.value.findings,[]);
assert.equal(cssReview.value.proposal_manifest.sha256,css.ref.sha256);
assert.equal(parent.value.files.length,369);assert.equal(fix.value.files.length,4);
const owned=[...fix.value.files.map(f=>f.path),'web/styles.css'].sort();assert.equal(new Set(owned).size,5);
assert.deepEqual(owned,['crates/app/src/server/ngram_corpus.rs','crates/app/src/server/resource_broken_sources.rs','crates/app/src/server/response_stream.rs','crates/app/src/server/response_stream_tests.rs','web/styles.css']);
const payloads=new Map(fix.value.files.map(f=>[f.path,{...f,sha256:f.after_sha256,from:fix.value.source_payload_directory+'/'+f.path}]));
for(const f of css.value.source_files)payloads.set(f.path,{...f,from:css.value.source_payload_directory+'/'+f.path});
const files=[],bindings=[];
for(const f of parent.value.files){
  const parentBytes=fs.readFileSync(E+'/combined-source-v10/'+f.path);
  for(const b of[parentBytes,fs.readFileSync(R+'/'+f.path)]){assert.equal(H(b),f.sha256,f.path);assert.equal(b.length,f.bytes,f.path);}
  const p=payloads.get(f.path),b=p?fs.readFileSync(p.from):parentBytes;
  if(p){assert.equal(H(b),p.sha256,f.path);assert.equal(b.length,p.bytes);assert.notEqual(H(b),f.sha256);if(p.before_sha256)assert.equal(p.before_sha256,f.sha256);bindings.push({path:f.path,before_sha256:f.sha256,before_bytes:f.bytes,after_sha256:H(b),bytes:b.length,proposal_path:p.from});}
  files.push({path:f.path,sha256:H(b),bytes:b.length});
}
assert.equal(bindings.length,5);assert.deepEqual(files.map(f=>f.path),files.map(f=>f.path).sort());
const testPath='crates/app/src/server/response_stream_tests.rs';
const newTests=fix.value.new_authored_full_test_names.map(full_name=>({path:testPath,name:full_name.split('::').at(-1),full_name}));assert.equal(newTests.length,4);
fs.mkdirSync(D);
for(const f of files){const b=fs.readFileSync(payloads.get(f.path)?.from??E+'/combined-source-v10/'+f.path);for(const child of['preformat','source']){const to=D+'/'+child+'/'+f.path;fs.mkdirSync(path.dirname(to),{recursive:true});fs.writeFileSync(to,b,{flag:'wx'});assert.equal(H(fs.readFileSync(to)),f.sha256);}}
const out={schema_version:1,kind:'combined_source_v11_draft_composition',status:'REVIEWED_PROPOSALS_COMPOSED_ACTUAL_FMT_TESTS_NOT_RUN',recorded_at:new Date().toISOString(),parent_manifest:parent.ref,parent_source_manifest_sha256:parent.ref.sha256,parent_source_commit:parent.value.source_commit,parent_suite:suite.ref,parent_source_inputs:369,source_inputs:369,owned_paths:5,changed_parent_inputs:5,new_input_files:0,unchanged_parent_inputs:364,source_directory:D+'/source',preformat_directory:D+'/preformat',files,owned_source_paths:owned,approved_product_bindings:bindings,response_proposal:fix.ref,response_review:review.ref,css_proposal:css.ref,css_review:cssReview.ref,new_authored_test_functions:newTests,required_names:602,qualified_names:606,minimum_expected_workspace_tests:1384,root_product_written:false,actual_format:null,actual_tests:null,actual_build:null};
fs.writeFileSync(D+'/preformat.json',JSON.stringify(out,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({result:out.status,preformat:ref(D+'/preformat.json'),source_inputs:369,changed_parent_inputs:5,new_rust_names:4}));
