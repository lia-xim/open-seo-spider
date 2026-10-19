'use strict';
// Pure portability controls. Real File/DOM/storage/native acceptance is separate.
const test=require('node:test'),assert=require('node:assert/strict');
const portability=require('../table-view-portability.js');
const explorer=require('../table-explorer.js'),definitions=require('../table-view-library.js');
const A='11111111-1111-4111-8111-111111111111',B='22222222-2222-4222-8222-222222222222',C='33333333-3333-4333-8333-333333333333',D='44444444-4444-4444-8444-444444444444';
const ok=r=>{assert.equal(r.ok,true,JSON.stringify(r));return r.value;},bad=(r,code)=>{assert.equal(r.ok,false,JSON.stringify(r));assert.equal(Object.hasOwn(r,'value'),false);if(code)assert.equal(r.error.code,code);return r.error;};
const utf8=s=>new TextEncoder().encode(s),clone=value=>JSON.parse(JSON.stringify(value));
function setup(version,limits={}){
  const library=(version===1?definitions.create:definitions.createExpression)(explorer,limits),port=portability.create(version,limits);
  const columns=[{key:'url',type:'text',mandatory:true,value:row=>row.url},{key:'count',type:'number',value:row=>row.count}],contract={section:'titles',query_semantics:version===1?definitions.QUERY_SEMANTICS:definitions.EXPRESSION_QUERY_SEMANTICS,columns,sectionFilters:['all','missing']};
  const rule={key:'count',op:'range',min:0,max:0},view={sectionFilter:'missing',search:'  Cafe\u0301  ',...(version===1?{rules:[rule]}:{expression:{kind:'any',children:[{kind:'rule',rule},{kind:'rule',rule:{key:'count',op:'unknown'}}]}}),sort:{key:'count',direction:-1},visibleKeys:['url']};
  const entry=(id=A,name='Café',section='titles',patch={})=>ok(library.makeEntry({id,name},{...contract,section},{...view,...patch}));
  const raw=entries=>ok(library.encodeCatalog(entries)),preview=(current,incoming)=>ok(port.previewImport(current,utf8(raw(incoming))));
  return{library,port,contract,view,entry,raw,preview};
}
for(const version of[1,2]){
  test('v'+version+' exports selected or all complete definitions with canonical stable bytes and no rows',()=>{
    const f=setup(version),entries=[f.entry(),f.entry(B,'Other','status')],raw=f.raw(entries),all=ok(f.port.exportDefinitions(raw,{kind:'all'})),selected=ok(f.port.exportDefinitions(raw,{kind:'selected',id:B}));
    assert.equal(all.raw,raw);assert.equal(all.bytes,Buffer.byteLength(raw));assert.equal(all.mime,'application/json');assert.equal(all.filename,'open-seo-spider-table-views-v'+version+'.json');
    assert.equal(selected.raw,f.raw([entries[1]]));assert.deepEqual(selected.entries,[entries[1]]);assert.equal(selected.filename,'open-seo-spider-table-views-v'+version+'-selected.json');assert(Object.isFrozen(selected.entries[0].view));
    assert.deepEqual(JSON.parse(all.raw).entries,entries);bad(f.port.exportDefinitions(raw,{kind:'selected',id:C}),'unknown_selection');bad(f.port.exportDefinitions(raw,{kind:'all',id:A}),'invalid_shape');
    assert.equal(ok(f.port.exportDefinitions(null,{kind:'all'})).raw,f.raw([]));bad(f.port.exportDefinitions('{broken',{kind:'all'}));
  });
  test('v'+version+' previews all sections without effects and only prepares complete immutable definitions',()=>{
    const f=setup(version),source=f.entry(),other=f.entry(B,'Other','status'),incomingBytes=utf8(f.raw([source,other])),preview=ok(f.port.previewImport(null,incomingBytes));
    incomingBytes.fill(0);assert.deepEqual(preview.items.map(x=>[x.id,x.section,x.disposition]),[[A,'titles','add'],[B,'status','add']]);assert(Object.isFrozen(preview.incomingEntries[0].view));assert.equal(preview.engine,'open-seo-spider/table-views-v'+version);
    const plan=ok(f.port.prepareImport(preview,{}));assert.deepEqual(plan.entries,[source,other]);assert.equal(plan.expectedRaw,null);assert.equal(plan.nextRaw,f.raw([source,other]));assert.equal(plan.changed,true);assert.deepEqual(plan.addedIds,[A,B]);assert(Object.isFrozen(plan));
    assert.equal(source.view.search,'  Cafe\u0301  ');assert.deepEqual(Object.keys(plan).sort(),['addedIds','changed','entries','expectedRaw','identicalIds','nextRaw','replacedIds','skippedIds'].sort());
  });
  test('v'+version+' exact ID equality is a no-op preserving observed whitespace bytes',()=>{
    const f=setup(version),entry=f.entry(),raw='  '+f.raw([entry])+'\n',preview=f.preview(raw,[entry]),plan=ok(f.port.prepareImport(preview,{}));
    assert.equal(preview.items[0].disposition,'identical');assert.equal(plan.changed,false);assert.equal(plan.expectedRaw,raw);assert.equal(plan.nextRaw,raw);assert.deepEqual(plan.identicalIds,[A]);assert.deepEqual(plan.addedIds,[]);
    const empty=ok(f.port.prepareImport(f.preview(null,[]),{}));assert.equal(empty.changed,false);assert.equal(empty.nextRaw,null);
  });
  test('v'+version+' ID conflict has no default overwrite; skip, replace and copy are explicit',()=>{
    const f=setup(version),existing=f.entry(),incoming=f.entry(A,'Incoming','titles',{search:'new query'}),base=f.raw([existing]),preview=f.preview(base,[incoming]);
    assert.equal(preview.items[0].replaceTargetId,A);bad(f.port.prepareImport(preview,{}),'unresolved_conflict');
    const skipped=ok(f.port.prepareImport(preview,{[A]:{action:'skip'}}));assert.equal(skipped.changed,false);assert.deepEqual(skipped.entries,[existing]);assert.deepEqual(skipped.skippedIds,[A]);
    const replaced=ok(f.port.prepareImport(preview,{[A]:{action:'replace',targetId:A}}));assert.deepEqual(replaced.entries,[{...incoming,name:existing.name}]);assert.deepEqual(replaced.replacedIds,[A]);
    const copied=ok(f.port.prepareImport(preview,{[A]:{action:'copy',id:C,name:'Explicit copy'}}));assert.deepEqual(copied.entries,[existing,{...incoming,id:C,name:'Explicit copy'}]);assert.deepEqual(copied.addedIds,[C]);assert.equal(base,f.raw([existing]));
  });
  test('v'+version+' normalized name conflict preserves replacement target ID and display name',()=>{
    const f=setup(version),existing=f.entry(A,'  Cafe\u0301  '),incoming=f.entry(B,'CAFÉ','titles',{search:'replacement'}),preview=f.preview(f.raw([existing]),[incoming]);
    assert.equal(preview.items[0].nameTargetId,A);assert.equal(preview.items[0].replaceTargetId,A);const plan=ok(f.port.prepareImport(preview,{[B]:{action:'replace',targetId:A}}));assert.deepEqual(plan.entries,[{...incoming,id:A,name:existing.name}]);
    bad(f.port.prepareImport(preview,{[B]:{action:'replace',targetId:B}}),'invalid_replace_target');bad(f.port.prepareImport(preview,{[B]:{action:'copy',id:C,name:'café'}}),'duplicate_name');
    const other=f.entry(B,'CAFÉ','status');assert.equal(f.preview(f.raw([existing]),[other]).items[0].disposition,'add');
  });
  test('v'+version+' ambiguous ID/name targets and cross-section IDs can only skip or fresh-copy',()=>{
    const f=setup(version),first=f.entry(A,'First'),second=f.entry(B,'Second'),incoming=f.entry(A,'Second'),preview=f.preview(f.raw([first,second]),[incoming]);
    assert.equal(preview.items[0].reason,'ambiguous_targets');assert.equal(preview.items[0].replaceTargetId,null);for(const targetId of[A,B])bad(f.port.prepareImport(preview,{[A]:{action:'replace',targetId}}),'invalid_replace_target');
    assert.equal(ok(f.port.prepareImport(preview,{[A]:{action:'copy',id:C,name:'Copied'}})).entries.length,3);
    const foreign=f.preview(f.raw([first]),[f.entry(A,'Other','status')]);assert.equal(foreign.items[0].reason,'foreign_section_id');bad(f.port.prepareImport(foreign,{[A]:{action:'replace',targetId:A}}),'invalid_replace_target');assert.equal(ok(f.port.prepareImport(foreign,{[A]:{action:'skip'}})).changed,false);
  });
  test('v'+version+' replacement mapping is atomic and copied IDs must be genuinely fresh',()=>{
    const f=setup(version),base=f.entry(A,'Original'),one=f.entry(A,'New title'),two=f.entry(B,'Original'),preview=f.preview(f.raw([base]),[one,two]);
    bad(f.port.prepareImport(preview,{[A]:{action:'replace',targetId:A},[B]:{action:'replace',targetId:A}}),'duplicate_replace_target');
    for(const id of[A,B])bad(f.port.prepareImport(preview,{[A]:{action:'copy',id,name:'Copy'},[B]:{action:'skip'}}),'copy_id_not_fresh');
    const resolved=ok(f.port.prepareImport(preview,{[A]:{action:'replace',targetId:A},[B]:{action:'copy',id:C,name:'Second copy'}}));assert.equal(resolved.entries.length,2);assert.equal(resolved.entries[0].name,'Original');assert.equal(resolved.entries[0].id,A);assert.equal(resolved.entries[1].id,C);
  });
  test('v'+version+' whole merged32-entry cap permits explicit skips without first32 salvage',()=>{
    const f=setup(version),entries=Array.from({length:32},(_,index)=>f.entry('00000000-0000-4000-8000-'+String(index).padStart(12,'0'),'View '+index,index%2?'status':'titles')),base=f.raw(entries),incoming=f.entry(D,'New'),preview=f.preview(base,[incoming]);
    bad(f.port.prepareImport(preview,{}),'entry_limit');const skipped=ok(f.port.prepareImport(preview,{[D]:{action:'skip'}}));assert.equal(skipped.changed,false);assert.equal(skipped.entries.length,32);assert.equal(skipped.nextRaw,base);
  });
  test('v'+version+' malformed/wrong-version file and corrupt destination refuse complete preview',()=>{
    const f=setup(version),raw=f.raw([f.entry()]);for(const text of['{broken','null','[]',JSON.stringify({...JSON.parse(raw),extra:[]}),JSON.stringify({...JSON.parse(raw),version:3})])bad(f.port.previewImport(null,utf8(text)));
    const other=setup(version===1?2:1);bad(f.port.previewImport(null,utf8(other.raw([other.entry()]))),'unsupported_version');bad(f.port.previewImport('{broken',utf8(raw)));bad(f.port.previewImport('x'.repeat(2097153),utf8(raw)));
    const duplicate=JSON.parse(raw);duplicate.entries.push(duplicate.entries[0]);bad(f.port.previewImport(null,utf8(JSON.stringify(duplicate))),'duplicate_id');const extra=JSON.parse(raw);extra.entries[0].rows=[];bad(f.port.previewImport(null,utf8(JSON.stringify(extra))));
  });
  test('v'+version+' file UTF8/BOM and exact physical size boundaries are explicit',()=>{
    const f=setup(version),raw=f.raw([f.entry()]),bytes=utf8(raw),bom=Uint8Array.from([239,187,191,...bytes]);assert.equal(ok(f.port.previewImport(null,bom)).incomingEntries[0].name,'Café');
    bad(f.port.previewImport(null,Uint8Array.from([239,187,191,...bom])),'duplicate_bom');bad(f.port.previewImport(null,Uint8Array.from([123,34,195,40,34,125])),'invalid_utf8');bad(f.port.previewImport(null,new Uint8Array()),'file_byte_limit');bad(f.port.previewImport(null,new Uint8Array(2097153)),'file_byte_limit');bad(f.port.previewImport(null,raw),'invalid_file_bytes');
    const exact=setup(version,{maxCatalogBytes:bytes.length});assert(exact.port.previewImport(null,bytes).ok);bad(exact.port.previewImport(null,Uint8Array.from([...bytes,32])),'file_byte_limit');
    assert.equal(ok(f.port.exportDefinitions(raw,{kind:'all'})).raw.charCodeAt(0),123);
  });
  test('v'+version+' choices reject unknown, partial, accessor, hidden and symbol data without getters',()=>{
    const f=setup(version),preview=f.preview(null,[f.entry()]);let reads=0;
    for(const choices of[[],{[B]:{action:'skip'}},{[A]:{action:'skip',targetId:B}},{[A]:{action:'replace'}},{[A]:{action:'invented'}}])bad(f.port.prepareImport(preview,choices));
    const getter={};Object.defineProperty(getter,A,{enumerable:true,get(){reads++;return{action:'skip'};}});bad(f.port.prepareImport(preview,getter));
    for(const key of['hidden',Symbol('hidden')]){const choice={action:'skip'};Object.defineProperty(choice,key,{value:true});bad(f.port.prepareImport(preview,{[A]:choice}));}
    const choice={};Object.defineProperty(choice,'action',{enumerable:true,get(){reads++;return'skip';}});bad(f.port.prepareImport(preview,{[A]:choice}));assert.equal(reads,0);
    bad(f.port.prepareImport(clone(preview),{}),'foreign_preview');bad(setup(version).port.prepareImport(preview,{}),'foreign_preview');bad(setup(version===1?2:1).port.prepareImport(preview,{}),'foreign_preview');
  });
  test('v'+version+' foreign semantics stay inspectable without implicit apply/migration',()=>{
    const f=setup(version),entry={...f.entry(),query_semantics:'future-semantic-version'},preview=f.preview(null,[entry]),plan=ok(f.port.prepareImport(preview,{}));assert.equal(plan.entries[0].query_semantics,'future-semantic-version');bad(f.library.prepareApply(plan.entries[0],f.contract),'query_semantics_mismatch');assert.equal(ok(f.port.exportDefinitions(plan.nextRaw,{kind:'all'})).raw,f.raw([entry]));
  });
  test('v'+version+' an invalid later entry or invalid copy name cannot produce a partial plan',()=>{
    const f=setup(version),base=f.raw([f.entry()]),incoming=JSON.parse(f.raw([f.entry(B,'Valid'),f.entry(C,'Later invalid')]));incoming.entries[1].view.visibleKeys.push('invented');bad(f.port.previewImport(base,utf8(JSON.stringify(incoming))));assert.equal(base,f.raw([f.entry()]));
    const preview=f.preview(base,[f.entry(B,'New')]);for(const name of['','  ','bad\nname','雪'.repeat(43)])bad(f.port.prepareImport(preview,{[B]:{action:'copy',id:C,name}}),'invalid_name');
    const accepted=ok(f.port.prepareImport(preview,{[B]:{action:'copy',id:C,name:'雪'.repeat(42)+'ab'}}));assert.equal(accepted.entries.length,2);assert.equal(accepted.entries[1].name,'雪'.repeat(42)+'ab');
  });
  test('v'+version+' bytes and prepared entries cannot be retargeted by user methods or later resolution edits',()=>{
    const f=setup(version),bytes=utf8(f.raw([f.entry()]));let reads=0;
    Object.defineProperty(bytes,'byteLength',{get(){reads++;throw Error('Must use internal typed-array slots');}});Object.defineProperty(bytes,Symbol.iterator,{get(){reads++;throw Error('Must not use caller iterator');}});Object.defineProperty(bytes,'slice',{get(){reads++;throw Error('Must not use caller slice');}});
    const preview=ok(f.port.previewImport(null,bytes)),choices={[A]:{action:'copy',id:C,name:'Frozen copy'}},prepared=ok(f.port.prepareImport(preview,choices));choices[A].name='Changed';choices[A].id=D;
    assert.equal(reads,0);assert.equal(prepared.entries[0].id,C);assert.equal(prepared.entries[0].name,'Frozen copy');assert(Object.isFrozen(prepared.entries[0].view));assert(Object.isFrozen(prepared.addedIds));assert.throws(()=>{prepared.entries[0].view.search='mutate';},TypeError);
  });
  test('v'+version+' exact entry and catalog fixture bounds fail as a whole without raising production caps',()=>{
    const f=setup(version),entry=f.entry(A,'x'),entryBytes=Buffer.byteLength(JSON.stringify(entry)),tightEntry=setup(version,{maxEntryBytes:entryBytes}),preview=tightEntry.preview(null,[entry]);
    assert.equal(ok(tightEntry.port.prepareImport(preview,{})).entries.length,1);bad(tightEntry.port.prepareImport(preview,{[A]:{action:'copy',id:C,name:'longer name'}}),'entry_byte_limit');
    const one=f.entry(A,'Same length A'),two=f.entry(B,'Same length B'),base=f.raw([one]),incoming=f.raw([two]),maximum=Math.max(Buffer.byteLength(base),Buffer.byteLength(incoming)),tightCatalog=setup(version,{maxCatalogBytes:maximum});
    bad(tightCatalog.port.prepareImport(ok(tightCatalog.port.previewImport(base,utf8(incoming))),{}),'catalog_byte_limit');assert.equal(base,f.raw([one]));
  });
}
test('portability factory rejects other versions/raised bounds and never exposes a persistence or Apply API',()=>{
  for(const version of[0,3,'1',null])assert.throws(()=>portability.create(version));assert.throws(()=>portability.create(1,{maxEntries:33}));
  assert.deepEqual(Object.keys(portability.create(1)),['exportDefinitions','previewImport','prepareImport']);assert.deepEqual(Object.keys(portability.create(2)),['exportDefinitions','previewImport','prepareImport']);
});
test('pure planning never acquires browser storage, fetch, locks or changes existing product exports',()=>{
  const before=Object.keys(definitions),saved=new Map(),names=['localStorage','fetch','navigator'];let reads=0;
  try{for(const name of names){saved.set(name,Object.getOwnPropertyDescriptor(globalThis,name));Object.defineProperty(globalThis,name,{configurable:true,get(){reads++;throw Error('No environment access in pure planner');}});}
    for(const version of[1,2]){const f=setup(version),entry=f.entry(),raw=f.raw([entry]);ok(f.port.exportDefinitions(raw,{kind:'all'}));ok(f.port.prepareImport(f.preview(null,[entry]),{}));}
    assert.equal(reads,0);assert.deepEqual(Object.keys(definitions),before);
  }finally{for(const [name,descriptor]of saved)if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name];}
});

// Independent reviewer inputs are hand-authored complete files. Their expected
// identities and byte budgets do not use library.makeEntry or a product query.
function declaredEntry(version,id,name){return{id,name,section:'titles',query_semantics:'open-seo-spider/table-query-v'+version,schema:[{key:'url',type:'text',mandatory:true},{key:'count',type:'number',mandatory:false}],section_filters:['all','missing'],view:{sectionFilter:'all',search:'',...(version===1?{rules:[{key:'count',op:'min',min:1}]}:{expression:{kind:'rule',rule:{key:'count',op:'min',min:1}}}),sort:{key:'url',direction:1},visibleKeys:['url']}};}
const declaredFile=(version,entries)=>JSON.stringify({engine:'open-seo-spider/table-views-v'+version,version,entries});
for(const version of[1,2]){
  test('independent v'+version+' two copy choices cannot reuse one new ID or poison a later preparation',()=>{
    const planner=portability.create(version),preview=ok(planner.previewImport(null,utf8(declaredFile(version,[declaredEntry(version,B,'B'),declaredEntry(version,C,'C')]))));
    bad(planner.prepareImport(preview,{[B]:{action:'copy',id:D,name:'Copy B'},[C]:{action:'copy',id:D,name:'Copy C'}}),'copy_id_not_fresh');
    const clean=ok(planner.prepareImport(preview,{[B]:{action:'skip'},[C]:{action:'copy',id:D,name:'Copy C'}}));assert.deepEqual(clean.entries,[{...declaredEntry(version,C,'C'),id:D,name:'Copy C'}]);
  });
  test('independent v'+version+' a skipped source identity remains reserved even without a destination',()=>{
    const planner=portability.create(version),preview=ok(planner.previewImport(null,utf8(declaredFile(version,[declaredEntry(version,B,'B'),declaredEntry(version,C,'C')]))));
    bad(planner.prepareImport(preview,{[B]:{action:'skip'},[C]:{action:'copy',id:B,name:'Copy C'}}),'copy_id_not_fresh');
  });
  test('independent v'+version+' name and selection accessors are refused without execution',()=>{
    const planner=portability.create(version),current=declaredFile(version,[declaredEntry(version,A,'A')]),preview=ok(planner.previewImport(current,utf8(declaredFile(version,[declaredEntry(version,B,'B')]))));let reads=0;
    const choice={action:'copy',id:C};Object.defineProperty(choice,'name',{enumerable:true,get(){reads++;return'Getter name';}});bad(planner.prepareImport(preview,{[B]:choice}),'invalid_shape');
    const selection={kind:'selected'};Object.defineProperty(selection,'id',{enumerable:true,get(){reads++;return A;}});bad(planner.exportDefinitions(current,selection),'invalid_shape');assert.equal(reads,0);
  });
  test('independent v'+version+' accepted BOM bytes count against the physical file budget',()=>{
    const bytes=utf8(declaredFile(version,[declaredEntry(version,A,'A')])),bom=Buffer.concat([Buffer.from([239,187,191]),bytes]),planner=portability.create(version,{maxCatalogBytes:bytes.length});
    assert(planner.previewImport(null,bytes).ok);bad(planner.previewImport(null,bom),'file_byte_limit');
  });
}
test('browser UMD installs only its namespace and performs complete v1/v2 planning without browser environment access',()=>{
  const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),environment={TextEncoder,TextDecoder};let reads=0;
  for(const name of['document','localStorage','navigator','fetch'])Object.defineProperty(environment,name,{get(){reads++;throw Error('No browser environment access');}});
  const context=vm.createContext(environment);for(const file of['table-explorer.js','table-view-library.js','table-view-portability.js'])vm.runInContext(fs.readFileSync(path.join(__dirname,'..',file),'utf8'),context,{filename:file});
  assert.deepEqual(Object.keys(context.TableViewPortability),['create']);assert(Object.isFrozen(context.TableViewPortability));
  const data=value=>vm.runInContext('JSON.parse('+JSON.stringify(JSON.stringify(value))+')',context);
  for(const version of[1,2]){
    const planner=context.TableViewPortability.create(version),raw=declaredFile(version,[declaredEntry(version,A,'Original')]),bytes=vm.runInContext('new Uint8Array('+JSON.stringify([...utf8(raw)])+')',context);
    const preview=ok(planner.previewImport(null,bytes)),plan=ok(planner.prepareImport(preview,data({})));assert.equal(plan.nextRaw,raw);assert.equal(ok(planner.exportDefinitions(plan.nextRaw,data({kind:'all'}))).raw,raw);assert.equal(ok(planner.exportDefinitions(plan.nextRaw,data({kind:'selected',id:A}))).entries.length,1);
  }
  assert.equal(reads,0);assert.deepEqual(Object.keys(environment).sort(),['TableExplorer','TableViewLibrary','TableViewPortability','TextDecoder','TextEncoder'].sort());
});
