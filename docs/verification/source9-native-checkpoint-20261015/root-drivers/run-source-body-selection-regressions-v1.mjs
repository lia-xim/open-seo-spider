import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import {spawn} from 'node:child_process';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013',node='C:/Program Files/nodejs/node.exe',proposal=E+'/source-body-selection-refresh-proposal-v1';
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const ref=p=>{const b=fs.readFileSync(p);return{path:p,sha256:sha(b),bytes:b.length}};
const mf=ref(proposal+'/manifest.json');assert.equal(mf.sha256,'f8181168e2c67aaab053eb1257fa27d119a79760d0c4c11ecd7adeb7bfc93606');
const m=JSON.parse(fs.readFileSync(mf.path));assert.equal(ref(m.regression.path).sha256,m.regression.sha256);
const review=ref(E+'/source-body-selection-refresh-independent-review-v1/receipt.json');assert.equal(review.sha256,'73a2fb41187125ae72c22fe88322da9833b5d0e2aedfb1a96cd3ff09720ac1fb');
const root=E+'/mcp-file-tools-v1/web/app.js',fixed=proposal+'/source/web/app.js';assert.equal(ref(root).sha256,m.files[0].before_sha256);assert.equal(ref(fixed).sha256,m.files[0].after_sha256);
const out=E+'/results/source-body-selection-regressions-v2';assert(!fs.existsSync(out));fs.mkdirSync(out,{recursive:true});
const results=[];
for(const [role,app,want] of [['actual_source9_baseline',root,1],['reviewed_one_line_proposal',fixed,0]]){
 const before=ref(app),log=out+'/'+role+'.log',fd=fs.openSync(log,'wx');const started_at=new Date().toISOString();
 const child=spawn(node,['--test-reporter=tap',m.regression.path,'--app',app,'--sha256',before.sha256],{stdio:['ignore',fd,fd],windowsHide:true});
 const close=await new Promise((resolve,reject)=>{child.on('error',reject);child.on('close',(code,signal)=>resolve({code,signal,closed_at:new Date().toISOString()}));});fs.closeSync(fd);
 assert.deepEqual(ref(app),before);const text=fs.readFileSync(log,'utf8');const passed=Number(text.match(/# pass (\d+)/)?.[1]),failed=Number(text.match(/# fail (\d+)/)?.[1]);assert.equal(passed+failed,3);assert.equal(close.code,want);assert.equal(close.signal,null);if(want===0){assert.equal(passed,3);assert.equal(failed,0);}else{assert(failed>=2);assert(text.includes('source_body_selection_null_to_first_archive_enables_refresh_without_document_change'));assert(text.includes('source_body_selection_clear_disables_refresh_and_discards_old_page'));}
 const row={role,source:before,started_at,pid:child.pid,close,passed,failed,log:ref(log)};results.push(row);process.stdout.write(JSON.stringify(row)+'\n');
}
const proof={schema_version:1,kind:'source_body_selection_actual_regressions_v1',status:'PASS_BASELINE_DETECTED_AND_REVIEWED_FIX_PASSES_ALL_THREE',recorded_at:new Date().toISOString(),proposal:mf,independent_review:review,harness:ref(m.regression.path),node:ref(node),results,product_root_unchanged:ref(root),cargo_tests_added:0,compiled_browser_rerun:null};fs.writeFileSync(out+'/proof.json',JSON.stringify(proof,null,2)+'\n');process.stdout.write(JSON.stringify(ref(out+'/proof.json'))+'\n');
