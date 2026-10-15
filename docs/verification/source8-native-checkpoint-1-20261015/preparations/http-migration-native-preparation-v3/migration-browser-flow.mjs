// AUTHORED, NOT RUN. Called only after a future Root-reviewed actual Source8 admission.
// The caller owns Chrome, its pipes, fixture/App servers and every actual close.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash} from 'node:crypto';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
async function bounded(promise,ms,label){let timer;try{return await Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(label)),ms);})]);}finally{clearTimeout(timer);}}

export async function runMigrationBrowser({browser,api,jobId,mappingPath,output,expectedRows,expectedCsvSha,checkpoint,assertWebsiteUnchanged}) {
  const context=await browser.newContext({acceptDownloads:true,viewport:{width:1440,height:1000}});
  context.setDefaultTimeout(30000);context.setDefaultNavigationTimeout(30000);
  let page,releasePreview,previewEnteredResolve,heldPreview;
  const previewEntered=new Promise(resolve=>{previewEnteredResolve=resolve;});
  let holdFirstPreview=true,previewRequests=0;
  const details=[];
  try {
    await context.addInitScript(({basename})=>{
      // This retains the actual File.arrayBuffer result, never substitutes bytes.
      const actual=File.prototype.arrayBuffer;
      let release;const held=new Promise(resolve=>{release=resolve;});
      window.__ownedMigrationFile={entered:false,release};
      File.prototype.arrayBuffer=async function(){const bytes=await actual.call(this);if(this.name===basename){window.__ownedMigrationFile.entered=true;await held;}return bytes;};
    },{basename:path.basename(mappingPath)});
    page=await context.newPage();
    await page.route('**/api/crawls/*/migration/preview',async route=>{
      previewRequests++;
      if(!holdFirstPreview){await route.continue();return;}
      holdFirstPreview=false;
      // Only timing is controlled: obtain the actual App response, retain its
      // original bytes/status/headers, then deliver that APIResponse unchanged.
      const response=await route.fetch({maxRedirects:0,maxRetries:0,timeout:30000});
      const bytes=await response.body();assert(bytes.length<=8*1024*1024);
      heldPreview={status:response.status(),bytes:bytes.length,sha256:sha(bytes),request:JSON.parse(route.request().postData())};
      const held=new Promise(resolve=>{releasePreview=resolve;});previewEnteredResolve();await held;
      await route.fulfill({response});
    });
    await page.goto(api+'/migration',{waitUntil:'domcontentloaded'});
    await page.locator('#saved-crawl option[value="'+jobId+'"]').waitFor({state:'attached'});
    await page.locator('#saved-crawl').selectOption(jobId);
    await page.locator('#migration-file').setInputFiles(mappingPath);
    await page.waitForFunction(()=>window.__ownedMigrationFile.entered===true);
    // Real configuration changes while the matching file result is held.
    await page.locator('#column-mode').selectOption('header');
    await page.locator('#old-column').fill('Old "URL"');
    await page.locator('#expected-column').fill('Expected');
    await page.locator('#start-row').fill('2');
    await page.locator('#header-row').fill('1');
    await page.locator('#expected-column').press('Tab');
    await page.evaluate(()=>window.__ownedMigrationFile.release());
    await page.waitForFunction(()=>document.getElementById('file-state').textContent.includes('Bytes'));
    checkpoint();
    const firstResponse=page.waitForResponse(response=>response.url()===api+'/api/crawls/'+jobId+'/migration/preview'&&response.request().method()==='POST');
    await page.locator('#preview').click();
    await bounded(previewEntered,35000,'Actual preview response barrier was not entered');
    for(const id of ['preview','outcome','next-page','export-json','export-csv'])assert.equal(await page.locator('#'+id).isDisabled(),true,'Actual busy guard '+id);
    assert.equal(await page.locator('#outcome').inputValue(),'');
    // Enter/form submission can still arrive from editable configuration fields.
    // Exercise the handler while the real response is held; it may not start a
    // second request. Disabled outcome is asserted before any DOM manipulation.
    await page.evaluate(()=>document.getElementById('migration-form').dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));
    assert.equal(previewRequests,1);
    assert.equal(typeof releasePreview,'function');releasePreview();releasePreview=null;
    const actualFirst=await firstResponse;assert.equal(actualFirst.status(),200);const delivered=await actualFirst.body();assert.equal(heldPreview.status,actualFirst.status());assert.equal(delivered.length,heldPreview.bytes);assert.equal(sha(delivered),heldPreview.sha256);const headerResult=JSON.parse(delivered.toString('utf8'));
    assert.equal(headerResult.csv_sha256,expectedCsvSha,'BOM/quoted source bytes remain exact through actual browser file upload');
    assert.equal(headerResult.total_rows,expectedRows);assert.equal(headerResult.website_requests,0);
    assert.deepEqual(headerResult.options.old_column,{kind:'header',name:'Old "URL"',row:1});
    await page.locator('#migration-results').waitFor({state:'visible'});
    await page.waitForFunction(()=>!document.getElementById('outcome').disabled);
    assert((await page.locator('#rows .row').count())>0);
    assert((await page.locator('#summary').textContent()).includes('Ziel weicht ab'));
    assert((await page.locator('#summary').textContent()).includes('Keine gespeicherte Beobachtung'));
    await assertWebsiteUnchanged('browser header/file/busy preview');
    details.push({name:'actual_file_result_survives_option_changes_and_actual_response_busy_is_real',csv_sha256:headerResult.csv_sha256,total_rows:headerResult.total_rows,actual_http_status:actualFirst.status(),held_actual_response:{bytes:heldPreview.bytes,sha256:heldPreview.sha256},busy_submit_preview_requests:previewRequests,header_options:headerResult.options.old_column});

    // Actual filter request and DOM result; whole-response timing does not supply
    // the expected verdict, which comes from the owned saved genuine crawl.
    let responsePromise=page.waitForResponse(r=>r.url().endsWith('/migration/preview')&&r.request().method()==='POST');
    await page.locator('#outcome').selectOption('missing_observation');
    let filtered=await (await responsePromise).json();assert(filtered.rows.length>0);assert(filtered.rows.every(row=>row.verdict==='missing_observation'));
    await page.waitForFunction(()=>[...document.querySelectorAll('#rows .row')].every(row=>row.dataset.verdict==='missing_observation'));
    await assertWebsiteUnchanged('browser actual outcome filter');
    details.push({name:'actual_filter_and_visible_rows_agree',outcome:'missing_observation',count:filtered.filtered_rows});

    // First finish the genuine all-outcomes header preview triggered by filter
    // reset, so its response cannot be mistaken for the later index request.
    responsePromise=page.waitForResponse(r=>r.url().endsWith('/migration/preview')&&r.request().method()==='POST');
    await page.locator('#outcome').selectOption('');
    assert.equal((await responsePromise).status(),200);
    await page.waitForFunction(()=>!document.getElementById('outcome').disabled);
    // Index mode is an explicit second preview, not client-side CSV parsing.
    await page.locator('#column-mode').selectOption('index');
    await page.locator('#old-column').fill('1');await page.locator('#expected-column').fill('2');
    await page.locator('#expected-column').press('Tab');
    responsePromise=page.waitForResponse(r=>r.url().endsWith('/migration/preview')&&r.request().method()==='POST');
    await page.locator('#preview').click();const actualIndex=await responsePromise;assert.equal(actualIndex.status(),200);const indexResult=await actualIndex.json();assert.equal(indexResult.total_rows,expectedRows);assert.equal(indexResult.csv_sha256,expectedCsvSha);assert.deepEqual(indexResult.options.old_column,{kind:'index',value:1});assert.deepEqual(indexResult.summary,headerResult.summary);
    await page.waitForFunction(()=>!document.getElementById('outcome').disabled);
    let shown=indexResult.rows.length;
    if(expectedRows>100){assert.equal(await page.locator('#next-page').isVisible(),true);responsePromise=page.waitForResponse(r=>r.url().endsWith('/migration/preview')&&r.request().method()==='POST');await page.locator('#next-page').click();const next=await(await responsePromise).json();assert.equal(next.offset,100);assert(next.rows.length>0);shown+=next.rows.length;assert(next.rows.every(row=>row.source_index>indexResult.rows.at(-1).source_index));}
    await page.waitForFunction(()=>!document.getElementById('outcome').disabled);
    await assertWebsiteUnchanged('browser index and actual pagination');
    details.push({name:'actual_index_preview_and_cursor_page',total_rows:indexResult.total_rows,shown_across_pages:shown,summary:indexResult.summary});

    const downloaded=[];
    for(const format of ['json','csv']){
      const pending=page.waitForEvent('download');await page.locator('#export-'+format).click();const download=await pending;assert.equal(download.suggestedFilename(),'migration.'+format);const destination=path.join(output,'browser-migration.'+format);await download.saveAs(destination);assert.equal(await download.failure(),null);const bytes=await fs.readFile(destination);assert(bytes.length>0&&bytes.length<=(format==='json'?8:64)*1024*1024);if(format==='json'){const value=JSON.parse(bytes.toString('utf8'));assert.equal(value.report.rows.length,expectedRows);assert.equal(value.report.input_sha256,expectedCsvSha);assert.equal(value.website_requests,0);assert.equal(value.snapshot.sql_attested,false);}else{const text=bytes.toString('utf8');assert(text.startsWith('"source_index","source_row"'));assert(text.includes('missing_observation')&&text.includes('destination_mismatch'));}
      downloaded.push({format,path:destination,bytes:bytes.length,sha256:sha(bytes)});
      await page.waitForFunction(()=>document.getElementById('migration-status').textContent.includes('Prüfsumme'));
    }
    await assertWebsiteUnchanged('browser complete downloads');details.push({name:'actual_browser_complete_download_files',files:downloaded});

    for(const width of [1440,320]){
      await page.setViewportSize({width,height:1000});
      const overflow=await page.evaluate(()=>({viewport:innerWidth,document:document.documentElement.scrollWidth,body:document.body.scrollWidth}));assert(overflow.document<=overflow.viewport&&overflow.body<=overflow.viewport,'Migration page horizontal overflow '+width);
      for(const id of ['migration-file','column-mode','old-column','expected-column','saved-crawl','preview','outcome','export-json','export-csv']){await page.locator('#'+id).scrollIntoViewIfNeeded();const bounds=await page.locator('#'+id).boundingBox();assert(bounds&&bounds.width>0&&bounds.x>=-1&&bounds.x+bounds.width<=width+1,'Reachable control '+id+' at '+width);}
      if(await page.locator('#next-page').isVisible()){await page.locator('#next-page').scrollIntoViewIfNeeded();const box=await page.locator('#next-page').boundingBox();assert(box&&box.x>=-1&&box.x+box.width<=width+1);}
      const image=path.join(output,'migration-'+width+'.png');await page.screenshot({path:image,fullPage:true});const imageBytes=await fs.readFile(image);details.push({name:'actual_responsive_controls_and_screenshot',width,...overflow,screenshot:{path:image,bytes:imageBytes.length,sha256:sha(imageBytes)}});
    }
    await assertWebsiteUnchanged('browser all read/preview/filter/download/viewport work');
    return {status:'PASS_ACTUAL_BROWSER_MIGRATION',details};
  }finally{
    if(releasePreview)releasePreview();
    await context.close();
  }
}
