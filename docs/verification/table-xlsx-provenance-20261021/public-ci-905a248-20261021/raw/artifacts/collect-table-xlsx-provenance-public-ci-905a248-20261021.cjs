'use strict';
// Read-only GitHub observation + fresh ignored raw evidence. Never cancel/rerun
// workflows, change the checkout, execute project tests, or modify public docs.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const ROOT=path.resolve(__dirname,'..'),OUTPUT=path.join(ROOT,'artifacts/table-xlsx-provenance-public-ci-905a248-20261021');
const COMMIT='905a2480c54d431a42eae1ed93e43f094a3e7a76',RUN=39609409363,RENDERER=119254001915;
const JOBS=new Map([[119254002447,{name:'test (windows-latest)',file:'windows.log'}],[119254002116,{name:'test (ubuntu-latest)',file:'ubuntu.log'}],[RENDERER,{name:'renderer',file:'renderer.log'}],[119254002100,{name:'container',file:'container.log'}]]);
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function gh(endpoint){return new Promise((resolve,reject)=>cp.execFile('gh',['api',endpoint],{cwd:ROOT,encoding:'buffer',maxBuffer:32*1024*1024,windowsHide:true},(error,stdout,stderr)=>error?reject(new Error('gh '+endpoint+': '+stderr.toString())):resolve(stdout)));}
function committed(name){return cp.execFileSync('git',['show',COMMIT+':'+name],{cwd:ROOT,encoding:'buffer',maxBuffer:2*1024*1024,windowsHide:true});}
(async()=>{
  assert(!fs.existsSync(OUTPUT),'Fresh ignored collection directory required');
  const routes=[['run.json','repos/lia-xim/open-seo-spider/actions/runs/'+RUN],['jobs.json','repos/lia-xim/open-seo-spider/actions/runs/'+RUN+'/jobs?per_page=100']];
  const metadata=await Promise.all(routes.map(async([name,endpoint])=>({name,endpoint,bytes:await gh(endpoint)})));
  const run=JSON.parse(metadata[0].bytes),jobResponse=JSON.parse(metadata[1].bytes),jobs=jobResponse.jobs;
  assert.equal(run.id,RUN);assert.equal(run.head_sha,COMMIT);assert.equal(run.head_branch,'main');assert.equal(run.event,'push');assert.equal(run.run_attempt,1);
  assert.equal(cp.execFileSync('git',['rev-parse',COMMIT+'^{tree}'],{cwd:ROOT,encoding:'utf8',windowsHide:true}).trim(),'6afaa4c322c9d467aa1dcf7d2951284bc6bc589a');
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
  assert.equal(expected.length,48);assert.equal(new Set(expected.map(item=>item.name)).size,48);assert.equal(expected.filter(item=>item.kind==='fixture').length,9);
  const log=raw.find(item=>item.name==='renderer.log')?.bytes.toString('utf8')||'',normalized=log.replace(/\x1b\[[0-9;]*m/g,'');
  const observed=[...normalized.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));
  const banners=[...normalized.matchAll(/^\d{4}-[^\r\n ]+ ##\[group\]Run node scripts\/ui-e2e\.mjs\r?$/gm)];
  const renderer=jobs.find(job=>job.id===RENDERER),uiSteps=renderer.steps.filter(step=>step.name==='Redesigned UI fixture and native end-to-end workflows');
  let uiAcceptance=false,uiRefusal;
  try{
    assert.equal(banners.length,1,'Exactly one canonical unfiltered coordinator command');
    const start=banners[0].index,after=normalized.slice(start),end=after.indexOf('\n',after.indexOf('\n')+1);
    const echo=after.slice(after.indexOf('\n')+1,end).replace(/^\d{4}-[^\r\n ]+ /,'').trim();assert.equal(echo,'node scripts/ui-e2e.mjs','Unfiltered original command echo');
    assert.equal(observed.length,48);assert.equal(new Set(observed.map(item=>item.name)).size,48);assert.deepEqual(observed,expected.map(({name,kind})=>({name,kind})));
    assert.equal(uiSteps.length,1);assert.equal(uiSteps[0].status,'completed');assert.equal(uiSteps[0].conclusion,'success');assert(observed.some(item=>item.name==='table-xlsx'&&item.kind==='owned-native'));assert(observed.some(item=>item.name==='table-xlsx-provenance'&&item.kind==='owned-native'));assert(observed.some(item=>item.name==='table-xlsx-availability'&&item.kind==='owned-native'));
    const uiEnd=normalized.indexOf('##[group]Run ',start+banners[0][0].length),bounded=normalized.slice(start,uiEnd<0?undefined:uiEnd);
    const boundedPass=[...bounded.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));assert.deepEqual(boundedPass,observed,'All suite records belong to the same UI step');
    uiAcceptance=true;
  }catch(error){uiRefusal=error.message;}
  const receipt={schema_version:1,kind:'exact_published_XLSX_source_availability_provenance_public_ci_terminal',result:allSuccess&&uiAcceptance?'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI48_INVOCATION':'TERMINAL_NOT_ACCEPTED_REVIEW_ACTUAL_RAW_LOGS',collected_at:new Date().toISOString(),commit:COMMIT,tree:cp.execFileSync('git',['rev-parse',COMMIT+'^{tree}'],{cwd:ROOT,encoding:'utf8',windowsHide:true}).trim(),run:RUN,url:run.html_url,head_branch:run.head_branch,event:run.event,status:run.status,conclusion:run.conclusion,run_attempt:run.run_attempt,jobs:jobSummary,renderer:{id:RENDERER,UI_step:uiSteps.length===1?uiSteps[0]:null,canonical_unfiltered_invocations:banners.length,passed_suites:observed.length,unique_passed_suites:new Set(observed.map(item=>item.name)).size,fixture_suites:observed.filter(item=>item.kind==='fixture').length,native_suites:observed.filter(item=>item.kind!=='fixture').length,suites_in_committed_order:expected,observed,accepted:uiAcceptance,refusal:uiRefusal},log_errors:logErrors,registered_suite_source:{commit:COMMIT,path:'scripts/ui-e2e.mjs',local_copy:'registered-ui-e2e.mjs',bytes:registry.length,sha256:sha(registry)},workflow_source:{commit:COMMIT,path:'.github/workflows/ci.yml',local_copy:'committed-ci.yml',bytes:workflow.length,sha256:sha(workflow)},raw:raw.map(({name,endpoint,bytes})=>({path:name,endpoint,bytes:bytes.length,sha256:sha(bytes)})),scope:'Exact published main905a248 only. This public result independently binds all four jobs and one complete unfiltered48-suite invocation to its immutable committed registry/workflow. The earlier609-input local source set is explicitly selected, not transitive toolchain/dependency or whole metadata closure; its608 unchanged files and one separately qualified post-gate whitespace policy remain correctly distinguished. No fresh full local48 run is inferred. Original native R1/R2/R3 failures remain FAIL; original R3 provenance35 PASS belongs to its failed aggregate, R4 availability8 PASS is selected-only, and local focused20 PASS is a separate invocation. Old22fc522 public46 success remains predecessor-only. Duplicate branch39609409247 cancellation is separate from this main run. No all29-schema/operator, specialist/cursor/bulk/streaming/multisheet XLSX acceptance, arbitrary corruption/crash/resource behavior, Excel/LibreOffice GUI interoperability, installed upgrade, full workflow completion or manufacturer parity is inferred.'};
  const receiptBytes=Buffer.from(JSON.stringify(receipt,null,2)+'\n');fs.writeFileSync(path.join(OUTPUT,'receipt.json'),receiptBytes,{flag:'wx'});
  console.log(JSON.stringify({result:receipt.result,commit:COMMIT,run:RUN,conclusion:run.conclusion,jobs:jobSummary,UI:{accepted:uiAcceptance,passed:observed.length,unique:new Set(observed.map(item=>item.name)).size,invocations:banners.length,refusal:uiRefusal},raw:receipt.raw,receipt_sha256:sha(receiptBytes),directory:OUTPUT},null,2));
  if(receipt.result!=='PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI48_INVOCATION')process.exitCode=1;
})().catch(error=>{console.error(error.stack);process.exitCode=1;});
