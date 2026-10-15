import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013',D=E+'/combined-source-v9-draft-v6',S=E+'/combined-source-v9',O=E+'/results/source9-new-feature-tests-v2',H=b=>createHash('sha256').update(b).digest('hex');
assert(!fs.existsSync(O));assert(!fs.existsSync(E+'/results/combined-native-outer-v4/active.lock'));
const copyRaw=fs.readFileSync(D+'/canonical-source-copy.json'),copy=JSON.parse(copyRaw),preRaw=fs.readFileSync(D+'/preformat.json'),pre=JSON.parse(preRaw);
assert.equal(H(preRaw),'ade29eed3c3be14028e0b099e33e38b9020de394c0b3b6986485a5a4f1d77367');assert.equal(copy.files.length,358);assert.equal(pre.new_authored_test_functions.length,37);
function verify(){assert.equal(H(fs.readFileSync(D+'/canonical-source-copy.json')),H(copyRaw));for(const f of copy.files){for(const root of [S,E+'/mcp-file-tools-v1']){const b=fs.readFileSync(root+'/'+f.path);assert.equal(H(b),f.sha256,f.path);assert.equal(b.length,f.bytes)}}}
verify();fs.mkdirSync(O);fs.mkdirSync(O+'/temporary');
const env={...process.env,CARGO_TARGET_DIR:E+'/cargo-target',CARGO_INCREMENTAL:'0',CARGO_TERM_COLOR:'never',CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0',TEMP:O+'/temporary',TMP:O+'/temporary'};delete env.OPEN_SEO_SPIDER_RENDERER;
for(const [key,value]of Object.entries(process.env))if(/^(RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|RUSTDOCFLAGS|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTC)$/.test(key))assert(!value,'Unexpected compiler override '+key);
const runs=[],failures=[];let sourceFinal=false;
try{
 for(const filter of ['broken_sources','directory_tree']){
  const args=['test','--workspace','--locked',filter,'--','--include-ignored','--test-threads=1'],file=O+'/'+filter+'.log',fd=fs.openSync(file,'wx');let error=null,exit;const started=performance.now(),child=spawn('C:/Users/matth/.cargo/bin/cargo.exe',args,{cwd:S,env,windowsHide:true,stdio:['ignore',fd,fd]});child.once('error',e=>error=String(e));
  try{exit=await new Promise(r=>child.once('close',(code,signal)=>r({code,signal})))}finally{fs.fsyncSync(fd);fs.closeSync(fd)}
  const bytes=fs.readFileSync(file),text=bytes.toString('utf8'),summaries=[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(x=>({state:x[1],passed:+x[2],failed:+x[3],ignored:+x[4]}));
  const row={filter,args,pid:child.pid,actual_close_event:true,exit,spawn_error:error,elapsed_ms:performance.now()-started,log_sha256:H(bytes),log_bytes:bytes.length,summaries};runs.push(row);fs.writeFileSync(O+'/'+filter+'.json',JSON.stringify(row,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({filter,exit,summaries}));assert.equal(error,null);assert.deepEqual(exit,{code:0,signal:null});assert(summaries.length>0);assert(summaries.every(x=>x.state==='ok'&&x.failed===0&&x.ignored===0));verify();
 }
 const text=runs.map(r=>fs.readFileSync(O+'/'+r.filter+'.log','utf8')).join('\n'),matches=[...text.matchAll(/^test ([^\r\n]+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$/gm)];
 const names=pre.new_authored_test_functions.map(t=>{const seen=matches.filter(m=>m[1].split('::').at(-1)===t.name);assert.equal(seen.length,1,'Missing/duplicate actual new test '+t.name);assert.equal(seen[0][2],'ok',t.name);return{required:t.name,actual:seen[0][1],status:'ok'}});assert.equal(names.length,37);fs.writeFileSync(O+'/actual-new-test-names.json',JSON.stringify(names,null,2)+'\n',{flag:'wx'});
}catch(e){failures.push({message:e.message,stack:e.stack});process.exitCode=1}
finally{try{verify();sourceFinal=true}catch(e){failures.push({message:e.message});process.exitCode=1}const receipt={kind:'source9_actual_new_feature_tests_only',recorded_at:new Date().toISOString(),result:failures.length?'FAIL_NEW_FEATURE_TESTS':'PASS_37_NEW_FEATURE_TESTS_ONLY',source_commit:copy.source_commit,source_directory:S,source_inputs:358,source_copy_sha256:H(copyRaw),preformat_sha256:H(preRaw),runs,failures,source_final_verified:sourceFinal,qualification:'Directed new-feature tests only. No complete workspace/renderer/fmt/clippy run, frozen Source9 admission, optimized candidate, native browser or installation PASS.'};fs.writeFileSync(O+'/proof.json',JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:receipt.result,out:O,proof_sha256:H(fs.readFileSync(O+'/proof.json'))}));}
