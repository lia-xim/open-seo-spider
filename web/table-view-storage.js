'use strict';
(function(host,factory){
  const api=factory();
  if(typeof module==='object'&&module.exports)module.exports=api;
  else host.TableViewStorage=api;
})(typeof globalThis==='object'?globalThis:this,function(){
  const KEY='open-seo-spider.table-views.v1';
  const LOCK_NAME=KEY+':mutation';
  const EXPRESSION_KEY='open-seo-spider.table-views.v2';
  const EXPRESSION_LOCK_NAME=EXPRESSION_KEY+':mutation';
  function refusal(code,message){return {ok:false,error:{code,message}};}
  function createStore({library,storage,locks},key,lockName){
    if(!library||typeof library.decodeCatalog!=='function'||typeof library.encodeCatalog!=='function'||typeof library.prepareReplacement!=='function')throw Error('Explicit TableViewLibrary instance required');
    const current=guard=>{try{return typeof guard==='function'&&guard()===true;}catch(_){return false;}};
    function argumentsFor(options,fields){
      try{
        if(!options||typeof options!=='object'||Array.isArray(options)||Object.keys(options).length!==fields.length)return null;
        const output={};for(const key of fields){const descriptor=Object.getOwnPropertyDescriptor(options,key);if(!descriptor||!Object.hasOwn(descriptor,'value'))return null;output[key]=descriptor.value;}return output;
      }catch(_){return null;}
    }
    function rawRead(){
      if(!storage||typeof storage.getItem!=='function')throw Error('Browser storage unavailable');
      const raw=storage.getItem(key);
      if(raw!==null&&typeof raw!=='string')throw Error('Unexpected browser storage value');
      return raw;
    }
    function read(){
      let raw;try{raw=rawRead();}catch(_){return refusal('storage_unavailable','Benannte Ansichten konnten nicht aus dem Browserspeicher gelesen werden.');}
      const decoded=library.decodeCatalog(raw);if(!decoded.ok)return {...decoded,observedRaw:raw};
      return {ok:true,value:Object.freeze({raw,entries:decoded.value.entries,missing:raw===null})};
    }
    async function locked(guard,action){
      if(!current(guard))return refusal('guard_refused','Die Ansicht wurde inzwischen geändert. Erneut ausdrücklich speichern.');
      let request;
      try{request=locks&&locks.request;}catch(_){return refusal('lock_unavailable','Der Browser stellt keine Schreibsperre für benannte Ansichten bereit.');}
      if(typeof request!=='function')return refusal('lock_unavailable','Der Browser stellt keine Schreibsperre für benannte Ansichten bereit.');
      try{
        return await Reflect.apply(request,locks,[lockName,{mode:'exclusive',ifAvailable:true},lock=>{
          if(!lock)return refusal('lock_busy','Benannte Ansichten werden in einem anderen Fenster geändert. Erneut ausdrücklich versuchen.');
          if(!current(guard))return refusal('guard_refused','Die Ansicht wurde während der Schreibvorbereitung geändert.');
          return action();
        }]);
      }catch(_){return refusal('lock_failed','Die Schreibsperre für benannte Ansichten konnte nicht verwendet werden.');}
    }
    async function mutate(options){
      const args=argumentsFor(options,['expectedRaw','entries','guard']);
      if(!args)return refusal('invalid_mutation','Ungültige Vorbereitung zum Speichern einer benannten Ansicht.');
      const {expectedRaw,entries,guard}=args;
      if(!current(guard))return refusal('guard_refused','Die Ansicht wurde inzwischen geändert. Erneut ausdrücklich speichern.');
      // Snapshot before waiting: caller edits cannot change the intended payload
      // while this operation is awaiting its cooperative browser-wide lock.
      const encoded=library.encodeCatalog(entries);if(!encoded.ok)return encoded;
      const snapshot=library.decodeCatalog(encoded.value);if(!snapshot.ok)return snapshot;
      return locked(guard,()=>{
          const loaded=read();if(!loaded.ok)return loaded;
          if(!current(guard))return refusal('guard_refused','Die Ansicht wurde während des Lesens geändert.');
          const replacement=library.prepareReplacement(expectedRaw,loaded.value.raw,snapshot.value.entries);if(!replacement.ok)return replacement;
          try{
            const setItem=storage&&storage.setItem;
            if(typeof setItem!=='function')throw Error('Browser storage unavailable');
            if(!current(guard))return refusal('guard_refused','Die Ansicht wurde vor dem Speichern geändert.');
            Reflect.apply(setItem,storage,[key,replacement.value.nextRaw]);
          }catch(_){return refusal('storage_write_failed','Benannte Ansichten wurden nicht gespeichert. Browserspeicher oder Speicherplatz ist nicht verfügbar.');}
          // No await between the last guard and this one atomic localStorage write.
          // Only cooperating tabs using this lock are serialized; external edits
          // are detected by byte-CAS, without a universal cross-process CAS claim.
          return {ok:true,value:Object.freeze({raw:replacement.value.nextRaw,entries:snapshot.value.entries,missing:false})};
      });
    }
    async function reset(options){
      const args=argumentsFor(options,['expectedRaw','guard']);
      if(!args||args.expectedRaw!==null&&typeof args.expectedRaw!=='string')return refusal('invalid_mutation','Ungültige Bestätigung zum Entfernen der benannten Ansichten.');
      const {expectedRaw,guard}=args;
      return locked(guard,()=>{
        let raw;try{raw=rawRead();}catch(_){return refusal('storage_unavailable','Benannte Ansichten konnten nicht aus dem Browserspeicher gelesen werden.');}
        // Recovery compares the exact observed string, including oversized or
        // unsupported prior formats. Never parse, truncate or migrate it here.
        if(raw!==expectedRaw)return refusal('stale_base','Die gespeicherten Ansichten wurden inzwischen geändert. Erneut prüfen.');
        try{
          const removeItem=storage&&storage.removeItem;
          if(typeof removeItem!=='function')throw Error('Browser storage unavailable');
          if(!current(guard))return refusal('guard_refused','Die Ansicht wurde vor dem Entfernen geändert.');
          Reflect.apply(removeItem,storage,[key]);
        }catch(_){return refusal('storage_remove_failed','Benannte Ansichten konnten nicht aus dem Browserspeicher entfernt werden.');}
        return {ok:true,value:Object.freeze({raw:null,entries:Object.freeze([]),missing:true})};
      });
    }
    return Object.freeze({read,mutate,reset});
  }
  function create(options){return createStore(options,KEY,LOCK_NAME);}
  function createExpression(options){return createStore(options,EXPRESSION_KEY,EXPRESSION_LOCK_NAME);}
  return Object.freeze({KEY,LOCK_NAME,EXPRESSION_KEY,EXPRESSION_LOCK_NAME,create,createExpression});
});
