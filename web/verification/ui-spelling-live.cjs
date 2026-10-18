'use strict';
const assert=require('node:assert/strict'),http=require('node:http'),fs=require('node:fs');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const artifactPath=require('./artifact-path.cjs');
const app=process.env.SPIDER_APP_URL;
assert(app&&new URL(app).hostname==='127.0.0.1'&&new URL(app).port!=='8787','Use an isolated test app');
const checks=[],errors=[],requests=[];let websiteHits=0,providerMode='success';
const website=http.createServer((req,res)=>{websiteHits++;res.setHeader('content-type','text/html');res.end('<html lang="en-US"><head><title>Owned spelling fixture</title></head><body><main>Thiss is the saved text for an owned spelling fixture.</main></body></html>');});
const engine=http.createServer(async(req,res)=>{
  const chunks=[];for await(const chunk of req)chunks.push(chunk);
  const form=new URLSearchParams(Buffer.concat(chunks).toString('utf8'));requests.push({path:req.url,text:form.get('text'),language:form.get('language')});
  if(providerMode==='failure'){res.writeHead(503);return res.end('Owned unavailable fixture');}
  const text=form.get('text'),offset=text.indexOf('Thiss');assert(offset>=0);
  res.setHeader('content-type','application/json');res.end(JSON.stringify({
    software:{name:'LanguageTool',version:'owned-e2e-v1',buildDate:'owned-fixture',apiVersion:1},
    language:{name:'English',code:'en-US'},warnings:{incompleteResults:false},
    matches:[{message:'Owned synthetic spelling issue',offset,length:5,context:{text,offset,length:5},replacements:[{value:'This'}],
      rule:{id:'OWNED_SPELLING',description:'Owned test rule',issueType:'misspelling',category:{id:'TYPOS',name:'Typos'}}}]}));
});
async function json(path,body){const response=await fetch(app+path,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'content-type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});assert.equal(response.ok,true,await response.clone().text());return response.json();}
async function until(work,label){for(let i=0;i<200;i++){const result=await work();if(result)return result;await new Promise(r=>setTimeout(r,50));}throw Error('Timeout: '+label);}
async function action(page,button,tail){const response=page.waitForResponse(r=>r.url().includes(tail)&&r.request().method()==='POST');await page.click(button);const actual=await response;assert.equal(actual.status(),200,await actual.text());return actual.json();}
(async()=>{
  await Promise.all([new Promise(r=>website.listen(0,'127.0.0.1',r)),new Promise(r=>engine.listen(0,'127.0.0.1',r))]);
  const seed=`http://127.0.0.1:${website.address().port}/document.html`,endpoint=`http://127.0.0.1:${engine.address().port}/v2/check`;
  const start=await json('/api/crawls',{seed_urls:[seed],crawl_links:false,discover_sitemaps:false,obey_robots:false,delay_ms:0,
    content_capture:{sources:['raw_http_html'],include_selectors:['main']}});
  await until(async()=>{const job=await json('/api/crawls/'+start.id);assert.notEqual(job.status,'failed',job.error);return job.status==='completed';},'owned native crawl');
  const capturedHits=websiteHits;
  const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser=await chromium.launch({headless:true,executablePath});const page=await browser.newPage();
  page.on('pageerror',error=>errors.push(error.message));
  try{
    await page.goto(app+'/spelling');await page.click('#refresh');await page.selectOption('#crawl',start.id);
    await page.click('#inventory');await page.locator('#captures input').first().waitFor();await page.locator('#captures input').first().check();
    await page.locator('details summary').first().click();await page.click('#load-dictionary');await until(()=>page.isEnabled('#save-dictionary'),'dictionary revision');
    const disabled=await action(page,'#check','/spelling/analyze');assert.equal(disabled.report.complete,false);assert.equal(disabled.report.accepted_count,null);assert.equal(requests.length,0);checks.push('default-off analysis is unknown and makes zero engine requests');
    await page.check('#enabled');await page.fill('#endpoint',endpoint);await page.fill('#engine-version','owned-e2e-v1');
    const refused=page.waitForResponse(r=>r.url().endsWith('/spelling/analyze'));await page.click('#check');assert.equal((await refused).status(),400);assert.equal(requests.length,0);checks.push('native API refuses text transfer without explicit acknowledgement');
    await page.check('#ack');const first=await action(page,'#check','/spelling/analyze');
    assert.equal(first.report.complete,true);assert.equal(first.report.accepted_count,1);await page.waitForFunction(()=>!document.getElementById('csv').disabled);assert.match(await page.locator('#rules').innerText(),/OWNED_SPELLING/);
    const download=page.waitForEvent('download');await page.click('#csv');const csv=await download;const csvBytes=fs.readFileSync(await csv.path(),'utf8');assert.match(csvBytes,/OWNED_SPELLING/);checks.push('real browser -> native saved-text adapter -> owned engine -> persisted run -> verified CSV download');
    await page.fill('#dictionary','en-US | Thiss');await action(page,'#save-dictionary','/api/spelling/dictionary');
    const ignored=await action(page,'#check','/spelling/analyze');assert.equal(ignored.report.complete,true);assert.equal(ignored.report.accepted_count,0);assert.equal(ignored.report.documents[0].checked.issues[0].disposition,'dictionary');
    const previous=await fetch(app+`/api/crawls/${start.id}/spelling/runs/${first.report.run_id}/csv`);assert.equal(previous.status,200);
    const currentPrefix=`"${first.report.run_id}","true","true","1",`,historicalPrefix=`"${first.report.run_id}","false","false","",`;
    assert.equal(csvBytes.split(currentPrefix).length,2);assert.equal(await previous.text(),csvBytes.replace(currentPrefix,historicalPrefix));
    checks.push('dictionary recheck ignores the word; old issue CSV retains every captured field and explicitly marks only its current-binding/complete/count columns historical');
    await page.click('#load-profile');await until(()=>page.isEnabled('#save-profile'),'profile revision');await page.check('#enabled');await page.check('#ack');await page.fill('#endpoint',endpoint);await page.fill('#engine-version','owned-e2e-v1');await action(page,'#save-profile','/spelling/profile');
    await page.reload();await page.click('#refresh');await page.selectOption('#crawl',start.id);await page.click('#load-profile');await until(()=>page.isEnabled('#save-profile'),'persisted profile reload');assert.equal(await page.isChecked('#enabled'),true);assert.equal(await page.inputValue('#endpoint'),endpoint);
    await page.locator('details summary').first().click();await page.click('#load-dictionary');await until(()=>page.isEnabled('#save-dictionary'),'persisted dictionary reload');assert.equal(await page.inputValue('#dictionary'),'en-US | Thiss');checks.push('profile and application dictionary survive browser reload through native persistent storage');
    await page.click('#inventory');await page.locator('#captures input').first().waitFor();await page.locator('#captures input').first().check();providerMode='failure';
    const failed=await action(page,'#check','/spelling/analyze');assert.equal(failed.report.complete,false);assert.equal(failed.report.accepted_count,null);checks.push('real provider HTTP503 remains incomplete instead of a successful zero');
    for(const width of [1440,320]){await page.setViewportSize({width,height:900});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:artifactPath(`spelling-${width}.png`),fullPage:true});}
    assert.equal(websiteHits,capturedHits,'Saved analysis must not refetch the website');assert.deepEqual(errors,[]);
    fs.writeFileSync(artifactPath('spelling-live.json'),JSON.stringify({result:'PASS',crawl_id:start.id,checks,engine_requests:requests,website_refetches:websiteHits-capturedHits,errors,
      scope:'Actual redesigned browser/native API/persistence/CSV with owned deterministic LanguageTool-compatible fixture. This does not prove actual LanguageTool language quality or cold process restart.'},null,2)+'\n');
    console.log(`${checks.length} native spelling workflows passed`);
  }catch(error){
    await page.screenshot({path:artifactPath('spelling-failure.png'),fullPage:true});
    const layout=await page.evaluate(()=>({viewport:innerWidth,document:document.documentElement.scrollWidth,
      overflow:[...document.querySelectorAll('body *')].filter(e=>e.getBoundingClientRect().right>innerWidth+1).slice(0,25).map(e=>({tag:e.tagName,id:e.id,width:e.getBoundingClientRect().width,text:e.textContent.slice(0,90)}))}));
    fs.writeFileSync(artifactPath('spelling-failure.json'),JSON.stringify({error:error.stack,checks,layout},null,2)+'\n');throw error;
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;}).finally(async()=>{
  for(const server of [website,engine]){server.closeAllConnections();await new Promise(r=>server.close(r));}
});
