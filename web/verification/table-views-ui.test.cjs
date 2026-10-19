'use strict';
// Pure controller/real library/storage controls. Native browser/App proof is separate.
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const explorer=require('../table-explorer.js'),definitions=require('../table-view-library.js'),storageModule=require('../table-view-storage.js');
const {createController}=require('../table-views-ui.js');
const library=definitions.create(explorer),{KEY}=storageModule;
const A='11111111-1111-4111-8111-111111111111',B='22222222-2222-4222-8222-222222222222',C='33333333-3333-4333-8333-333333333333';
const columns=[{key:'url',type:'text',mandatory:true,value:row=>row.url},{key:'title',type:'text',value:row=>row.title},{key:'count',type:'number',value:row=>row.count}];
const initialContract={section:'titles',query_semantics:definitions.QUERY_SEMANTICS,columns,sectionFilters:['all','missing']};
const initialQuery={sectionFilter:'missing',search:'Cafe\u0301',rules:[{key:'count',op:'range',min:0,max:2}],sort:{key:'count',direction:-1},visibleKeys:['url','count']};
const clone=value=>JSON.parse(JSON.stringify(value));
function accepted(result){assert.equal(result.ok,true,JSON.stringify(result));return result.value;}
function refused(result,code){assert.equal(result.ok,false,JSON.stringify(result));assert.equal(result.error.code,code);return result.error;}
const entry=(id=A,name='Café',contract=initialContract,query=initialQuery)=>accepted(library.makeEntry({id,name},contract,query));
const raw=entries=>accepted(library.encodeCatalog(entries));
function fixture({entries=[],rawCatalog=entries.length?raw(entries):null,locks='immediate',apply}={}){
  const data=new Map([['legacy-columns','keep exact legacy bytes']]);if(rawCatalog!==null)data.set(KEY,rawCatalog);
  const calls=[],pending=[];
  const storage={getItem(key){calls.push(['get',key]);return data.has(key)?data.get(key):null;},setItem(key,value){calls.push(['set',key,value]);data.set(key,value);},removeItem(key){calls.push(['remove',key]);data.delete(key);}};
  const lockAPI=locks===null?null:{request(name,options,action){calls.push(['lock',name,options]);if(locks==='held')return new Promise((resolve,reject)=>pending.push(()=>{try{resolve(action({name,mode:'exclusive'}));}catch(error){reject(error);}}));return Promise.resolve(action(locks==='busy'?null:{name,mode:'exclusive'}));}};
  const model={owner:{projectId:'crawl-a',section:'titles',generation:1,segment:null},revision:0,available:true,contract:{...initialContract},query:clone(initialQuery),snapshots:0,applies:0,notifications:0};
  const adapter=storageModule.create({library,storage,locks:lockAPI});
  let controller;
  const hooks={library,store:adapter,available:()=>model.available,uuid:()=>[A,B,C].find(id=>!controller.status().entries.some(row=>row.id===id)),owner:()=>({...model.owner}),queryRevision:()=>model.revision,contract:()=>model.contract,
    snapshot:()=>{model.snapshots++;return{ok:true,value:clone(model.query)};},
    apply:(prepared,guard)=>{model.applies++;if(apply)return apply(prepared,guard,model,controller);if(!guard())return{ok:false,error:{code:'stale_apply',message:'Stale apply'}};const {section,query_semantics,...query}=prepared;model.query=clone(query);model.revision++;controller.invalidate();return{ok:true,value:prepared};},notify:()=>{model.notifications++;}};
  controller=createController(hooks);
  return{controller,model,data,calls,pending,adapter,hooks,storage,bytes:()=>data.has(KEY)?data.get(KEY):null,writes:()=>calls.filter(call=>['set','remove'].includes(call[0])),read:()=>accepted(controller.refresh()),edit:()=>{model.revision++;controller.invalidate();}};
}

test('status and explicit read never auto-apply or write, and entries are section scoped',()=>{
  const f=fixture({entries:[entry(),entry(B,'Metadata',{...initialContract,section:'meta'})]});
  assert.deepEqual(f.controller.status().entries,[]);assert.deepEqual(f.calls,[]);const query=clone(f.model.query);
  f.read();assert.deepEqual(f.controller.status().entries.map(row=>row.id),[A]);assert.deepEqual(f.model.query,query);assert.equal(f.model.applies,0);assert.deepEqual(f.writes(),[]);
  f.model.owner={...f.model.owner,section:'meta'};f.model.contract={...initialContract,section:'meta'};
  assert.deepEqual(f.controller.status().entries.map(row=>row.id),[B]);assert.equal(f.controller.status().selectedId,'');assert.equal(f.model.applies,0);assert.deepEqual(f.writes(),[]);
});
test('create saves the complete explicit query and does not apply or change the working table',async()=>{
  const f=fixture(),query=clone(f.model.query);f.read();f.controller.name('  Full Café  ');
  const saved=accepted(await f.controller.create()).entries[0];assert.equal(saved.name,'  Full Café  ');assert.deepEqual(saved.view,query);
  assert.deepEqual(f.model.query,query);assert.equal(f.model.snapshots,1);assert.equal(f.model.applies,0);assert.equal(f.writes().length,1);assert.equal(f.controller.status().selectedId,A);
  assert(!f.bytes().includes('crawl-a'));assert.equal(f.data.get('legacy-columns'),'keep exact legacy bytes');
});
test('selection never applies; Apply commits every query field once and rebinds after intentional App invalidation',()=>{
  const saved=entry(),f=fixture({entries:[saved]});f.read();f.model.query={sectionFilter:'all',search:'other',rules:[],sort:{key:'url',direction:1},visibleKeys:['url','title']};
  accepted(f.controller.select(A));assert.equal(f.model.applies,0);const before=f.bytes();accepted(f.controller.apply());
  assert.equal(f.model.applies,1);assert.deepEqual(f.model.query,initialQuery);assert.deepEqual(f.controller.status().active,{id:A,name:'Café',modified:false});assert.equal(f.controller.status().busy,false);assert.equal(f.bytes(),before);assert.deepEqual(f.writes(),[]);
  f.edit();assert.deepEqual(f.controller.status().active,{id:A,name:'Café',modified:true});assert.equal(f.bytes(),before);
});
test('failed atomic Apply preserves the active query and exposes the concrete Result',()=>{
  const f=fixture({entries:[entry()],apply:()=>({ok:false,error:{code:'invalid_working_view',message:'Invalid original query'}})});f.read();f.controller.select(A);const before=clone(f.model.query);
  refused(f.controller.apply(),'invalid_working_view');assert.deepEqual(f.model.query,before);assert.equal(f.controller.status().active,null);assert.match(f.controller.status().error,/Invalid original query/);assert.equal(f.controller.status().busy,false);
});
test('rename and replace require explicit actions, preserve working query and never autosave edits',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.controller.select(A);const before=f.bytes();f.controller.name('Renamed');f.model.query.search='A different exact search';
  assert.equal(f.bytes(),before);assert.deepEqual(f.writes(),[]);accepted(await f.controller.rename());assert.equal(JSON.parse(f.bytes()).entries[0].name,'Renamed');assert.deepEqual(JSON.parse(f.bytes()).entries[0].view,initialQuery);
  const changed=clone(f.model.query);accepted(await f.controller.replace());assert.deepEqual(JSON.parse(f.bytes()).entries[0].view,changed);assert.deepEqual(f.model.query,changed);assert.equal(f.model.applies,0);assert.equal(f.writes().length,2);
});
test('rename uniqueness errors preserve existing catalog bytes and working query',async()=>{
  const f=fixture({entries:[entry(),entry(B,'Other')]});f.read();f.controller.select(B);f.controller.name(' cafe\u0301 ');const before=f.bytes(),query=clone(f.model.query);
  refused(await f.controller.rename(),'duplicate_name');assert.equal(f.bytes(),before);assert.deepEqual(f.model.query,query);assert.equal(f.writes().length,0);assert(f.controller.status().error);
});
test('saving the current query over its active definition clears the unsaved badge without Apply or table mutation',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.controller.select(A);accepted(f.controller.apply());f.model.query.search='Changed current query';f.edit();assert.equal(f.controller.status().active.modified,true);
  const query=clone(f.model.query),applies=f.model.applies;accepted(await f.controller.replace());assert.deepEqual(f.controller.status().active,{id:A,name:'Café',modified:false});assert.deepEqual(f.model.query,query);assert.equal(f.model.applies,applies);assert.deepEqual(JSON.parse(f.bytes()).entries[0].view,query);
});
test('rename of the active definition updates its badge name without clearing unsaved query state',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.controller.select(A);accepted(f.controller.apply());f.edit();f.controller.name('Active renamed');const query=clone(f.model.query),applies=f.model.applies;
  accepted(await f.controller.rename());assert.deepEqual(f.controller.status().active,{id:A,name:'Active renamed',modified:true});assert.deepEqual(f.model.query,query);assert.equal(f.model.applies,applies);
});
test('delete of an active definition removes its badge while preserving the current working query',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.controller.select(A);accepted(f.controller.apply());f.model.query.search='Unsaved work';f.edit();const query=clone(f.model.query),applies=f.model.applies;
  f.controller.requestDelete();accepted(await f.controller.confirmDelete());assert.equal(f.controller.status().active,null);assert.deepEqual(f.model.query,query);assert.equal(f.model.applies,applies);assert.deepEqual(JSON.parse(f.bytes()).entries,[]);
});
test('selection and saving, renaming or deleting another definition never rebind the active view',async()=>{
  const f=fixture({entries:[entry(),entry(B,'Other')]});f.read();f.controller.select(A);accepted(f.controller.apply());f.model.query.search='Working unsaved query';f.edit();const active={id:A,name:'Café',modified:true},query=clone(f.model.query),applies=f.model.applies;
  f.controller.select(B);assert.deepEqual(f.controller.status().active,active);accepted(await f.controller.replace());assert.deepEqual(f.controller.status().active,active);
  f.controller.name('Other renamed');accepted(await f.controller.rename());assert.deepEqual(f.controller.status().active,active);f.controller.requestDelete();accepted(await f.controller.confirmDelete());assert.deepEqual(f.controller.status().active,active);
  f.controller.name('Another new definition');accepted(await f.controller.create());assert.deepEqual(f.controller.status().active,active);assert.deepEqual(f.model.query,query);assert.equal(f.model.applies,applies);
});
test('delete requires its second confirmation, removes only its selected definition and preserves query',async()=>{
  const f=fixture({entries:[entry(),entry(B,'Other')]});f.read();f.controller.select(A);const before=f.bytes(),query=clone(f.model.query);
  refused(f.controller.confirmDelete(),'stale_confirmation');accepted(f.controller.requestDelete());assert.equal(f.bytes(),before);assert.equal(f.controller.status().deleteConfirmation.id,A);
  accepted(await f.controller.confirmDelete());assert.deepEqual(JSON.parse(f.bytes()).entries.map(row=>row.id),[B]);assert.deepEqual(f.model.query,query);assert.equal(f.controller.status().selectedId,'');assert.equal(f.writes().length,1);
});
test('selection, name, immediate query and owner changes invalidate delete confirmation',async t=>{
  for(const phase of ['selection','name','query','owner','contract','cancel'])await t.test(phase,async()=>{
    const f=fixture({entries:[entry(),entry(B,'Other')]});f.read();f.controller.select(A);f.controller.requestDelete();const before=f.bytes();
    if(phase==='selection')f.controller.select(B);if(phase==='name')f.controller.name('new');if(phase==='query')f.model.revision++;if(phase==='owner')f.model.owner={...f.model.owner,projectId:'crawl-b'};if(phase==='contract')f.model.contract={...initialContract,sectionFilters:['all']};if(phase==='cancel')f.controller.cancelDelete();
    refused(await f.controller.confirmDelete(),'stale_confirmation');assert.equal(f.bytes(),before);assert.deepEqual(f.writes(),[]);
  });
});
test('unread catalog and invalid/incompatible definitions refuse actions before any App commit',async()=>{
  const f=fixture({entries:[entry()]});f.controller.name('Unread');refused(await f.controller.create(),'catalog_unavailable');assert.equal(f.model.snapshots,0,'Unread catalog is refused before snapshot');
  f.read();refused(f.controller.select('nonexistent'),'unknown_view');refused(f.controller.apply(),'no_selection');
  f.controller.select(A);f.model.contract={...initialContract,columns:[...columns,{key:'extra',type:'text',value:row=>row.extra}]};f.controller.synchronize();f.controller.select(A);refused(f.controller.apply(),'schema_mismatch');assert.equal(f.model.applies,0);assert.deepEqual(f.writes(),[]);
});
test('missing crawl refuses create, replace and apply, while rename/delete/reset remain catalog actions',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.model.owner={...f.model.owner,projectId:null};f.controller.synchronize();f.controller.select(A);f.controller.name('Readable without crawl');const before=f.bytes();
  for(const method of ['create','replace','apply'])refused(await f.controller[method](),'no_crawl');assert.equal(f.bytes(),before);assert.equal(f.model.snapshots,0);assert.equal(f.model.applies,0);
  accepted(await f.controller.rename());assert.equal(JSON.parse(f.bytes()).entries[0].name,'Readable without crawl');f.controller.requestDelete();accepted(await f.controller.confirmDelete());assert.equal(JSON.parse(f.bytes()).entries.length,0);
  f.controller.refresh();f.controller.requestReset();accepted(await f.controller.confirmReset());assert.equal(f.bytes(),null);assert.equal(f.data.get('legacy-columns'),'keep exact legacy bytes');
});
test('genuine external catalog event makes old list unusable until explicit Reload, preserving table and bytes',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.controller.select(A);accepted(f.controller.apply());const query=clone(f.model.query),replacement=raw([entry(B,'New tab')]);f.data.set(KEY,replacement);f.controller.externalChange();
  const s=f.controller.status();assert.equal(s.catalogValid,false);assert.equal(s.entries[0].id,A);assert.deepEqual(s.active,{id:A,name:'Café',modified:false});assert.equal(s.deleteConfirmation,null);assert.equal(s.resetConfirmation,null);assert.match(s.notice,/anderen Fenster/);
  refused(f.controller.apply(),'catalog_unavailable');f.controller.name('Blocked draft');refused(await f.controller.create(),'catalog_unavailable');assert.equal(f.bytes(),replacement);assert.deepEqual(f.model.query,query);assert.equal(f.model.applies,1);assert.deepEqual(f.writes(),[]);
  f.read();assert.equal(f.controller.status().catalogValid,true);assert.deepEqual(f.controller.status().entries.map(row=>row.id),[B]);assert.deepEqual(f.model.query,query);assert.equal(f.model.applies,1);
});
test('same-tab byte conflict refuses original catalog save even without a storage event',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.controller.select(A);f.controller.name('Attempted rename');const other=raw([entry(B,'Other writer')]);f.data.set(KEY,other);
  refused(await f.controller.rename(),'stale_base');assert.equal(f.bytes(),other);assert.equal(f.writes().length,0);assert.match(f.controller.status().error,/inzwischen|changed/i);assert.equal(f.controller.status().busy,false);
});
test('pending mutation is invalidated by immediate name, selection, query, owner or contract change before lock acquisition',async t=>{
  for(const phase of ['name','selection','query','owner','contract'])await t.test(phase,async()=>{
    const f=fixture({entries:[entry(),entry(B,'Other')],locks:'held'});f.read();f.controller.select(A);f.controller.name('Pending rename');const before=f.bytes(),pending=f.controller.rename();assert.equal(f.pending.length,1);assert.equal(f.controller.status().busy,true);
    if(phase==='name')f.controller.name('Later name');if(phase==='selection')f.controller.select(B);if(phase==='query')f.model.revision++;if(phase==='owner')f.model.owner={...f.model.owner,projectId:'crawl-b'};if(phase==='contract')f.model.contract={...initialContract,sectionFilters:['all']};
    f.pending[0]();refused(await pending,'stale_operation');assert.equal(f.bytes(),before);assert.deepEqual(f.writes(),[]);assert.equal(f.controller.status().busy,false);
  });
});
test('stale completion never clears a newer pending operation or its accepted name',async()=>{
  const f=fixture({locks:'held'});f.read();f.controller.name('First');const first=f.controller.create();f.controller.name('Second');const second=f.controller.create();assert.equal(f.pending.length,2);
  f.pending[0]();refused(await first,'stale_operation');assert.equal(f.controller.status().busy,true);assert.equal(f.controller.status().name,'Second');assert.equal(f.bytes(),null);
  f.pending[1]();accepted(await second);assert.equal(f.controller.status().busy,false);assert.equal(JSON.parse(f.bytes()).entries[0].name,'Second');assert.equal(f.writes().length,1);
});
test('missing or busy locks still allow read/Apply and visibly refuse mutations',async t=>{
  for(const locks of [null,'busy'])await t.test(String(locks),async()=>{
    const f=fixture({entries:[entry()],locks});f.read();f.controller.select(A);accepted(f.controller.apply());const before=f.bytes();f.controller.name('Cannot write');refused(await f.controller.rename(),locks===null?'lock_unavailable':'lock_busy');
    assert.equal(f.bytes(),before);assert.equal(f.model.applies,1);assert(f.controller.status().error);assert.equal(f.controller.status().busy,false);assert.deepEqual(f.writes(),[]);
  });
});
test('corrupt or unsupported bytes stay untouched until exact observed second reset confirmation',async t=>{
  for(const bytes of ['{broken',JSON.stringify({engine:definitions.ENGINE,version:17,entries:[]})])await t.test(bytes,async()=>{
    const f=fixture({rawCatalog:bytes}),query=clone(f.model.query);assert.equal(f.controller.refresh().ok,false);assert.equal(f.controller.status().catalogValid,false);assert.equal(f.controller.status().resetAvailable,true);assert.equal(f.bytes(),bytes);
    refused(await f.controller.confirmReset(),'stale_confirmation');accepted(f.controller.requestReset());assert.equal(f.bytes(),bytes);assert.deepEqual(f.writes(),[]);accepted(await f.controller.confirmReset());
    assert.equal(f.bytes(),null);assert.equal(f.controller.status().catalogValid,true);assert.deepEqual(f.controller.status().entries,[]);assert.deepEqual(f.model.query,query);assert.deepEqual(f.writes(),[['remove',KEY]]);assert.equal(f.data.get('legacy-columns'),'keep exact legacy bytes');
  });
});
test('reset confirmation refuses query changes and foreign bytes without removing anything',async t=>{
  for(const phase of ['query','bytes','cancel','name'])await t.test(phase,async()=>{
    const f=fixture({rawCatalog:'{broken'});f.controller.refresh();f.controller.requestReset();if(phase==='query')f.model.revision++;if(phase==='bytes')f.data.set(KEY,'{new foreign bytes');if(phase==='cancel')f.controller.cancelReset();if(phase==='name')f.controller.name('edited');
    const before=f.bytes();refused(await f.controller.confirmReset(),phase==='bytes'?'stale_base':'stale_confirmation');assert.equal(f.bytes(),before);assert.deepEqual(f.writes(),[]);
  });
});
test('pending reset observes immediate query revision and releases only its own ticket',async()=>{
  const f=fixture({rawCatalog:'{broken',locks:'held'});f.controller.refresh();f.controller.requestReset();const result=f.controller.confirmReset();assert.equal(f.pending.length,1);f.model.revision++;f.pending[0]();refused(await result,'stale_operation');assert.equal(f.controller.status().busy,false);assert.equal(f.bytes(),'{broken');assert.deepEqual(f.writes(),[]);
});
test('unavailable read cannot arm recovery using previously observed bytes',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.storage.getItem=()=>{throw Error('Storage denied');};refused(f.controller.refresh(),'storage_unavailable');assert.equal(f.controller.status().catalogValid,false);assert.equal(f.controller.status().resetAvailable,false);refused(f.controller.requestReset(),'unobserved_catalog');assert.deepEqual(f.writes(),[]);
});
test('inactive query mode refuses all catalog mutations and Apply before snapshots or native commits',async()=>{
  const f=fixture({entries:[entry()]});f.read();f.controller.select(A);f.model.available=false;
  f.controller.synchronize();f.controller.select(A);f.controller.name('Inactive');const before=f.bytes(),query=clone(f.model.query);
  for(const method of['create','replace','rename','apply','requestDelete','requestReset'])refused(await f.controller[method](),'inactive_query_mode');
  assert.equal(f.model.snapshots,0);assert.equal(f.model.applies,0);assert.equal(f.bytes(),before);assert.deepEqual(f.model.query,query);assert.deepEqual(f.writes(),[]);
});
test('query mode participates in the pending lock guard even without a query revision callback',async t=>{
  for(const action of['rename','reset'])await t.test(action,async()=>{
    const f=fixture({entries:[entry()],locks:'held'});f.read();f.controller.select(A);f.controller.name('Stale mode');
    if(action==='reset')f.controller.requestReset();const promise=action==='reset'?f.controller.confirmReset():f.controller.rename();
    assert.equal(f.pending.length,1);const before=f.bytes();f.model.available=false;f.pending[0]();refused(await promise,'stale_operation');assert.equal(f.bytes(),before);assert.deepEqual(f.writes(),[]);assert.equal(f.controller.status().busy,false);
  });
});

// The explorer UI boundary is exercised without a browser: a minimal control
// adapter lets the real create/draft/install methods run with the real compiler.
// This does not assert rendered layout, native storage or App transaction proof.
function explorerUI(){
  class Control{
    constructor(tag='input'){this.tag=tag;this.value='';this.children=[];this.events=new Map();this.dataset={};}
    addEventListener(event,handler){this.events.set(event,handler);}
    replaceChildren(){this.children=[];this.value='';}
    append(...children){this.children.push(...children);if(this.tag==='select'&&!this.value)this.value=this.children[0]?.value||'';}
    get options(){return this.children;}
  }
  const controls=new Map(),selects=new Set(['table-rule-column','table-rule-op','table-sort-column','table-sort-direction']);
  const document={getElementById(id){if(!controls.has(id))controls.set(id,new Control(selects.has(id)?'select':'input'));return controls.get(id);},createElement:tag=>new Control(tag)};
  const calls=[],data=new Map(),window={TableExplorer:explorer,localStorage:{getItem:key=>data.get(key)??null,setItem:(key,value)=>{calls.push(['set',key,value]);data.set(key,value);}},addEventListener(){}};
  vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../table-explorer-ui.js'),'utf8'),{window,document},{filename:'table-explorer-ui.js'});
  const model={owner:{projectId:'crawl-a',section:'titles',generation:1},sort:{key:'url',direction:1},changes:0,edits:0,editSnapshots:[]};
  const ui=window.TableExplorerUI.create({owner:()=>model.owner,sort:()=>model.sort,setSort:sort=>{model.sort=sort;},queryEdited:()=>{model.edits++;model.editSnapshots.push({draft:clone(accepted(ui.draft())),sort:{...model.sort}});},changed:()=>{model.changes++;},identity:{identity:row=>row.url}});
  ui.synchronize(columns);return{ui,model,calls,controls};
}
test('explorer draft/install retain deep-copy ownership and do not render, notify, save or apply captured rows',()=>{
  const f=explorerUI(),view=clone(initialQuery);accepted(f.ui.installPrepared(view));
  assert.deepEqual(clone(accepted(f.ui.draft())),{rules:initialQuery.rules,visibleKeys:initialQuery.visibleKeys});assert.equal(f.ui.hasRules(),true);assert.equal(f.ui.canExport(),true);
  view.rules[0].min=999;view.visibleKeys.push('title');const detached=accepted(f.ui.draft());detached.rules[0].min=888;detached.visibleKeys.pop();
  assert.deepEqual(clone(accepted(f.ui.draft())),{rules:initialQuery.rules,visibleKeys:initialQuery.visibleKeys});assert.equal(f.model.edits,0);assert.equal(f.model.changes,0);assert.deepEqual(f.calls,[]);
  const rows=[{url:'a',title:'Café',count:0},{url:'b',title:'Cafe\u0301',count:2},{url:'c',title:'Café',count:null},{url:'d',title:'Café',count:3}];
  const selected=f.ui.rows(rows,columns,initialQuery.search,initialQuery.sort);assert.deepEqual([...selected],[rows[1],rows[0]]);assert.equal(selected[0],rows[1]);assert.deepEqual([...f.ui.columns(columns)].map(column=>column.key),['url','count']);
});
test('invalid explorer install rejects every partial payload before changing accepted rules or columns',()=>{
  const f=explorerUI();accepted(f.ui.installPrepared(initialQuery));const before=clone(accepted(f.ui.draft()));
  for(const invalid of [{...initialQuery,rules:[{key:'count',op:'range',min:4,max:0}]},{...initialQuery,visibleKeys:['url','not-present']},{...initialQuery,sort:{key:'count',direction:0}},{...initialQuery,rules:null},null]){
    assert.equal(f.ui.installPrepared(invalid).ok,false);assert.deepEqual(clone(accepted(f.ui.draft())),before);assert.equal(f.ui.canExport(),true);
  }
  assert.equal(f.model.edits,0);assert.equal(f.model.changes,0);assert.deepEqual(f.calls,[]);
});
test('manual rule/clear/sort callbacks invalidate the named transaction immediately before accepted query mutation',()=>{
  const f=explorerUI();accepted(f.ui.installPrepared(initialQuery));
  const ruleColumn=f.controls.get('table-rule-column'),ruleOp=f.controls.get('table-rule-op');ruleColumn.value='count';ruleOp.value='min';f.controls.get('table-rule-min').value='0';
  f.controls.get('table-rule-form').events.get('submit')({preventDefault(){}});assert.equal(f.model.edits,1);assert.equal(accepted(f.ui.draft()).rules.length,2);assert.equal(f.model.changes,1);
  f.controls.get('table-rules-clear').events.get('click')();assert.equal(f.model.edits,2);assert.deepEqual(clone(accepted(f.ui.draft()).rules),[]);assert.equal(f.model.changes,2);
  f.controls.get('table-sort-column').value='count';f.controls.get('table-sort-direction').value='-1';f.controls.get('table-sort-column').events.get('change')();assert.equal(f.model.edits,3);assert.deepEqual(clone(f.model.sort),{key:'count',direction:-1});assert.equal(f.model.changes,3);assert.deepEqual(f.calls,[]);
  assert.deepEqual(f.model.editSnapshots.map(item=>item.draft.rules.length),[1,2,0]);assert(f.model.editSnapshots.every(item=>item.sort.key==='url'&&item.sort.direction===1),'Invalidation sees the old sort before it is changed');
});
