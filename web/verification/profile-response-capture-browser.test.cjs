'use strict';
// Real Chromium transport controls only. This is an explicitly synthetic,
// owned HTTP fixture; it does not replace native configuration-profile E2E.
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),http=require('node:http');
const {createHash}=require('node:crypto');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const capture=require('./profile-response-capture.cjs');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const route='/api/config-profiles/inspect';
const originalBytes=Buffer.from(' \n{"transport_fixture":"雪 – exact UTF-8","u64":9007199254740993,"escaped":"quote \\\" and slash \\\\"}\r\n');
const alternateBytes=Buffer.concat([originalBytes,Buffer.from('\t')]);
const conflictBytes=Buffer.from('{"error":"owned_transport_CAS_conflict"}');
function deferred(){let resolve;const promise=new Promise(done=>resolve=done);return {promise,resolve};}
function protectedResponse(response,counter){return new Proxy(response,{get(target,key){if(['body','text','json'].includes(key))return ()=>{counter.calls++;throw Error('CDP response body must never be used');};const value=Reflect.get(target,key,target);return typeof value==='function'?value.bind(target):value;}});}
async function ownedSite(){
  const requests=[],documents=[],streams=new Map(),arrivals=new Map();
  const server=http.createServer(async(request,response)=>{
    if(request.method==='GET'&&request.url==='/'){documents.push(request.url);response.writeHead(200,{'content-type':'text/html','cache-control':'no-store'});return response.end('<!doctype html><title>Owned clone transport controls</title>');}
    if(request.method==='GET'&&request.url==='/favicon.ico'){response.writeHead(204);return response.end();}
    const chunks=[];for await(const chunk of request)chunks.push(chunk);const body=Buffer.concat(chunks).toString();
    requests.push({method:request.method,url:request.url,body,sha256:sha(Buffer.from(body))});
    const metadataTarget=/^\/api\/crawls\/[0-9a-f-]+\/screenshot\?/.test(request.url)?new URL(request.url,'http://fixture.invalid').searchParams.get('url'):null;
    const mode=body?JSON.parse(body).case:metadataTarget&&new URL(metadataTarget).pathname==='/held-a'?'abort':'get';
    response.writeHead(mode==='association-b'?201:mode==='owned-conflict'?409:200,{'content-type':'application/json; charset=utf-8','x-owned-transport':'actual-http-fixture'});
    if(['abort','error','stall'].includes(mode)){
      const closed=deferred();response.once('close',()=>closed.resolve());streams.set(mode,{response,closed:closed.promise});
      response.write(mode==='stall'?originalBytes.subarray(0,5):'{"incomplete":');response.flushHeaders();arrivals.get(mode)?.resolve();return;
    }
    response.end(mode==='association-b'?alternateBytes:mode==='owned-conflict'?conflictBytes:originalBytes);
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
  if([8787,8790].includes(server.address().port)){await new Promise(resolve=>server.close(resolve));throw Error('Owned fixture unexpectedly bound a reserved application port');}
  return {server,requests,documents,streams,origin:'http://127.0.0.1:'+server.address().port,
    expect(mode){const item=deferred();arrivals.set(mode,item);return item.promise;},
    async close(){for(const stream of streams.values())stream.response.destroy();server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}};
}
async function beginFetch(page,body,{method='POST',url=route,abort=false,implicit=false}={}){
  const target=new URL(url,page.url());target.hash='';
  const responsePending=page.waitForResponse(response=>response.url()===target.href&&response.request().method()===method);
  await page.evaluate(({body,method,url,abort,implicit})=>{
    const controller=new AbortController();window.ownedController=controller;
    const pending=implicit?fetch(url):fetch(url,{method,headers:method==='GET'?{}:{'content-type':'application/json'},body:method==='GET'?undefined:body,...(abort?{signal:controller.signal}:{})});
    window.ownedOriginal=pending.then(async response=>{
      const before=response.bodyUsed,bytes=[...new Uint8Array(await response.arrayBuffer())];
      return {ok:true,status:response.status,header:response.headers.get('x-owned-transport'),before,after:response.bodyUsed,response_instance:response instanceof Response,bytes};
    }).catch(error=>({ok:false,name:error.name,message:error.message}));
  },{body,method,url,abort,implicit});
  return responsePending;
}
async function original(page,expected=originalBytes,status=200){const result=await page.evaluate(()=>window.ownedOriginal);assert.equal(result.ok,true);assert.equal(result.status,status);assert.equal(result.before,false);assert.equal(result.after,true);assert.equal(result.response_instance,true);assert.equal(result.header,'actual-http-fixture');assert.deepEqual(Buffer.from(result.bytes),expected);return result;}
async function boundEvidence(page,record,mark,body,site,expected=originalBytes,status=200,{method='POST',path=route}={}){
  const evidence=record.evidence,history=await capture.history(page),request=site.requests.findLast(item=>item.body===body&&item.method===method&&item.url===path);
  assert(request);assert.equal(request.method,method);assert.equal(request.url,path);assert.equal(request.sha256,sha(Buffer.from(body)));
  assert.deepEqual(record.bytes,expected);assert.equal(record.status,status);assert.equal(record.headers['x-owned-transport'],'actual-http-fixture');
  assert.deepEqual(evidence,{engine:capture.ENGINE,source:'actual_browser_fetch_response_clone',epoch:mark.epoch,after_sequence:mark.afterSequence,
    sequence:evidence.sequence,method,url:site.origin+path,request_sha256:sha(Buffer.from(body)),request_bytes:Buffer.byteLength(body),
    response_sha256:sha(expected),response_bytes:expected.length,headers_sha256:sha(Buffer.from(JSON.stringify(record.headers))),fixture_limits:history.limits,...(history.policy?{capture_policy:history.policy}:{}),http_replay:false,buffer_released:true});
  assert(Number.isSafeInteger(evidence.sequence)&&evidence.sequence>mark.afterSequence);assert.equal(history.engine,capture.ENGINE);assert.equal(history.epoch,mark.epoch);assert.equal(history.origin,site.origin);
  const actual=history.records.filter(item=>item.sequence===evidence.sequence);assert.equal(actual.length,1);
  assert.deepEqual(actual[0],{sequence:evidence.sequence,method,url:site.origin+path,requestSha256:sha(Buffer.from(body)),requestBytes:Buffer.byteLength(body),status,
    responseSha256:sha(expected),responseBytes:expected.length,phase:'released',error:null,headerNames:Object.keys(record.headers)});
  assert.equal(JSON.stringify(history).includes('private-input-sentinel'),false);assert.equal(JSON.stringify(history).includes('actual-http-fixture'),false);
  return evidence;
}

test('actual Chromium Response.clone profile transport controls (not native product acceptance)',{timeout:60000},async t=>{
  const executable=process.env.OPEN_SEO_BROWSER_PATH||process.env.SPIDER_CHROME;assert(executable&&fs.statSync(executable).isFile(),'Explicit pinned Chromium required');
  const site=await ownedSite();let browser,context;
  t.after(async()=>{try{if(context)await context.close();}finally{try{if(browser)await browser.close();}finally{await site.close();}}});
  browser=await chromium.launch({headless:true,executablePath:executable});context=await browser.newContext();
  const pageErrors=[];context.on('page',owned=>owned.on('pageerror',error=>pageErrors.push({url:owned.url(),message:error.message})));
  const page=await context.newPage();await capture.install(page);await page.goto(site.origin);const cdp={calls:0};assert.deepEqual((await capture.history(page)).limits,capture.DEFAULT_LIMITS);

  await t.test('actual UTF8/u64 bytes and original Response remain independently consumable without CDP',async()=>{
    const mark=await capture.mark(page),body=JSON.stringify({case:'exact',profile_text:'{"delay_ms":9007199254740993,"label":"雪","private":"private-input-sentinel"}'}),before=site.requests.length;
    const response=await beginFetch(page,body),record=await capture.capture(page,protectedResponse(response,cdp),mark);
    assert.deepEqual(record.bytes,originalBytes);assert.equal(record.status,200);assert.equal(record.headers['x-owned-transport'],'actual-http-fixture');
    await original(page);assert.equal(site.requests.length,before+1);assert.equal(cdp.calls,0);
    await boundEvidence(page,record,mark,body,site);
  });

  await t.test('two distinct bodies on one route bind their own sequence and actual response',async()=>{
    const mark=await capture.mark(page),before=site.requests.length,bodyA=JSON.stringify({case:'association-a'}),bodyB=JSON.stringify({case:'association-b'});
    const first=await beginFetch(page,bodyA);await original(page);const second=await beginFetch(page,bodyB);await original(page,alternateBytes,201);
    const b=await boundEvidence(page,await capture.capture(page,protectedResponse(second,cdp),mark),mark,bodyB,site,alternateBytes,201);
    const a=await boundEvidence(page,await capture.capture(page,protectedResponse(first,cdp),mark),mark,bodyA,site);
    assert.equal(a.sequence,mark.afterSequence+1);assert.equal(b.sequence,mark.afterSequence+2);assert.equal(site.requests.length,before+2);assert.equal(cdp.calls,0);
  });

  await t.test('implicit catalog-list and explicit catalog-item GET bind exact bytes and absent request bodies',async()=>{
    const before=site.requests.length;
    for(const [path,implicit]of [['/api/config-profile-catalog',true],['/api/config-profile-catalog/8e5a6e4e-4b60-46c1-9be4-6d6f64ff6503',false]]){
      const mark=await capture.mark(page),response=await beginFetch(page,'',{method:'GET',url:path,implicit});
      assert.equal(response.request().method(),'GET');assert.equal(response.request().postDataBuffer(),null);assert.equal(response.request().postData(),null);
      const record=await capture.capture(page,protectedResponse(response,cdp),mark);await original(page);
      const evidence=await boundEvidence(page,record,mark,'',site,originalBytes,200,{method:'GET',path});assert.equal(evidence.sequence,mark.afterSequence+1);
      assert.equal(evidence.request_sha256,sha(Buffer.alloc(0)));assert.equal(evidence.request_bytes,0);assert.equal(cdp.calls,0);
    }
    assert.equal(site.requests.length,before+2);
  });

  await t.test('same request body repeated after one marker is refused as ambiguous',async()=>{
    const mark=await capture.mark(page),body=JSON.stringify({case:'duplicate'}),before=site.requests.length;
    const first=await beginFetch(page,body);await original(page);const second=await beginFetch(page,body);await original(page);
    await assert.rejects(capture.capture(page,protectedResponse(first,cdp),mark),/ambig|multiple|duplicate/i);
    await assert.rejects(capture.capture(page,protectedResponse(second,cdp),mark),/ambig|multiple|duplicate/i);
    assert.equal(site.requests.length,before+2);assert.equal(cdp.calls,0);
  });

  await t.test('marker from the prior real document is refused after reload',async()=>{
    const old=await capture.mark(page);await page.reload();const current=await capture.mark(page);assert.notEqual(current.epoch,old.epoch);
    const response=await beginFetch(page,JSON.stringify({case:'reload'}));await original(page);
    await assert.rejects(capture.capture(page,protectedResponse(response,cdp),old),/epoch|document|stale|reload/i);
    assert.deepEqual((await capture.capture(page,protectedResponse(response,cdp),current)).bytes,originalBytes);assert.equal(cdp.calls,0);
  });

  for(const mode of ['abort','error'])await t.test('actual '+mode+' stream cannot become a successful captured body',async()=>{
    const mark=await capture.mark(page),arrival=site.expect(mode),before=site.requests.length;
    const response=await beginFetch(page,JSON.stringify({case:mode}),{abort:true});await arrival;
    const rejected=assert.rejects(capture.capture(page,protectedResponse(response,cdp),mark),/abort|error|stream|failed|body/i);
    if(mode==='abort')await page.evaluate(()=>window.ownedController.abort());else site.streams.get(mode).response.destroy();
    const consumed=await page.evaluate(()=>window.ownedOriginal);assert.equal(consumed.ok,false);if(mode==='abort')assert.equal(consumed.name,'AbortError');
    await rejected;await site.streams.get(mode).closed;assert.equal(site.requests.length,before+1);assert.equal(cdp.calls,0);
  });

  await t.test('GET, unrelated route and native-start route are unmodified and never captured or replayed',async()=>{
    const before=site.requests.length,history=await capture.history(page);
    for(const [url,method]of [[route,'GET'],['/unrelated','POST'],['/api/crawls/from-profile','POST']]){
      const mark=await capture.mark(page),response=await beginFetch(page,JSON.stringify({case:'excluded'}),{url,method});await original(page);
      await assert.rejects(capture.capture(page,protectedResponse(response,cdp),mark));
    }
    assert.equal(site.requests.length,before+3);assert.equal(cdp.calls,0);assert.equal((await capture.history(page)).sequence,history.sequence);
  });

  await t.test('real lowered clone byte cap refuses capture while original Response still succeeds',async()=>{
    const limited=await context.newPage();await capture.install(limited,{maxResponseBytes:originalBytes.length-1});await limited.goto(site.origin);
    const mark=await capture.mark(limited),before=site.requests.length,response=await beginFetch(limited,JSON.stringify({case:'bounded'}));
    await assert.rejects(capture.capture(limited,protectedResponse(response,cdp),mark),/response_byte_limit/);await original(limited);
    const history=await capture.history(limited);assert.equal(history.records.length,1);assert.equal(history.records[0].phase,'error');assert.equal(history.records[0].error,'response_byte_limit');
    assert.equal(history.aggregateBytes,0);assert(history.peakAggregateBytes<=history.limits.maxAggregateBytes);assert.equal(site.requests.length,before+1);assert.equal(cdp.calls,0);await limited.goto('about:blank');assert.deepEqual(pageErrors,[]);await limited.close();
  });

  await t.test('stalled clone read times out without aborting the original HTTP response',async()=>{
    const limited=await context.newPage();await capture.install(limited,{maxReadMs:250});await limited.goto(site.origin);
    const mark=await capture.mark(limited),arrival=site.expect('stall'),before=site.requests.length,response=await beginFetch(limited,JSON.stringify({case:'stall'}));await arrival;
    await assert.rejects(capture.capture(limited,protectedResponse(response,cdp),mark),/response_read_timeout/);
    assert.equal(site.streams.get('stall').response.destroyed,false,'Observer timeout must leave original HTTP connection alive');
    site.streams.get('stall').response.end(originalBytes.subarray(5));await original(limited);await site.streams.get('stall').closed;
    const history=await capture.history(limited);assert.equal(history.records.length,1);assert.equal(history.records[0].error,'response_read_timeout');assert.equal(history.aggregateBytes,0);
    assert.equal(site.requests.length,before+1);assert.equal(cdp.calls,0);await limited.goto('about:blank');assert.deepEqual(pageErrors,[]);await limited.close();
  });

  await t.test('real about:blank transitions are error-free and each HTTP return creates a fresh capture epoch',async()=>{
    let prior=await capture.mark(page);
    for(let cycle=0;cycle<2;cycle++){
      await page.goto('about:blank');assert.equal(page.url(),'about:blank');assert.deepEqual(pageErrors,[]);
      await assert.rejects(capture.mark(page),/not installed/i);
      await page.goto(site.origin);const current=await capture.mark(page);assert.notEqual(current.epoch,prior.epoch);assert.equal(current.afterSequence,0);
      assert.equal((await capture.history(page)).origin,site.origin);assert.deepEqual(pageErrors,[]);prior=current;
    }
    await page.goto('about:blank');assert.deepEqual(pageErrors,[]);await page.close();assert.deepEqual(pageErrors,[]);
  });
  assert.equal(site.requests.length,15,'Exact owned API request count; no recorder HTTP replay');assert.equal(site.documents.length,6,'Only initial/reload/two bounded controls/two returns from about:blank');assert.equal(cdp.calls,0);assert.deepEqual(pageErrors,[]);
});


// All original pure-policy controls above remain. This additional owned HTTP
// fixture verifies only opt-in transport; no native catalog/job success is implied.
// Exact Promise/Response reference identity is additionally asserted by VM tests.
test('actual Chromium explicitly owned JSON mutation capture remains observational and origin bound',{timeout:60000},async t=>{
 const executable=process.env.OPEN_SEO_BROWSER_PATH||process.env.SPIDER_CHROME;assert(executable&&fs.statSync(executable).isFile(),'Explicit pinned Chromium required');
 const site=await ownedSite();let other,browser,context;const pageErrors=[],cdp={calls:0};
 t.after(async()=>{try{if(context)await context.close();}finally{try{if(browser)await browser.close();}finally{await Promise.all([site.close(),...(other?[other.close()]:[])]);}}});
 other=await ownedSite();
 browser=await chromium.launch({headless:true,executablePath:executable});context=await browser.newContext();context.on('page',page=>page.on('pageerror',error=>pageErrors.push({url:page.url(),message:error.message})));
 const catalog='/api/config-profile-catalog',item=catalog+'/10101010-1111-4222-8333-444444444444',saved='/api/crawls/10101010-1111-4222-8333-444444444444/config-profile';
 const routes=[['POST',catalog],['PUT',item],['DELETE',item],['POST',item+'/rename'],['POST','/api/crawls/from-profile'],['GET',saved]],policy={name:capture.OWNED_POLICY,origin:site.origin};
 const pure=await context.newPage();await capture.install(pure);await pure.goto(site.origin);
 await t.test('default pure capture still excludes every extra operation while actual HTTP and original responses succeed once',async()=>{
  const before=site.requests.length;
  for(const [method,path]of routes){const mark=await capture.mark(pure),body=method==='GET'?'':JSON.stringify({case:'owned-default-excluded',path}),response=await beginFetch(pure,body,{method,url:path});await original(pure);assert.equal(capture.canCapture(method,site.origin+path),false);await assert.rejects(capture.capture(pure,protectedResponse(response,cdp),mark),/Only allowlisted/);}
  assert.equal(site.requests.length,before+routes.length);assert.equal((await capture.history(pure)).sequence,0);assert.equal((await capture.history(pure)).policy,null);assert.equal(cdp.calls,0);
 });
 const owned=await context.newPage();await capture.install(owned,{},policy);await owned.goto(site.origin);
 await t.test('explicit owned policy captures each exact operation plus409 with full byte/header/status/request binding and no replay',async()=>{
  const before=site.requests.length,cases=[...routes.map(([method,path])=>({method,path,mode:'owned-success',status:200,bytes:originalBytes})),{method:'PUT',path:item,mode:'owned-conflict',status:409,bytes:conflictBytes}];
  for(const [index,{method,path,mode,status,bytes}]of cases.entries()){
   const body=method==='GET'?'':JSON.stringify({case:mode,operation:index,header_values:{'X-Owned':'private-input-sentinel'}}),mark=await capture.mark(owned),count=site.requests.length,response=await beginFetch(owned,body,{method,url:path});assert(capture.canCapture(method,site.origin+path,policy));
   const record=await capture.capture(owned,protectedResponse(response,cdp),mark);await original(owned,bytes,status);await boundEvidence(owned,record,mark,body,site,bytes,status,{method,path});assert.equal(site.requests.length,count+1);assert.deepEqual(record.evidence.capture_policy,policy);assert.equal(cdp.calls,0);
  }
  const history=await capture.history(owned);assert.equal(site.requests.length,before+cases.length);assert.deepEqual(history.policy,policy);assert.equal(history.aggregateBytes,0);assert(history.records.every(row=>row.phase==='released'));assert(!JSON.stringify(history).includes('private-input-sentinel'));
 });
 await t.test('wrong methods, path shapes and query or fragment requests cannot become successful owned captures',async()=>{
  const before=site.requests.length,history=await capture.history(owned),cases=[['PATCH',item],['POST',item],['POST',catalog+'?wrong=1'],['POST',catalog+'#']];
  for(const [method,path]of cases){const mark=await capture.mark(owned),response=await beginFetch(owned,JSON.stringify({case:'owned-wrong-route'}),{method,url:path});await original(owned);assert.equal(capture.canCapture(method,site.origin+path,policy),false);await assert.rejects(capture.capture(owned,protectedResponse(response,cdp),mark));}
  assert.equal(site.requests.length,before+cases.length);assert.equal((await capture.history(owned)).sequence,history.sequence);assert.equal(cdp.calls,0);
 });
 await t.test('a different owned HTTP document is not wrapped by a policy for the first origin',async()=>{
  const foreign=await context.newPage();await capture.install(foreign,{},policy);await foreign.goto(other.origin);await assert.rejects(capture.mark(foreign),/not installed/);
  const before=other.requests.length,response=await beginFetch(foreign,JSON.stringify({case:'owned-other-origin'}),{method:'POST',url:catalog});await original(foreign);assert.equal(other.requests.length,before+1);assert.equal(capture.canCapture('POST',other.origin+catalog,policy),false);assert.equal(response.status(),200);await foreign.goto('about:blank');await foreign.close();
 });
 assert.equal(site.requests.length,17,'Six excluded, six owned+one409 and four wrong-route requests, each exactly once');assert.equal(other.requests.length,1);assert.equal(site.documents.length,2);assert.equal(other.documents.length,1);assert.equal(cdp.calls,0);assert.deepEqual(pageErrors,[]);await pure.goto('about:blank');await owned.goto('about:blank');assert.deepEqual(pageErrors,[]);
});


test('actual Chromium screenshot-suite JSON policy captures original query responses and refuses obsolete metadata without CDP bodies',{timeout:60000},async t=>{
 const executable=process.env.OPEN_SEO_BROWSER_PATH||process.env.SPIDER_CHROME;assert(executable&&fs.statSync(executable).isFile(),'Explicit pinned Chromium required');const site=await ownedSite();let browser,context;const errors=[],cdp={calls:0};
 t.after(async()=>{try{if(context)await context.close();}finally{try{if(browser)await browser.close();}finally{await site.close();}}});browser=await chromium.launch({headless:true,executablePath:executable});context=await browser.newContext();context.on('page',page=>page.on('pageerror',error=>errors.push(error.message)));
 const id='10101010-1111-4222-8333-444444444444',metadata='/api/crawls/'+id+'/screenshot',policy={name:capture.OWNED_SCREENSHOT_POLICY,origin:site.origin},urlFor=(value,ordinal)=>metadata+'?'+new URLSearchParams({url:site.origin+value,...(ordinal===undefined?{}:{ordinal})}),page=await context.newPage();await capture.install(page,{maxResponseBytes:65536},policy);await page.goto(site.origin);
 await t.test('three profile operations and metadata with optional0 capture actual bytes/status/headers/request once',async()=>{
  const routes=[['POST','/api/config-profiles/inspect'],['POST','/api/crawls/from-profile'],['GET','/api/crawls/'+id+'/config-profile'],['GET',urlFor('/page-a')],['GET',urlFor('/page-b','0')]],before=site.requests.length;
  for(const [method,path]of routes){const body=method==='GET'?'':JSON.stringify({case:'screenshot-suite-original'}),mark=await capture.mark(page),response=await beginFetch(page,body,{method,url:path}),found=await capture.capture(page,protectedResponse(response,cdp),mark);await original(page);await boundEvidence(page,found,mark,body,site,originalBytes,200,{method,path});assert.deepEqual(found.evidence.capture_policy,policy);assert.equal(found.evidence.fixture_limits.maxResponseBytes,65536);}
  assert.equal(site.requests.length,before+5);assert.equal(cdp.calls,0);
 });
 await t.test('default and owned profile policies still reject real metadata queries and the suite rejects PNG/extra query',async()=>{
  for(const existing of [null,{name:capture.OWNED_POLICY,origin:site.origin}]){const other=await context.newPage();await capture.install(other,{},existing);await other.goto(site.origin);const mark=await capture.mark(other),path=urlFor('/old-policy'),response=await beginFetch(other,'',{method:'GET',url:path});await original(other);assert.equal(capture.canCapture('GET',site.origin+path,existing),false);await assert.rejects(capture.capture(other,protectedResponse(response,cdp),mark),/Only allowlisted/);assert.equal((await capture.history(other)).sequence,0);await other.goto('about:blank');await other.close();}
  for(const path of [urlFor('/bad')+'&extra=1',metadata+'.png?url='+encodeURIComponent(site.origin+'/bad')]){const mark=await capture.mark(page),response=await beginFetch(page,'',{method:'GET',url:path});await original(page);await assert.rejects(capture.capture(page,protectedResponse(response,cdp),mark),/Only allowlisted/);}
 });
 await t.test('genuinely held A clone can abort while exact later B succeeds once, and current captured error remains observable',async()=>{
  const held=site.expect('abort'),oldMark=await capture.mark(page),pathA=urlFor('/held-a'),responseA=await beginFetch(page,'',{method:'GET',url:pathA,abort:true});await held;await page.evaluate(()=>{window.oldMetadataController=window.ownedController;window.oldMetadataOriginal=window.ownedOriginal;});
  const mark=await capture.mark(page),pathB=urlFor('/current-b'),before=site.requests.length,responseB=await beginFetch(page,'',{method:'GET',url:pathB});await page.evaluate(()=>window.oldMetadataController.abort());const found=await capture.capture(page,protectedResponse(responseB,cdp),mark);await original(page);await boundEvidence(page,found,mark,'',site,originalBytes,200,{method:'GET',path:pathB});assert.equal(site.requests.length,before+1);const old=await page.evaluate(()=>window.oldMetadataOriginal);assert.equal(old.ok,false);await assert.rejects(capture.capture(page,protectedResponse(responseA,cdp),oldMark),/response_read_failed/);assert.equal(cdp.calls,0);
 });
 await t.test('reload invalidates old metadata marks and aboutblank never becomes a capture document',async()=>{
  const old=await capture.mark(page);await page.reload();const mark=await capture.mark(page);assert.notEqual(mark.epoch,old.epoch);const path=urlFor('/after-reload'),response=await beginFetch(page,'',{method:'GET',url:path});await assert.rejects(capture.capture(page,protectedResponse(response,cdp),old),/document_epoch_changed/);await capture.capture(page,protectedResponse(response,cdp),mark);await original(page);await page.goto('about:blank');await assert.rejects(capture.mark(page),/not installed/);assert.deepEqual(errors,[]);assert.equal(cdp.calls,0);
 });
});
