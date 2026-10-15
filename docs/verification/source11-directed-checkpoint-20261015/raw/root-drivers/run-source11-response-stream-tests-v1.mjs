import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const D=E+'/combined-source-v11-draft-v1', S=D+'/source', O=E+'/results/source11-response-stream-tests-v1';
const H=b=>createHash('sha256').update(b).digest('hex');
const [preSHA,clSHA]=process.argv.slice(2);assert.equal(process.argv.length,4);
for(const s of[preSHA,clSHA])assert.match(s,/^[a-f0-9]{64}$/);
assert(!fs.existsSync(O));assert(!fs.existsSync(E+'/results/combined-native-outer-v4/active.lock'));
const raw=fs.readFileSync(D+'/preformat.json'),pre=JSON.parse(raw),clRaw=fs.readFileSync(D+'/actual-preliminary-clippy.json'),cl=JSON.parse(clRaw);
assert.equal(H(raw),preSHA);assert.equal(H(clRaw),clSHA);assert.equal(cl.result,'PASS_PRELIMINARY_CLIPPY_ONLY');assert.equal(cl.files.length,369);assert.equal(pre.new_authored_test_functions.length,4);
const oldText=fs.readFileSync(E+'/combined-source-v10/crates/app/src/server/response_stream_tests.rs','utf8');
const oldNames=[...oldText.matchAll(/^async fn (response_[a-z0-9_]+)\(/gm)].map(m=>'server::response_stream::tests::'+m[1]);assert.equal(oldNames.length,16);
const expected=[...oldNames,...pre.new_authored_test_functions.map(t=>t.full_name)].sort();assert.equal(new Set(expected).size,20);
function verify(){assert.equal(H(fs.readFileSync(D+'/preformat.json')),preSHA);assert.equal(H(fs.readFileSync(D+'/actual-preliminary-clippy.json')),clSHA);for(const f of cl.files){const b=fs.readFileSync(S+'/'+f.path);assert.equal(H(b),f.sha256,f.path);assert.equal(b.length,f.bytes);}}
verify();fs.mkdirSync(O);fs.mkdirSync(O+'/temporary');
const env={...process.env,CARGO_TARGET_DIR:E+'/cargo-target',CARGO_INCREMENTAL:'0',CARGO_TERM_COLOR:'never',CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0',TEMP:O+'/temporary',TMP:O+'/temporary'};
delete env.OPEN_SEO_SPIDER_RENDERER;
for(const[k,v]of Object.entries(process.env))if(/^(RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|RUSTDOCFLAGS|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTC)$/.test(k))assert(!v,k);
const runs=[],failures=[];let sourceFinal=false,names=[],newNames=[];
try{
 const filter='server::response_stream::tests',args=['test','-p','open-seo-spider','--locked',filter,'--','--include-ignored','--test-threads=1'];
 const log=O+'/response-stream.log',fd=fs.openSync(log,'wx');let error=null,exit;
 const start=performance.now(),child=spawn('C:/Users/matth/.cargo/bin/cargo.exe',args,{cwd:S,env,windowsHide:true,stdio:['ignore',fd,fd]});
 child.once('error',e=>error=String(e));try{exit=await new Promise(r=>child.once('close',(code,signal)=>r({code,signal})));}finally{fs.fsyncSync(fd);fs.closeSync(fd);}
 const b=fs.readFileSync(log),t=b.toString('utf8'),summaries=[...t.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(x=>({state:x[1],passed:+x[2],failed:+x[3],ignored:+x[4]}));
 const row={filter,args,pid:child.pid,actual_close_event:true,exit,spawn_error:error,elapsed_ms:performance.now()-start,log_sha256:H(b),log_bytes:b.length,summaries};runs.push(row);fs.writeFileSync(O+'/response-stream.json',JSON.stringify(row,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({filter,exit,summaries}));
 assert.equal(error,null);assert.deepEqual(exit,{code:0,signal:null});assert(summaries.length>0);assert(summaries.every(x=>x.state==='ok'&&x.failed===0&&x.ignored===0));
 const matches=[...t.matchAll(/^test ([^\r\n]+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$/gm)];
 assert.deepEqual(matches.map(m=>m[1]).sort(),expected);
 names=expected.map(full=>{const seen=matches.filter(m=>m[1]===full);assert.equal(seen.length,1);assert.equal(seen[0][2],'ok');return {required:full.split('::').at(-1),actual:full,status:'ok'};});
 newNames=names.filter(t=>!oldNames.includes(t.actual));assert.equal(newNames.length,4);
 fs.writeFileSync(O+'/actual-response-stream-test-names.json',JSON.stringify(names,null,2)+'\n',{flag:'wx'});
 fs.writeFileSync(O+'/actual-new-test-names.json',JSON.stringify(newNames,null,2)+'\n',{flag:'wx'});
}catch(e){failures.push({message:e.message,stack:e.stack});process.exitCode=1;}
finally{
 try{verify();sourceFinal=true;}catch(e){failures.push({message:e.message});process.exitCode=1;}
 const proof={kind:'source11_actual_response_stream_tests_only',recorded_at:new Date().toISOString(),result:failures.length?'FAIL_RESPONSE_STREAM_TESTS':'PASS_20_RESPONSE_STREAM_TEST_NAMES_ONLY',source_directory:S,source_inputs:369,preformat_sha256:preSHA,preliminary_clippy_sha256:clSHA,runs,old_names_verified:names.filter(t=>oldNames.includes(t.actual)).length,new_names_verified:newNames.length,failures,source_final_verified:sourceFinal,qualification:'Directed isolated old and new response lifecycle tests with genuine local HTTP. No full workspace/renderer, product publication, optimized build, native browser or installation proof.'};
 fs.writeFileSync(O+'/proof.json',JSON.stringify(proof,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:proof.result,sha256:H(fs.readFileSync(O+'/proof.json'))}));
}
