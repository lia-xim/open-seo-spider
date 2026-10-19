'use strict';
// Injected storage/Web Lock scheduling controls; actual browser proof is separate.
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const explorer=require('../table-explorer.js'),definitions=require('../table-view-library.js'),storageModule=require('../table-view-storage.js');
const library=definitions.create(explorer),{KEY,LOCK_NAME}=storageModule;
const ID_A='11111111-1111-4111-8111-111111111111',ID_B='22222222-2222-4222-8222-222222222222';
const columns=[{key:'url',type:'text',mandatory:true,value:row=>row.url},{key:'count',type:'number',value:row=>row.count}];
const contract={section:'status',query_semantics:definitions.QUERY_SEMANTICS,columns,sectionFilters:['all','missing']};
const query={sectionFilter:'missing',search:'café',rules:[{key:'count',op:'min',min:0}],sort:{key:'count',direction:-1},visibleKeys:['url','count']};
function ok(result){assert.equal(result.ok,true,JSON.stringify(result));return result.value;}
function rejected(result,code){assert.equal(result.ok,false);assert.equal(result.error.code,code);assert.equal(Object.hasOwn(result,'value'),false);}
const entry=(id=ID_A,name='First')=>ok(library.makeEntry({id,name},contract,query));
function stored(initial=null){
  const legacy='open-seo-spider.table-columns.v1:owned-crawl:status';
  const data=new Map([[legacy,'old exact column bytes']]);if(initial!==null)data.set(KEY,initial);
  const calls=[];return {data,calls,legacy,getItem(key){calls.push(['get',key]);return data.has(key)?data.get(key):null;},setItem(key,value){calls.push(['set',key,value]);data.set(key,value);},removeItem(key){calls.push(['remove',key]);data.delete(key);}};
}
function immediateLocks(){const calls=[];return {calls,request(name,options,callback){calls.push([name,options]);return Promise.resolve(callback({name,mode:'exclusive'}));}};}
function heldLocks(){const calls=[],pending=[];return {calls,pending,request(name,options,callback){calls.push([name,options]);return new Promise((resolve,reject)=>pending.push({acquire(lock={name,mode:'exclusive'}){try{resolve(callback(lock));}catch(error){reject(error);}}}));}};}

test('explicit read of absent catalog has no write/lock effects and never touches legacy column keys',()=>{
  const storage=stored(),locks=immediateLocks(),adapter=storageModule.create({library,storage,locks});
  assert.deepEqual(ok(adapter.read()),{raw:null,entries:[],missing:true});
  assert.deepEqual(storage.calls,[['get',KEY]]);assert.deepEqual(locks.calls,[]);assert.equal(storage.data.get(storage.legacy),'old exact column bytes');
});
test('explicit complete catalog create/rename/overwrite/delete uses one fixed exclusive lock and one write each',async()=>{
  const storage=stored(),locks=immediateLocks(),adapter=storageModule.create({library,storage,locks}),first=entry(),second=entry(ID_B,'Second');
  let state=ok(await adapter.mutate({expectedRaw:null,entries:[first,second],guard:()=>true}));assert.equal(state.missing,false);assert(Object.isFrozen(state.entries));
  assert.deepEqual(state.entries.map(value=>value.name),['First','Second']);assert.deepEqual(state.entries[0].view,query);
  state=ok(await adapter.mutate({expectedRaw:state.raw,entries:[{...first,name:'Renamed'},second],guard:()=>true}));
  assert.equal(state.entries[0].name,'Renamed');
  const changed=ok(library.makeEntry({id:ID_A,name:'Renamed'},contract,{...query,search:'new',sectionFilter:'all'}));
  state=ok(await adapter.mutate({expectedRaw:state.raw,entries:[changed,second],guard:()=>true}));assert.equal(state.entries[0].view.search,'new');
  state=ok(await adapter.mutate({expectedRaw:state.raw,entries:[second],guard:()=>true}));assert.deepEqual(state.entries.map(value=>value.id),[ID_B]);
  assert.deepEqual(ok(adapter.read()),state);assert.equal(storage.calls.filter(call=>call[0]==='set').length,4);
  assert.equal(locks.calls.length,4);assert(locks.calls.every(([name,options])=>name===LOCK_NAME&&options.mode==='exclusive'&&options.ifAvailable===true));
  assert(storage.calls.every(call=>call[1]===KEY));assert.equal(storage.data.get(storage.legacy),'old exact column bytes');
});
test('missing or busy locks refuse mutation without storage writes, while read remains available',async()=>{
  for(const locks of [null,{}, {get request(){throw Error('Blocked locks');}}]){
    const storage=stored(),adapter=storageModule.create({library,storage,locks});assert(ok(adapter.read()).missing);
    rejected(await adapter.mutate({expectedRaw:null,entries:[entry()],guard:()=>true}),'lock_unavailable');assert.equal(storage.calls.filter(call=>call[0]==='set').length,0);
  }
  const storage=stored(),locks={request(name,options,callback){return Promise.resolve(callback(null));}},adapter=storageModule.create({library,storage,locks});
  rejected(await adapter.mutate({expectedRaw:null,entries:[entry()],guard:()=>true}),'lock_busy');assert.deepEqual(storage.calls,[]);
});
test('both absent-to-present and present-to-absent byte-CAS conflicts preserve exact current storage',async()=>{
  const raw=ok(library.encodeCatalog([entry()]));
  for(const [expectedRaw,currentRaw]of [[null,raw],[raw,null],[raw,raw+' ']]){
    const storage=stored(currentRaw),adapter=storageModule.create({library,storage,locks:immediateLocks()});
    rejected(await adapter.mutate({expectedRaw,entries:[],guard:()=>true}),'stale_base');
    assert.equal(storage.data.has(KEY)?storage.data.get(KEY):null,currentRaw);assert.equal(storage.calls.filter(call=>call[0]==='set').length,0);
  }
});
test('two cooperating callers with the same old bytes cannot silently overwrite the first committed catalog',async()=>{
  const storage=stored(),locks=immediateLocks(),first=storageModule.create({library,storage,locks}),second=storageModule.create({library,storage,locks});
  const initialA=ok(first.read()),initialB=ok(second.read());
  const accepted=ok(await first.mutate({expectedRaw:initialA.raw,entries:[entry()],guard:()=>true}));
  rejected(await second.mutate({expectedRaw:initialB.raw,entries:[entry(ID_B,'Other tab')],guard:()=>true}),'stale_base');
  assert.equal(storage.data.get(KEY),accepted.raw);assert.equal(storage.calls.filter(call=>call[0]==='set').length,1);
});
test('immediate query revision changes while awaiting the lock refuse before any delayed-render commit',async()=>{
  const storage=stored(),locks=heldLocks(),adapter=storageModule.create({library,storage,locks});let queryRevision=4;
  const pending=adapter.mutate({expectedRaw:null,entries:[entry()],guard:()=>queryRevision===4});assert.equal(locks.pending.length,1);
  queryRevision++;locks.pending[0].acquire();rejected(await pending,'guard_refused');assert.deepEqual(storage.calls,[]);assert.equal(storage.data.has(KEY),false);
  for(const guard of [undefined,()=>false,()=>{throw null;}])rejected(await adapter.mutate({expectedRaw:null,entries:[entry()],guard}),'guard_refused');
  assert.equal(locks.calls.length,1);
});
test('guards are rechecked after held-lock read, replacement preparation and setItem method acquisition',async()=>{
  for(const phase of ['read','prepare','method']){
    const storage=stored(),realRead=storage.getItem.bind(storage);let current=true;
    const injected={...library,prepareReplacement(...args){const result=library.prepareReplacement(...args);if(phase==='prepare')current=false;return result;}};
    if(phase==='read')storage.getItem=key=>{const result=realRead(key);current=false;return result;};
    if(phase==='method')Object.defineProperty(storage,'setItem',{get(){current=false;return ()=>{throw Error('Stale write must not run');};}});
    const adapter=storageModule.create({library:injected,storage,locks:immediateLocks()});
    rejected(await adapter.mutate({expectedRaw:null,entries:[entry()],guard:()=>current}),'guard_refused');assert.equal(storage.data.has(KEY),false);
  }
});
test('caller draft mutation while awaiting a lock cannot retarget the validated immutable payload',async()=>{
  const storage=stored(),locks=heldLocks(),adapter=storageModule.create({library,storage,locks}),entries=[entry()];
  const pending=adapter.mutate({expectedRaw:null,entries,guard:()=>true});entries.push(entry(ID_B,'Added later'));locks.pending[0].acquire();
  const result=ok(await pending);assert.deepEqual(result.entries.map(value=>value.name),['First']);assert.deepEqual(ok(library.decodeCatalog(storage.data.get(KEY))).entries,result.entries);
});
test('corrupt/unsupported catalog, quota and read failures preserve old bytes without resets or retries',async()=>{
  for(const raw of ['{broken',JSON.stringify({engine:definitions.ENGINE,version:2,entries:[]})]){
    const storage=stored(raw),adapter=storageModule.create({library,storage,locks:immediateLocks()});assert.equal(adapter.read().ok,false);
    assert.equal((await adapter.mutate({expectedRaw:raw,entries:[],guard:()=>true})).ok,false);assert.equal(storage.data.get(KEY),raw);assert.equal(storage.calls.filter(call=>call[0]==='set').length,0);
  }
  const raw=ok(library.encodeCatalog([entry()])),storage=stored(raw),locks=immediateLocks();let writes=0;
  storage.setItem=()=>{writes++;throw Object.assign(Error('Owned quota refusal'),{name:'QuotaExceededError'});};
  const adapter=storageModule.create({library,storage,locks});rejected(await adapter.mutate({expectedRaw:raw,entries:[],guard:()=>true}),'storage_write_failed');assert.equal(storage.data.get(KEY),raw);assert.equal(writes,1);assert.equal(locks.calls.length,1);
  storage.getItem=()=>{throw null;};rejected(adapter.read(),'storage_unavailable');rejected(await adapter.mutate({expectedRaw:raw,entries:[],guard:()=>true}),'storage_unavailable');assert.equal(writes,1);
});
test('invalid replacement data and lock rejection refuse as Results without effects or retry',async()=>{
  const storage=stored(),locks=immediateLocks(),adapter=storageModule.create({library,storage,locks});
  assert.equal((await adapter.mutate({expectedRaw:null,entries:new Array(1),guard:()=>true})).ok,false);assert.deepEqual(storage.calls,[]);assert.deepEqual(locks.calls,[]);
  rejected(await adapter.mutate(null),'invalid_mutation');
  const rejecting=storageModule.create({library,storage,locks:{request(){throw null;}}});rejected(await rejecting.mutate({expectedRaw:null,entries:[entry()],guard:()=>true}),'lock_failed');assert.equal(storage.calls.filter(call=>call[0]==='set').length,0);
});
test('both UMD modules install only namespaces without acquiring browser environment or storage',()=>{
  const context=vm.createContext({TextEncoder,Intl});let forbidden=0;
  for(const key of ['fetch','navigator','localStorage'])Object.defineProperty(context,key,{get(){forbidden++;throw Error('Unexpected global environment access');}});
  for(const file of ['table-view-library.js','table-view-storage.js'])vm.runInContext(fs.readFileSync(path.join(__dirname,'..',file),'utf8'),context);
  assert.equal(typeof context.TableViewLibrary.create,'function');assert.equal(typeof context.TableViewStorage.create,'function');
  const isolated=context.TableViewLibrary.create(explorer),adapter=context.TableViewStorage.create({library:isolated,storage:null,locks:null});assert.equal(adapter.read().ok,false);assert.equal(forbidden,0);
});
test('invalid existing bytes are exposed only as transient observedRaw and stay unchanged until explicit guarded reset',async()=>{
  for(const raw of ['{corrupt', 'x'.repeat(definitions.LIMITS.maxCatalogBytes+1)]){
    const storage=stored(raw),locks=immediateLocks(),adapter=storageModule.create({library,storage,locks}),result=adapter.read();
    assert.equal(result.ok,false);assert.equal(result.observedRaw,raw);assert.equal(Object.hasOwn(result,'value'),false);assert.equal(result.error.message.includes(raw),false);
    assert.equal((await adapter.mutate({expectedRaw:raw,entries:[],guard:()=>true})).ok,false);assert.equal(storage.data.get(KEY),raw);assert.equal(storage.calls.filter(call=>call[0]==='remove').length,0);
    const decodedNever={...library,decodeCatalog(){throw Error('Recovery must not parse prior bytes');}},recovery=storageModule.create({library:decodedNever,storage,locks});
    assert.deepEqual(ok(await recovery.reset({expectedRaw:result.observedRaw,guard:()=>true})),{raw:null,entries:[],missing:true});
    assert.equal(storage.data.has(KEY),false);assert.equal(storage.calls.filter(call=>call[0]==='remove').length,1);assert.equal(storage.data.get(storage.legacy),'old exact column bytes');
    assert(locks.calls.every(([name,options])=>name===LOCK_NAME&&options.mode==='exclusive'&&options.ifAvailable===true));
  }
});
test('stale reset and successor-owner reset cannot delete changed catalog bytes',async()=>{
  const raw='{initial',storage=stored(raw),locks=heldLocks(),adapter=storageModule.create({library,storage,locks});let owner='first';
  const pending=adapter.reset({expectedRaw:raw,guard:()=>owner==='first'});owner='successor';locks.pending[0].acquire();rejected(await pending,'guard_refused');assert.equal(storage.data.get(KEY),raw);
  const stale=adapter.reset({expectedRaw:raw,guard:()=>true});storage.data.set(KEY,'{changed');locks.pending[1].acquire();rejected(await stale,'stale_base');assert.equal(storage.data.get(KEY),'{changed');
  assert.equal(storage.calls.filter(call=>call[0]==='remove').length,0);
});
test('reset rechecks guard after reading and method lookup; unavailable/busy/failed removal changes no key',async()=>{
  const raw='{preserve';
  for(const phase of ['read','method','remove']){
    const storage=stored(raw),realRead=storage.getItem.bind(storage);let current=true;
    if(phase==='read')storage.getItem=key=>{const value=realRead(key);current=false;return value;};
    if(phase==='method')Object.defineProperty(storage,'removeItem',{get(){current=false;return ()=>{throw Error('Stale removal must not run');};}});
    if(phase==='remove')storage.removeItem=()=>{throw null;};
    const adapter=storageModule.create({library,storage,locks:immediateLocks()});rejected(await adapter.reset({expectedRaw:raw,guard:()=>current}),phase==='remove'?'storage_remove_failed':'guard_refused');assert.equal(storage.data.get(KEY),raw);assert.equal(storage.data.get(storage.legacy),'old exact column bytes');
  }
  const storage=stored(raw),missing=storageModule.create({library,storage,locks:null});rejected(await missing.reset({expectedRaw:raw,guard:()=>true}),'lock_unavailable');
  const busy=storageModule.create({library,storage,locks:{request(name,options,callback){return callback(null);}}});rejected(await busy.reset({expectedRaw:raw,guard:()=>true}),'lock_busy');assert.equal(storage.data.get(KEY),raw);
  storage.getItem=()=>{throw Error('Unavailable');};rejected(await storageModule.create({library,storage,locks:immediateLocks()}).reset({expectedRaw:raw,guard:()=>true}),'storage_unavailable');
});
test('malformed mutation/reset arguments refuse without invoking accessors or acquiring a lock',async()=>{
  const storage=stored(),locks=immediateLocks(),adapter=storageModule.create({library,storage,locks});let getters=0;
  const options={entries:[],guard:()=>true};Object.defineProperty(options,'expectedRaw',{enumerable:true,get(){getters++;return null;}});
  rejected(await adapter.mutate(options),'invalid_mutation');rejected(await adapter.mutate(new Proxy({},{ownKeys(){throw null;}})),'invalid_mutation');
  rejected(await adapter.reset({expectedRaw:1,guard:()=>true}),'invalid_mutation');rejected(await adapter.reset({expectedRaw:null,guard:()=>true,extra:1}),'invalid_mutation');
  assert.equal(getters,0);assert.deepEqual(locks.calls,[]);assert.deepEqual(storage.calls,[]);
});

test('a catalog change while waiting for the lock is compared after acquisition and cannot be overwritten',async()=>{
  const initial=ok(library.encodeCatalog([entry()])),successor=ok(library.encodeCatalog([entry(ID_B,'Successor')]));
  for(const currentRaw of [successor,null,initial+' ']){
    const storage=stored(initial),locks=heldLocks(),adapter=storageModule.create({library,storage,locks});
    const pending=adapter.mutate({expectedRaw:initial,entries:[],guard:()=>true});assert.equal(locks.pending.length,1);assert.deepEqual(storage.calls,[]);
    if(currentRaw===null)storage.data.delete(KEY);else storage.data.set(KEY,currentRaw);
    locks.pending[0].acquire();rejected(await pending,'stale_base');
    assert.equal(storage.data.has(KEY)?storage.data.get(KEY):null,currentRaw);assert.equal(storage.calls.filter(call=>call[0]==='set'||call[0]==='remove').length,0);assert.equal(storage.data.get(storage.legacy),'old exact column bytes');
  }
});

test('nested draft edits during a held lock cannot change accepted search, rules, sort, projection or name',async()=>{
  const storage=stored(),locks=heldLocks(),adapter=storageModule.create({library,storage,locks}),draft=JSON.parse(JSON.stringify(entry())),accepted=JSON.stringify(draft);
  const pending=adapter.mutate({expectedRaw:null,entries:[draft],guard:()=>true});assert.equal(locks.pending.length,1);
  draft.name='Changed later';draft.view.search='retarget';draft.view.rules[0].min=100;draft.view.sort.direction=1;draft.view.visibleKeys.pop();
  locks.pending[0].acquire();const result=ok(await pending);
  assert.equal(JSON.stringify(result.entries[0]),accepted);assert.equal(JSON.stringify(ok(library.decodeCatalog(storage.data.get(KEY))).entries[0]),accepted);
  assert(Object.isFrozen(result.entries[0].view.rules[0]));assert.equal(storage.calls.filter(call=>call[0]==='set').length,1);
});

test('explicit reset of valid and absent catalogs removes only the exact key once without rewriting or parsing',async()=>{
  for(const raw of [null,ok(library.encodeCatalog([entry()]))]){
    const storage=stored(raw),locks=immediateLocks(),decodeCalls=[];
    const decodeForbidden={...library,decodeCatalog(){decodeCalls.push(true);throw Error('Explicit reset must not decode');}};
    const adapter=storageModule.create({library:decodeForbidden,storage,locks});
    assert.deepEqual(ok(await adapter.reset({expectedRaw:raw,guard:()=>true})),{raw:null,entries:[],missing:true});
    assert.equal(storage.data.has(KEY),false);assert.deepEqual(decodeCalls,[]);assert.deepEqual(storage.calls,[['get',KEY],['remove',KEY]]);assert.equal(storage.data.get(storage.legacy),'old exact column bytes');
    assert.deepEqual(locks.calls,[[LOCK_NAME,{mode:'exclusive',ifAvailable:true}]]);
  }
});
