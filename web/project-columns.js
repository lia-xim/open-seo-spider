(function(host,factory){
  const api=factory();
  if(typeof module==='object'&&module.exports)module.exports=api;
  else host.ProjectColumns=api;
})(typeof globalThis==='object'?globalThis:this,function(){
  'use strict';
  const ENGINE='open-seo-spider/table-column-preferences-v1';
  const DEADLINE_MS=15000;
  const LIMITS=Object.freeze({bytes:524288,requestBytes:528384,replyBytes:528384,entries:64,columns:128,keyBytes:256,sectionBytes:128});
  const encoder=new TextEncoder(),uuid=/^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/,hash=/^[a-f0-9]{64}$/;
  const isUUID=value=>typeof value==='string'&&uuid.test(value),isHash=value=>typeof value==='string'&&hash.test(value);
  function failure(code,message){return{ok:false,error:{code,message}};}
  function requireValue(condition,message){if(!condition)throw Error(message);}
  function shape(value,keys){requireValue(value&&typeof value==='object'&&!Array.isArray(value),'Ein Datenobjekt fehlt.');const actual=Object.keys(value);requireValue(actual.length===keys.length&&keys.every(key=>actual.includes(key)),'Unerwartete oder fehlende Datenfelder.');for(const key of keys)requireValue(Object.hasOwn(Object.getOwnPropertyDescriptor(value,key)||{},'value'),'Datenfelder dürfen keine Getter sein.');}
  function text(value,bytes){requireValue(typeof value==='string'&&value.trim().length>0&&encoder.encode(value).length<=bytes&&!/[\u0000-\u001f\u007f-\u009f]/u.test(value),'Ungültiger oder zu langer Schlüssel.');return value;}
  function dense(value,limit){requireValue(Array.isArray(value)&&value.length<=limit,'Ungültige oder zu große Liste.');for(let i=0;i<value.length;i++)requireValue(Object.hasOwn(Object.getOwnPropertyDescriptor(value,String(i))||{},'value'),'Listen müssen vollständig gespeicherte Werte enthalten.');return value;}
  function copy(value){return JSON.parse(JSON.stringify(value));}
  function equal(a,b){return JSON.stringify(a)===JSON.stringify(b);}
  function fingerprint(value){
    requireValue(value&&typeof value==='object','Der native Dateistatus fehlt.');
    if(value.state==='missing'){shape(value,['state']);return{state:'missing'};}
    if(value.state==='present'){shape(value,['state','revision','sha256']);requireValue(isUUID(value.revision)&&isHash(value.sha256),'Ungültige native Revision oder SHA-256.');return{state:'present',revision:value.revision,sha256:value.sha256};}
    shape(value,['state','sha256']);requireValue(value.state==='invalid'&&isHash(value.sha256),'Ungültiger nativer Fehlerstatus.');return{state:'invalid',sha256:value.sha256};
  }
  function schema(value){
    const keys=new Set(),columns=dense(value,LIMITS.columns).map(column=>{shape(column,['key','type','mandatory']);const key=text(column.key,LIMITS.keyBytes);requireValue(!keys.has(key),'Doppelter Spaltenschlüssel.');keys.add(key);requireValue(['text','number','auto'].includes(column.type)&&typeof column.mandatory==='boolean','Ungültiger Spaltentyp.');return{key,type:column.type,mandatory:column.mandatory};});
    requireValue(columns.length>0&&columns.some(column=>column.mandatory),'Spaltenschema ohne Pflichtspalte.');return columns;
  }
  function entry(value){
    shape(value,['section','schema','visible_keys']);const section=text(value.section,LIMITS.sectionBytes),columns=schema(value.schema),available=new Set(columns.map(column=>column.key)),seen=new Set();
    const visible=dense(value.visible_keys,LIMITS.columns).map(key=>{text(key,LIMITS.keyBytes);requireValue(available.has(key)&&!seen.has(key),'Unbekannte oder doppelte sichtbare Spalte.');seen.add(key);return key;});
    const mandatory=columns.filter(column=>column.mandatory).map(column=>column.key);requireValue(visible.length>0&&equal(visible.slice(0,mandatory.length),mandatory),'Eine Pflichtspalte fehlt oder hat nicht die feste Reihenfolge.');return{section,schema:columns,visible_keys:visible};
  }
  function entries(value){const seen=new Set();return dense(value,LIMITS.entries).map(input=>{const checked=entry(input);requireValue(!seen.has(checked.section),'Doppelter Tabellenbereich.');seen.add(checked.section);return checked;});}
  function envelope(value,projectId){
    shape(value,['project_id','fingerprint','document','error']);requireValue(isUUID(projectId)&&value.project_id===projectId,'Die Antwort gehört zu einem anderen Crawl-Projekt.');const base=fingerprint(value.fingerprint);
    if(base.state==='missing'){requireValue(value.document===null&&value.error===null,'Fehlende Speicherung enthält unerwartete Daten.');return{project_id:projectId,fingerprint:base,document:null,error:null};}
    if(base.state==='invalid'){requireValue(value.document===null,'Ungültige Speicherung darf kein Standarddokument werden.');shape(value.error,['code','message']);requireValue(value.error.code==='stored_document_invalid'&&typeof value.error.message==='string'&&value.error.message.length<=4096,'Ungültiger nativer Fehler.');return{project_id:projectId,fingerprint:base,document:null,error:{code:value.error.code,message:value.error.message}};}
    requireValue(value.error===null,'Gespeicherte Spalten mit unerwartetem Fehler.');shape(value.document,['engine','version','project_id','revision','entries']);const document=value.document;
    requireValue(document.engine===ENGINE&&document.version===1&&document.project_id===projectId&&document.revision===base.revision,'Nativer Dokumentvertrag oder Revision stimmt nicht.');
    const checked={engine:ENGINE,version:1,project_id:projectId,revision:base.revision,entries:entries(document.entries)};requireValue(encoder.encode(JSON.stringify(checked)).length<=LIMITS.bytes,'Das native Spaltendokument überschreitet die Dateigrenze.');return{project_id:projectId,fingerprint:base,document:checked,error:null};
  }
  function attempt(work){try{return{ok:true,value:work()};}catch(error){return failure('invalid_preferences_response',error instanceof Error?error.message:'Ungültige native Spaltendaten.');}}
  function validateEntry(value){return attempt(()=>entry(value));}
  function validateEnvelope(value,projectId){return attempt(()=>envelope(value,projectId));}
  async function readResponse(response,signal){
    requireValue(response&&typeof response.status==='number'&&response.headers&&typeof response.headers.get==='function','Keine native HTTP-Antwort.');
    requireValue(/^application\/json(?:\s*;|$)/i.test(response.headers.get('content-type')||''),'Die native Antwort enthält kein JSON.');
    const length=response.headers.get('content-length');if(length!==null)requireValue(/^(0|[1-9][0-9]*)$/.test(length)&&Number(length)<=LIMITS.replyBytes,'Die native Antwort überschreitet die Lesegrenze.');
    requireValue(!signal.aborted,'Die native Anfrage wurde abgebrochen.');requireValue(response.body&&typeof response.body.getReader==='function','Der native Antwortstream fehlt.');const reader=response.body.getReader(),chunks=[];let size=0;
    const cancel=()=>{reader.cancel().catch(()=>{});};signal.addEventListener('abort',cancel,{once:true});
    try{for(;;){const item=await reader.read();requireValue(!signal.aborted,'Die native Anfrage wurde abgebrochen.');if(item.done)break;size+=item.value.byteLength;requireValue(size<=LIMITS.replyBytes,'Die native Antwort überschreitet die Lesegrenze.');chunks.push(item.value);}}
    finally{signal.removeEventListener('abort',cancel);cancel();}
    const bytes=new Uint8Array(size);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}
    if(length!==null)requireValue(Number(length)===size,'Die native Antwort wurde nicht vollständig übertragen.');return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes));
  }
  function create(hooks){
    requireValue(hooks&&['fetch','owner','queryRevision','schema','visibleKeys','apply'].every(key=>typeof hooks[key]==='function'),'Explizite Projektspalten-Hooks erforderlich.');
    let sequence=0,operation=null,baseline=null,applying=false;
    const state={busy:false,phase:'idle',projectId:null,status:'unloaded',fingerprint:null,error:null,message:'Projektspalten wurden noch nicht nativ geladen.'};
    const notify=()=>{if(typeof hooks.changed==='function')hooks.changed(copy(state));};
    function owner(){const value=hooks.owner();requireValue(value&&isUUID(value.projectId),'Zuerst ein natives Crawl-Projekt auswählen.');text(value.section,LIMITS.sectionBytes);requireValue(Number.isSafeInteger(value.generation)&&value.generation>=0,'Der Tabellenstand ist ungültig.');const query=hooks.queryRevision();requireValue(Number.isSafeInteger(query)&&query>=0,'Die Tabellenrevision ist ungültig.');return{projectId:value.projectId,section:value.section,generation:value.generation,nativeGeneration:value.nativeGeneration??null,segment:value.segment??null,query};}
    const available=()=>typeof hooks.available!=='function'||hooks.available()===true;
    const same=value=>{try{return available()&&equal(owner(),value);}catch{return false;}};
    function invalidate(){if(applying)return;const interruptedLoad=operation?.kind==='load';sequence++;if(operation){operation.controller.abort();operation.clearDeadline?.();if(operation.kind!=='load')baseline=null;}operation=null;state.busy=false;state.phase='idle';state.error=null;let currentId=null;try{currentId=hooks.owner()?.projectId??null;}catch{}if(currentId!==state.projectId){baseline=null;state.projectId=currentId;state.status='unloaded';state.fingerprint=null;state.message='Projektspalten wurden noch nicht nativ geladen.';}else if(!baseline){state.status='unloaded';state.fingerprint=null;state.message='Projektspalten erneut nativ laden, bevor sie gespeichert werden.';}else if(interruptedLoad){state.message='Das native Laden wurde abgebrochen. Die aktuelle Tabellenansicht bleibt erhalten.';}notify();}
    function refused(code,message){state.error={code,message};state.message='';notify();return failure(code,message);}
    function begin(kind){if(state.busy)return failure('preferences_busy','Eine native Projektspalten-Aktion läuft bereits.');let captured;try{requireValue(available(),'Projektspalten sind in dieser Ansicht nicht verfügbar.');captured=owner();}catch(error){return refused('unavailable',error.message);}
      if(state.projectId!==captured.projectId){baseline=null;state.projectId=captured.projectId;state.status='unloaded';state.fingerprint=null;}
      if(kind!=='load'&&(!baseline||baseline.project_id!==captured.projectId))return refused('load_required','Projektspalten zuerst nativ laden, damit der gespeicherte Stand geprüft werden kann.');
      if(kind==='save'&&baseline.fingerprint.state==='invalid')return refused('invalid_saved_preferences','Die native Speicherung ist beschädigt. Zum bewussten Leeren die getrennte Aktion für alle Projektspalten verwenden.');
      const controller=new AbortController(),token=++sequence;operation={kind,token,owner:captured,controller};state.busy=true;state.phase=kind;state.error=null;state.message=kind==='load'?'Projektspalten werden nativ geladen.':kind==='save'?'Spalten dieses Bereichs werden nativ gespeichert.':'Alle nativen Projektspalten werden geleert.';notify();return{ok:true,value:operation};
    }
    async function request(op,method,payload){
      const route='/api/crawls/'+encodeURIComponent(op.owner.projectId)+'/table-preferences'+(op.kind==='reset'?'/reset':'');let body;
      if(payload!==undefined){body=JSON.stringify(payload);requireValue(encoder.encode(body).length<=LIMITS.requestBytes,'Die Projektspalten überschreiten die Schreibgrenze.');}
      const signal=op.controller.signal;let rejectAbort;
      const abortPromise=new Promise((_,reject)=>{rejectAbort=()=>{const error=new Error(op.timedOut?'Die native Projektspalten-Aktion hat die Antwortfrist überschritten. Den gespeicherten Stand vor dem nächsten Schreibversuch neu laden.':'Die native Projektspalten-Aktion wurde abgebrochen.');error.code=op.timedOut?'preferences_timeout':'preferences_cancelled';reject(error);};signal.addEventListener('abort',rejectAbort,{once:true});});
      const timer=setTimeout(()=>{op.timedOut=true;op.controller.abort();},DEADLINE_MS);op.clearDeadline=()=>{clearTimeout(timer);signal.removeEventListener('abort',rejectAbort);};
      const transfer=(async()=>{op.sent=true;const response=await hooks.fetch(route,{method,headers:body===undefined?{}:{'content-type':'application/json'},body,cache:'no-store',redirect:'error',credentials:'same-origin',signal}),data=await readResponse(response,signal);
        if(response.status<200||response.status>=300){const code=typeof data?.code==='string'?data.code:'preferences_http_error',message=typeof data?.error==='string'?data.error:'Native Projektspalten-Aktion fehlgeschlagen (HTTP '+response.status+').';const error=new Error(message);error.code=code;error.status=response.status;throw error;}
        return envelope(data,op.owner.projectId);})();
      try{return await Promise.race([transfer,abortPromise]);}finally{op.clearDeadline();op.clearDeadline=null;}
    }
    function current(op){return operation===op&&sequence===op.token&&same(op.owner);}
    function finish(op,loaded,message){if(operation!==op)return;baseline=copy(loaded);state.projectId=loaded.project_id;state.fingerprint=copy(loaded.fingerprint);state.status=loaded.fingerprint.state==='present'?(loaded.document.entries.length?'present':'empty'):loaded.fingerprint.state;state.message=message;state.error=loaded.error?copy(loaded.error):null;state.busy=false;state.phase='idle';operation=null;notify();}
    function failed(op,error){if(!current(op)){if(operation===op)invalidate();return failure('stale_preferences','Crawl oder Tabellenansicht wurden inzwischen geändert.');}
      if(op.sent){baseline=null;state.status='unloaded';state.fingerprint=null;}
      state.busy=false;state.phase='idle';operation=null;state.error={code:typeof error?.code==='string'?error.code:'preferences_failed',message:typeof error?.message==='string'?error.message:'Native Projektspalten-Aktion fehlgeschlagen.'};state.message='';notify();return failure(state.error.code,state.error.message);
    }
    async function load(){const started=begin('load');if(!started.ok)return started;const op=started.value;
      try{const loaded=await request(op,'GET');if(!current(op))return failed(op,Error('stale'));
        if(loaded.fingerprint.state==='invalid'){finish(op,loaded,'Die native Projektspalten-Datei ist ungültig; die aktuelle Tabelle bleibt erhalten.');return failure(loaded.error.code,loaded.error.message);}
        const selected=loaded.document?.entries.find(value=>value.section===op.owner.section);
        if(selected){const actual=schema(hooks.schema());requireValue(equal(actual,selected.schema),'Das gespeicherte Spaltenschema passt nicht mehr zum aktuellen Tabellenbereich.');const mandatory=actual.filter(column=>column.mandatory).map(column=>column.key);requireValue(equal(selected.visible_keys.slice(0,mandatory.length),mandatory),'Pflichtspalten stehen nicht in der aktuellen festen Reihenfolge.');if(!current(op))return failed(op,Error('stale'));applying=true;let applied;try{applied=hooks.apply([...selected.visible_keys],()=>current(op));}finally{applying=false;}requireValue(applied&&applied.ok===true,'Die Projektspalten konnten nicht atomar angewendet werden.');}
        finish(op,loaded,selected?'Native Projektspalten für diesen Bereich geladen. Die Filterauswahl bleibt erhalten.':loaded.fingerprint.state==='missing'?'Für dieses Crawl-Projekt wurden noch keine nativen Spalten gespeichert.':'Für diesen Bereich sind keine nativen Projektspalten gespeichert.');return{ok:true,value:{applied:Boolean(selected),fingerprint:copy(loaded.fingerprint)}};
      }catch(error){return failed(op,error);}
    }
    async function save(){const started=begin('save');if(!started.ok)return started;const op=started.value;
      try{const selected=entry({section:op.owner.section,schema:hooks.schema(),visible_keys:hooks.visibleKeys()}),next=baseline.document?copy(baseline.document.entries):[],index=next.findIndex(value=>value.section===selected.section);if(index<0)next.push(selected);else next[index]=selected;const checked=entries(next),expected=copy(baseline.fingerprint);if(!current(op))return failed(op,Error('stale'));const loaded=await request(op,'PUT',{expected,entries:checked});if(!current(op))return failed(op,Error('stale'));requireValue(loaded.fingerprint.state==='present'&&equal(loaded.document.entries,checked),'Die native Speicherung bestätigt nicht die vorbereiteten Projektspalten.');requireValue(expected.state!=='present'||loaded.fingerprint.revision!==expected.revision,'Die native Speicherung hat keine neue Revision veröffentlicht.');finish(op,loaded,'Spalten dieses Bereichs wurden nativ gespeichert.');return{ok:true,value:{fingerprint:copy(loaded.fingerprint)}};
      }catch(error){return failed(op,error);}
    }
    async function reset(){const started=begin('reset');if(!started.ok)return started;const op=started.value;
      try{const expected=copy(baseline.fingerprint),loaded=await request(op,'POST',{expected});if(!current(op))return failed(op,Error('stale'));requireValue(loaded.fingerprint.state==='present'&&loaded.document.entries.length===0,'Das native Leeren hat kein gültiges leeres Dokument veröffentlicht.');requireValue(expected.state!=='present'||loaded.fingerprint.revision!==expected.revision,'Das native Leeren hat keine neue Revision veröffentlicht.');finish(op,loaded,'Alle nativ gespeicherten Projektspalten wurden geleert. Die aktuelle Tabellenansicht bleibt aktiv.');return{ok:true,value:{fingerprint:copy(loaded.fingerprint)}};
      }catch(error){return failed(op,error);}
    }
    const currentBaseline=()=>{try{return baseline&&baseline.project_id===owner().projectId&&!state.busy&&available();}catch{return false;}};
    return Object.freeze({load,save,reset,invalidate,state:()=>copy(state),canSave:()=>Boolean(currentBaseline()&&baseline.fingerprint.state!=='invalid'),canReset:()=>Boolean(currentBaseline())});
  }
  return Object.freeze({ENGINE,DEADLINE_MS,LIMITS,validateEntry,validateEnvelope,create});
});
