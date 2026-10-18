'use strict';
(function (host) {
  const ENGINE='open-seo-spider/screenshot-v1', MAX_PNG_BYTES=67108864, MAX_METADATA_BYTES=65536;
  const hashPattern=/^[a-f0-9]{64}$/, encoder=new TextEncoder();
  const safeInteger=value=>Number.isSafeInteger(value)&&value>=0;
  const exactKeys=(value,keys)=>value&&typeof value==='object'&&!Array.isArray(value)&&Object.keys(value).length===keys.length&&keys.every(key=>Object.prototype.hasOwnProperty.call(value,key));
  const text=value=>typeof value==='string'&&encoder.encode(value).length<=65536;
  const ownerEqual=(a,b)=>Boolean(a&&b&&['id','url','ordinal','nativeGeneration','uiGeneration','selection','view','segmentRevision','tab','section'].every(key=>a[key]===b[key]));
  const hash=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),byte=>byte.toString(16).padStart(2,'0')).join('');
  function metadataRoute(owner) {
    const query=new URLSearchParams({url:owner.url});
    if(owner.ordinal!==null&&owner.ordinal!==undefined)query.set('ordinal',String(owner.ordinal));
    return '/api/crawls/'+encodeURIComponent(owner.id)+'/screenshot?'+query;
  }
  function pngRoute(metadata) {
    const query=new URLSearchParams({url:metadata.url,ordinal:String(metadata.ordinal),expected_snapshot_sha256:metadata.snapshot_sha256,expected_image_sha256:metadata.png_sha256});
    if(metadata.generation!==null)query.set('expected_generation',String(metadata.generation));
    return '/api/crawls/'+encodeURIComponent(metadata.id)+'/screenshot.png?'+query;
  }
  function checkedMetadata(value,owner) {
    const keys=['engine','version','id','ordinal','url','final_url','generation','snapshot_sha256','available','status','reason','viewport','png_sha256','bytes','width','height'];
    if(!exactKeys(value,keys)||value.engine!==ENGINE||value.version!==1||value.id!==owner.id||value.url!==owner.url||!text(value.final_url)||!safeInteger(value.ordinal)||!hashPattern.test(value.snapshot_sha256))throw Error('Die Screenshot-Metadaten gehören nicht zur ausgewählten Seitenbeobachtung.');
    if(owner.ordinal!==null&&owner.ordinal!==undefined&&value.ordinal!==owner.ordinal)throw Error('Die Screenshot-Seitenbeobachtung stimmt nicht überein.');
    if(value.generation!==null&&(!safeInteger(value.generation)||value.generation===0))throw Error('Ungültige native Screenshot-Generation.');
    if(value.generation!==((owner.nativeGeneration??0)>0?owner.nativeGeneration:null))throw Error('Der Screenshot gehört zu einer anderen Crawl-Generation.');
    if(value.viewport!==null&&(!exactKeys(value.viewport,['width','height'])||Object.values(value.viewport).some(dimension=>!Number.isInteger(dimension)||dimension<0||dimension>4294967295)))throw Error('Ungültige gespeicherte Browser-Ansicht.');
    if(!['available','disabled','render_failed','missing','corrupt'].includes(value.status)||typeof value.available!=='boolean'||(value.reason!==null&&(!text(value.reason)||!value.reason.length)))throw Error('Unbekannter Screenshot-Verfügbarkeitsbefund.');
    if(value.available){
      if(value.status!=='available'||value.reason!==null||!hashPattern.test(value.png_sha256)||!safeInteger(value.bytes)||value.bytes<57||value.bytes>MAX_PNG_BYTES||!safeInteger(value.width)||!safeInteger(value.height)||value.width<1||value.height<1||value.width>4096||value.height>4096)throw Error('Unvollständige Screenshot-Dateimetadaten.');
      if(value.viewport&&(Object.values(value.viewport).some(dimension=>dimension<100||dimension>4096)||value.width!==value.viewport.width||value.height!==value.viewport.height))throw Error('PNG und gespeicherte Browser-Ansicht stimmen nicht überein.');
    }else if(value.status==='available'||value.reason===null||['png_sha256','bytes','width','height'].some(key=>value[key]!==null)||value.viewport&&Object.values(value.viewport).some(dimension=>dimension<100||dimension>4096)&&value.status!=='corrupt')throw Error('Fehlende Screenshot-Evidenz darf keine Dateibestätigung enthalten.');
    return value;
  }
  async function transport(route,{signal,kind}) {
    const response=await fetch(route,{method:'GET',cache:'no-store',redirect:'error',signal:AbortSignal.any([signal,AbortSignal.timeout(30000)])});
    const cap=response.ok?(kind==='png'?MAX_PNG_BYTES:MAX_METADATA_BYTES):65536;
    const length=response.headers.get('content-length');
    if(length!==null&&(!/^\d+$/.test(length)||Number(length)>cap)){try{await response.body?.cancel();}catch(_){}throw Error('Die Screenshot-Antwort überschreitet das Bytebudget.');}
    if(!response.body)throw Error('Die native Screenshot-Antwort enthält keinen Body.');
    const reader=response.body.getReader(),chunks=[];let total=0;
    try{for(;;){const item=await reader.read();if(item.done)break;total+=item.value.byteLength;if(total>cap)throw Error('Die Screenshot-Antwort überschreitet das Bytebudget.');chunks.push(item.value);}}
    catch(error){try{await reader.cancel();}catch(_){}throw error;}
    const bytes=new Uint8Array(total);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}
    if(!response.ok){let detail='';try{const value=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes));if(typeof value.error==='string')detail=value.error;}catch(_){}const error=Error(detail||`Screenshot-Anfrage verweigert (HTTP${response.status}).`);error.status=response.status;throw error;}
    return {headers:response.headers,bytes};
  }
  async function checkedResponse(reply,mime,expectedBytes) {
    if(!reply||!reply.headers||typeof reply.headers.get!=='function'||!ArrayBuffer.isView(reply.bytes)||reply.bytes.BYTES_PER_ELEMENT!==1)throw Error('Ungültige Screenshot-Transportantwort.');
    const {headers,bytes}=reply,length=headers.get('content-length'),sha=headers.get('x-content-sha256');
    if(headers.get('content-type')?.toLowerCase().split(';')[0].trim()!==mime||!headers.get('cache-control')?.toLowerCase().split(',').some(value=>value.trim()==='no-store')||!/^\d+$/.test(length||'')||Number(length)!==bytes.byteLength||!hashPattern.test(sha||'')||sha!==await hash(bytes))throw Error('Screenshot-Datei oder Antwortheader bestehen die Integritätsprüfung nicht.');
    if(expectedBytes!==undefined&&bytes.byteLength!==expectedBytes)throw Error('Die Screenshot-Dateigröße hat sich geändert.');
    return bytes;
  }
  const crcTable=Array.from({length:256},(_,index)=>{let value=index;for(let bit=0;bit<8;bit++)value=value&1?0xedb88320^(value>>>1):value>>>1;return value>>>0;});
  function pngDimensions(bytes) {
    if(bytes.byteLength<57||bytes.byteLength>MAX_PNG_BYTES||[137,80,78,71,13,10,26,10].some((value,index)=>bytes[index]!==value))throw Error('Die Datei hat keine gültige PNG-Signatur.');
    const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);let offset=8,width,height,seenData=false,ended=false;
    while(offset<bytes.byteLength){
      if(bytes.byteLength-offset<12)throw Error('Unvollständiger PNG-Chunk.');
      const length=view.getUint32(offset),end=offset+12+length;if(end>bytes.byteLength)throw Error('PNG-Chunk überschreitet die Datei.');
      const kind=String.fromCharCode(...bytes.subarray(offset+4,offset+8));let crc=0xffffffff;
      for(let index=offset+4;index<end-4;index++)crc=crcTable[(crc^bytes[index])&255]^(crc>>>8);
      if(((crc^0xffffffff)>>>0)!==view.getUint32(end-4))throw Error('PNG-Chunk-Prüfsumme stimmt nicht überein.');
      if(offset===8){if(kind!=='IHDR'||length!==13)throw Error('PNG-Kopf fehlt.');width=view.getUint32(offset+8);height=view.getUint32(offset+12);if(!width||!height)throw Error('PNG-Abmessungen fehlen.');}
      else if(kind==='IHDR')throw Error('Doppelter PNG-Kopf.');
      if(kind==='IDAT')seenData=true;
      if(kind==='IEND'){if(length!==0||end!==bytes.byteLength||!seenData)throw Error('Ungültiges PNG-Ende.');ended=true;break;}
      offset=end;
    }
    if(!ended)throw Error('PNG-Ende fehlt.');
    return {width,height};
  }
  async function checkedPng(reply,metadata) {
    const bytes=await checkedResponse(reply,'image/png',metadata.bytes),headers=reply.headers;
    if(bytes.byteLength>MAX_PNG_BYTES||await hash(bytes)!==metadata.png_sha256)throw Error('Die Screenshot-Prüfsumme hat sich geändert.');
    const expected={'snapshot-sha256':metadata.snapshot_sha256,'image-sha256':metadata.png_sha256,generation:metadata.generation===null?'unknown':String(metadata.generation),ordinal:String(metadata.ordinal),'url-sha256':await hash(encoder.encode(metadata.url)),'job-id':metadata.id};
    expected['identity-sha256']=await hash(encoder.encode(JSON.stringify([ENGINE,metadata.id,String(metadata.ordinal),metadata.url,metadata.generation===null?null:String(metadata.generation),metadata.snapshot_sha256,metadata.png_sha256])));
    for(const[key,value]of Object.entries(expected))if(headers.get('x-screenshot-'+key)!==value)throw Error('Die Screenshot-Datei ist an einen anderen Snapshot oder eine andere Seite gebunden.');
    const dimensions=pngDimensions(bytes);
    if(dimensions.width!==metadata.width||dimensions.height!==metadata.height)throw Error('Die PNG-Abmessungen stimmen nicht mit den bestätigten Metadaten überein.');
    return bytes;
  }
  async function checkedPreview(base64,metadata) {
    if(!metadata||metadata.available!==true||!safeInteger(metadata.bytes)||metadata.bytes<57||metadata.bytes>MAX_PNG_BYTES||!hashPattern.test(metadata.png_sha256||'')||![metadata.width,metadata.height].every(value=>Number.isInteger(value)&&value>0&&value<=4096))throw Error('Keine bestätigte PNG-Basis für eine Vorschau.');
    // Check lengths before atob or allocating a decoded buffer. Do not let an
    // imported raw field bypass the same native artifact budget and dimensions.
    if(typeof base64!=='string'||base64.length>Math.ceil(MAX_PNG_BYTES/3)*4||base64.length!==Math.ceil(metadata.bytes/3)*4||!/^[A-Za-z0-9+/]*={0,2}$/.test(base64))throw Error('Gespeicherte Vorschau überschreitet das Bytebudget oder ist kein passendes Base64.');
    let decoded;try{decoded=atob(base64);}catch(_){throw Error('Ungültige gespeicherte Base64-Vorschau.');}
    if(decoded.length!==metadata.bytes)throw Error('Vorschau und bestätigte PNG-Länge stimmen nicht überein.');
    const bytes=new Uint8Array(decoded.length);for(let index=0;index<decoded.length;index++)bytes[index]=decoded.charCodeAt(index);
    if(await hash(bytes)!==metadata.png_sha256)throw Error('Vorschau und bestätigte PNG-Prüfsumme stimmen nicht überein.');
    const dimensions=pngDimensions(bytes);
    if(dimensions.width!==metadata.width||dimensions.height!==metadata.height)throw Error('Vorschau und bestätigte PNG-Abmessungen stimmen nicht überein.');
    return bytes;
  }
  function createController(options) {
    const state={metadata:null,busy:false,error:'',phase:'idle'};let operation=null;
    const notify=()=>options.changed?.(state),send=options.transport||transport;
    const current=token=>operation===token&&!token.controller.signal.aborted&&ownerEqual(token.owner,options.owner());
    function invalidate(){operation?.controller.abort();operation=null;state.metadata=null;state.busy=false;state.error='';state.phase='idle';notify();}
    async function run(download) {
      const owner=options.owner();if(!owner)return null;
      operation?.controller.abort();const token={owner,controller:new AbortController()};operation=token;
      state.busy=true;state.error='';state.phase=download?'download':'metadata';notify();
      try{
        const response=await send(metadataRoute(owner),{kind:'metadata',signal:token.controller.signal});if(!current(token))return null;
        const bytes=await checkedResponse(response,'application/json');if(!current(token))return null;
        if(bytes.byteLength>MAX_METADATA_BYTES)throw Error('Zu große Screenshot-Metadaten.');
        const metadata=checkedMetadata(JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes)),owner);if(!current(token))return null;
        state.metadata=metadata;notify();if(!current(token))return null;
        if(!download)return metadata;
        if(!metadata.available)throw Error('Kein bestätigter PNG-Screenshot verfügbar: '+metadata.reason);
        const png=await send(pngRoute(metadata),{kind:'png',signal:token.controller.signal});if(!current(token))return null;
        const image=await checkedPng(png,metadata);if(!current(token))return null;
        options.download(new Blob([image],{type:'image/png'}),'open-seo-spider-screenshot-'+metadata.ordinal+'.png');
        state.phase='downloaded';notify();return metadata;
      }catch(error){if(current(token)){state.error=error instanceof Error?error.message:'Screenshot-Prüfung fehlgeschlagen.';state.metadata=null;state.phase='error';notify();}return null;}
      finally{if(current(token)){state.busy=false;operation=null;notify();}}
    }
    return {state,invalidate,inspect:()=>run(false),download:()=>run(true)};
  }
  const api={ENGINE,MAX_PNG_BYTES,metadataRoute,pngRoute,checkedMetadata,pngDimensions,checkedPng,checkedPreview,sameOwner:ownerEqual,createController};
  if(typeof module!=='undefined'&&module.exports)module.exports=api;
  host.SpiderScreenshots=api;
})(typeof window!=='undefined'?window:globalThis);
