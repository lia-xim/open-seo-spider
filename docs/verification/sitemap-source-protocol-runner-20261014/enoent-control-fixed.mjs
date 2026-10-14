import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawn as actualSpawn} from 'node:child_process';
import {createHash,randomUUID} from 'node:crypto';
import {fileURLToPath} from 'node:url';

const reviewRoot=path.dirname(fileURLToPath(import.meta.url));
const suitePath='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/work/run-sitemap-source-protocol-suite-v2.mjs';
const expectedSha='6994b969fb65812c0bf3f424166a50c842a29626d8f9deafc8046aafe808bfea';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const suiteBytes=fs.readFileSync(suitePath);
assert.equal(sha(suiteBytes),expectedSha);
const text=suiteBytes.toString('utf8');
const start=text.indexOf('async function run(name,args){');
assert(start>=0);
const end=text.indexOf('\ntry{',start);
assert(end>start);
const exactRun=text.slice(start,end);
assert(exactRun.endsWith('}'));
const out=path.join(reviewRoot,'enoent-control');
assert(!fs.existsSync(out));fs.mkdirSync(out);
const source=out;
const env={};
const runs=[];
const missingExecutable=path.join(out,'never-created-'+randomUUID()+'.exe');
assert(!fs.existsSync(missingExecutable));
let attempts=0,openedFd,closedFd;
const controlledFs=new Proxy(fs,{get(target,key){
  if(key==='openSync')return (...args)=>{assert.equal(openedFd,undefined);openedFd=fs.openSync(...args);return openedFd;};
  if(key==='closeSync')return fd=>{assert.equal(fd,openedFd);closedFd=fd;return fs.closeSync(fd);};
  return target[key];
}});
function spawn(executable,args,options){
  assert.equal(executable,'C:/Users/matth/.cargo/bin/cargo.exe');
  assert.deepEqual(args,[]);
  assert.equal(options.cwd,source);
  assert.equal(options.windowsHide,true);
  assert(!fs.existsSync(missingExecutable));
  attempts++;
  assert.equal(attempts,1);
  // The only actual spawn target is an absolute, never-created fixture path.
  // No compiler, real Cargo, renderer, browser, server or full runner is invoked.
  return actualSpawn(missingExecutable,args,options);
}
function write(name,value){fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n',{flag:'wx'});}
const AsyncFunction=Object.getPrototypeOf(async function(){}).constructor;
const invoke=new AsyncFunction('assert','fs','path','spawn','createHash','source','env','out','sha','runs','write','console',exactRun+'\nawait run("deliberate-missing-executable",[]);');
let rejection;
try{await invoke(assert,controlledFs,path,spawn,createHash,source,env,out,sha,runs,write,console);}catch(error){rejection=error;}
assert(rejection);
assert.match(rejection.message,/ENOENT/);
assert.equal(attempts,1);
assert.equal(openedFd,closedFd);
assert.throws(()=>fs.fstatSync(openedFd),error=>error.code==='EBADF');
assert.equal(runs.length,1);
const run=runs[0];
assert.equal(run.actual_close_event,true);
assert.match(run.spawn_error,/ENOENT/);
assert.notEqual(run.exit.code,0);
assert.equal(run.exit.signal,null);
assert.equal(run.summaries.length,0);
assert.equal(run.pid,undefined);
const log=fs.readFileSync(path.join(out,'deliberate-missing-executable.log'));
assert.equal(log.length,0);
assert.equal(run.log_sha256,sha(log));
const persisted=JSON.parse(fs.readFileSync(path.join(out,'deliberate-missing-executable.json'),'utf8'));
assert.equal(persisted.actual_close_event,true);
assert.equal(persisted.spawn_error,run.spawn_error);
assert.equal(sha(fs.readFileSync(suitePath)),expectedSha);
const proof={schema_version:1,result:'PASS_ACTUAL_NODE_ENOENT_CLOSE_AND_LOG_RECEIPT_CONTROL_ONLY',recorded_at:new Date().toISOString(),suite_sha256:expectedSha,extracted_run_body_sha256:sha(exactRun),source_transformation:'None: exact run function text; spawn is explicitly injected wrapper replacing only executable with verified never-created absolute fixture',spawn_attempts:attempts,actual_target:missingExecutable,actual_close_event:run.actual_close_event,exit:run.exit,spawn_error:run.spawn_error,closed_log_descriptor:true,empty_log_sha256:run.log_sha256,persisted_command_receipt:true,expected_rejection_after_receipt:true,full_suite_executed:false,cargo_compiler_renderer_browser_native_started:false,qualification:'Actual Node missing-executable child lifecycle only; no Cargo, compiler, suite, worker, browser, native application or feature correctness acceptance'};
fs.writeFileSync(path.join(reviewRoot,'enoent-control-proof.json'),JSON.stringify(proof,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({result:proof.result,actual_close_event:proof.actual_close_event,closed_log_descriptor:true,persisted_command_receipt:true}));

