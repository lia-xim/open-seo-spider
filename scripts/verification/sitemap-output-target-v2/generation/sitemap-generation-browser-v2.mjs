// Actual Chrome gate for a later Root-supplied native executable. No mocked responses.
import assert from 'node:assert/strict';
import path from 'node:path';
import * as fs from 'node:fs/promises';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {isDeepStrictEqual} from 'node:util';
import {verifyBundle} from './sitemap-generation-zip-v1.mjs';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const clone=value=>JSON.parse(JSON.stringify(value));
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const fields={public_base_url:'public-base',max_urls_per_file:'max-urls',max_urlset_uncompressed_bytes:'max-file-bytes',max_index_uncompressed_bytes:'max-index-bytes',max_total_output_bytes:'max-total-bytes'};
const id=name=>'#sitemap-bundle-'+name;

export async function runNativeGeneratorBrowser(env){
  const {api,sourceRoot,runtimeRoot,output,job,siteOrigin,offlineOrigin,options,expect,assetRoutes,budgets,stamp,raw,json,readInterval,bounded,writeNew,jsonNew,hashFile,check,onProducer}=env;
  const require=createRequire(path.join(runtimeRoot,'scripts/package.json'));
  const playwright=require(path.join(runtimeRoot,'renderer/node_modules/playwright-core'));
  const chrome=await fs.realpath(env.chrome||'C:/Program Files/Google/Chrome/Application/chrome.exe');
  assert(path.isAbsolute(chrome)&&(await fs.lstat(chrome)).isFile());
  const directory=path.join(output,'browser');await fs.mkdir(directory);await fs.mkdir(path.join(output,'browser-downloads'));
  const routeName=`/api/crawls/${job.id}/sitemap-bundle`,requests=[],responses=[],downloads=[],external=[],oracles=[],cases=[],errors=[],responseTasks=[],active=new Set(),byRequest=new Map();
  const producer={kind:'owned_headless_chrome',pid:null,chrome,chrome_sha256:await hashFile(chrome),actual_close_event:false,server_close_event:false,context_closed:false,browser_disconnected:false,planned_close:false,unexpected_close:false,forced:false,version:null};onProducer(producer);
  let server,browser,browserProcess,context,page,processClosed,phase=null,oraclePhase=false,lastActivity=Date.now(),serial=0,responseSerial=0,downloadSerial=0,mainError;
  async function quiet(){
    const end=Date.now()+budgets.request;
    while(Date.now()<end){if(active.size===0&&Date.now()-lastActivity>=budgets.quiet)return;await delay(25);}
    throw Error('Browser requests did not become quiet');
  }
  async function draft(){return page.locator(id('form')).evaluate(form=>Array.from(form.querySelectorAll('input,select,textarea'),node=>({id:node.id,value:node.value,checked:node.checked,disabled:node.disabled})));}
  async function form(value){
    await page.locator(id('generate')).waitFor({state:'visible'});assert.equal(await page.locator(id('generate')).isDisabled(),false);
    for(const [key,suffix] of Object.entries(fields))await page.fill(id(suffix),String(value[key]));
    await page.selectOption(id('lastmod-source'),value.lastmod.source);
    if(value.lastmod.source==='provided')await page.fill(id('provided-values'),value.lastmod.values.map(row=>[row.page_url,row.value,row.reference].join('\t')).join('\n'));
    await page.locator(id('images')).setChecked(value.include_images);assert.equal(await page.locator(id('external-images')).isDisabled(),!value.include_images);
    if(value.include_images)await page.locator(id('external-images')).setChecked(value.allow_external_images);
    else assert.equal(await page.locator(id('external-images')).isChecked(),false);
    await page.locator(id('hreflang')).setChecked(value.include_hreflang);
    await page.selectOption(id('changefreq'),value.changefreq??'');await page.fill(id('priority'),value.priority??'');
  }
  async function invoke(label,value,status,expected){
    assert.equal(phase,null);await form(value);const draftBefore=await draft();
    const downloadCount=downloads.length,filesBefore=(await fs.readdir(path.join(output,'browser-downloads'))).sort();
    const native=await raw(routeName,value,status);
    if(status===200)verifyBundle(native.bytes,expected);
    phase={label,options:clone(value),status,requests:0};
    const responsePromise=page.waitForResponse(response=>new URL(response.url()).origin===api&&new URL(response.url()).pathname===routeName&&response.request().method()==='POST');responsePromise.catch(()=>{});
    const downloadPromise=status===200?page.waitForEvent('download'):null;downloadPromise?.catch(()=>{});
    try{
      await page.click(id('generate'));const response=await bounded(responsePromise,budgets.request,'Chrome native response '+label);
      assert.equal(response.status(),status);assert.deepEqual(JSON.parse(response.request().postData()),value);assert.equal(phase.requests,1,'Exactly one permitted UI generation POST');
      const headers=await response.allHeaders();
      if(status===200){
        const download=await bounded(downloadPromise,budgets.request,'Actual Chrome download '+label);assert.equal(await download.failure(),null);
        assert.equal(download.suggestedFilename(),`open-seo-spider-${job.id}-sitemap-bundle.zip`);assert(download.url().startsWith('blob:'+api+'/'));
        const file=path.join(directory,'ui-'+label+'.zip');try{await fs.lstat(file);throw Error('Refusing existing browser download proof');}catch(error){if(error.code!=='ENOENT')throw error;}
        await bounded(download.saveAs(file),budgets.request,'Chrome saveAs '+label);const bytes=await fs.readFile(file);assert(bytes.length<=value.max_total_output_bytes);assert.deepEqual(bytes,native.bytes,'Actual Chrome ZIP must equal separate actual API response');
        assert.equal(sha(bytes),headers['x-sitemap-bundle-sha256']);assert.equal(sha(bytes),native.headers['x-sitemap-bundle-sha256']);const proof=verifyBundle(bytes,expected);
        await page.waitForFunction(()=>!document.getElementById('sitemap-bundle-generate').disabled&&!document.getElementById('sitemap-bundle-result').hidden);
        assert.equal(await page.locator(id('error')).isVisible(),false);assert((await page.locator(id('result-fields')).textContent()).includes(sha(bytes)));assert.equal(downloads.length,downloadCount+1);
        const item={label,status,options:clone(value),actual_download_event:true,suggested_filename:download.suggestedFilename(),blob_url:download.url(),bytes:bytes.length,sha256:sha(bytes),api_raw_sha256:sha(native.bytes),report_sha256:proof.manifest.report_sha256,manifest:proof.manifest};cases.push(item);await jsonNew(path.join(directory,label+'.json'),item);
      }else{
        const failure=await response.json();assert.deepEqual(failure,JSON.parse(native.bytes));
        await page.waitForFunction(()=>!document.getElementById('sitemap-bundle-generate').disabled&&!document.getElementById('sitemap-bundle-error').hidden);
        const message=await page.locator(id('error')).textContent();assert(message&&message.trim());const nativeMessage=typeof failure.message==='string'?failure.message:failure.error;if(typeof nativeMessage==='string')assert(message.includes(nativeMessage));
        assert.equal(await page.locator(id('result')).isVisible(),false);assert.match(await page.locator(id('status')).textContent(),/Kein neues Archiv/);
        await quiet();assert.equal(downloads.length,downloadCount,'Error must not create a Chrome download event');assert.deepEqual((await fs.readdir(path.join(output,'browser-downloads'))).sort(),filesBefore,'Error must not create partial download files');
        const item={label,status,options:clone(value),native_failure:failure,error_text:message,downloads_before:downloadCount,downloads_after:downloads.length,partial_files_unchanged:true};cases.push(item);await jsonNew(path.join(directory,label+'.json'),item);
      }
      assert.deepEqual(await draft(),draftBefore,'Form values survive generation and errors');
    }finally{phase=null;}
  }
  try{
    server=await bounded(playwright.chromium.launchServer({headless:true,host:'127.0.0.1',port:0,executablePath:chrome,downloadsPath:path.join(output,'browser-downloads'),env:{...process.env,TMP:output,TEMP:output}}),budgets.start,'Owned headless Chrome server');
    const process=server.process();browserProcess=process;assert(Number.isSafeInteger(process.pid)&&process.pid>0);producer.pid=process.pid;
    const ws=new URL(server.wsEndpoint());assert.equal(ws.hostname,'127.0.0.1');assert(![8787,8790].includes(Number(ws.port)));producer.ws_origin=ws.origin;
    processClosed=new Promise(resolve=>process.once('close',(code,signal)=>{producer.actual_close_event=true;producer.unexpected_close||=!producer.planned_close;producer.code=code;producer.signal=signal;producer.closed_ms=stamp();resolve();}));server.once('close',()=>producer.server_close_event=true);
    browser=await bounded(playwright.chromium.connect(server.wsEndpoint()),budgets.start,'Connect owned Chrome');producer.version=browser.version();
    context=await browser.newContext({acceptDownloads:true,serviceWorkers:'block',viewport:{width:1440,height:1000}});context.setDefaultTimeout(budgets.request);
    await context.route('**/*',async route=>{
      const request=route.request(),url=new URL(request.url()),row={serial:++serial,url:url.href,method:request.method(),resource_type:request.resourceType(),payload:request.postData(),at_ms:stamp(),oracle_phase:oraclePhase,delivered:false,label:phase?.label??null};byRequest.set(request,row);lastActivity=Date.now();
      if(url.origin!==api){external.push(row);if(oraclePhase)oracles.push({...row,reason:'external_blocked'});return route.abort('blockedbyclient');}
      const jobRead=url.pathname===`/api/crawls/${job.id}`||url.pathname.startsWith(`/api/crawls/${job.id}/`);
      const read=request.method()==='GET'&&(Object.hasOwn(assetRoutes,url.pathname)||url.pathname==='/favicon.ico'||url.pathname==='/api/health'||url.pathname==='/api/crawls'||jobRead);
      let matchesOptions=false;try{matchesOptions=phase!==null&&isDeepStrictEqual(JSON.parse(request.postData()),phase.options);}catch{}
      const generate=request.method()==='POST'&&url.pathname===routeName&&matchesOptions;
      if(!read&&!generate){if(oraclePhase)oracles.push({...row,reason:'mutation_blocked'});else requests.push(row);return route.abort('blockedbyclient');}
      if(generate)phase.requests++;row.delivered=true;requests.push(row);return route.continue();
    });
    context.on('request',request=>{active.add(request);lastActivity=Date.now();});
    for(const event of ['requestfinished','requestfailed'])context.on(event,request=>{active.delete(request);lastActivity=Date.now();});
    context.on('page',other=>{if(page&&other!==page){errors.push('Unexpected extra browser page');other.close().catch(()=>{});}});
    page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));page.on('download',download=>{downloads.push({ordinal:++downloadSerial,at_ms:stamp(),suggested_filename:download.suggestedFilename(),url:download.url(),label:phase?.label??null});lastActivity=Date.now();});
    page.on('response',response=>{
      const responseId=++responseSerial;
      const task=(async()=>{
        const url=new URL(response.url());if(url.origin!==api)return;const headers=await response.allHeaders(),row={url:url.href,status:response.status(),headers,request:byRequest.get(response.request())??null};
        if(headers['content-type']==='application/zip'){row.body_capture='actual_Chrome_download_companion_and_separate_native_API_binary';}
        else{const bytes=await response.body();assert(bytes.length<=budgets.api);row.bytes=bytes.length;row.sha256=sha(bytes);const name=String(responseId).padStart(4,'0');await writeNew(path.join(directory,name+'.response.bin'),bytes);}
        responses.push(row);
      })();task.catch(()=>{});responseTasks.push(task);
    });
    await readInterval('native Chrome all generator date modes options downloads and draft preservation',async()=>{
      await page.goto(api);await page.locator('#connection-state.connected').waitFor();await page.selectOption('#crawl-history',job.id);await page.click('[data-section=sitemap-membership]');
      await page.locator(id('panel')+' > summary').click();await page.locator(id('form')+' > details > summary').click();
      await page.waitForFunction(id=>document.getElementById('sitemap-bundle-selected-crawl').textContent.includes(id)&&!document.getElementById('sitemap-bundle-generate').disabled,job.id);
      assert.deepEqual(await page.locator(id('lastmod-source')+' option').evaluateAll(nodes=>nodes.map(n=>n.value)),['omit','captured_sitemap','provided']);
      const frequencies=['','always','hourly','daily','weekly','monthly','yearly','never'];assert.deepEqual(await page.locator(id('changefreq')+' option').evaluateAll(nodes=>nodes.map(n=>n.value)),frequencies);for(const value of frequencies){await page.selectOption(id('changefreq'),value);assert.equal(await page.locator(id('changefreq')).inputValue(),value);}
      const captured={...clone(options),max_urlset_uncompressed_bytes:4096,max_index_uncompressed_bytes:4096,max_total_output_bytes:65536};await invoke('captured',captured,200,expect);
      const omit={...clone(captured),max_urls_per_file:1,include_images:false,allow_external_images:false,include_hreflang:false,lastmod:{source:'omit'},changefreq:null,priority:null};await invoke('omit',omit,200,{base:expect.base,eligible:expect.eligible,lastmods:Object.fromEntries(expect.eligible.map(url=>[url,null])),images:Object.fromEntries(expect.eligible.map(url=>[url,[]]))});
      const originalValue=' 2026-10-14T01:02:03+02:00 ',reference='Eigenes Änderungsprotokoll #17';
      const provided={...clone(captured),allow_external_images:true,lastmod:{source:'provided',values:[{page_url:siteOrigin+'/ambiguous',value:originalValue,reference}]},changefreq:'yearly',priority:'+.500000000000000000000000000000000000000'};
      await form(provided);await page.fill('#seed-url','https://native-draft.fixture/preserve');const before=await draft(),seedBefore=await page.locator('#seed-url').inputValue();
      const refresh=page.waitForResponse(response=>new URL(response.url()).origin===api&&new URL(response.url()).pathname===`/api/crawls/${job.id}/sitemaps`&&response.request().method()==='GET'&&response.status()===200);refresh.catch(()=>{});
      await page.click('#sitemap-membership-refresh');const refreshed=await bounded(refresh,budgets.request,'Actual saved-data UI refresh');const data=await refreshed.json();assert.equal(data.job_id,job.id);await page.waitForFunction(()=>!document.getElementById('sitemap-membership-refresh').disabled);
      assert.deepEqual(await draft(),before);assert.equal(await page.locator('#seed-url').inputValue(),seedBefore);cases.push({label:'draft-refresh',native_read_url:refreshed.url(),same_job:job.id,form_values_before:before,form_values_after:await draft(),seed_draft_unchanged:true});
      await page.setViewportSize({width:320,height:740});assert.equal(await page.evaluate(()=>innerWidth),320);
      const allImages=[siteOrigin+'/image.png?a=1&b=2',offlineOrigin+'/declared-image.png?captured=1'].sort();await invoke('provided-320',provided,200,{base:expect.base,eligible:expect.eligible,lastmods:{[siteOrigin+'/ambiguous']:'2026-10-14T01:02:03+02:00'},images:{[siteOrigin+'/start']:allImages}});
      const selected=cases.find(row=>row.label==='provided-320').manifest.memberships.find(row=>row.url===siteOrigin+'/ambiguous');assert.equal(selected.lastmod.source.original_value,originalValue);assert.equal(selected.lastmod.source.reference,reference);
      await page.locator(id('result')+' > summary').click();
      const boxes=[];for(const selector of [id('public-base'),id('provided-values'),id('external-images'),id('priority'),id('max-index-bytes'),id('max-total-bytes'),id('generate'),id('result-fields'),'.main-footer']){
        const node=page.locator(selector);await node.scrollIntoViewIfNeeded();const box=await node.boundingBox();assert(box&&box.x>=-1&&box.x+box.width<=321&&box.y<740&&box.y+box.height>0,selector+' must remain reachable at320px');boxes.push({selector,...box});
      }
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);await page.screenshot({path:path.join(directory,'native-generator-320.png')});await jsonNew(path.join(directory,'layout-320.json'),{viewport:{width:320,height:740},boxes,document_overflow:false});
      assert.equal(await page.locator('#seed-url').inputValue(),seedBefore);assert.deepEqual(errors,[]);assert.deepEqual(external,[]);assert(requests.every(row=>row.delivered));
      check('08 actual Chrome generator all date modes options API-equal downloads draft refresh and320px access',{downloads:downloads.length});
    });
    await readInterval('native Chrome errors create no partial download and request fences are witnessed',async()=>{
      const good={...clone(options),max_urlset_uncompressed_bytes:4096,max_index_uncompressed_bytes:4096,max_total_output_bytes:65536};
      await invoke('error400-priority',{...clone(good),priority:'2.0'},400);
      await invoke('error400-date',{...clone(good),lastmod:{source:'provided',values:[{page_url:siteOrigin+'/start',value:'2026-02-30',reference:'explicit invalid date control'}]}},400);
      await invoke('error413-budget',{...clone(good),max_total_output_bytes:1},413);
      await invoke('error422-no-eligible',{...clone(good),public_base_url:offlineOrigin+'/outside/'},422);
      assert.deepEqual(new Set(cases.filter(row=>row.status>=400).map(row=>row.status)),new Set([400,413,422]));assert.equal(downloads.length,3);await quiet();await bounded(Promise.all(responseTasks),budgets.request,'Browser response capture');
      assert.deepEqual(errors,[]);assert.deepEqual(external,[]);assert(requests.every(row=>row.delivered));const beforeJobs=await json('/api/crawls');oraclePhase=true;
      try{await page.evaluate(async origin=>{try{await fetch('/api/crawls',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({seed_urls:[origin+'/must-not-crawl']})});}catch{}try{await fetch(origin+'/browser-must-not-request');}catch{}},offlineOrigin);}finally{oraclePhase=false;}
      await quiet();assert.equal(oracles.length,2);assert(oracles.every(row=>row.delivered===false));assert.deepEqual(new Set(oracles.map(row=>row.reason)),new Set(['mutation_blocked','external_blocked']));assert.deepEqual(await json('/api/crawls'),beforeJobs);assert.equal(external.filter(row=>!row.oracle_phase).length,0);assert(requests.every(row=>row.delivered));
      check('09 actual Chrome400/413/422 no partial ZIP downloads and witnessed mutation/external fences',{error_cases:cases.filter(row=>row.status>=400).length,downloads:downloads.length});
    });
  }catch(error){mainError=error;}
  finally{
    if(browserProcess&&(browserProcess.exitCode!==null||browserProcess.signalCode!==null))producer.unexpected_close=true;
    producer.planned_close=true;
    if(context){try{await bounded(context.close(),budgets.stop,'Owned Chrome context close');producer.context_closed=true;}catch(error){mainError??=error;}}
    if(browser){try{await bounded(browser.close(),budgets.stop,'Owned Chrome disconnect');producer.browser_disconnected=true;}catch(error){mainError??=error;}}
    if(server){try{await bounded(server.close(),budgets.stop,'Owned Chrome server close');if(processClosed)await bounded(processClosed,budgets.stop,'Actual Chrome process close event');}catch(error){mainError??=error;producer.forced=true;await bounded(server.kill(),budgets.stop,'Force own Chrome cleanup').catch(()=>{});if(processClosed)await bounded(processClosed,budgets.stop,'Forced actual Chrome close').catch(()=>{});}}
    try{await bounded(Promise.all(responseTasks),budgets.stop,'Browser response captures after close');}catch(error){mainError??=error;}
    if(producer.pid!==null&&!producer.actual_close_event)mainError??=Error('Owned Chrome process closure was not witnessed');
    if(producer.unexpected_close)mainError??=Error('Owned Chrome exited before planned closure');
    if(!mainError&&downloads.length!==3)mainError=Error('Unexpected extra or missing actual Chrome download');
    try{assert.equal(await hashFile(chrome),producer.chrome_sha256);}catch(error){mainError??=error;}
    await jsonNew(path.join(directory,'browser-proof.json'),{scope:'Actual native Chrome; no fulfilled/mocked responses; saved job only; all downloads compare byte-for-byte with separately requested native API ZIP',producer,cases,requests,responses,downloads,blocked_external_attempts:external,oracle_controls:oracles,javascript_errors:errors,error:mainError?{message:mainError.message,stack:mainError.stack}:null});
  }
  if(mainError)throw mainError;assert(producer.actual_close_event&&producer.server_close_event&&!producer.forced);return {producer,cases,requests,responses,downloads,external,oracles,javascript_errors:errors};
}
