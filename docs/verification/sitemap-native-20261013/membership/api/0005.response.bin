'use strict';
(() => {
// Local import state; starting a crawl is a separate application action.
// Transport is supplied by the same-origin UI adapter; it only receives
// the frozen /api/lists/inspect and /api/lists/import relative endpoints.
const MAX_INPUT_BYTES=8*1024*1024;
const clone=value=>structuredClone(value);
const positive=(value,label)=>{const number=Number(value);if(!Number.isSafeInteger(number)||number<1)throw new Error(`${label}: positive Ganzzahl erforderlich.`);return number};

function buildListOptions(selection,limits){
  const kind=selection.kind;
  if(!['txt','csv','xml','xlsx'].includes(kind))throw new Error('TXT, CSV, XML oder XLSX ausdrücklich auswählen.');
  const format={kind};
  if(kind==='csv'||kind==='xlsx'){
    if(selection.column_kind==='header'){
      if(typeof selection.header_name!=='string'||!selection.header_name.length)throw new Error('Exakten Spaltennamen eingeben.');
      format.column={kind:'header',name:selection.header_name,row:positive(selection.header_row,'Headerzeile')};
    }else if(selection.column_kind==='index')format.column={kind:'index',value:positive(selection.column_index,'Spalte')};
    else throw new Error('Spaltenindex oder exakten Header auswählen.');
    format.start_row=positive(selection.start_row,'Erste Datenzeile');
    if(format.column.kind==='header'&&format.start_row<=format.column.row)throw new Error('Datenzeile muss nach der Headerzeile liegen.');
  }
  if(kind==='csv'){
    if(!['comma','semicolon','tab'].includes(selection.delimiter))throw new Error('CSV-Trennzeichen ausdrücklich auswählen.');
    format.delimiter=selection.delimiter;
  }
  if(kind==='xlsx'){
    if(selection.sheet_kind==='index')format.sheet={kind:'index',value:positive(selection.sheet_index,'Sheet')};
    else if(selection.sheet_kind==='name'&&typeof selection.sheet_name==='string'&&selection.sheet_name.length)format.sheet={kind:'name',value:selection.sheet_name};
    else throw new Error('Ein geprüftes Worksheet auswählen.');
    const policy=selection.formula_policy||'reject';
    if(!['reject','cached_string_only'].includes(policy))throw new Error('Gültige Formel-Cache-Regel auswählen.');
    format.formula_policy=policy;
  }
  return {...(limits?{limits:clone(limits)}:{}),format};
}

function summarizeRows(rows){
  const addresses=new Set(),page=new Set(),sitemap=new Set();let invalid=0,duplicates=0,accepted=0;
  for(const row of rows){
    if(row.error!=null||typeof row.normalized_url!=='string'||!row.normalized_url){invalid++;continue}
    accepted++;
    if(addresses.has(row.normalized_url))duplicates++;else addresses.add(row.normalized_url);
    (row.candidate_kind==='sitemap_document'?sitemap:page).add(row.normalized_url);
  }
  return {rows:rows.length,accepted,invalid,duplicates,unique_addresses:addresses.size,page_urls:[...page],sitemap_urls:[...sitemap]};
}

function bytesToBase64(bytes){
  let text='';for(let offset=0;offset<bytes.length;offset+=32768)text+=String.fromCharCode(...bytes.subarray(offset,offset+32768));
  return btoa(text);
}
function validateEnvelope(value,source,inspection){
  if(!value||typeof value.engine!=='string'||typeof value.input_sha256!=='string'||!/^[a-f0-9]{64}$/.test(value.input_sha256)||value.input_bytes!==source.bytes||value.website_requests!==0)throw new Error('Ungültiger lokaler Importbefund; kein Ergebnis übernommen.');
  if(!Array.isArray(value.warnings)||!Array.isArray(value.limitations)||(inspection?!Array.isArray(value.sheets):!Array.isArray(value.rows)))throw new Error('Unvollständige Importantwort.');
  if(!inspection){
    let previous=0;
    for(const row of value.rows){
      if(!row||!Number.isSafeInteger(row.source_index)||row.source_index<=previous||typeof row.raw_value!=='string'||!['page_url','sitemap_document'].includes(row.candidate_kind)||(row.error!=null&&typeof row.error!=='string')||(row.normalized_url!=null&&typeof row.normalized_url!=='string'))throw new Error('Ungültige Importzeilen; Herkunft nicht übernommen.');
      previous=row.source_index;
    }
  }
}

class ImportPreviewState{
  constructor({transport,emit=()=>{}}){
    if(typeof transport!=='function')throw new Error('Local transport required.');
    this.transport=transport;this.emit=emit;this.generation=0;this.fileGeneration=0;
    this.state={phase:'empty',source:null,options:null,inspection:null,result:null,accepted:null,error:''};
  }
  publish(){this.emit(clone(this.state))}
  invalidate(){this.generation++;this.state.result=null;this.state.accepted=null;this.state.error='';return this.generation}
  current(generation){return generation===this.generation}
  fail(generation,error){if(this.current(generation)){this.state.phase='error';this.state.result=null;this.state.accepted=null;this.state.error=error.message||String(error);this.publish()}return null}
  async setFile(file){
    this.invalidate();const fileGeneration=++this.fileGeneration;this.state.source=null;this.state.inspection=null;this.state.options=null;this.state.phase='reading';this.publish();
    try{
      if(!file||!Number.isSafeInteger(file.size)||file.size<1||file.size>MAX_INPUT_BYTES)throw new Error('Dateigröße muss 1 Byte bis 8 MiB betragen.');
      const bytes=new Uint8Array(await file.arrayBuffer());
      if(fileGeneration!==this.fileGeneration)return null;
      if(bytes.length!==file.size||bytes.length>MAX_INPUT_BYTES)throw new Error('Gelesene Dateigröße stimmt nicht mit der ausgewählten Datei überein.');
      this.state.source={name:String(file.name||'Lokale Datei'),origin:'file',bytes:bytes.length,content_base64:bytesToBase64(bytes)};
      this.state.phase='selected';this.publish();return clone(this.state.source);
    }catch(error){return fileGeneration===this.fileGeneration?this.fail(this.generation,error):null}
  }
  setOptions(options){this.invalidate();this.state.options=clone(options);this.state.phase=this.state.source?'selected':this.state.phase==='reading'?'reading':'empty';this.publish()}
  cancelPending(){
    if(!['reading','inspecting','parsing'].includes(this.state.phase))return;
    this.invalidate();this.fileGeneration++;this.state.phase=this.state.source?'selected':'empty';this.publish();
  }
  setEditedText(text){
    const generation=this.invalidate();this.fileGeneration++;this.state.source=null;this.state.inspection=null;this.state.options={format:{kind:'txt'}};
    try{
      const bytes=new TextEncoder().encode(String(text));
      if(bytes.length<1||bytes.length>MAX_INPUT_BYTES)throw new Error('Bearbeiteter Text muss 1 Byte bis 8 MiB betragen.');
      this.state.source={name:'Bearbeitete URL-Liste.txt',origin:'edited_text',bytes:bytes.length,content_base64:bytesToBase64(bytes)};
      this.state.phase='selected';this.publish();return clone(this.state.source);
    }catch(error){return this.fail(generation,error)}
  }
  async inspect(limits){
    if(!this.state.source)throw new Error('Zuerst lokale Datei auswählen.');
    const source=clone(this.state.source),generation=this.invalidate();this.state.phase='inspecting';this.state.inspection=null;this.publish();
    try{
      const value=await this.transport('/api/lists/inspect',{content_base64:source.content_base64,...(limits?{limits:clone(limits)}:{})});
      if(!this.current(generation))return null;
      validateEnvelope(value,source,true);this.state.inspection=clone(value);this.state.phase='selected';this.publish();return clone(value);
    }catch(error){return this.fail(generation,error)}
  }
  async preview(){
    if(!this.state.source||!this.state.options)throw new Error('Datei und eindeutige Formatoptionen erforderlich.');
    const source=clone(this.state.source),options=clone(this.state.options),generation=this.invalidate();this.state.phase='parsing';this.publish();
    try{
      const value=await this.transport('/api/lists/import',{content_base64:source.content_base64,options});
      if(!this.current(generation))return null;
      validateEnvelope(value,source,false);this.state.result=clone(value);this.state.phase='preview';this.publish();return clone(value);
    }catch(error){return this.fail(generation,error)}
  }
  accept(){
    if(this.state.phase!=='preview'||!this.state.result)throw new Error('Aktuelle fehlerfrei gelesene Vorschau erforderlich.');
    const summary=summarizeRows(this.state.result.rows);
    if(!summary.accepted)throw new Error('Keine akzeptierten HTTP(S)-Kandidaten vorhanden.');
    this.state.accepted=clone(this.state.result);this.state.phase='accepted';this.publish();
    return {input_imports:[clone(this.state.accepted)],seed_urls:summary.page_urls,sitemap_urls:summary.sitemap_urls};
  }
}

const helpers={MAX_INPUT_BYTES,buildListOptions,summarizeRows,ImportPreviewState};
if(typeof module!=='undefined'&&module.exports)module.exports=helpers;else window.ListImportTools=helpers;
})();
