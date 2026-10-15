import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
const D='E:/CodexWork/open-seo-spider-01a12ac3-20261013/combined-source-v9-draft-v4';
const H=b=>createHash('sha256').update(b).digest('hex');
assert.equal(process.argv.length,6,'--preformat-sha SHA --format-sha SHA');
assert.equal(process.argv[2],'--preformat-sha');assert.equal(process.argv[4],'--format-sha');
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
assert(!fs.existsSync('E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/combined-native-outer-v4/active.lock'),'No concurrent native job');
const preRaw=fs.readFileSync(D+'/preformat.json'),fmtRaw=fs.readFileSync(D+'/actual-format.json');
assert.equal(H(preRaw),process.argv[3]);assert.equal(H(fmtRaw),process.argv[5]);
const pre=JSON.parse(preRaw),fmt=JSON.parse(fmtRaw);assert.equal(fmt.result,'PASS_FORMAT_COMMAND_ONLY');assert.equal(fmt.preformat_sha256,H(preRaw));assert.equal(fmt.exit.code,0);assert.equal(fmt.exit.signal,null);assert(fmt.actual_close_event);assert.equal(pre.files.length,358);assert.equal(pre.owned_paths.length,30);assert.equal(pre.new_authored_test_functions.length,37);
const owned=new Set(pre.owned_paths),changed=new Map(fmt.changed_files.map(f=>[f.path,f]));
for(const f of fmt.changed_files)assert(owned.has(f.path),'Formatter must not change an unowned file');
const files=pre.files.map(f=>{const b=fs.readFileSync(D+'/source/'+f.path),change=changed.get(f.path);assert.equal(H(b),change?change.after_sha256:f.sha256,f.path);return{path:f.path,sha256:H(b),bytes:b.length}});
function verify(){assert.equal(H(fs.readFileSync(D+'/preformat.json')),process.argv[3]);assert.equal(H(fs.readFileSync(D+'/actual-format.json')),process.argv[5]);for(const f of files){const b=fs.readFileSync(D+'/source/'+f.path);assert.equal(b.length,f.bytes,f.path);assert.equal(H(b),f.sha256,f.path)}}
verify();assert(!fs.existsSync(D+'/actual-preliminary-clippy.json'));
const env={...process.env,CARGO_TARGET_DIR:'E:/CodexWork/open-seo-spider-01a12ac3-20261013/cargo-target',CARGO_INCREMENTAL:'0',CARGO_TERM_COLOR:'never',CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0'};
for(const [name,value]of Object.entries(process.env)){if(/^(RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|RUSTDOCFLAGS|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTC)$/.test(name))assert(!value,'Unexpected compiler override '+name)}
const logfile=D+'/actual-preliminary-clippy.log',fd=fs.openSync(logfile,'wx');let close,spawnError=null,sourceFinal=false;
const started=performance.now(),child=spawn('C:/Users/matth/.cargo/bin/cargo.exe',['clippy','--workspace','--all-targets','--locked','--','-D','warnings'],{cwd:D+'/source',env,windowsHide:true,stdio:['ignore',fd,fd]});
child.once('error',e=>spawnError=String(e));try{close=await new Promise(r=>child.once('close',(code,signal)=>r({code,signal})))}finally{fs.fsyncSync(fd);fs.closeSync(fd)}
verify();sourceFinal=true;const receipt={kind:'source9_actual_preliminary_clippy',recorded_at:new Date().toISOString(),result:!spawnError&&close.code===0&&close.signal===null?'PASS_PRELIMINARY_CLIPPY_ONLY':'FAIL_PRELIMINARY_CLIPPY',executed_tests:0,actual_close_event:true,pid:child.pid,exit:close,spawn_error:spawnError,elapsed_ms:performance.now()-started,preformat_sha256:H(preRaw),format_sha256:H(fmtRaw),source_final_verified:sourceFinal,log_sha256:H(fs.readFileSync(logfile)),source_inputs:files.length,files,qualification:'All-targets Clippy compilation only on isolated composed Source9. No executed Rust/native tests, product promotion, private publication or installation.'};
fs.writeFileSync(D+'/actual-preliminary-clippy.json',JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:receipt.result,sha256:H(fs.readFileSync(D+'/actual-preliminary-clippy.json')),exit:close,elapsed_ms:receipt.elapsed_ms,source_inputs:files.length}));assert.equal(receipt.result,'PASS_PRELIMINARY_CLIPPY_ONLY');
