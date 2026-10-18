'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),http=require('node:http');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const artifactPath=require('./artifact-path.cjs');
const app=process.env.SPIDER_APP_URL;
assert(app&&new URL(app).hostname==='127.0.0.1'&&!['8787','8790'].includes(new URL(app).port),'Use an isolated test app');
let hits=0;const checks=[],errors=[];
const website=http.createServer((req,res)=>{hits++;res.end('Owned witness: preview must never fetch this page');});
async function until(work,label){for(let i=0;i<300;i++){if(await work())return;await new Promise(r=>setTimeout(r,25));}throw Error('Timeout '+label);}
async function responseFor(page,selector,route,status=200){const pending=page.waitForResponse(r=>r.url()===app+route&&r.request().method()==='POST');await page.click(selector);const response=await pending;assert.equal(response.status(),status,await response.text());const value=await response.json();await until(()=>page.isEnabled('#preview'),'native preview UI complete');return value;}
(async()=>{
  await new Promise(r=>website.listen(0,'127.0.0.1',r));const url=`http://127.0.0.1:${website.address().port}/owned`;
  const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser=await chromium.launch({headless:true,executablePath}),page=await browser.newPage({acceptDownloads:true});
  page.on('pageerror',e=>errors.push(e.message));
  try{
    await page.goto(app+'/api-enrichment');assert(await page.isDisabled('#start'));await page.fill('#input',url+'\n'+url+'\nnot-a-url');
    const preview=await responseFor(page,'#preview','/api/enrichments/preview');assert.equal(preview.unique_provider_urls,1);assert.equal(preview.supplied_rows,3);assert.equal(preview.application_website_requests,0);assert.equal(preview.provider_may_fetch_submitted_site,true);assert.equal(hits,0);assert(await page.isDisabled('#start'));
    const refused=await fetch(app+'/api/enrichments',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({acknowledge_remote_fetch:false,input_utf8:url,import:{format:{kind:'txt'}},options:{version:1,strategy:'mobile',credential_profile:'anonymous'}})});assert.equal(refused.status,400);await refused.body.cancel();assert.equal(hits,0);
    await page.check('#ack');assert(await page.isEnabled('#start'));await page.fill('#input','not-yet-previewed');assert(await page.isDisabled('#start'));
    checks.push('real native URL preview deduplicates without crawling; UI and API require acknowledgement, and input changes invalidate start permission');
    await page.fill('#input','not-a-url');await responseFor(page,'#preview','/api/enrichments/preview',422);await until(async()=>(await page.locator('#error').innerText()).includes('No accepted supplied'),'visible native invalid input error');assert(await page.isDisabled('#start'));
    checks.push('native invalid-only import refusal is shown with its actionable error and no enabled stale start');
    await page.fill('#input',url);await page.selectOption('#profile','page_speed_environment');await responseFor(page,'#preview','/api/enrichments/preview');await page.check('#ack');
    const start=page.waitForResponse(r=>r.url()===app+'/api/enrichments'&&r.request().method()==='POST');await page.click('#start');const failure=await start;assert.equal(failure.status(),422);await until(async()=>(await page.locator('#error').innerText()).includes('credential profile is unavailable'),'visible missing provider profile');assert.equal(await page.inputValue('#job'),'');
    checks.push('missing server credential profile fails before job creation or provider transfer, with native reason visible');
    await page.fill('#job','00000000-0000-0000-0000-000000000000');const missing=page.waitForResponse(r=>r.url().endsWith('/api/enrichments/00000000-0000-0000-0000-000000000000/configuration'));await page.click('#reopen');const missingResponse=await missing;assert.equal(missingResponse.status(),422);const missingBody=await missingResponse.json(),missingMessage=typeof missingBody.error==='string'?missingBody.error:missingBody.error.message;await until(async()=>(await page.locator('#error').innerText())===missingMessage&&await page.isEnabled('#reopen'),'missing saved job error');assert(await page.isDisabled('#export'));assert(await page.isDisabled('#start'));assert.equal(await page.locator('#rows tr').count(),0);
    for(const width of [1440,320]){await page.setViewportSize({width,height:900});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'enrichment overflow '+width);await page.locator('#export').scrollIntoViewIfNeeded();const bounds=await page.locator('#export').boundingBox();assert(bounds.y<900&&bounds.y+bounds.height>0);await page.screenshot({path:artifactPath('enrichment-'+width+'.png'),fullPage:true});}
    assert.equal(hits,0);assert.deepEqual(errors,[]);
    fs.writeFileSync(artifactPath('enrichment-live.json'),JSON.stringify({result:'PASS',checks,website_requests:hits,errors,scope:'Native preview, acknowledgement, configuration invalidation, invalid-input and missing-credential/job errors. No eligible URL is submitted to Google. Successful provider execution, persisted provider metrics, pagination and cold reopen remain unproven.'},null,2)+'\n');
    console.log(checks.length+' native API-enrichment UI workflows passed (provider-success boundary remains open)');
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;}).finally(()=>new Promise(resolve=>{website.close(resolve);website.closeAllConnections();}));
