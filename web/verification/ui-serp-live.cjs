const artifactPath = require('./artifact-path.cjs');
const assert=require('node:assert/strict');
const http=require('node:http');
const fs=require('node:fs');
const path=require('node:path');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const app=process.env.SPIDER_URL||'http://127.0.0.1:8787';
const executable=process.env.SPIDER_BROWSER_EXECUTABLE||(()=>{for(const candidate of ['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'])if(fs.existsSync(candidate))return candidate;return undefined;})();
const checks=[];const checked=message=>{checks.push(message);console.log(`PASS ${message}`);};
const fixture=http.createServer((req,res)=>{websiteRequests++;res.end('Offline editor must never fetch this fixture');});let websiteRequests=0;
(async()=>{
 await new Promise(resolve=>fixture.listen(0,'127.0.0.1',resolve));
 const seed=`http://127.0.0.1:${fixture.address().port}/unrequested`;
 const browser=await chromium.launch(executable?{executablePath:executable}:{channel:process.platform==='win32'?'chrome':undefined});
 const context=await browser.newContext({acceptDownloads:true});const page=await context.newPage();const errors=[];page.on('pageerror',error=>errors.push(error.message));
 try{
  await page.goto(app+'/serp');await page.evaluate(()=>localStorage.removeItem('open-seo-spider.serp.v1'));await page.reload();
  await page.fill('#url',seed);await page.fill('#title','Über "uns"');await page.fill('#description','Zeile eins\nZeile zwei');await page.click('button[type=submit]');
  await page.waitForFunction(()=>document.querySelector('#status').textContent.includes('1 Zeilen gemessen'));
  assert.match(await page.locator('#preview-title').innerText(),/Über "uns"/);assert.match(await page.locator('#measurements').innerText(),/10 Zeichen/);assert.equal(websiteRequests,0);checked('Actual offline form uses native metrics without requesting website URL');
  const csv=`URL,Title,Description\r\n${seed},"Über ""uns""","two\nlines"\r\n${seed},😀," \t=FORMULA()"\r\n`;
  await page.setInputFiles('#csv-file',{name:'owned-serp.csv',mimeType:'text/csv',buffer:Buffer.from(csv)});await page.waitForFunction(()=>document.querySelector('#status').textContent.includes('2 Zeilen importiert'));
  assert.equal(await page.locator('#rows-table tbody tr').count(),2);await page.click('[aria-label="Zeile 2 bearbeiten"]');assert.equal(await page.inputValue('#title'),'😀');assert.match(await page.locator('#measurements').innerText(),/1 Zeichen · unbekannt/);assert.equal(websiteRequests,0);checked('CSV quoted newline Unicode duplicate URL order and unknown width survive actual import');
  await page.reload();await page.waitForFunction(()=>document.querySelector('#status').textContent.includes('2 Zeilen gemessen'));assert.equal(await page.locator('#rows-table tbody tr').count(),2);checked('Offline draft survives reload with all original rows');
  const csvDownload=page.waitForEvent('download');await page.click('#export-csv');const csvFile=artifactPath('ui-serp-live-export.csv');await(await csvDownload).saveAs(csvFile);const exported=fs.readFileSync(csvFile,'utf8');assert.match(exported,/' \t=FORMULA\(\)/);assert.match(exported,/😀/);
  const jsonDownload=page.waitForEvent('download');await page.click('#export-json');const jsonFile=artifactPath('ui-serp-live-draft.json');await(await jsonDownload).saveAs(jsonFile);const draft=JSON.parse(fs.readFileSync(jsonFile,'utf8'));assert.equal(draft.rows[1].description,' \t=FORMULA()');assert.equal(draft.rows[0].description,'two\nlines');assert.equal(draft.rows[0].url,draft.rows[1].url);checked('CSV formula protection and exact JSON draft values have separate real downloads');
  const bad=await fetch(app+'/api/serp/import',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({csv:'URL,Title\na,b\n'})});assert.equal(bad.status,400);checked('Malformed CSV has an explicit API error');
  for(const width of [1440,390,320]){await page.setViewportSize({width,height:900});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);await page.locator('footer').scrollIntoViewIfNeeded();assert.equal(await page.locator('footer').isVisible(),true);}checked('Live offline editor fits desktop and narrow mobile with reachable footer');
  assert.equal(websiteRequests,0);assert.deepEqual(errors,[]);
  const result={tested_at:new Date().toISOString(),result:'PASS',scope:'Actual browser→offline Rust import/measurement/export with owned HTTP fixture receiving zero requests',website_requests:websiteRequests,checks,javascript_runtime_errors:errors};fs.writeFileSync(artifactPath('ui-serp-live-results.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
 }finally{await context.close();await browser.close();await new Promise(resolve=>fixture.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;fixture.close();});
