'use strict';
// Read-only GitHub observation + fresh ignored raw evidence. Never cancel/rerun
// workflows, change the checkout, execute project tests, or modify public docs.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const ROOT=path.resolve(__dirname,'..'),OUTPUT=path.join(ROOT,'artifacts/table-xlsx-public-ci-22fc522-20261021');
const COMMIT='22fc5220333c7a1e5a1204ad11692a8d1b62ac76',RUN=39599316343,RENDERER=119219455951;
const JOBS=new Map([[119219455758,{name:'test (windows-latest)',file:'windows.log'}],[119219455948,{name:'test (ubuntu-latest)',file:'ubuntu.log'}],[RENDERER,{name:'renderer',file:'renderer.log'}],[119219456437,{name:'container',file:'container.log'}]]);
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function gh(endpoint){return new Promise((resolve,reject)=>cp.execFile('gh',['api',endpoint],{cwd:ROOT,encoding:'buffer',maxBuffer:32*1024*1024,windowsHide:true},(error,stdout,stderr)=>error?reject(new Error('gh '+endpoint+': '+stderr.toString())):resolve(stdout)));}
function committed(name){return cp.execFileSync('git',['show',COMMIT+':'+name],{cwd:ROOT,encoding:'buffer',maxBuffer:2*1024*1024,windowsHide:true});}
(async()=>{
  assert(!fs.existsSync(OUTPUT),'Fresh ignored collection directory required');
  const routes=[['run.json','repos/lia-xim/open-seo-spider/actions/runs/'+RUN],['jobs.json','repos/lia-xim/open-seo-spider/actions/runs/'+RUN+'/jobs?per_page=100']];
  const metadata=await Promise.all(routes.map(async([name,endpoint])=>({name,endpoint,bytes:await gh(endpoint)})));
  const run=JSON.parse(metadata[0].bytes),jobResponse=JSON.parse(metadata[1].bytes),jobs=jobResponse.jobs;
  assert.equal(run.id,RUN);assert.equal(run.head_sha,COMMIT);assert.equal(run.head_branch,'main');assert.equal(run.event,'push');assert.equal(run.run_attempt,1);
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
  assert.equal(expected.length,46);assert.equal(new Set(expected.map(item=>item.name)).size,46);assert.equal(expected.filter(item=>item.kind==='fixture').length,9);
  const log=raw.find(item=>item.name==='renderer.log')?.bytes.toString('utf8')||'',normalized=log.replace(/\x1b\[[0-9;]*m/g,'');
  const observed=[...normalized.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));
  const banners=[...normalized.matchAll(/^\d{4}-[^\r\n ]+ ##\[group\]Run node scripts\/ui-e2e\.mjs\r?$/gm)];
  const renderer=jobs.find(job=>job.id===RENDERER),uiSteps=renderer.steps.filter(step=>step.name==='Redesigned UI fixture and native end-to-end workflows');
  let uiAcceptance=false,uiRefusal;
  try{
    assert.equal(banners.length,1,'Exactly one canonical unfiltered coordinator command');
    const start=banners[0].index,after=normalized.slice(start),end=after.indexOf('\n',after.indexOf('\n')+1);
    const echo=after.slice(after.indexOf('\n')+1,end).replace(/^\d{4}-[^\r\n ]+ /,'').trim();assert.equal(echo,'node scripts/ui-e2e.mjs','Unfiltered original command echo');
    assert.equal(observed.length,46);assert.equal(new Set(observed.map(item=>item.name)).size,46);assert.deepEqual(observed,expected.map(({name,kind})=>({name,kind})));
    assert.equal(uiSteps.length,1);assert.equal(uiSteps[0].status,'completed');assert.equal(uiSteps[0].conclusion,'success');assert(observed.some(item=>item.name==='table-xlsx'&&item.kind==='owned-native'));
    const uiEnd=normalized.indexOf('##[group]Run ',start+banners[0][0].length),bounded=normalized.slice(start,uiEnd<0?undefined:uiEnd);
    const boundedPass=[...bounded.matchAll(/^\d{4}-[^\r\n ]+ PASS ([a-z0-9-]+) \((fixture|live|owned-native)\)\r?$/gm)].map(match=>({name:match[1],kind:match[2]}));assert.deepEqual(boundedPass,observed,'All suite records belong to the same UI step');
    uiAcceptance=true;
  }catch(error){uiRefusal=error.message;}
  const receipt={schema_version:1,kind:'exact_published_XLSX_public_ci_terminal',result:allSuccess&&uiAcceptance?'PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI46_INVOCATION':'TERMINAL_NOT_ACCEPTED_REVIEW_ACTUAL_RAW_LOGS',collected_at:new Date().toISOString(),commit:COMMIT,tree:cp.execFileSync('git',['rev-parse',COMMIT+'^{tree}'],{cwd:ROOT,encoding:'utf8',windowsHide:true}).trim(),run:RUN,url:run.html_url,head_branch:run.head_branch,event:run.event,status:run.status,conclusion:run.conclusion,run_attempt:run.run_attempt,jobs:jobSummary,renderer:{id:RENDERER,UI_step:uiSteps.length===1?uiSteps[0]:null,canonical_unfiltered_invocations:banners.length,passed_suites:observed.length,unique_passed_suites:new Set(observed.map(item=>item.name)).size,fixture_suites:observed.filter(item=>item.kind==='fixture').length,native_suites:observed.filter(item=>item.kind!=='fixture').length,suites_in_committed_order:expected,observed,accepted:uiAcceptance,refusal:uiRefusal},log_errors:logErrors,registered_suite_source:{commit:COMMIT,path:'scripts/ui-e2e.mjs',local_copy:'registered-ui-e2e.mjs',bytes:registry.length,sha256:sha(registry)},workflow_source:{commit:COMMIT,path:'.github/workflows/ci.yml',local_copy:'committed-ci.yml',bytes:workflow.length,sha256:sha(workflow)},raw:raw.map(({name,endpoint,bytes})=>({path:name,endpoint,bytes:bytes.length,sha256:sha(bytes)})),scope:'Exact published main22fc522 only. Public job conclusions and the complete unfiltered46-suite invocation are checked independently from the earlier selected605-input local evidence. The selected local manifest is not a transitive toolchain/dependency or whole metadata closure. This public result does not claim a fresh local46 run, all-table/segment/selected-recrawl/specialist/cursor/bulk native XLSX coverage, raw Boolean fidelity in every source adapter, large-corpus or GUI interoperability, installation, full workflow completion or manufacturer parity. Separate successor Boolean/source-availability and new provenance work is outside this commit. Earlier focused R1/R2 failures remain unchanged, and duplicate branch39599316540 cancellation is separate from this main run.'};
  const receiptBytes=Buffer.from(JSON.stringify(receipt,null,2)+'\n');fs.writeFileSync(path.join(OUTPUT,'receipt.json'),receiptBytes,{flag:'wx'});
  console.log(JSON.stringify({result:receipt.result,commit:COMMIT,run:RUN,conclusion:run.conclusion,jobs:jobSummary,UI:{accepted:uiAcceptance,passed:observed.length,unique:new Set(observed.map(item=>item.name)).size,invocations:banners.length,refusal:uiRefusal},raw:receipt.raw,receipt_sha256:sha(receiptBytes),directory:OUTPUT},null,2));
  if(receipt.result!=='PASS_ALL_FOUR_JOBS_AND_ONE_CANONICAL_UI46_INVOCATION')process.exitCode=1;
})().catch(error=>{console.error(error.stack);process.exitCode=1;});
