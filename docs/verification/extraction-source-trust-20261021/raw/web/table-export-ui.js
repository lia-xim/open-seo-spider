'use strict';
(function(host,factory){
  if(typeof module==='object'&&module.exports)module.exports=factory();
  else host.TableExportUI=factory();
})(typeof globalThis==='object'?globalThis:this,function(){
const MIME='application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
const own=(value,key)=>Object.prototype.hasOwnProperty.call(value,key);
const error=(code,message)=>Object.freeze({code,message});
const stale=()=>error('stale','Die Tabelle hat sich geändert. Bitte den Export erneut starten.');
class Refusal extends Error{constructor(reason){super(reason.message);this.reason=reason;}}
function refuse(code,message){throw new Refusal(error(code,message));}
function reasonFrom(value,code,message){
  return value&&typeof value.code==='string'&&value.code&&typeof value.message==='string'
    ?error(value.code,value.message):error(code,message);
}
function failed(reason){return Object.freeze({ok:false,error:reason});}
// Copy data, rather than retain the caller's mutable stamp. Reject accessors and
// opaque identities so a reused object/array cannot hide a query/owner change.
function stampKey(value){
  const seen=new Set();let nodes=0;
  function copy(item,depth){
    if(++nodes>4096||depth>32)refuse('stamp','Der Exportzustand ist nicht vergleichbar.');
    if(item===null)return['null'];
    if(item===undefined)return['undefined'];
    if(typeof item==='string'||typeof item==='boolean')return[typeof item,item];
    if(typeof item==='number'&&Number.isFinite(item))return['number',Object.is(item,-0)?'-0':String(item)];
    if(typeof item!=='object'||seen.has(item))refuse('stamp','Der Exportzustand ist nicht vergleichbar.');
    seen.add(item);let output;
    if(Array.isArray(item)){
      if(Object.getPrototypeOf(item)!==Array.prototype)refuse('stamp','Der Exportzustand ist nicht vergleichbar.');
      const length=Object.getOwnPropertyDescriptor(item,'length');
      if(!length||!own(length,'value')||length.value>4096||Reflect.ownKeys(item).length!==length.value+1)refuse('stamp','Der Exportzustand ist nicht vergleichbar.');
      output=['array'];
      for(let i=0;i<length.value;i++){
        const field=Object.getOwnPropertyDescriptor(item,String(i));
        if(!field||!own(field,'value')||!field.enumerable)refuse('stamp','Der Exportzustand ist nicht vergleichbar.');
        output.push(copy(field.value,depth+1));
      }
    }else{
      if(![Object.prototype,null].includes(Object.getPrototypeOf(item)))refuse('stamp','Der Exportzustand ist nicht vergleichbar.');
      const keys=Reflect.ownKeys(item);
      if(keys.some(key=>typeof key!=='string'))refuse('stamp','Der Exportzustand ist nicht vergleichbar.');
      output=['object'];
      for(const key of keys.sort()){
        const field=Object.getOwnPropertyDescriptor(item,key);
        if(!field||!own(field,'value')||!field.enumerable)refuse('stamp','Der Exportzustand ist nicht vergleichbar.');
        output.push([key,copy(field.value,depth+1)]);
      }
    }
    seen.delete(item);return output;
  }
  if(value===undefined)refuse('stamp','Der Exportzustand fehlt.');
  return JSON.stringify(copy(value,0));
}
function create(hooks,env=globalThis){
  if(!hooks||!hooks.writer||typeof hooks.writer.prepareTable!=='function'||typeof hooks.writer.encode!=='function')throw new TypeError('An XLSX writer is required.');
  for(const name of ['available','stamp','connected','snapshot','download'])if(typeof hooks[name]!=='function')throw new TypeError('Missing export hook: '+name);
  if(hooks.changed!==undefined&&typeof hooks.changed!=='function'||hooks.yieldControl!==undefined&&typeof hooks.yieldControl!=='function')throw new TypeError('Invalid export callback.');
  let active=null,current=Object.freeze({busy:false,phase:'idle',error:null,message:''});
  const status=(busy,phase,reason,message)=>Object.freeze({busy,phase,error:reason,message:reason?reason.message:message});
  function notify(next){if(hooks.changed)hooks.changed(next);}
  function check(ticket,initial=false){
    if(active!==ticket)throw new Refusal(ticket.reason||stale());
    if(hooks.available()!==true)refuse('unavailable','Für diese Tabelle ist derzeit kein vollständiger Export verfügbar.');
    if(active!==ticket)throw new Refusal(ticket.reason||stale());
    if(hooks.connected()!==true)refuse('detached','Das ursprüngliche Export-Bedienelement ist nicht mehr verbunden.');
    if(active!==ticket)throw new Refusal(ticket.reason||stale());
    const key=stampKey(hooks.stamp());
    if(active!==ticket)throw new Refusal(ticket.reason||stale());
    if(initial)ticket.stamp=key;
    else if(key!==ticket.stamp)throw new Refusal(stale());
  }
  function publish(ticket,phase,message){
    check(ticket);const next=status(true,phase,null,message);current=next;
    try{notify(next);}catch(_){refuse('ui_failed','Der Exportstatus konnte nicht angezeigt werden.');}
    check(ticket);
  }
  function finish(ticket,phase,reason,message){
    if(active!==ticket)return ticket.reason||stale();
    active=null;const next=status(false,phase,reason,message);current=next;
    try{notify(next);}catch(_){
      const fault=error('ui_failed','Der Exportstatus konnte nicht angezeigt werden.');
      if(current===next)current=status(false,'refused',fault,'');
      return fault;
    }
    return reason;
  }
  function unwrap(result,code,message){
    if(!result||result.ok!==true){throw new Refusal(reasonFrom(result&&result.error,code,message));}
    return result.value;
  }
  async function yieldAt(ticket,phase){
    check(ticket);
    try{
      if(hooks.yieldControl)await hooks.yieldControl(phase);
      else await new Promise((resolve,reject)=>{
        if(!env||typeof env.setTimeout!=='function')reject(new Error('No task scheduler'));
        else env.setTimeout(resolve,0);
      });
    }catch(_){refuse('yield_failed','Die XLSX-Erstellung konnte nicht fortgesetzt werden.');}
    check(ticket);
  }
  async function run(){
    if(active)return failed(error('busy','Ein Tabellenexport läuft bereits.'));
    const ticket={stamp:null,reason:null};active=ticket;
    // Reserve the ticket before calling hooks; even a reentrant run fails closed.
    current=status(true,'prepare',null,'XLSX-Export wird vorbereitet …');
    try{
      check(ticket,true);
      publish(ticket,'prepare','XLSX-Export wird vorbereitet …');
      // A genuine task handoff lets the busy control paint and receives pending
      // query/owner changes before any potentially expensive table capture.
      await yieldAt(ticket,'prepare');
      check(ticket);
      let result;
      try{result=hooks.snapshot();}catch(_){refuse('read_failed','Die vollständige Tabelle konnte nicht gelesen werden.');}
      check(ticket);
      const input=unwrap(result,'read_failed','Die vollständige Tabelle konnte nicht gelesen werden.');
      if(!input||typeof input.filename!=='string'||!input.filename||!/^.+\.xlsx$/iu.test(input.filename)||/[\\/\u0000-\u001f\u007f]/u.test(input.filename))refuse('filename','Ein gültiger XLSX-Dateiname ist erforderlich.');
      const filename=input.filename;
      check(ticket);
      let prepared;
      try{prepared=hooks.writer.prepareTable(input.data);}catch(_){refuse('prepare_failed','Die vollständige XLSX-Datei konnte nicht vorbereitet werden.');}
      check(ticket);
      const snapshot=unwrap(prepared,'prepare_failed','Die vollständige XLSX-Datei konnte nicht vorbereitet werden.');
      publish(ticket,'encode','XLSX-Datei wird erstellt …');
      await yieldAt(ticket,'encode');
      check(ticket);
      let encoded;
      try{encoded=hooks.writer.encode(snapshot);}catch(_){refuse('encode_failed','Die vollständige XLSX-Datei konnte nicht erstellt werden.');}
      check(ticket);
      const workbook=unwrap(encoded,'encode_failed','Die vollständige XLSX-Datei konnte nicht erstellt werden.');
      if(!workbook||!(workbook.bytes instanceof Uint8Array)||workbook.bytes.byteLength===0)refuse('encode_failed','Der XLSX-Writer hat keine vollständige Datei geliefert.');
      const length=workbook.bytes.byteLength;
      publish(ticket,'download','XLSX-Download wird vorbereitet …');
      check(ticket);
      if(!env||typeof env.Blob!=='function')refuse('blob_unavailable','Dieser Browser kann keine XLSX-Datei herunterladen.');
      let blob;
      try{blob=new env.Blob([workbook.bytes],{type:MIME});}catch(_){refuse('blob_failed','Die vollständige XLSX-Datei konnte nicht für den Download bereitgestellt werden.');}
      check(ticket);
      if(blob.size!==length||blob.type!==MIME)refuse('blob_failed','Die XLSX-Datei wurde nicht vollständig für den Download bereitgestellt.');
      // The synchronous dispatch hook is the only side effect. It is never retried.
      check(ticket);
      try{hooks.download(blob,filename);}catch(_){refuse('download_failed','Der XLSX-Download konnte nicht gestartet werden.');}
      const fault=finish(ticket,'complete',null,'XLSX-Download wurde gestartet.');
      if(fault)return failed(fault);
      return Object.freeze({ok:true,value:Object.freeze({filename,bytes:length})});
    }catch(cause){
      const reason=active!==ticket?ticket.reason||stale():cause instanceof Refusal?cause.reason:error('export_failed','Die vollständige XLSX-Datei konnte nicht exportiert werden.');
      const fault=finish(ticket,'refused',reason,'');
      return failed(fault||reason);
    }
  }
  function stop(phase,reason){
    const ticket=active;
    if(ticket){ticket.reason=reason;finish(ticket,phase,reason,'');}
    return current;
  }
  return Object.freeze({run,state:()=>current,
    cancel:()=>stop('cancelled',error('cancelled','Der XLSX-Export wurde abgebrochen.')),
    invalidate:()=>stop('refused',stale())});
}
return Object.freeze({create,MIME});
});
