'use strict';
// Read-only GitHub observation + fresh ignored raw evidence. Never cancel/rerun
// workflows, change the checkout, execute project tests, or modify public docs.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const ROOT=path.resolve(__dirname,'..'),OUTPUT=path.join(ROOT,'artifacts/extraction-source-public-ci-7250b9a-20261021');
const COMMIT='7250b9a0007d0f307321e3b4c7076fa95430dbc3',RUN=39638818285,RENDERER=119356860489;
const JOBS=new Map([[119356860870,{name:'test (windows-latest)',file:'windows.log'}],[119356861256,{name:'test (ubuntu-latest)',file:'ubuntu.log'}],[RENDERER,{name:'renderer',file:'renderer.log'}],[119356861162,{name:'container',file:'container.log'}]]);
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function gh(endpoint){return new Promise((resolve,reject)=>cp.execFile('gh',['api',endpoint],{cwd:ROOT,encoding:'buffer',maxBuffer:32*1024*1024,windowsHide:true},(error,stdout,stderr)=>error?reject(new Error('gh '+endpoint+': '+stderr.toString())):resolve(stdout)));}
function committed(name){return cp.execFileSync('git',['show',COMMIT+':'+name],{cwd:ROOT,encoding:'buffer',maxBuffer:2*1024*1024,windowsHide:true});}
(async()=>{
  assert(!fs.existsSync(OUTPUT),'Fresh ignored collection directory required');
  const routes=[['run.json','repos/lia-xim/open-seo-spider/actions/runs/'+RUN],['jobs.json','repos/lia-xim/open-seo-spider/actions/runs/'+RUN+'/jobs?per_page=100']];
  const metadata=await Promise.all(routes.map(async([name,endpoint])=>({name,endpoint,bytes:await gh(endpoint)})));
  const run=JSON.parse(metadata[0].bytes),jobResponse=JSON.parse(metadata[1].bytes),jobs=jobResponse.jobs;
  assert.equal(run.id,RUN);assert.equal(run.head_sha,COMMIT);assert.equal(run.head_branch,'main');assert.equal(run.event,'push');assert.equal(run.run_attempt,1);
  assert.equal(cp.execFileSync('git',['rev-parse',COMMIT+'^{tree}'],{cwd:ROOT,encoding:'utf8',windowsHide:true}).trim(),'d5e2252877fc74c9de52872ecfec04f544856b5c');
  assert.equal(jobResponse.total_count,4);assert.equal(jobs.length,4);assert.equal(new Set(jobs.map(job=>job.id)).size,4);
  for(const job of jobs){assert.equal(JOBS.get(job.id)?.name,job.name);assert.equal(job.run_id,RUN);assert.equal(job.head_sha,COMMIT);}
  if(run.status!=='completed'){
    console.log(JSON.stringify({result:'PENDING_EXACT_MAIN_RUN_NO_WRITES',run:RUN,commit:COMMIT,status:run.status,jobs:jobs.map(({id,name,status,conclusion,steps})=>({id,name,status,conclusion,current_step:steps.find(step=>step.status==='in_progress')?.name}))},null,2));process.exitCode=2;return;
  }
  const raw=[...metadata],logErrors=[];
  const logs=await Promise.allSettled(jobs.map(async job=>{
    const endpoint='repos/lia-xim/open-seo-spider/actions/jobs/'+job.id+'/logs';return{name:JOBS.get(job.id).file,endpoint,bytes:await gh(endpoint)};
  }));
  logs.forEach((result,index)=>result.status==='fulfilled'?raw.push(result.value):logErrors.push({job:jobs[index].id,error:result.reason.message}));
  const registry=committed('scripts/ui-e2e.mjs'),workflow=committed('.github/workflows/ci.yml');
  fs.mkdirSync(OUTPUT);for(const item of raw)fs.writeFileSync(path.join(OUTPUT,item.name),item.bytes,{flag:'wx'});
  fs.writeFileSync(path.join(OUTPUT,'registered-ui-e2e.mjs'),registry,{flag:'wx'});fs.writeFileSync(path.join(OUTPUT,'committed-ci.yml'),workflow,{flag:'wx'});
  const jobSummary=jobs.map(({id,name,status,conclusion,started_at,completed_at,steps})=>({id,name,status,conclusion,started_at,completed_at,failed_steps:steps.filter(step=>step.conclusion==='failure').map(({number,name,status,conclusion})=>({number,name,status,conclusion}))}));
  const allSuccess=run.conclusion==='success'&&jobs.every(job=>job.status==='completed'&&job.conclusion==='success')&&!logErrors.length;
  const expected=[...registry.toString('utf8').matchAll(/^\s+\['([a-z0-9-]+)',\s*'([^']+)',\s*'(fixture|live|owned-native)'/gm)].map(match=>({name:match[1],script:match[2],kind:match[3]}));
  assert.equal(expected.length,49);assert.equal(new Set(expected.map(item=>item.name)).size,49);assert.equal(expected.filter(item=>item.kind==='fixture').length,9);
  const log=raw.find(item=>item.name==='renderer.log')?.bytes.toString('utf8')||'',normalized=log.replace(/\x1b\[[0-9;]*m/g,'');
  const observed=[...normalized.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));
  const banners=[...normalized.matchAll(/^\d{4}-[^\r\n ]+ ##\[group\]Run node scripts\/ui-e2e\.mjs\r?$/gm)];
  const renderer=jobs.find(job=>job.id===RENDERER),uiSteps=renderer.steps.filter(step=>step.name==='Redesigned UI fixture and native end-to-end workflows');
  let uiAcceptance=false,uiRefusal;
  try{
    assert.equal(banners.length,1,'Exactly one canonical unfiltered coordinator command');
    const start=banners[0].index,after=normalized.slice(start),end=after.indexOf('\n',after.indexOf('\n')+1);
    const echo=after.slice(after.indexOf('\n')+1,end).replace(/^\d{4}-[^\r\n ]+ /,'').trim();assert.equal(echo,'node scripts/ui-e2e.mjs','Unfiltered original command echo');
    assert.equal(observed.length,49);assert.equal(new Set(observed.map(item=>item.name)).size,49);assert.deepEqual(observed,expected.map(({name,kind})=>({name,kind})));
    assert.equal(uiSteps.length,1);assert.equal(uiSteps[0].status,'completed');assert.equal(uiSteps[0].conclusion,'success');assert(observed.some(item=>item.name==='table-xlsx'&&item.kind==='owned-native'));assert(observed.some(item=>item.name==='table-xlsx-provenance'&&item.kind==='owned-native'));assert(observed.some(item=>item.name==='table-xlsx-availability'&&item.kind==='owned-native'));assert(observed.some(item=>item.name==='extraction-source-trust'&&item.kind==='owned-native'));
    const uiEnd=normalized.indexOf('##[group]Run ',start+banners[0][0].length),bounded=normalized.slice(start,uiEnd<0?undefined:uiEnd);
    const boundedPass=[...bounded.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));assert.deepEqual(boundedPass,observed,'All suite records belong to the same UI step');
    uiAcceptance=true;
  }catch(error){uiRefusal=error.message;}
  const receipt={schema_version:1,kind:'exact_published_extraction_source_trust_public_ci_terminal',result:allSuccess&&uiAcceptance?'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI49_INVOCATION':'TERMINAL_NOT_ACCEPTED_REVIEW_ACTUAL_RAW_LOGS',collected_at:new Date().toISOString(),commit:COMMIT,tree:cp.execFileSync('git',['rev-parse',COMMIT+'^{tree}'],{cwd:ROOT,encoding:'utf8',windowsHide:true}).trim(),run:RUN,url:run.html_url,head_branch:run.head_branch,event:run.event,status:run.status,conclusion:run.conclusion,run_attempt:run.run_attempt,jobs:jobSummary,renderer:{id:RENDERER,UI_step:uiSteps.length===1?uiSteps[0]:null,canonical_unfiltered_invocations:banners.length,passed_suites:observed.length,unique_passed_suites:new Set(observed.map(item=>item.name)).size,fixture_suites:observed.filter(item=>item.kind==='fixture').length,native_suites:observed.filter(item=>item.kind!=='fixture').length,suites_in_committed_order:expected,observed,accepted:uiAcceptance,refusal:uiRefusal},log_errors:logErrors,registered_suite_source:{commit:COMMIT,path:'scripts/ui-e2e.mjs',local_copy:'registered-ui-e2e.mjs',bytes:registry.length,sha256:sha(registry)},workflow_source:{commit:COMMIT,path:'.github/workflows/ci.yml',local_copy:'committed-ci.yml',bytes:workflow.length,sha256:sha(workflow)},raw:raw.map(({name,endpoint,bytes})=>({path:name,endpoint,bytes:bytes.length,sha256:sha(bytes)})),scope:"Exact published main7250b9a only. Original2c573d9 public UI failure at its first dashboard fixture remains FAIL with zero completed canonical suites; its other Windows/Ubuntu jobs were cancelled by configured new-push concurrency, separately from its successful container. The corrected local dashboard-only invocation remains a separate selected1 PASS, not a full49 run. This public result binds all four terminal jobs and one complete unfiltered49-suite invocation to immutable committed registry/workflow bytes, including extraction-source-trust. The earlier423 selected compiler/product inputs and selected pure/live source pins remain explicit source sets, not transitive toolchain/dependency or whole metadata closure. No fresh full local49 run is inferred. Original Rust R1/R2 and native extraction R1/R2/R3 failures remain FAIL; native R4 selected1 PASS, separate existing named-view/expression selected2 PASS, ordinary workspace tests and four Core renderer opt-ins remain separately scoped in their original receipts. App native_enrichment_browser_fixture opt-in is not inferred executed unless this exact public log expressly proves it. Earlier905a248 public48 and22fc522 public46 apply only to those predecessor commits. Duplicate branch39638818133 cancellation is separate from this main run. The inert archived .gitattributes.raw storage-path qualification changes no raw source bytes. Subsequent TODO implementation is outside this commit. No all29-schema/operator, specialist/cursor/bulk/streaming/multisheet export acceptance, arbitrary corruption/crash/resource behavior, Excel/LibreOffice GUI interoperability, installed upgrade, full workflow completion or manufacturer parity is inferred."};
  const receiptBytes=Buffer.from(JSON.stringify(receipt,null,2)+'\n');fs.writeFileSync(path.join(OUTPUT,'receipt.json'),receiptBytes,{flag:'wx'});
  console.log(JSON.stringify({result:receipt.result,commit:COMMIT,run:RUN,conclusion:run.conclusion,jobs:jobSummary,UI:{accepted:uiAcceptance,passed:observed.length,unique:new Set(observed.map(item=>item.name)).size,invocations:banners.length,refusal:uiRefusal},raw:receipt.raw,receipt_sha256:sha(receiptBytes),directory:OUTPUT},null,2));
  if(receipt.result!=='PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI49_INVOCATION')process.exitCode=1;
})().catch(error=>{console.error(error.stack);process.exitCode=1;});
