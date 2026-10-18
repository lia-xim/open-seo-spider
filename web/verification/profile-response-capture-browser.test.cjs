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
function deferred(){let resolve;const promise=new Promise(done=>resolve=done);return {promise,resolve};}
function protectedResponse(response,counter){return new Proxy(response,{get(target,key){if(['body','text','json'].includes(key))return ()=>{counter.calls++;throw Error('CDP response body must never be used');};const value=Reflect.get(target,key,target);return typeof value==='function'?value.bind(target):value;}});}
async function ownedSite(){
  const requests=[],documents=[],streams=new Map(),arrivals=new Map();
  const server=http.createServer(async(request,response)=>{
    if(request.method==='GET'&&request.url==='/'){documents.push(request.url);response.writeHead(200,{'content-type':'text/html','cache-control':'no-store'});return response.end('<!doctype html><title>Owned clone transport controls</title>');}
    if(request.method==='GET'&&request.url==='/favicon.ico'){response.writeHead(204);return response.end();}
    const chunks=[];for await(const chunk of request)chunks.push(chunk);const body=Buffer.concat(chunks).toString();
    requests.push({method:request.method,url:request.url,body,sha256:sha(Buffer.from(body))});
    const mode=body?JSON.parse(body).case:'get';
    response.writeHead(mode==='association-b'?201:200,{'content-type':'application/json; charset=utf-8','x-owned-transport':'actual-http-fixture'});
    if(['abort','error','stall'].includes(mode)){
      const closed=deferred();response.once('close',()=>closed.resolve());streams.set(mode,{response,closed:closed.promise});
      response.write(mode==='stall'?originalBytes.subarray(0,5):'{"incomplete":');response.flushHeaders();arrivals.get(mode)?.resolve();return;
    }
    response.end(mode==='association-b'?alternateBytes:originalBytes);
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
  if([8787,8790].includes(server.address().port)){await new Promise(resolve=>server.close(resolve));throw Error('Owned fixture unexpectedly bound a reserved application port');}
  return {server,requests,documents,streams,origin:'http://127.0.0.1:'+server.address().port,
    expect(mode){const item=deferred();arrivals.set(mode,item);return item.promise;},
    async close(){for(const stream of streams.values())stream.response.destroy();server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}};
}
async function beginFetch(page,body,{method='POST',url=route,abort=false,implicit=false}={}){
  const responsePending=page.waitForResponse(response=>new URL(response.url()).pathname===url&&response.request().method()===method);
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
    response_sha256:sha(expected),response_bytes:expected.length,headers_sha256:sha(Buffer.from(JSON.stringify(record.headers))),fixture_limits:history.limits,http_replay:false,buffer_released:true});
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
