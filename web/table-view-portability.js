'use strict';
(function(host,factory){
  if(typeof module==='object'&&module.exports)module.exports=factory(require('./table-explorer.js'),require('./table-view-library.js'));
  else host.TableViewPortability=factory(host.TableExplorer,host.TableViewLibrary);
})(typeof globalThis==='object'?globalThis:this,function(explorer,definitions){
// Portable declaration planning only. File/DOM/storage transactions and Apply
// remain owned by the caller. V1 and V2 stay separate; no automatic migration.
const own=(value,key)=>Object.prototype.hasOwnProperty.call(value,key);
const encoder=new TextEncoder();
const byteLength=Object.getOwnPropertyDescriptor(Object.getPrototypeOf(Uint8Array.prototype),'byteLength').get;
function reject(code,message){const error=new Error(message);error.code=code;throw error;}
function result(action){try{return{ok:true,value:action()};}catch(error){let code='invalid_definition',message='Die Ansichtsdefinitionen konnten nicht vorbereitet werden.';try{if(typeof error?.code==='string')code=error.code;if(typeof error?.message==='string')message=error.message;}catch(_){}return{ok:false,error:{code,message}};}}
const libraryMessages=Object.freeze({corrupt_catalog:'Der Ansichtskatalog enthält kein gültiges JSON.',unsupported_version:'Die Datei gehört zu einem anderen Filtermodus oder einer nicht unterstützten Version.',entry_limit:'Der vollständige Zielkatalog überschreitet die Anzahl erlaubter Ansichten.',catalog_byte_limit:'Der vollständige Ansichtskatalog überschreitet seine Byte-Grenze.',entry_byte_limit:'Eine vollständige Ansicht überschreitet ihre Byte-Grenze.',duplicate_id:'Eine Ansichtskennung kommt mehrfach im Katalog vor.',duplicate_name:'Ein Ansichtsname kommt nach NFC-, Leerzeichen- und Groß-/Kleinschreibungsabgleich mehrfach in diesem Bereich vor.',invalid_name:'Der Ansichtsname fehlt, enthält Steuerzeichen oder überschreitet die UTF-8-Byte-Grenze.',invalid_id:'Die Ansichtskennung ist keine gültige kleingeschriebene UUID.',invalid_shape:'Der Ansichtskatalog enthält unbekannte, fehlende oder berechnete Felder.'});
function checked(result){if(!result.ok)reject(result.error.code,own(libraryMessages,result.error.code)?libraryMessages[result.error.code]:result.error.message);return result.value;}
function freeze(value){if(value&&typeof value==='object'){for(const child of Object.values(value))freeze(child);Object.freeze(value);}return value;}
function dataObject(value,fields){
  if(!value||typeof value!=='object'||Array.isArray(value))reject('invalid_shape','Ein Datenobjekt mit eigenen Feldern ist erforderlich.');
  const prototype=Object.getPrototypeOf(value),keys=Reflect.ownKeys(value);
  if(prototype!==Object.prototype&&prototype!==null||keys.length!==fields.length||keys.some(key=>typeof key!=='string'||!fields.includes(key)))reject('invalid_shape','Die Definition enthält unbekannte oder fehlende Felder.');
  const copy={};for(const key of fields){const descriptor=Object.getOwnPropertyDescriptor(value,key);if(!descriptor||!own(descriptor,'value')||!descriptor.enumerable)reject('invalid_shape','Berechnete oder versteckte Felder sind keine gültigen Ansichtsdefinitionen.');Object.defineProperty(copy,key,{value:descriptor.value,enumerable:true});}return copy;
}
function create(version,overrides={}){
  if(version!==1&&version!==2)throw Error('Den einfachen Katalog (Version 1) oder den Gruppenkatalog (Version 2) ausdrücklich auswählen.');
  if(!explorer||typeof explorer.normalizeText!=='function'||!definitions||typeof definitions.create!=='function'||typeof definitions.createExpression!=='function')throw Error('Die Tabellenmodule für portable Ansichten sind nicht geladen.');
  const library=(version===1?definitions.create:definitions.createExpression)(explorer,overrides),previews=new WeakMap();
  const filename='open-seo-spider-table-views-v'+version;
  const nameKey=entry=>JSON.stringify([entry.section,explorer.normalizeText(entry.name)]);
  function catalog(raw){return checked(library.decodeCatalog(raw));}
  function encoded(entries){return checked(library.encodeCatalog(entries));}
  function exportDefinitions(raw,scope){return result(()=>{
    const fields=Reflect.ownKeys(scope||{}),selection=dataObject(scope,fields.includes('id')?['kind','id']:['kind']),current=catalog(raw);
    let entries;if(selection.kind==='all'&&!own(selection,'id'))entries=current.entries;
    else if(selection.kind==='selected'&&own(selection,'id')){const selected=current.entries.find(entry=>entry.id===selection.id);if(!selected)reject('unknown_selection','Die ausgewählte Ansicht ist nicht mehr im Katalog vorhanden.');entries=[selected];}
    else reject('invalid_shape','Den vollständigen Katalog dieses Filtermodus oder eine vorhandene Ansicht auswählen.');
    const output=encoded(entries),document=catalog(output);
    return freeze({raw:output,filename:filename+(selection.kind==='selected'?'-selected':'')+'.json',mime:'application/json',bytes:encoder.encode(output).length,entries:document.entries});
  });}
  function fileText(input){
    if(!(input instanceof Uint8Array))reject('invalid_file_bytes','Die vollständige Datei muss als UTF-8-Bytes vorliegen.');
    // Read typed-array internal slots and copy without an attacker-supplied
    // iterator, slice, byteLength getter or Buffer-specific conversion.
    const length=Reflect.apply(byteLength,input,[]);
    if(length<1||length>library.limits.maxCatalogBytes)reject('file_byte_limit','Die Datei ist leer oder überschreitet die Byte-Grenze dieses Katalogs.');
    const copy=new Uint8Array(length);Reflect.apply(Uint8Array.prototype.set,copy,[input]);
    const bom=copy[0]===239&&copy[1]===187&&copy[2]===191,content=bom?copy.subarray(3):copy;
    let text;try{text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(content);}catch(_){reject('invalid_utf8','Die Datei enthält ungültige UTF-8-Zeichen.');}
    if(text.charCodeAt(0)===0xfeff)reject('duplicate_bom','Die Datei darf nur eine UTF-8-Kennung am Anfang enthalten.');
    return text;
  }
  function classify(current,incoming){
    const ids=new Map(current.entries.map(entry=>[entry.id,entry])),names=new Map(current.entries.map(entry=>[nameKey(entry),entry]));
    return incoming.entries.map(entry=>{
      const id=ids.get(entry.id),name=names.get(nameKey(entry)),same=id&&JSON.stringify(id)===JSON.stringify(entry);
      const foreign=id&&id.section!==entry.section,ambiguous=id&&name&&id.id!==name.id;
      return{id:entry.id,name:entry.name,section:entry.section,disposition:same?'identical':!id&&!name?'add':'conflict',idTargetId:id?.id??null,nameTargetId:name?.id??null,replaceTargetId:foreign||ambiguous?null:(id?.id??name?.id??null),reason:foreign?'foreign_section_id':ambiguous?'ambiguous_targets':same?'identical':id?'id':name?'name':'new'};
    });
  }
  function previewImport(expectedRaw,fileBytes){return result(()=>{
    const current=catalog(expectedRaw),incomingRaw=fileText(fileBytes),incoming=catalog(incomingRaw),items=classify(current,incoming);
    const preview=freeze({engine:incoming.engine,version:incoming.version,currentEntries:current.entries,incomingEntries:incoming.entries,items});
    previews.set(preview,{expectedRaw,incomingRaw});return preview;
  });}
  function choicesFor(resolutions,incoming){
    if(!resolutions||typeof resolutions!=='object'||Array.isArray(resolutions))reject('invalid_shape','Jede Importentscheidung muss der Kennung einer eingehenden Ansicht zugeordnet sein.');
    const keys=Reflect.ownKeys(resolutions);if(keys.length>library.limits.maxEntries||keys.some(key=>typeof key!=='string'))reject('invalid_shape','Die Importentscheidungen enthalten zu viele oder ungültige Kennungen.');
    const data=dataObject(resolutions,keys),ids=new Set(incoming.entries.map(entry=>entry.id)),output=new Map();
    for(const id of keys){
      if(!ids.has(id))reject('unknown_resolution','Eine Importentscheidung gehört zu keiner eingehenden Ansicht.');
      const raw=data[id],descriptor=raw&&Object.getOwnPropertyDescriptor(raw,'action');
      if(!descriptor||!own(descriptor,'value'))reject('invalid_shape','Die Importentscheidung benötigt ein eigenes Aktionsfeld.');
      const action=descriptor.value,fields=action==='skip'?['action']:action==='replace'?['action','targetId']:action==='copy'?['action','id','name']:null;
      if(!fields)reject('invalid_resolution','Überspringen, Ersetzen oder als Kopie importieren ausdrücklich auswählen.');output.set(id,dataObject(raw,fields));
    }
    return output;
  }
  function prepareImport(preview,resolutions){return result(()=>{
    const ticket=previews.get(preview);if(!ticket)reject('foreign_preview','Eine gültige Importvorschau dieses Filtermodus ist erforderlich.');
    // Revalidate the complete original base and incoming file. The preview is
    // immutable and never becomes a caller-editable source of accepted entries.
    const current=catalog(ticket.expectedRaw),incoming=catalog(ticket.incomingRaw),items=classify(current,incoming),choices=choicesFor(resolutions,incoming);
    const entries=[...current.entries],indices=new Map(entries.map((entry,index)=>[entry.id,index])),claimed=new Set(),reserved=new Set([...entries,...incoming.entries].map(entry=>entry.id));
    const addedIds=[],replacedIds=[],skippedIds=[],identicalIds=[];
    for(let index=0;index<incoming.entries.length;index++){
      const entry=incoming.entries[index],item=items[index],choice=choices.get(entry.id);
      if(choice?.action==='skip'){skippedIds.push(entry.id);continue;}
      if(choice?.action==='copy'){
        if(reserved.has(choice.id))reject('copy_id_not_fresh','Eine Kopie benötigt eine neue Kennung, die weder in der Datei noch im Zielkatalog vorkommt.');
        reserved.add(choice.id);entries.push({...entry,id:choice.id,name:choice.name});addedIds.push(choice.id);continue;
      }
      if(choice?.action==='replace'){
        if(item.disposition!=='conflict'||item.replaceTargetId===null||choice.targetId!==item.replaceTargetId)reject('invalid_replace_target','Nur das eindeutig zugeordnete Konfliktziel desselben Bereichs kann ersetzt werden.');
        if(claimed.has(choice.targetId))reject('duplicate_replace_target','Zwei eingehende Ansichten können nicht dasselbe Ziel ersetzen.');claimed.add(choice.targetId);
        const position=indices.get(choice.targetId),target=current.entries[position],replacement={...entry,id:target.id,name:target.name};
        if(JSON.stringify(replacement)===JSON.stringify(target))identicalIds.push(entry.id);
        else{entries[position]=replacement;replacedIds.push(target.id);}continue;
      }
      if(item.disposition==='conflict')reject('unresolved_conflict','Für jede widersprüchliche Ansicht ist eine ausdrückliche Importentscheidung erforderlich.');
      if(item.disposition==='identical'){identicalIds.push(entry.id);continue;}
      entries.push(entry);addedIds.push(entry.id);
    }
    // One full catalog validation after all explicit choices; no first32 or
    // partial salvage. Preparation returns data only and performs zero writes.
    const canonical=encoded(entries),accepted=catalog(canonical),changed=canonical!==encoded(current.entries);
    return freeze({expectedRaw:ticket.expectedRaw,entries:accepted.entries,nextRaw:changed?canonical:ticket.expectedRaw,changed,addedIds,replacedIds,skippedIds,identicalIds});
  });}
  return Object.freeze({exportDefinitions,previewImport,prepareImport});
}
return Object.freeze({create});
});
