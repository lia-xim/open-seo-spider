'use strict';
// Test-fixture limits only. These are neither product limits nor physical RSS
// claims: merging and hashing can temporarily hold a second bounded buffer.
const assert=require('node:assert/strict'),crypto=require('node:crypto');
const ENGINE='open-seo-spider/test-profile-response-capture-v1';
const KEY='__openSeoProfileResponseCaptureV1';
const OWNED_POLICY='open-seo-spider/owned-profile-json-v1';
const OWNED_SCREENSHOT_POLICY='open-seo-spider/owned-screenshot-suite-json-v1';
const UUID='[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}';
const PURE_ROUTES=Object.freeze([['POST','^/api/config-profiles/inspect$'],['GET','^/api/config-profile-catalog(?:/'+UUID+')?$']]);
const OWNED_ROUTES=Object.freeze([['POST','^/api/config-profile-catalog$'],['PUT','^/api/config-profile-catalog/'+UUID+'$'],['DELETE','^/api/config-profile-catalog/'+UUID+'$'],['POST','^/api/config-profile-catalog/'+UUID+'/rename$'],['POST','^/api/crawls/from-profile$'],['GET','^/api/crawls/'+UUID+'/config-profile$']]);
const SCREENSHOT_ROUTES=Object.freeze([['POST','^/api/config-profiles/inspect$'],['POST','^/api/crawls/from-profile$'],['GET','^/api/crawls/'+UUID+'/config-profile$'],['GET','^/api/crawls/'+UUID+'/screenshot$']]);
function screenshotQuery(location){
  if(location.href!==location.origin+location.pathname+location.search||!location.search)return false;
  const entries=[...location.searchParams];
  if(entries.length<1||entries.length>2||entries[0][0]!=='url'||entries.length===2&&entries[1][0]!=='ordinal')return false;
  if(new URLSearchParams(entries).toString()!==location.search.slice(1))return false;
  const value=entries[0][1];if(!value||new TextEncoder().encode(value).length>8192||/[\u0000-\u001f\u007f]/.test(value))return false;
  let selected;try{selected=new URL(value);}catch(_){return false;}
  if(!['http:','https:'].includes(selected.protocol)||selected.href!==value||selected.username||selected.password||selected.hash)return false;
  if(entries.length===2){const ordinal=entries[1][1];if(!/^(0|[1-9][0-9]*)$/.test(ordinal)||!Number.isSafeInteger(Number(ordinal)))return false;}
  return true;
}
function policyFor(value){
  if(value===undefined||value===null)return null;
  assert(value&&typeof value==='object'&&!Array.isArray(value),'Explicit owned capture policy object required');
  assert.deepEqual(Object.keys(value).sort(),['name','origin'],'Owned policy permits only name and exact origin');
  assert([OWNED_POLICY,OWNED_SCREENSHOT_POLICY].includes(value.name),'Unknown owned capture policy');assert.equal(typeof value.origin,'string','Owned origin must be a string');
  const url=new URL(value.origin),port=Number(url.port);
  assert(['http:','https:'].includes(url.protocol)&&['127.0.0.1','[::1]','localhost'].includes(url.hostname),'Owned policy requires an explicit loopback HTTP origin');
  assert(url.origin===value.origin&&url.href===value.origin+'/'&&url.port&&port>1024&&![8787,8790].includes(port),'Expected canonical non-reserved owned origin without path, credentials, query or fragment');
  return Object.freeze({name:value.name,origin:value.origin});
}
function routesFor(policy){return policy?.name===OWNED_SCREENSHOT_POLICY?[...SCREENSHOT_ROUTES]:policy?[...PURE_ROUTES,...OWNED_ROUTES]:[...PURE_ROUTES];}
function canCapture(method,url,policy){
  const selected=policyFor(policy);if(typeof method!=='string'||typeof url!=='string')return false;
  let location;try{location=new URL(url);}catch(_){return false;}
  if(location.href!==url||selected&&location.origin!==selected.origin)return false;
  if(selected?.name===OWNED_SCREENSHOT_POLICY&&new RegExp('^/api/crawls/'+UUID+'/screenshot$').test(location.pathname)){if(!screenshotQuery(location))return false;}
  else if(location.href!==location.origin+location.pathname)return false;
  return routesFor(selected).some(([verb,pattern])=>verb===method&&new RegExp(pattern).test(location.pathname));
}
const DEFAULT_LIMITS=Object.freeze({maxResponseBytes:2097152,maxAggregateBytes:16777216,maxRecords:64,maxRequestBytes:2097152,transferChunkBytes:32768,maxReadMs:10000});
const installed=new WeakMap();
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function limitsFor(options={}){assert(options&&typeof options==='object'&&!Array.isArray(options));for(const key of Object.keys(options))assert(Object.hasOwn(DEFAULT_LIMITS,key),'Unknown capture fixture limit');const limits={...DEFAULT_LIMITS,...options};for(const[key,value]of Object.entries(limits))assert(Number.isSafeInteger(value)&&value>0&&value<=DEFAULT_LIMITS[key],'Capture limits may only be lowered');return limits;}
function normalizedHeaders(value){const rows=Object.entries(value).map(([name,text])=>[name.toLowerCase(),String(text)]).sort(([a],[b])=>a<b?-1:a>b?1:0);assert.equal(new Set(rows.map(row=>row[0])).size,rows.length,'Duplicate normalized response header');return Object.fromEntries(rows);}

// This exact function is installed in each real document and exercised by the
// Node controls. It closes over no Node state and never issues a network call.
function installInDocument({key,engine,limits,policy,routes}) {
  // The page's lifecycle intentionally navigates through about:blank. Its
  // insecure/opaque crypto surface is not a profile API capture document.
  let documentUrl;try{documentUrl=new URL(location.href);}catch(_){return;}
  if(documentUrl.protocol!=='http:'&&documentUrl.protocol!=='https:')return;
  if(policy&&documentUrl.origin!==policy.origin)return;
  if(Object.prototype.hasOwnProperty.call(globalThis,key))return;
  const originalFetch=globalThis.fetch,then=Promise.prototype.then,encoder=new TextEncoder();
  const epoch=crypto.randomUUID(),origin=location.origin;
  const routeMatchers=routes.map(([method,pattern])=>({method,pattern:new RegExp(pattern)}));
  const screenshotQuery=function screenshotQuery(location){
  if(location.href!==location.origin+location.pathname+location.search||!location.search)return false;
  const entries=[...location.searchParams];
  if(entries.length<1||entries.length>2||entries[0][0]!=='url'||entries.length===2&&entries[1][0]!=='ordinal')return false;
  if(new URLSearchParams(entries).toString()!==location.search.slice(1))return false;
  const value=entries[0][1];if(!value||new TextEncoder().encode(value).length>8192||/[\u0000-\u001f\u007f]/.test(value))return false;
  let selected;try{selected=new URL(value);}catch(_){return false;}
  if(!['http:','https:'].includes(selected.protocol)||selected.href!==value||selected.username||selected.password||selected.hash)return false;
  if(entries.length===2){const ordinal=entries[1][1];if(!/^(0|[1-9][0-9]*)$/.test(ordinal)||!Number.isSafeInteger(Number(ordinal)))return false;}
  return true;
};
  const screenshotPolicy=policy?.name==='open-seo-spider/owned-screenshot-suite-json-v1';
  const screenshotPath=/^\/api\/crawls\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\/screenshot$/;
  const records=[];let sequence=0,used=0,peak=0,overflow=null;
  const digest=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),value=>value.toString(16).padStart(2,'0')).join('');
  const reserve=bytes=>{if(bytes>limits.maxAggregateBytes-used)return false;used+=bytes;peak=Math.max(peak,used);return true;};
  const release=bytes=>{used-=bytes;};
  const observePromise=(promise,success,failure)=>{try{const observed=then.call(promise,success,failure);then.call(observed,undefined,()=>{});}catch(_){}};
  const cancelReader=reader=>{try{observePromise(reader.cancel(),()=>{},()=>{});}catch(_){}};
  const finishError=(record,code)=>{record.error=record.error||code;if(record.retained){release(record.retained);record.retained=0;}record.buffer=null;record.phase='error';if(record.reader){cancelReader(record.reader);record.reader=null;}};
  function utf8Length(text){let length=0;for(let index=0;index<text.length;index++){const unit=text.charCodeAt(index);if(unit<128)length++;else if(unit<2048)length+=2;else if(unit>=0xd800&&unit<=0xdbff&&index+1<text.length&&text.charCodeAt(index+1)>=0xdc00&&text.charCodeAt(index+1)<=0xdfff){length+=4;index++;}else length+=3;if(length>limits.maxRequestBytes)return null;}return length;}
  function requestShape(input,init){
    // Nonmatching string URLs delegate without reading init at all. Do not
    // invoke arbitrary URL coercions, body getters, or consume Request streams.
    if(typeof input!=='string')return null;
    let parsed;try{parsed=new URL(input,location.href);}catch(_){return null;}
    if(parsed.origin!==origin)return null;
    if(screenshotPolicy&&screenshotPath.test(parsed.pathname)){if(!screenshotQuery(parsed))return null;}
    else if(parsed.href!==origin+parsed.pathname)return null;
    const url=parsed.href,matches=routeMatchers.filter(route=>route.pattern.test(parsed.pathname));
    if(!matches.length)return null;
    const method=init&&Object.getOwnPropertyDescriptor(init,'method');
    if(method&&(!Object.prototype.hasOwnProperty.call(method,'value')||typeof method.value!=='string')||!method&&init&&'method' in init){
      return {url,method:null,error:'unsupported_request_shape'};
    }
    const verb=method?method.value.toUpperCase():'GET';
    if(!matches.some(route=>route.method===verb))return null;
    const body=init&&Object.getOwnPropertyDescriptor(init,'body');
    if(verb==='GET'){
      if(body&&(!Object.prototype.hasOwnProperty.call(body,'value')||body.value!==undefined&&body.value!==null)||!body&&init&&'body' in init)return {url,method:verb,error:'unsupported_request_body'};
      return {url,method:verb,body:''};
    }
    if(!body||!Object.prototype.hasOwnProperty.call(body,'value')||typeof body.value!=='string')return {url,method:verb,error:'unsupported_request_body'};
    return {url,method:verb,body:body.value};
  }
  function begin(shape){
    sequence++;
    if(records.length>=limits.maxRecords){if(!overflow)overflow={firstSequence:sequence,lastSequence:sequence,count:0};overflow.lastSequence=sequence;overflow.count++;return null;}
    const record={sequence,method:shape.method,url:shape.url,requestSha256:null,requestBytes:null,status:null,headers:null,responseSha256:null,responseBytes:null,phase:'pending',error:shape.error||null,retained:0,buffer:null};records.push(record);
    if(record.error){record.phase='error';return record;}
    const count=utf8Length(shape.body);record.requestBytes=count;
    if(count===null){finishError(record,'request_byte_limit');return record;}
    if(!reserve(count)){finishError(record,'aggregate_byte_limit');return record;}
    const bytes=encoder.encode(shape.body);
    observePromise(digest(bytes),value=>{record.requestSha256=value;release(count);if(record.responseSha256&&!record.error)record.phase='complete';},()=>{release(count);finishError(record,'request_hash_failed');});
    return record;
  }
  async function readClone(record,clone){
    let reader;const chunks=[];let expire;
    const deadline=new Promise(resolve=>{expire=()=>{finishError(record,'response_read_timeout');resolve({captureTimeout:true});};});
    const timer=setTimeout(expire,limits.maxReadMs);
    try{
      const length=clone.headers.get('content-length');
      if(length!==null&&(!/^\d+$/.test(length)||Number(length)>limits.maxResponseBytes)){finishError(record,'response_byte_limit');if(clone.body)cancelReader(clone.body.getReader());return;}
      if(!clone.body){finishError(record,'response_body_missing');return;}
      reader=clone.body.getReader();record.reader=reader;let total=0;
      for(;;){const item=await Promise.race([reader.read(),deadline]);if(item.captureTimeout||record.error){cancelReader(reader);return;}if(item.done)break;const bytes=item.value;
        if(!ArrayBuffer.isView(bytes)||bytes.BYTES_PER_ELEMENT!==1){finishError(record,'response_chunk_invalid');cancelReader(reader);return;}
        if(bytes.byteLength>limits.maxResponseBytes-total){finishError(record,'response_byte_limit');cancelReader(reader);return;}
        if(!reserve(bytes.byteLength)){finishError(record,'aggregate_byte_limit');cancelReader(reader);return;}
        total+=bytes.byteLength;record.retained+=bytes.byteLength;chunks.push(bytes);
      }
      if(record.error)return;if(length!==null&&Number(length)!==total){finishError(record,'response_length_mismatch');return;}
      const bytes=new Uint8Array(total);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}chunks.length=0;
      const sha=await digest(bytes);if(record.error)return;record.reader=null;
      record.buffer=bytes;record.responseBytes=total;record.responseSha256=sha;if(record.requestSha256)record.phase='complete';
    }catch(_){finishError(record,'response_read_failed');if(reader)cancelReader(reader);}
    finally{clearTimeout(timer);chunks.length=0;record.reader=null;}
  }
  function observeResponse(record,response){
    if(record.error)return;
    try{
      record.status=response.status;const entries=Array.from(response.headers.entries()).map(([name,value])=>[name.toLowerCase(),value]).sort(([a],[b])=>a<b?-1:a>b?1:0);
      if(entries.length>128||entries.some(([a,b])=>a.length>32768||b.length>32768)||entries.reduce((n,[a,b])=>n+encoder.encode(a).length+encoder.encode(b).length,0)>32768){finishError(record,'response_headers_limit');return;}
      record.headers=Object.fromEntries(entries);
      // Synchronous observer runs before the application's fulfillment handler.
      // The original Response/body is neither read nor substituted.
      const clone=response.clone();observePromise(readClone(record,clone),()=>{},()=>finishError(record,'observer_failed'));
    }catch(_){finishError(record,'response_clone_failed');}
  }
  globalThis.fetch=function(...args){
    const promise=Reflect.apply(originalFetch,this,args);
    try{const shape=requestShape(args[0],args[1]);if(shape){const record=begin(shape);if(record)observePromise(promise,response=>observeResponse(record,response),()=>finishError(record,'fetch_rejected'));}}
    catch(_){/* Observational argument handling must never alter original fetch. */}
    return promise;
  };
  function summary(){return{engine,epoch,origin,sequence,policy:policy?{...policy}:null,limits:{...limits},aggregateBytes:used,peakAggregateBytes:peak,overflow:overflow?{...overflow}:null,records:records.map(record=>({sequence:record.sequence,method:record.method,url:record.url,requestSha256:record.requestSha256,requestBytes:record.requestBytes,status:record.status,responseSha256:record.responseSha256,responseBytes:record.responseBytes,phase:record.phase,error:record.error,headerNames:record.headers?Object.keys(record.headers):[]}))};}
  function select(query){
    if(query.epoch!==epoch)return{state:'error',error:'document_epoch_changed'};
    if(!Number.isSafeInteger(query.afterSequence)||query.afterSequence<0||query.afterSequence>sequence)return{state:'error',error:'invalid_capture_mark'};
    if(overflow&&overflow.lastSequence>query.afterSequence)return{state:'error',error:'record_limit'};
    const scope=records.filter(record=>record.sequence>query.afterSequence&&(record.method===query.method||record.error==='unsupported_request_shape')&&record.url===query.url);
    if(scope.some(record=>record.error&&!record.requestSha256))return{state:'error',error:scope.find(record=>record.error&&!record.requestSha256).error};
    if(scope.some(record=>!record.requestSha256))return{state:'pending'};
    const matching=scope.filter(record=>record.requestSha256===query.requestSha256);
    if(matching.length>1)return{state:'error',error:'ambiguous_capture_records'};
    if(!matching.length)return{state:'error',error:'matching_capture_not_found'};
    const record=matching[0];if(record.error)return{state:'error',error:record.error};if(record.phase==='released')return{state:'error',error:'capture_bytes_released'};if(record.phase!=='complete')return{state:'pending'};
    if(record.status!==query.status)return{state:'error',error:'response_status_mismatch'};
    if(JSON.stringify(record.headers)!==JSON.stringify(query.headers))return{state:'error',error:'response_headers_mismatch'};
    return{state:'ready',epoch,sequence:record.sequence,requestBytes:record.requestBytes,responseBytes:record.responseBytes,responseSha256:record.responseSha256,status:record.status,headers:record.headers};
  }
  function chunk({epoch:expected,sequence:seq,offset,maxBytes}){
    if(expected!==epoch)throw Error('document_epoch_changed');const record=records.find(item=>item.sequence===seq);
    if(!record||record.phase!=='complete'||!record.buffer)throw Error('capture_bytes_unavailable');
    if(!Number.isSafeInteger(offset)||offset<0||offset>record.buffer.length||!Number.isSafeInteger(maxBytes)||maxBytes<1||maxBytes>limits.transferChunkBytes)throw Error('invalid_transfer_chunk');
    const bytes=record.buffer.subarray(offset,Math.min(offset+maxBytes,record.buffer.length));let binary='';for(let start=0;start<bytes.length;start+=8192)binary+=String.fromCharCode(...bytes.subarray(start,start+8192));
    return{epoch,sequence:seq,offset,bytes:bytes.length,base64:btoa(binary)};
  }
  function releaseCapture(query){const selected=select(query);if(selected.state!=='ready')throw Error(selected.error||'capture_changed');const record=records.find(item=>item.sequence===selected.sequence);release(record.retained);record.retained=0;record.buffer=null;record.phase='released';return{epoch,sequence:record.sequence,responseSha256:record.responseSha256,released:true};}
  Object.defineProperty(globalThis,key,{value:Object.freeze({mark:()=>({epoch,afterSequence:sequence}),summary,select,chunk,releaseCapture}),configurable:false,enumerable:false,writable:false});
}

async function install(page,options={},policy){const configuration={limits:limitsFor(options),policy:policyFor(policy)};if(installed.has(page)){assert.deepEqual(installed.get(page),configuration,'Capture already installed with other fixture limits or policy');return;}await page.addInitScript(installInDocument,{key:KEY,engine:ENGINE,...configuration,routes:routesFor(configuration.policy)});installed.set(page,configuration);}
async function mark(page){return page.evaluate(key=>{const api=globalThis[key];if(!api)throw Error('Profile response capture is not installed in this document');return api.mark();},KEY);}
async function history(page){return page.evaluate(key=>{const api=globalThis[key];if(!api)throw Error('Profile response capture is not installed in this document');return api.summary();},KEY);}
async function capture(page,response,marker){
  assert(marker&&typeof marker.epoch==='string'&&Number.isSafeInteger(marker.afterSequence)&&marker.afterSequence>=0,'An awaited pre-action capture mark is required');
  const request=response.request(),method=request.method(),url=response.url();
  const configuration=installed.get(page);assert(configuration,'Capture must first be installed on this page');
  const location=new URL(url),policy=configuration.policy;
  assert(canCapture(method,url,policy),'Only allowlisted profile reads or explicitly owned JSON operations are captured');
  const current=await history(page);assert.equal(location.origin,current.origin,'Expected same-origin capture');assert.equal(current.epoch,marker.epoch,'document_epoch_changed');
  const posted=typeof request.postDataBuffer==='function'?request.postDataBuffer():request.postData()===null?null:Buffer.from(request.postData(),'utf8');
  if(method==='GET')assert.equal(posted,null,'GET capture requires absent request data');else assert(posted!==null,'A raw string request body is required');
  const raw=posted===null?Buffer.alloc(0):posted;
  const requestSha256=hash(raw),headers=normalizedHeaders(await(typeof response.allHeaders==='function'?response.allHeaders():response.headers()));
  const query={...marker,method,url,requestSha256,status:response.status(),headers};
  const handle=await page.waitForFunction(({key,query})=>{const api=globalThis[key];return!api||api.select(query).state!=='pending';},{key:KEY,query});if(handle?.dispose)await handle.dispose();
  const selected=await page.evaluate(({key,query})=>{const api=globalThis[key];if(!api)throw Error('document_epoch_changed');return api.select(query);},{key:KEY,query});
  assert.equal(selected.state,'ready','Browser profile capture refused: '+(selected.error||selected.state));
  const limits=configuration.limits;assert(selected.responseBytes<=limits.maxResponseBytes,'Captured response exceeds fixture limit');const chunks=[];
  for(let offset=0;offset<selected.responseBytes;){const item=await page.evaluate(({key,query})=>globalThis[key].chunk(query),{key:KEY,query:{epoch:marker.epoch,sequence:selected.sequence,offset,maxBytes:limits.transferChunkBytes}});assert.equal(item.epoch,marker.epoch);assert.equal(item.sequence,selected.sequence);assert.equal(item.offset,offset);const bytes=Buffer.from(item.base64,'base64');assert.equal(bytes.toString('base64'),item.base64);assert.equal(bytes.length,item.bytes);assert(bytes.length>0&&bytes.length<=limits.transferChunkBytes);chunks.push(bytes);offset+=bytes.length;assert(offset<=selected.responseBytes);}
  const bytes=Buffer.concat(chunks);assert.equal(bytes.length,selected.responseBytes);assert.equal(hash(bytes),selected.responseSha256,'Actual browser clone byte digest');
  const final=await page.evaluate(({key,query})=>globalThis[key].select(query),{key:KEY,query});assert.deepEqual(final,selected,'Capture changed during byte transfer');
  const released=await page.evaluate(({key,query})=>globalThis[key].releaseCapture(query),{key:KEY,query});assert.deepEqual(released,{epoch:marker.epoch,sequence:selected.sequence,responseSha256:selected.responseSha256,released:true});
  return{bytes,status:selected.status,headers:selected.headers,evidence:{engine:ENGINE,source:'actual_browser_fetch_response_clone',epoch:marker.epoch,after_sequence:marker.afterSequence,sequence:selected.sequence,method,url,request_sha256:requestSha256,request_bytes:selected.requestBytes,response_sha256:selected.responseSha256,response_bytes:selected.responseBytes,headers_sha256:hash(Buffer.from(JSON.stringify(headers))),fixture_limits:{...limits},...(policy?{capture_policy:{...policy}}:{}),http_replay:false,buffer_released:true}};
}
module.exports={ENGINE,OWNED_POLICY,OWNED_SCREENSHOT_POLICY,DEFAULT_LIMITS,install,mark,capture,history,canCapture,installInDocument};
