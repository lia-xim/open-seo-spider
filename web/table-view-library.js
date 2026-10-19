'use strict';
(function(host,factory){
  const api=factory();
  if(typeof module==='object'&&module.exports)module.exports=api;
  else host.TableViewLibrary=api;
})(typeof globalThis==='object'?globalThis:this,function(){
// Declarative browser-local views. No DOM, storage, network or captured rows.
const ENGINE = 'open-seo-spider/table-views-v1';
// Bump whenever raw getter OR built-in section-filter meanings change, even if
// key/type/mandatory metadata is unchanged. Functions are never serialized.
const QUERY_SEMANTICS = 'open-seo-spider/table-query-v1';
const EXPRESSION_ENGINE = 'open-seo-spider/table-views-v2';
const EXPRESSION_QUERY_SEMANTICS = 'open-seo-spider/table-query-v2';
const LIMITS = Object.freeze({maxEntries:32,maxNameBytes:128,maxEntryBytes:524288,maxCatalogBytes:2097152});
const encoder = new TextEncoder();
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const own = (value,key) => Object.prototype.hasOwnProperty.call(value,key);
function fail(code,message) { const error = new Error(message); error.code=code; throw error; }
function attempt(action) {
  try{return {ok:true,value:action()};}catch(error){
    let code='invalid_definition',message='Die benannte Ansicht hat ein ungültiges Format.';
    try{if(error&&typeof error.code==='string')code=error.code;if(error&&typeof error.message==='string')message=error.message;}catch(_){/* Hostile thrown values still return an error Result. */}
    return {ok:false,error:{code,message}};
  }
}
function shape(value,fields) {
  if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).length!==fields.length||fields.some(key=>!own(value,key)))fail('invalid_shape','Unexpected or missing definition field');
  for(const key of fields){const descriptor=Object.getOwnPropertyDescriptor(value,key);if(!descriptor||!own(descriptor,'value'))fail('invalid_shape','Accessors are not definition data');}
}
function text(value,max,code,nonblank=false) {
  if(typeof value!=='string'||value.length>max||encoder.encode(value).length>max||nonblank&&!value.trim())fail(code,'Missing or oversized definition text');
  return value;
}
function identifier(value,max,code) {text(value,max,code,true);if(/[\u0000-\u001f\u007f-\u009f]/u.test(value))fail(code,'Control characters are not identifiers');return value;}
function denseArray(value,maximum,code) {
  if(!Array.isArray(value)||value.length>maximum)fail(code,'Invalid declaration array length');
  const output=[];
  for(let index=0;index<value.length;index++){
    const descriptor=Object.getOwnPropertyDescriptor(value,String(index));
    if(!descriptor||!own(descriptor,'value'))fail(code,'Declaration arrays require dense own data indices');
    output.push(descriptor.value);
  }
  return output;
}
function strictShape(value,fields) {
  if(!value||typeof value!=='object'||Array.isArray(value))fail('invalid_shape','Definition fields must be a data object');
  const prototype=Object.getPrototypeOf(value),keys=Reflect.ownKeys(value);
  if(prototype!==Object.prototype&&prototype!==null||keys.length!==fields.length||keys.some(key=>typeof key!=='string'||!fields.includes(key)))fail('invalid_shape','Unexpected or missing definition field');
  for(const key of fields){const descriptor=Object.getOwnPropertyDescriptor(value,key);if(!descriptor||!own(descriptor,'value')||!descriptor.enumerable)fail('invalid_shape','Definition fields must be enumerable own data');}
}
function strictArray(value,maximum,code) {
  if(!Array.isArray(value))fail(code,'Definition arrays must be dense own data');
  const length=Object.getOwnPropertyDescriptor(value,'length').value,keys=Reflect.ownKeys(value);
  if(length>maximum||keys.length!==length+1||keys.some(key=>typeof key!=='string'||key!=='length'&&!/^(0|[1-9][0-9]*)$/.test(key)))fail(code,'Unexpected declaration array fields');
  const output=[];for(let index=0;index<length;index++){const descriptor=Object.getOwnPropertyDescriptor(value,String(index));if(!descriptor||!own(descriptor,'value')||!descriptor.enumerable)fail(code,'Definition arrays must be dense own data');output.push(descriptor.value);}return output;
}
function cloneData(value,depth=0,budget={nodes:0}) {
  if(depth>16||++budget.nodes>4096)fail('definition_complexity','Definition data nesting/node limit exceeded');
  if(value===null||typeof value==='boolean'||typeof value==='string')return value;
  if(typeof value==='number'&&Number.isFinite(value))return value;
  if(Array.isArray(value))return denseArray(value,4096,'invalid_array').map(item=>cloneData(item,depth+1,budget));
  if(value&&typeof value==='object'){
    const output={};
    for(const key of Object.keys(value)){
      const descriptor=Object.getOwnPropertyDescriptor(value,key);
      if(!descriptor||!own(descriptor,'value'))fail('invalid_shape','Accessors are not definition data');
      Object.defineProperty(output,key,{value:cloneData(descriptor.value,depth+1,budget),enumerable:true,writable:true,configurable:true});
    }
    return output;
  }
  fail('invalid_data','Only finite JSON definition data is permitted');
}
function freeze(value){if(value&&typeof value==='object'){for(const child of Object.values(value))freeze(child);Object.freeze(value);}return value;}
function createVersion(explorer,overrides={},expression=false) {
  if(!explorer||typeof explorer.compile!=='function'||typeof explorer.normalizeText!=='function'||!explorer.LIMITS)throw Error('Explicit existing TableExplorer dependency required');
  if(expression&&(typeof explorer.compileExpression!=='function'||typeof explorer.validateExpression!=='function'))throw Error('Explicit TableExplorer expression dependency required');
  if(!overrides||typeof overrides!=='object'||Array.isArray(overrides))throw Error('Invalid library limits');
  for(const key of Object.keys(overrides))if(!own(LIMITS,key))throw Error('Unknown library limit');
  const limits={...LIMITS,...overrides};
  for(const [key,value]of Object.entries(limits))if(!Number.isSafeInteger(value)||value<1||value>LIMITS[key])throw Error('Fixture limits can only be lowered');
  const byteLimit=(raw,max,code)=>text(raw,max,code);
  const checkShape=expression?strictShape:shape,checkArray=expression?strictArray:denseArray;
  const engine=expression?EXPRESSION_ENGINE:ENGINE,version=expression?2:1,querySemantics=expression?EXPRESSION_QUERY_SEMANTICS:QUERY_SEMANTICS;
  function schema(value){
    value=checkArray(value,explorer.LIMITS.maxColumns,'invalid_schema');if(!value.length)fail('invalid_schema','Invalid schema length');
    const seen=new Set();
    return value.map(column=>{
      checkShape(column,['key','type','mandatory']);identifier(column.key,explorer.LIMITS.maxKeyBytes,'invalid_column');
      if(seen.has(column.key)||!['text','number','auto'].includes(column.type)||typeof column.mandatory!=='boolean')fail('invalid_schema','Duplicate key, invalid type or mandatory flag');
      seen.add(column.key);return {key:column.key,type:column.type,mandatory:column.mandatory};
    });
  }
  function filters(value){
    value=checkArray(value,128,'invalid_section_filters');if(!value.length)fail('invalid_section_filters','Invalid section filter contract');
    const seen=new Set();return value.map(key=>{identifier(key,128,'invalid_section_filter');if(seen.has(key))fail('invalid_section_filters','Duplicate section filter');seen.add(key);return key;});
  }
  // Descriptor hydration is metadata-only. Accidental evaluation must throw;
  // there is no synthetic row or fabricated observation in validation.
  const hydration=metadata=>metadata.map(column=>({...column,rawValue(){throw Error('View validation must not evaluate rows');}}));
  function checkedView(columns,metadata,sectionFilters,value){
    if(expression){
      // Validate the original declaration before cloning. Otherwise cloning
      // could silently erase a hidden/accessor/unknown AST field.
      checkShape(value,['sectionFilter','search','expression','sort','visibleKeys']);
      if(!sectionFilters.includes(value.sectionFilter))fail('invalid_section_filter','Section filter is not allowed by the explicit contract');
      const compiled=explorer.compileExpression(columns,{version:2,search:value.search,expression:value.expression,sort:value.sort,visibleKeys:value.visibleKeys});
      if(!compiled.ok)fail('query_'+compiled.error.code,compiled.error.message);
      const declared=explorer.validateExpression(columns,value.expression);if(!declared.ok)fail('query_'+declared.error.code,declared.error.message);
      const mandatory=metadata.filter(column=>column.mandatory).map(column=>column.key),projected=compiled.value.columns.map(column=>column.key);
      return {sectionFilter:value.sectionFilter,search:value.search,expression:declared.value,sort:cloneData(value.sort),visibleKeys:[...mandatory,...projected.filter(key=>!mandatory.includes(key))]};
    }
    const view=cloneData(value);shape(view,['sectionFilter','search','rules','sort','visibleKeys']);
    if(!sectionFilters.includes(view.sectionFilter))fail('invalid_section_filter','Section filter is not allowed by the explicit contract');
    const query={search:view.search,rules:view.rules,sort:view.sort,visibleKeys:view.visibleKeys};
    const compiled=explorer.compile(columns,query);
    if(!compiled.ok)fail('query_'+compiled.error.code,compiled.error.message);
    const mandatory=metadata.filter(column=>column.mandatory).map(column=>column.key);
    const projected=compiled.value.columns.map(column=>column.key);
    return {sectionFilter:view.sectionFilter,search:view.search,rules:view.rules,sort:view.sort,visibleKeys:[...mandatory,...projected.filter(key=>!mandatory.includes(key))]};
  }
  function contract(value){
    checkShape(value,['section','query_semantics','columns','sectionFilters']);identifier(value.section,explorer.LIMITS.maxSectionBytes,'invalid_section');identifier(value.query_semantics,256,'invalid_query_semantics');
    if(value.query_semantics!==querySemantics)fail('query_semantics_mismatch','Unsupported current query/section-filter semantics');
    const columns=checkArray(value.columns,explorer.LIMITS.maxColumns,'invalid_schema');
    if(expression)for(const column of columns){
      if(!column||typeof column!=='object'||Array.isArray(column))fail('invalid_schema','Invalid column contract');
      for(const key of ['key','type','mandatory','value','rawValue'])if(key in column){const descriptor=Object.getOwnPropertyDescriptor(column,key);if(!descriptor||!own(descriptor,'value'))fail('invalid_schema','Column metadata/getters must be own data');}
    }
    const metadata=schema(columns.map(column=>({key:column.key,type:column.type===undefined?'auto':column.type,mandatory:column.mandatory===true})));
    return {section:value.section,query_semantics:value.query_semantics,metadata,sectionFilters:filters(value.sectionFilters),columns};
  }
  function entry(value){
    checkShape(value,['id','name','section','query_semantics','schema','section_filters','view']);
    if(typeof value.id!=='string'||!uuid.test(value.id))fail('invalid_id','Definition ID must be a canonical lowercase UUID');
    identifier(value.name,limits.maxNameBytes,'invalid_name');identifier(value.section,explorer.LIMITS.maxSectionBytes,'invalid_section');identifier(value.query_semantics,256,'invalid_query_semantics');
    const metadata=schema(value.schema),sectionFilters=filters(value.section_filters);
    const output={id:value.id,name:value.name,section:value.section,query_semantics:value.query_semantics,schema:metadata,section_filters:sectionFilters,view:checkedView(hydration(metadata),metadata,sectionFilters,value.view)};
    byteLimit(JSON.stringify(output),limits.maxEntryBytes,'entry_byte_limit');return output;
  }
  function catalog(entries){
    entries=checkArray(entries,limits.maxEntries,'entry_limit');
    const ids=new Set(),names=new Set();const normalized=entries.map(value=>{
      // Name uniqueness uses the existing NFC/trim/German-lowercase policy.
      // Preserve original display spelling; accents/internal spaces stay distinct.
      const item=entry(value),key=JSON.stringify([item.section,explorer.normalizeText(item.name)]);
      if(ids.has(item.id))fail('duplicate_id','Duplicate definition ID');
      if(names.has(key))fail('duplicate_name','Duplicate NFC/trim/case-normalized name within this section');
      ids.add(item.id);names.add(key);return item;
    });
    const output={engine,version,entries:normalized},raw=JSON.stringify(output);
    byteLimit(raw,limits.maxCatalogBytes,'catalog_byte_limit');return {document:output,raw};
  }
  function makeEntry(metadata,currentContract,view){return attempt(()=>{
    checkShape(metadata,['id','name']);const current=contract(currentContract);
    return freeze(entry({id:metadata.id,name:metadata.name,section:current.section,query_semantics:current.query_semantics,schema:current.metadata,section_filters:current.sectionFilters,view:checkedView(current.columns,current.metadata,current.sectionFilters,view)}));
  });}
  function decodeCatalog(raw){return attempt(()=>{
    if(raw===null)return freeze({engine,version,entries:[]});
    byteLimit(raw,limits.maxCatalogBytes,'catalog_byte_limit');let value;try{value=JSON.parse(raw);}catch(_){fail('corrupt_catalog','Catalog is not valid JSON');}
    checkShape(value,['engine','version','entries']);if(value.engine!==engine||value.version!==version)fail('unsupported_version','Unsupported view catalog format');
    return freeze(catalog(value.entries).document);
  });}
  function encodeCatalog(entries){return attempt(()=>catalog(entries).raw);}
  function preparedView(current,value){
    const view=checkedView(current.columns,current.metadata,current.sectionFilters,value);
    return freeze({section:current.section,query_semantics:current.query_semantics,...view});
  }
  function prepareView(currentContract,value){return attempt(()=>preparedView(contract(currentContract),value));}
  function prepareApply(value,currentContract){return attempt(()=>{
    const selected=entry(value),current=contract(currentContract);
    if(selected.section!==current.section)fail('section_mismatch','Select the exact current section before Apply');
    if(selected.query_semantics!==current.query_semantics)fail('query_semantics_mismatch','Raw getter/query semantics changed; no automatic migration');
    if(JSON.stringify(selected.schema)!==JSON.stringify(current.metadata)||JSON.stringify(selected.section_filters)!==JSON.stringify(current.sectionFilters))fail('schema_mismatch','Full schema or section filter contract changed');
    return preparedView(current,selected.view);
  });}
  function prepareReplacement(expectedRaw,currentRaw,nextEntries){return attempt(()=>{
    for(const raw of [expectedRaw,currentRaw])if(raw!==null)byteLimit(raw,limits.maxCatalogBytes,'catalog_byte_limit');
    if(currentRaw!==expectedRaw)fail('stale_base','Catalog bytes changed; refresh before explicitly saving');
    const decoded=decodeCatalog(currentRaw);if(!decoded.ok)fail(decoded.error.code,decoded.error.message);
    // Storage adapter owns serialization, re-read and owner/query revision guards.
    return freeze({expectedRaw,nextRaw:catalog(nextEntries).raw});
  });}
  return Object.freeze({limits:Object.freeze(limits),makeEntry,decodeCatalog,encodeCatalog,prepareView,prepareApply,prepareReplacement});
}
function create(explorer,overrides={}){return createVersion(explorer,overrides,false);}
function createExpression(explorer,overrides={}){return createVersion(explorer,overrides,true);}
return Object.freeze({ENGINE,QUERY_SEMANTICS,EXPRESSION_ENGINE,EXPRESSION_QUERY_SEMANTICS,LIMITS,create,createExpression});
});
