'use strict';
// Pure library controls only; real browser/native/App acceptance is separate.
const test=require('node:test'),assert=require('node:assert/strict');
const explorer=require('../table-explorer.js');
const {create:createLibrary,ENGINE,QUERY_SEMANTICS,LIMITS}=require('../table-view-library.js');
const library=createLibrary(explorer);
const A='11111111-1111-4111-8111-111111111111',B='22222222-2222-4222-8222-222222222222',C='33333333-3333-4333-8333-333333333333';
let getterReads=0;
const columns=[{key:'url',type:'text',mandatory:true,value:row=>{getterReads++;return row.url;}},{key:'title',type:'text',value:row=>row.title},{key:'count',type:'number',value:row=>row.count}];
const contract={section:'titles',query_semantics:QUERY_SEMANTICS,columns,sectionFilters:['all','missing']};
const view={sectionFilter:'all',search:' CAFE\u0301 ',rules:[{key:'count',op:'range',min:0,max:2},{key:'title',op:'equals',text:'café'}],sort:{key:'count',direction:-1},visibleKeys:['url']};
function ok(result){assert.equal(result.ok,true,JSON.stringify(result));return result.value;}
function rejected(result,code){assert.equal(result.ok,false);assert.equal(Object.hasOwn(result,'value'),false);if(code)assert.equal(result.error.code,code);return result.error;}
const entry=(id=A,name='Café',current=contract,query=view)=>ok(library.makeEntry({id,name},current,query));

test('full atomic payload retains every accepted query field and no capture identity or result data',()=>{
  const original=JSON.stringify(view),selected=entry(),applied=ok(library.prepareApply(selected,contract));
  assert.deepEqual(applied,{section:'titles',query_semantics:QUERY_SEMANTICS,sectionFilter:'all',search:' CAFE\u0301 ',rules:[{key:'count',op:'range',min:0,max:2},{key:'title',op:'equals',text:'café'}],sort:{key:'count',direction:-1},visibleKeys:['url']});
  assert.equal(JSON.stringify(view),original);assert.equal(getterReads,0,'Definition validation never evaluates row getters');
  assert(Object.isFrozen(applied));assert(Object.isFrozen(applied.rules[0]));assert(Object.isFrozen(selected));
  const raw=ok(library.encodeCatalog([selected]));assert.deepEqual(ok(library.decodeCatalog(raw)).entries,[selected]);
  assert.deepEqual(Object.keys(selected),['id','name','section','query_semantics','schema','section_filters','view']);
  for(const forbidden of ['projectId','snapshot_token','generation','rows','counts'])assert(!raw.includes('"'+forbidden+'":'));
});
test('existing compiler independently selects complete original rows with hidden NFC text and measured zero',()=>{
  const selected=entry(),payload=ok(library.prepareApply(selected,contract));
  const rows=[{url:'two',title:'CAFÉ',count:2},{url:'zero',title:'Cafe\u0301',count:0},{url:'unknown',title:'Café',count:null},{url:'empty',title:'',count:0},{url:'outside',title:'Café',count:3}];
  const {section,query_semantics,sectionFilter,...query}=payload;
  const compiled=ok(explorer.compile(columns,query)),result=ok(compiled.rows(rows));
  assert.deepEqual(result,[rows[0],rows[1]]);assert.equal(result[0],rows[0]);assert.equal(result[1],rows[1]);
  assert.deepEqual(compiled.columns.map(column=>column.key),['url']);
  const zeroQuery={...query,rules:[{key:'count',op:'range',min:0,max:0}],search:''};assert.deepEqual(ok(ok(explorer.compile(columns,zeroQuery)).rows(rows)),[rows[3],rows[1]]);
});
test('section-name uniqueness folds NFC/trim/German case but preserves display spelling, accents and inner spaces',()=>{
  const first=entry(A,'  Cafe\u0301  '),distinct=entry(B,'Cafe'),other=entry(C,'CAFÉ',{...contract,section:'meta'}),source=[first,distinct,other],raw=ok(library.encodeCatalog(source));
  assert.deepEqual(ok(library.decodeCatalog(raw)).entries.map(item=>[item.section,item.name]),[['titles','  Cafe\u0301  '],['titles','Cafe'],['meta','CAFÉ']]);
  for(const name of ['Café','CAFÉ',' cafe\u0301 ','  café'])rejected(library.encodeCatalog([first,entry(B,name)]),'duplicate_name');
  assert(library.encodeCatalog([entry(A,'Café Plan'),entry(B,'Café  Plan')]).ok,'Internal whitespace is not collapsed');
  assert(library.encodeCatalog([entry(A,'Cafe'),entry(B,'Café')]).ok,'Accents are not folded');
  rejected(library.encodeCatalog([first,entry(A,'another')]),'duplicate_id');assert.deepEqual(source,[first,distinct,other]);
});
test('section, full schema, filter set and explicitly maintained raw getter semantics must all match',()=>{
  const selected=entry(),before=JSON.stringify(selected);
  rejected(library.prepareApply(selected,{...contract,section:'meta'}),'section_mismatch');
  rejected(library.prepareApply(selected,{...contract,query_semantics:'table-query-v1/titles-raw-v2'}),'query_semantics_mismatch');
  for(const changed of [{...contract,columns:[...columns,{key:'new',type:'text',value:row=>row.new}]},{...contract,columns:columns.map(column=>column.key==='count'?{...column,type:'text'}:column)},{...contract,sectionFilters:['all']}])rejected(library.prepareApply(selected,changed),'schema_mismatch');
  assert.equal(JSON.stringify(selected),before);
  // Getter identity is deliberately not serialized. Maintainers must bump the
  // explicit semantics version for changed raw meanings with the same keys.
  assert(ok(library.prepareApply(selected,{...contract,columns:columns.map(column=>({...column,value:column.value}))})));
});
test('invalid typed queries refuse complete Apply without replacing accepted data or coercing unknowns to zero',()=>{
  const accepted=entry(),before=JSON.stringify(accepted);
  for(const query of [{...view,sectionFilter:'invented'},{...view,sort:{key:'missing',direction:1}},{...view,rules:[{key:'count',op:'range',min:2,max:0}]},{...view,rules:[{key:'count',op:'min',min:'0'}]},{...view,rules:[{key:'title',op:'min',min:1}]},{...view,rules:[{key:'count',op:'min',min:Infinity}]},{...view,visibleKeys:['url','url']},{...view,rows:[]}])rejected(library.makeEntry({id:B,name:'invalid'},contract,query));
  assert.equal(JSON.stringify(accepted),before);assert.deepEqual(ok(library.prepareApply(accepted,contract)).rules,view.rules);
  assert.deepEqual(entry(B,'mandatory',contract,{...view,visibleKeys:['title']}).view.visibleKeys,['url','title']);
});
test('corrupt, wrong-version, unknown-field and stale-base bytes are preserved and never planned as an overwrite',()=>{
  const raw=ok(library.encodeCatalog([entry()]));
  const invalid=['{broken','null',JSON.stringify({engine:ENGINE,version:2,entries:[]}),JSON.stringify({engine:ENGINE,version:1,entries:[],extra:true})];
  for(const bytes of invalid){const before=bytes;rejected(library.decodeCatalog(bytes));rejected(library.prepareReplacement(bytes,bytes,[]));assert.equal(bytes,before);}
  rejected(library.prepareReplacement(raw,raw+' ',[]),'stale_base');assert.equal(ok(library.decodeCatalog(raw)).entries.length,1);
  const planned=ok(library.prepareReplacement(raw,raw,[entry(B,'replacement')]));assert.equal(planned.expectedRaw,raw);assert.equal(ok(library.decodeCatalog(planned.nextRaw)).entries[0].name,'replacement');
  assert.deepEqual(ok(library.decodeCatalog(null)),{engine:ENGINE,version:1,entries:[]});
});
test('exact UTF8 name boundary, controls, IDs and entry count have meaningful refusal boundaries',()=>{
  assert.equal(entry(A,'雪'.repeat(42)+'ab').name.length,44);rejected(library.makeEntry({id:A,name:'雪'.repeat(43)},contract,view),'invalid_name');
  for(const name of ['', '   ','bad\nname','bad\u0080name'])rejected(library.makeEntry({id:A,name},contract,view),'invalid_name');
  rejected(library.makeEntry({id:A.toUpperCase().replace('11111111','AAAAAAAA'),name:'bad'},contract,view),'invalid_id');
  const lower=createLibrary(explorer,{maxEntries:1});assert(lower.encodeCatalog([entry()]).ok);rejected(lower.encodeCatalog([entry(),entry(B,'two')]),'entry_limit');
});
test('physical serialized byte exact/-1 boundaries reject without truncation or cap changes',()=>{
  const selected=entry(),entryBytes=Buffer.byteLength(JSON.stringify(selected)),raw=ok(library.encodeCatalog([selected])),catalogBytes=Buffer.byteLength(raw);
  assert.equal(ok(createLibrary(explorer,{maxEntryBytes:entryBytes}).encodeCatalog([selected])),raw);
  rejected(createLibrary(explorer,{maxEntryBytes:entryBytes-1}).encodeCatalog([selected]),'entry_byte_limit');
  assert.equal(ok(createLibrary(explorer,{maxCatalogBytes:catalogBytes}).encodeCatalog([selected])),raw);
  rejected(createLibrary(explorer,{maxCatalogBytes:catalogBytes-1}).encodeCatalog([selected]),'catalog_byte_limit');
  rejected(createLibrary(explorer,{maxCatalogBytes:catalogBytes-1}).decodeCatalog(raw),'catalog_byte_limit');
  assert.equal(Buffer.byteLength(raw),catalogBytes);assert.equal(ok(library.encodeCatalog([selected])),raw);
});
test('existing rule/search bounds and exact schemas remain authoritative; unsupported data cannot be persisted',()=>{
  const text='雪'.repeat(1365)+'a';assert.equal(Buffer.byteLength(text),4096);assert(library.makeEntry({id:A,name:'boundary'},contract,{...view,search:text}).ok);
  rejected(library.makeEntry({id:A,name:'too big'},contract,{...view,search:text+'a'}),'query_search_text_limit');
  assert(library.makeEntry({id:A,name:'sixteen'},contract,{...view,rules:Array.from({length:16},()=>({key:'count',op:'known'}))}).ok);
  rejected(library.makeEntry({id:A,name:'seventeen'},contract,{...view,rules:Array.from({length:17},()=>({key:'count',op:'known'}))}),'query_rule_limit');
  for(const patch of [{projectId:'must-not-save'},{snapshot_token:'must-not-save'},{generation:1},{rows:[{url:'must-not-save'}]}])rejected(library.decodeCatalog(JSON.stringify({engine:ENGINE,version:1,entries:[{...entry(),...patch}]})));
  for(const limits of [{maxEntries:33},{maxCatalogBytes:0},{unknown:1}])assert.throws(()=>createLibrary(explorer,limits));
});
test('sparse and inherited indices cannot produce a successful unreadable catalog or Apply payload',()=>{
  const selected=entry();
  function arrayWithMissingOrInheritedIndex(item,inherited){
    const value=new Array(1);
    if(inherited){const prototype=Object.create(Array.prototype);Object.defineProperty(prototype,'0',{value:item,enumerable:true});Object.setPrototypeOf(value,prototype);}
    return value;
  }
  for(const inherited of [false,true]){
    const entries=arrayWithMissingOrInheritedIndex(selected,inherited);
    rejected(library.encodeCatalog(entries),'entry_limit');rejected(library.prepareReplacement(null,null,entries),'entry_limit');
    for(const [field,item]of [['rules',{key:'count',op:'known'}],['visibleKeys','url']])rejected(library.makeEntry({id:B,name:'bad '+field},contract,{...view,[field]:arrayWithMissingOrInheritedIndex(item,inherited)}),'invalid_array');
    rejected(library.makeEntry({id:B,name:'bad columns'},{...contract,columns:arrayWithMissingOrInheritedIndex(columns[0],inherited)},view),'invalid_schema');
    rejected(library.makeEntry({id:B,name:'bad filter'},{...contract,sectionFilters:arrayWithMissingOrInheritedIndex('all',inherited)},view),'invalid_section_filters');
    rejected(library.encodeCatalog([{...selected,schema:arrayWithMissingOrInheritedIndex(selected.schema[0],inherited)}]),'invalid_schema');
    rejected(library.prepareApply({...selected,section_filters:arrayWithMissingOrInheritedIndex('all',inherited)},contract),'invalid_section_filters');
    for(const [field,item]of [['rules',{key:'count',op:'known'}],['visibleKeys','url']])rejected(library.prepareApply({...selected,view:{...selected.view,[field]:arrayWithMissingOrInheritedIndex(item,inherited)}},contract),'invalid_array');
  }
  let reads=0;const accessor=new Array(1);Object.defineProperty(accessor,'0',{get(){reads++;return selected;}});rejected(library.encodeCatalog(accessor),'entry_limit');assert.equal(reads,0,'Own accessors must not become declaration data either');
  const decorated=[selected];Object.defineProperty(decorated,'map',{get(){reads++;throw Error('Do not execute array-provided mapping code');}});assert.equal(ok(library.decodeCatalog(ok(library.encodeCatalog(decorated)))).entries.length,1);assert.equal(reads,0,'Only the validated own index data is copied before mapping');
});
test('every successful encoded fixture and replacement plan decodes to complete stable definition data',()=>{
  const fixtures=[[],[entry()],[entry(A,'First'),entry(B,'Second')],[entry(A,'  CAFE\u0301  '),entry(B,'Another',{...contract,section:'meta'},{...view,sectionFilter:'missing',search:'',rules:[],sort:{key:'url',direction:1},visibleKeys:['title','count']})]];
  for(const entries of fixtures){
    const raw=ok(library.encodeCatalog(entries)),decoded=ok(library.decodeCatalog(raw));
    assert.equal(ok(library.encodeCatalog(decoded.entries)),raw,'Successful encoding has a stable readable roundtrip');
    assert.equal(decoded.entries.length,entries.length);for(let index=0;index<entries.length;index++){assert.equal(decoded.entries[index].name,entries[index].name);assert.deepEqual(decoded.entries[index].view,entries[index].view);}
    const replacement=ok(library.prepareReplacement(null,null,entries));assert.equal(replacement.nextRaw,raw);assert.deepEqual(ok(library.decodeCatalog(replacement.nextRaw)),decoded);
  }
});
test('non-default section filter survives encoding, decoding and the complete explicit Apply payload',()=>{
  const selected=entry(A,'Missing titles',contract,{...view,sectionFilter:'missing',search:'',rules:[],sort:{key:'url',direction:1},visibleKeys:['url','count']});
  const raw=ok(library.encodeCatalog([selected])),decoded=ok(library.decodeCatalog(raw));
  assert.deepEqual(ok(library.prepareApply(decoded.entries[0],contract)),{section:'titles',query_semantics:QUERY_SEMANTICS,sectionFilter:'missing',search:'',rules:[],sort:{key:'url',direction:1},visibleKeys:['url','count']});
  assert.equal(ok(library.encodeCatalog(decoded.entries)),raw);
});
test('stale absent-to-present and present-to-absent bases refuse independently without replacement plans',()=>{
  const raw=ok(library.encodeCatalog([entry()]));
  rejected(library.prepareReplacement(null,raw,[]),'stale_base');
  rejected(library.prepareReplacement(raw,null,[]),'stale_base');
  assert.equal(ok(library.decodeCatalog(raw)).entries.length,1);assert.equal(ok(library.decodeCatalog(null)).entries.length,0);
});
test('schema and section-filter ordering is deliberately conservative and filter meaning belongs to query_semantics',()=>{
  const selected=entry();
  rejected(library.prepareApply(selected,{...contract,columns:[columns[0],columns[2],columns[1]]}),'schema_mismatch');
  rejected(library.prepareApply(selected,{...contract,sectionFilters:['missing','all']}),'schema_mismatch');
  rejected(library.prepareApply(selected,{...contract,query_semantics:'table-query-v1/titles-filter-meaning-v2'}),'query_semantics_mismatch');
});
test('prepareView independently validates the complete App payload without requiring a saved definition ID or name',()=>{
  const payload=ok(library.prepareView(contract,{...view,sectionFilter:'missing',visibleKeys:['title']}));
  assert.deepEqual(payload,{section:'titles',query_semantics:QUERY_SEMANTICS,sectionFilter:'missing',search:view.search,rules:view.rules,sort:view.sort,visibleKeys:['url','title']});
  assert(Object.isFrozen(payload));assert(Object.isFrozen(payload.rules));assert.equal(Object.hasOwn(payload,'id'),false);
  rejected(library.prepareView(contract,{...view,sectionFilter:'unsupported'}),'invalid_section_filter');
  rejected(library.prepareView(contract,{...view,projectId:'must-not-retarget'}),'invalid_shape');
  const hostile=new Proxy({},{ownKeys(){throw null;}});rejected(library.prepareView(contract,hostile),'invalid_definition');
});
test('stored foreign semantics stay inspectable but cannot be applied through an unsupported current contract',()=>{
  const selected={...entry(),query_semantics:'open-seo-spider/table-query-v2'};
  const raw=ok(library.encodeCatalog([selected])),decoded=ok(library.decodeCatalog(raw));
  assert.equal(decoded.entries[0].query_semantics,selected.query_semantics);
  rejected(library.prepareApply(decoded.entries[0],contract),'query_semantics_mismatch');
  rejected(library.prepareView({...contract,query_semantics:selected.query_semantics},view),'query_semantics_mismatch');
  assert.equal(ok(library.encodeCatalog(decoded.entries)),raw);
});
test('fixed configuration bounds enforce actual2MiB aggregate refusal without lowering query limits',()=>{
  assert.deepEqual(LIMITS,{maxEntries:32,maxNameBytes:128,maxEntryBytes:524288,maxCatalogBytes:2097152});
  const large={...view,search:'x'.repeat(4096),rules:Array.from({length:16},()=>({key:'title',op:'contains',text:'x'.repeat(4096)}))};
  const entries=Array.from({length:32},(_,index)=>entry('00000000-0000-4000-8000-'+String(index).padStart(12,'0'),'Large '+index,contract,large));
  for(const selected of entries)assert(Buffer.byteLength(JSON.stringify(selected))<LIMITS.maxEntryBytes);
  assert(library.encodeCatalog(entries.slice(0,28)).ok);rejected(library.encodeCatalog(entries),'catalog_byte_limit');
  assert.equal(large.search.length,4096);assert.equal(large.rules.length,16);
});

test('the global32-entry boundary covers every section and a33rd entry is refused intact',()=>{
  const declarations=Array.from({length:32},(_,index)=>entry('00000000-0000-4000-8000-'+String(index).padStart(12,'0'),'View '+index,{...contract,section:index%2?'meta':'titles'}));
  const raw=ok(library.encodeCatalog(declarations)),decoded=ok(library.decodeCatalog(raw));
  assert.equal(decoded.entries.length,32);assert.equal(decoded.entries.filter(value=>value.section==='titles').length,16);assert.equal(decoded.entries.filter(value=>value.section==='meta').length,16);
  const overflow=entry('00000000-0000-4000-8000-000000000032','Thirty third',{...contract,section:'other'});
  rejected(library.encodeCatalog([...declarations,overflow]),'entry_limit');
  assert.equal(ok(library.encodeCatalog(declarations)),raw);assert.deepEqual(decoded.entries.map(value=>value.id),declarations.map(value=>value.id));
});
