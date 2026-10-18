'use strict';
// Invoked only by run-enrichment-browser-checks.mjs against its owned native host.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),{createHash}=require('node:crypto');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const artifactPath=require('./artifact-path.cjs');
const app=process.env.SPIDER_APP_URL,control=process.env.SPIDER_ENRICHMENT_CONTROL,phase=process.env.SPIDER_ENRICHMENT_PHASE;
for(const origin of [app,control])assert(origin&&new URL(origin).hostname==='127.0.0.1'&&!['8787','8790'].includes(new URL(origin).port),'Only owned isolated origins');
assert(['initial','cold'].includes(phase));
const fixture=JSON.parse(fs.readFileSync(process.env.SPIDER_ENRICHMENT_FIXTURE)),handoffPath=process.env.SPIDER_ENRICHMENT_HANDOFF;
const checks=[],errors=[],exportProof=[],sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(work,label){for(let i=0;i<1200;i++){const result=await work();if(result)return result;await sleep(50);}throw Error('Timeout: '+label);}
function check(label){checks.push(label);console.log('PASS '+label);}
async function state(){const response=await fetch(control+'/state');assert.equal(response.status,200);return response.json();}
async function command(name){const response=await fetch(control+'/'+name);assert.equal(response.status,200);await response.text();}
async function api(route,body,status=200){const response=await fetch(app+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'content-type':'application/json'},body:body===undefined?undefined:JSON.stringify(body),cache:'no-store'});const bytes=Buffer.from(await response.arrayBuffer());assert.equal(response.status,status,bytes.toString());return status===200?JSON.parse(bytes):bytes.toString();}
function read(snapshot=null,after=null){return {expected_snapshot:snapshot,after,limit:100,outcome:null};}
function capture(page,route,method='POST'){return page.waitForResponse(r=>r.url()===app+route&&r.request().method()===method).then(response=>{const body=response.body();return body.then(bytes=>({response,bytes,value:JSON.parse(bytes)}));});}
async function clickJson(page,selector,route,method='POST'){const pending=capture(page,route,method);await page.click(selector);const result=await pending;assert.equal(result.response.status(),200,result.bytes.toString());return result.value;}
async function preview(page,input){if(input!==undefined)await page.fill('#input',input);const before=(await state()).attempts.length;const value=await clickJson(page,'#preview','/api/enrichments/preview');await until(()=>page.isEnabled('#preview'),'preview controls released');assert.equal((await state()).attempts.length,before,'Preview makes no provider calls');assert(await page.isDisabled('#start'),'Fresh preview still needs explicit acknowledgement');return value;}
async function start(page){await page.check('#ack');const value=await clickJson(page,'#start','/api/enrichments');assert.equal(value.generation,1);assert.equal(value.application_website_requests,0);return value.id;}
function expected(index,revision){
  if(index===2)return {outcome:'unavailable',http:200,score:null,lcp:null,failure:null};
  if(index===3)return {outcome:'failed',http:403,score:null,lcp:null,failure:'authentication'};
  if(index===4)return {outcome:'failed',http:200,score:null,lcp:null,failure:'lighthouse_runtime'};
  return {outcome:'success',http:200,score:index===0&&revision===1?0:Number((index/200+(revision-1)/10).toFixed(3)),lcp:index===1?null:index===0&&revision===1?0:1000+index*7+(revision-1)*200,failure:null};
}
function validateRows(rows,revision){
  assert.equal(rows.length,108);const supplied=fixture.input.split('\n');
  rows.forEach((row,i)=>{
    assert.equal(row.input.raw_value,supplied[i]);assert.equal(row.input.source_row,i+1);
    if(i===107){assert.equal(row.input.normalized_url,null);assert(row.input.error);assert.equal(row.provider_observation,null);return;}
    const index=i<105?i:0,url=fixture.urls[index],observation=row.provider_observation,exp=expected(index,revision);
    assert.equal(row.input.normalized_url,url);assert.equal(observation.requested_url,url);assert.equal(observation.outcome,exp.outcome);assert.equal(observation.api_http_status,exp.http);assert.equal(observation.failure,exp.failure);assert.equal(observation.provider_request_attempts,index===5?2:1);
    assert.equal(observation.metrics?.performance_score??null,exp.score);assert.equal(observation.metrics?.largest_contentful_paint_ms??null,exp.lcp);
    if(exp.outcome==='success'){assert.equal(observation.metrics.provider_final_url,url+'?provider-final=1');assert.equal(observation.metrics.first_contentful_paint_ms,500+index);assert.equal(observation.metrics.total_blocking_time_ms,index);assert.equal(observation.metrics.cumulative_layout_shift,index/1000);}
  });
  assert.deepEqual(rows[105].provider_observation,rows[0].provider_observation);assert.deepEqual(rows[106].provider_observation,rows[0].provider_observation);
}
function cells(rows){return rows.map(r=>[r.input.source_row,r.input.raw_value,r.provider_observation?.outcome||'input_rejected',r.provider_observation?.api_http_status??'—',r.provider_observation?.metrics?.performance_score??'—',r.provider_observation?.metrics?.largest_contentful_paint_ms??'—'].map(String));}
async function dom(page,rows){await until(async()=>JSON.stringify(await page.locator('#rows tr').evaluateAll(trs=>trs.map(tr=>[...tr.children].map(td=>td.textContent))))===JSON.stringify(cells(rows)),'exact visible saved rows '+rows.length);}
async function finished(page,id,total=108){
  await until(async()=>{const value=await api('/api/enrichments/'+id);assert(!['failed','interrupted'].includes(value.status),JSON.stringify(value));return ['complete','partial','cancelled'].includes(value.status);},'native completion');
  await until(async()=>await page.locator('#rows tr').count()===Math.min(100,total)&&await page.isEnabled('#export'),'UI first saved page');
  const first=await api(`/api/enrichments/${id}/results`,read());assert.equal(first.generation,1);assert.equal(first.total,total);assert.equal(first.application_website_requests,0);await dom(page,first.rows);
  let rows=[...first.rows],current=first;
  while(current.after){const next=await clickJson(page,'#next',`/api/enrichments/${id}/results`);assert.equal(next.snapshot,first.snapshot);assert.equal(next.generation,1);assert.equal(next.total,total);rows.push(...next.rows);await dom(page,rows);current=next;}
  assert.equal(rows.length,total);assert(await page.isDisabled('#next'));assert(await page.isDisabled('#cancel'),'Terminal reports have no active Cancel action');assert((await page.locator('#status').innerText()).includes(total+' Eingabezeilen'));return {id,first,rows};
}
function parseCsv(text){const records=[];let record=[],value='',quoted=false;for(let i=0;i<text.length;i++){const c=text[i];if(c==='"'){if(quoted&&text[i+1]==='"'){value+='"';i++;}else quoted=!quoted;}else if(c===','&&!quoted){record.push(value);value='';}else if(c==='\n'&&!quoted){record.push(value.replace(/\r$/,''));records.push(record);record=[];value='';}else value+=c;}assert(!quoted);if(record.length||value){record.push(value);records.push(record);}return records;}
async function csv(page,saved,label){
  const before=await state(),responsePending=page.waitForResponse(r=>r.url()===app+`/api/enrichments/${saved.id}/csv`&&r.request().method()==='POST'),downloadPending=page.waitForEvent('download');
  await page.click('#export');const observed=await responsePending,download=await downloadPending;assert.equal(observed.status(),200);const filename=artifactPath(`enrichment-${phase}-${label}.csv`);await download.saveAs(filename);assert.equal(await download.failure(),null);const bytes=fs.readFileSync(filename);
  const oracle=await fetch(observed.url(),{method:'POST',headers:{'content-type':'application/json'},body:observed.request().postData(),cache:'no-store'}),native=Buffer.from(await oracle.arrayBuffer());assert.equal(oracle.status,200,native.toString());assert.deepEqual(bytes,native);
  for(const headers of [observed.headers(),Object.fromEntries(oracle.headers)]){assert.equal(headers['x-content-sha256'],sha(bytes));assert.equal(Number(headers['content-length']),bytes.length);assert.equal(headers['cache-control'],'no-store');}
  const records=parseCsv(bytes.toString('utf8')),header=records.shift();assert.deepEqual(header,['source_index','source_row','raw_value','normalized_url','input_error','provider','outcome','api_http_status','provider_request_attempts','performance_score','lcp_ms','provider_final_url']);assert.equal(records.length,saved.rows.length);
  records.forEach((values,i)=>{const row=saved.rows[i],obs=row.provider_observation;assert.deepEqual(values,[String(row.input.source_index),String(row.input.source_row),row.input.raw_value,row.input.normalized_url||'',row.input.error||'','pagespeed_lighthouse_v1',obs?.outcome||'input_rejected',obs?.api_http_status===null||!obs?'':String(obs.api_http_status),obs?String(obs.provider_request_attempts):'',obs?.metrics?.performance_score==null?'':String(obs.metrics.performance_score),obs?.metrics?.largest_contentful_paint_ms==null?'':String(obs.metrics.largest_contentful_paint_ms),obs?.metrics?.provider_final_url||'']);});
  assert.equal(bytes.includes(Buffer.from('PRIVATE_PROVIDER_SENTINEL_NEVER_SAVE')),false);assert.deepEqual(await state(),before,'Export reads never call provider or website');exportProof.push({label,id:saved.id,bytes:bytes.length,sha256:sha(bytes)});return {sha256:sha(bytes),bytes:bytes.length};
}
async function reopen(page,id){const before=await state();await page.fill('#job',id);const config=await clickJson(page,'#reopen',`/api/enrichments/${id}/configuration`,'GET');await until(()=>page.isEnabled('#reopen'),'configuration loaded');assert.equal(await page.inputValue('#input'),config.input_utf8);assert.equal(await page.inputValue('#strategy'),config.options.strategy);assert.equal(await page.inputValue('#profile'),config.options.credential_profile);assert.equal(await page.isChecked('#ack'),false);assert(await page.isDisabled('#start'));assert.deepEqual(await state(),before,'Reopen never calls provider');return config;}
function disk(id){const dir=path.join(process.env.SPIDER_ENRICHMENT_STORE,'api-enrichments',id);return Object.fromEntries(['input.bin','plan.json','report.json','completion.json'].map(name=>[name,sha(fs.readFileSync(path.join(dir,name)))]));}
(async()=>{
  const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser=await chromium.launch({headless:true,executablePath}),page=await browser.newPage({acceptDownloads:true});page.on('pageerror',error=>errors.push(error.message));
  try{
    await page.goto(app+'/api-enrichment');assert(await page.isDisabled('#start'));assert(await page.isDisabled('#export'));assert.equal(await page.isChecked('#ack'),false);
    if(phase==='initial'){
      const plan=await preview(page,fixture.input);assert.equal(plan.supplied_rows,108);assert.equal(plan.unique_provider_urls,105);assert.equal(plan.application_website_requests,0);assert.equal((await state()).attempts.length,0);
      const id=await start(page);await until(async()=>(await state()).heldInitial,'provider request held');await until(async()=>(await page.locator('#status').innerText())==='running','visible running polling');assert(await page.isEnabled('#cancel'));assert(await page.isDisabled('#export'));await command('release');
      const original=await finished(page,id);validateRows(original.rows,1);assert.equal(original.first.status,'partial');assert.equal(original.first.provider_request_attempts,106);const originalCsv=await csv(page,original,'original');const originalDisk=disk(id);
      check('Browser acknowledgement starts real native service; owned HTTP provider produces exact 100+8 DOM rows, zero and missing metrics, duplicates, rejected input, retry and failures');
      for(const width of [1440,320]){await page.setViewportSize({width,height:900});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'populated overflow '+width);await page.locator('#export').scrollIntoViewIfNeeded();const box=await page.locator('#export').boundingBox();assert(box.y>=0&&box.y+box.height<=900);await page.screenshot({path:artifactPath(`enrichment-success-${width}.png`),fullPage:true});}
      const config=await reopen(page,id);assert.equal(config.input_utf8,fixture.input);assert.equal(config.acknowledge_remote_fetch,false);await finished(page,id);const beforeRefresh=await state();await command('revision2');assert.equal((await state()).attempts.length,beforeRefresh.attempts.length);await preview(page);assert.equal((await state()).attempts.length,beforeRefresh.attempts.length);
      const refreshedId=await start(page);assert.notEqual(refreshedId,id);const refreshed=await finished(page,refreshedId);validateRows(refreshed.rows,2);assert.equal(refreshed.first.provider_request_attempts,106);await csv(page,refreshed,'refresh');assert.equal((await state()).attempts.length,212);assert.deepEqual(disk(id),originalDisk);
      const old=await api(`/api/enrichments/${id}/results`,read(original.first.snapshot));assert.deepEqual(old,original.first);await reopen(page,id);const reopened=await finished(page,id);assert.deepEqual(reopened.rows,original.rows);assert.deepEqual(await csv(page,reopened,'original-after-refresh'),originalCsv);assert.deepEqual(disk(id),originalDisk);
      check('Explicit fresh acknowledgement creates a new refresh job and new provider calls; old generation, rows, full CSV and all four saved files stay byte-identical');
      await api(`/api/enrichments/${id}/results`,read('f'.repeat(64)),409);await api(`/api/enrichments/${id}/csv`,read('f'.repeat(64)),409);await api(`/api/enrichments/${refreshedId}/results`,read(refreshed.first.snapshot,original.first.after),409);
      check('Native stale snapshot and cross-job continuation are rejected for results and full export');
      await preview(page,fixture.cancelInput);const cancelledId=await start(page);await until(async()=>(await state()).heldCancel,'owned cancellable provider held');await until(()=>page.isEnabled('#cancel'),'running cancellation control');await clickJson(page,'#cancel',`/api/enrichments/${cancelledId}/cancel`);const cancelled=await finished(page,cancelledId,2);assert.equal(cancelled.first.status,'cancelled');assert.equal(cancelled.first.provider_request_attempts,1);assert.deepEqual(cancelled.rows.map(r=>r.provider_observation.outcome),['cancelled','cancelled']);assert.deepEqual(cancelled.rows.map(r=>r.provider_observation.provider_request_attempts),[1,0]);assert(cancelled.rows.every(r=>r.provider_observation.metrics===null));await until(async()=>(await state()).cancelAborts===1,'provider request was aborted');await csv(page,cancelled,'cancelled');
      check('Actual held provider request is cancelled and disconnected; unattempted tail remains cancelled with unknown metrics and zero attempts');
      fs.writeFileSync(handoffPath,JSON.stringify({id,config,original,originalCsv,originalDisk,refreshId:refreshedId,cancelledId,attempts:(await state()).attempts.length},null,2));
    }else{
      const handoff=JSON.parse(fs.readFileSync(handoffPath));assert.equal(await page.inputValue('#job'),'','New browser has no seeded local selection');const before=await state();assert.equal(before.attempts.length,handoff.attempts);
      const config=await reopen(page,handoff.id);assert.deepEqual(config,handoff.config);const saved=await finished(page,handoff.id);assert.deepEqual(saved,handoff.original);assert.deepEqual(disk(handoff.id),handoff.originalDisk);assert.deepEqual(await csv(page,saved,'cold-original'),handoff.originalCsv);assert.equal(await page.isChecked('#ack'),false);assert(await page.isDisabled('#start'));assert.deepEqual(await state(),before);
      check('New browser and actual cold native process reopen exact saved configuration, generation, 100+8 rows and byte-identical full CSV without provider resume or stale acknowledgement');
    }
    const witness=await state();assert.equal(witness.websiteHits,0);assert.deepEqual(witness.violations,[]);assert.deepEqual(errors,[]);
    fs.writeFileSync(artifactPath(`enrichment-success-${phase}.json`),JSON.stringify({result:'PASS',checks,exports:exportProof,errors,website_requests:witness.websiteHits,provider_attempts:witness.attempts.length,scope:'Actual full native cfg(test) router/service with owned HTTP provider. Real Google, CLI and other providers remain unproven.'},null,2)+'\n');
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
