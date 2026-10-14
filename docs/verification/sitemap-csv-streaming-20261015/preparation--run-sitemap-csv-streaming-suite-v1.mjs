import assert from 'node:assert/strict';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath,pathToFileURL} from 'node:url';

const task=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const runtimeRoot=path.join(task,'outputs/open-seo-spider');
const repo = "E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-csv-streaming-integration-v1";
const root='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed','New large runs remain held until the coordinated execution window is released');
for (const name of ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']) { assert(!process.env[name], 'Unexpected externally configured compiler setting: ' + name); }
const source=path.join(root,'sitemap-csv-streaming-source-v1');
const sourceRaw=fs.readFileSync(path.join(source,'manifest.json'));
const manifest=JSON.parse(sourceRaw);
assert.equal(createHash('sha256').update(sourceRaw).digest('hex'),'a31936510381cca40901eb547f15090a77c833f617f84f51f67b057fe7a5714e');
assert.equal(manifest.files.length,265);
assert.equal(manifest.required_tests.reduce((n,x)=>n+x.names.length,0),288);
const out=path.join(root,'results/sitemap-csv-streaming-suite-v1');
const sha=b=>createHash('sha256').update(b).digest('hex');
assert(!fs.existsSync(out));fs.mkdirSync(out);
const temp=path.join(out,'temp');fs.mkdirSync(temp);
const env={...process.env,CARGO_TARGET_DIR:path.join(root,'cargo-target'),CARGO_INCREMENTAL:'0',CARGO_PROFILE_DEV_DEBUG:'0',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_TERM_COLOR:'never',OPEN_SEO_BROWSER_PATH:'C:/Program Files/Google/Chrome/Application/chrome.exe',TEMP:temp,TMP:temp};
const runs=[],failures=[];
let worker,fixture,workerAddress,fixtureAddress,closed=false;
function verify(){for(const f of manifest.files){assert.equal(sha(fs.readFileSync(path.join(source,f.path))),f.sha256);assert.equal(sha(fs.readFileSync(path.join(repo,f.path))),f.sha256);}}
function verifyRuntime(){for(const f of manifest.files.filter(x=>x.path.startsWith('renderer/')))assert.equal(sha(fs.readFileSync(path.join(runtimeRoot,f.path))),f.sha256);}
function write(name,value){fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n',{flag:'wx'});}
async function run(name,args){
  const log=fs.openSync(path.join(out,name+'.log'),'wx');
  const started=Date.now();
  let child,spawnError=null,exit;
  try{
    child=spawn('C:/Users/matth/.cargo/bin/cargo.exe',args,{cwd:source,env,windowsHide:true,stdio:['ignore',log,log]});
    child.once('error',error=>{spawnError=error.message;});
    exit=await new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));
  }finally{fs.closeSync(log);}
  const bytes=fs.readFileSync(path.join(out,name+'.log'));const text=bytes.toString('utf8');
  const summaries=[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
  const record={name,args,pid:child.pid,actual_close_event:true,exit,spawn_error:spawnError,elapsed_ms:Date.now()-started,log_sha256:sha(bytes),summaries};
  runs.push(record);write(name+'.json',record);
  console.log(JSON.stringify({name,exit,passed:summaries.reduce((n,x)=>n+x.passed,0),ignored:summaries.reduce((n,x)=>n+x.ignored,0)}));
  assert.equal(spawnError,null,spawnError??"spawn");
  assert.equal(exit.code,0,text.slice(-3500));
  assert.equal(exit.signal,null);
}
try{
  verify();verifyRuntime();
  const {createRendererServer}=await import(pathToFileURL(path.join(runtimeRoot,'renderer/server.mjs')).href);
  worker=createRendererServer({env,concurrency:4});workerAddress=await worker.listen(0);
  assert(![8787,8790].includes(workerAddress.port));
  fixture=http.createServer((req,res)=>{res.setHeader('content-type','text/html');res.end('<html><head><title>Warmup</title></head><body><script>document.title="Owned ready";</script></body></html>');});
  await new Promise(resolve=>fixture.listen(0,'127.0.0.1',resolve));fixtureAddress=fixture.address();
  assert(![8787,8790].includes(fixtureAddress.port));
  const origin='http://127.0.0.1:'+workerAddress.port;
  const health=await(await fetch(origin+'/health',{signal:AbortSignal.timeout(10000)})).json();assert.equal(health.browser.available,true);
  const response=await fetch(origin+'/render',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({url:'http://127.0.0.1:'+fixtureAddress.port+'/',wait_ms:20}),signal:AbortSignal.timeout(35000)});
  assert.equal(response.status,200);
  write('owned-worker.json',{origin,fixture_origin:'http://127.0.0.1:'+fixtureAddress.port,health,warmup:await response.json(),shared_worker_contacted:false});
  env.OPEN_SEO_SPIDER_RENDERER=origin+'/render';
  await run('format',['fmt','--all','--check']);
  await run('workspace-live',['test','--workspace','--locked','--','--include-ignored','--test-threads=1']);
  const workspaceText=fs.readFileSync(path.join(out,'workspace-live.log'),'utf8');
  for(const group of manifest.required_tests)for(const name of group.names)assert(workspaceText.includes('::'+name+' ... ok'),'Required actual test not green: '+group.path+'::'+name);
  await run('clippy',['clippy','--workspace','--all-targets','--locked','--','-D','warnings']);
  verify();verifyRuntime();
}catch(error){failures.push({message:error.message,stack:error.stack});process.exitCode=1;console.error(error.message.slice(-3500));}
finally{
  try{if(worker)await worker.close();if(fixture)await new Promise(resolve=>fixture.close(resolve));closed=(!worker||!worker.server.listening)&&(!fixture||!fixture.listening);}catch(error){failures.push({phase:'owned close',message:error.message});process.exitCode=1;}
  const tests=runs.find(x=>x.name==='workspace-live');const passed=tests?.summaries.reduce((n,x)=>n+x.passed,0)??0;const ignored=tests?.summaries.reduce((n,x)=>n+x.ignored,0)??0;
  const result=failures.length===0&&closed&&passed>=1070&&ignored===0?'PASS_LOCAL_CURRENT_WORKSPACE_REAL_RENDERER_CLIPPY_FMT':'FAIL_LOCAL_CURRENT_WORKSPACE';
  const proof={schema_version:1,result,recorded_at:new Date().toISOString(),source_manifest_sha256:sha(sourceRaw),files:manifest.files.length,passed,ignored,minimum_expected:1070,required_test_names:manifest.required_tests.reduce((n,x)=>n+x.names.length,0),runs,owned_worker_port:workerAddress?.port,owned_worker_closed:closed,user_renderer_contacted:false,failures,qualification:'Actual combined265-source Windows workspace, all288 named source cases required, including four real browser gates on an owned warmed ephemeral renderer. Authored source delta adds276 tests to accepted794; minimum1070. S3/S4 profiles and ordinary verified raw/source windows plus raw CLI are included. Rich inventory, controlled same-TX allstored visitor and bounded two-pass CSV are included as authored source. Selected materialization, physical CSV64MiB/+1, seven full finding filters, source matrix, RSS/latency, recovery and native/platform acceptance remain open. GitHub/Linux/container gates separate and unclaimed.'};
  write('proof.json',proof);console.log(JSON.stringify({result,passed,ignored,closed,out}));
  if(result.startsWith('FAIL'))process.exitCode=1;
}
