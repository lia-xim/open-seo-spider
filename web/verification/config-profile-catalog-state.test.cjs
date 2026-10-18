'use strict';
// Controller and actual App-closure regressions only. Injected native replies
// do not prove persistence, real HTTP, native validation or browser behavior.
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const {webcrypto,createHash}=require('node:crypto');
const context=vm.createContext({TextEncoder,TextDecoder,AbortController,DOMException,Uint8Array,Map,Set,setTimeout,clearTimeout,crypto:webcrypto});
vm.runInContext(fs.readFileSync(path.join(__dirname,'../config-profiles.js'),'utf8'),context);const tools=context.ConfigProfiles;
const hash=text=>createHash('sha256').update(text).digest('hex'),size=text=>Buffer.byteLength(text);
const A='10000000-0000-4000-8000-000000000001',B='10000000-0000-4000-8000-000000000002';
function profile(agent='original') { const config='{"seed_urls":["https://owned.invalid/'+agent+'"],"crawl_links":false,"user_agent":'+JSON.stringify(agent)+',"delay_ms":9007199254740993,"max_body_bytes":1572865,"custom_headers":{},"renderer":null}';return {engine:tools.ENGINE,version:1,profile_json:'{"version":1,"config":'+config+',"required_headers":["X-Owned"]}',config_json:config,effective_config_json:config,normalization:[],missing_headers:['X-Owned'],requires_targets:false,max_bytes:tools.MAX_BYTES}; }
function entry(id=A,name='Owned profile',value=profile(),revision='1') {return {id,name,revision,profile_sha256:hash(value.profile_json),profile_bytes:size(value.profile_json),created_at:'2026-10-18T12:00:00Z',updated_at:'2026-10-18T12:00:00.123456789Z'};}
const list=entries=>({engine:tools.CATALOG_ENGINE,version:1,scope:'selected_application_data_directory',entries,limits:{...tools.CATALOG_LIMITS}});
function deferred(){let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return{promise,resolve,reject};}
async function harness(transport) {
  const bytes=new Map(),store={writes:0,getItem:key=>bytes.get(key)??null,setItem(key,value){this.writes++;bytes.set(key,value);}};let catalog;
  const base=profile(),profiles=tools.createController({storage:store,transport:async()=>base,invalidated:()=>catalog?.invalidate()});profiles.begin();await profiles.defaults();await profiles.save([]);profiles.header('X-Owned','volatile-owned-secret');
  const calls=[];catalog=tools.createCatalogController({profiles,transport:async(route,body,options)=>{calls.push({route,body,options});return transport(route,body,options);}});
  const snapshot=()=>({active:profiles.state.active,working:profiles.state.working,text:profiles.state.text,stored:store.getItem(tools.KEY),writes:store.writes,secret:profiles.headerValue('X-Owned')});
  return{base,profiles,catalog,calls,store,snapshot};
}
function sameProfile(h,before){const now=h.snapshot();assert.equal(now.active,before.active);assert.equal(now.working,before.working);assert.equal(now.text,before.text);assert.equal(now.stored,before.stored);assert.equal(now.writes,before.writes);assert.equal(now.secret,before.secret);}
test('construction and dialog reopen never auto-fetch a native catalog or store browser catalog metadata',async()=>{
  const h=await harness(async()=>{throw Error('No catalog fetch expected');}),before=h.snapshot();h.profiles.close();h.profiles.begin();assert.equal(h.calls.length,0);assert.equal(h.catalog.state.entries.length,0);assert.equal(h.catalog.state.loaded,false);assert.equal(h.store.writes,before.writes);assert.equal(h.store.getItem(tools.KEY),before.stored);
});
test('explicit list and selection expose metadata only without loading or adopting a profile',async()=>{
  const h=await harness(async()=>list([entry(A,'<img src=x onerror=owned>'),entry(B,' /native/name ')])),before=h.snapshot();await h.catalog.list();h.catalog.select(B);
  assert.equal(h.calls.length,1);assert.equal(h.calls[0].route,'/api/config-profile-catalog');assert.equal(h.calls[0].body,undefined);assert.equal(h.catalog.selected().name,' /native/name ');sameProfile(h,before);
});
test('verified catalog load stages only the dialog draft and clears runtime bindings without adopting or repairing storage',async()=>{
  const loaded=profile('catalog'),metadata=entry(A,'Catalog',loaded),h=await harness(async route=>route.endsWith(A)?{entry:metadata,profile:loaded}:list([metadata]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot();h.profiles.state.restoreError='Owned damaged-storage guard';
  const staged=await h.catalog.load();assert.equal(staged.profile_json,loaded.profile_json);assert.equal(h.profiles.state.text,loaded.profile_json);assert.equal(h.profiles.state.working.profile_json,loaded.profile_json);assert.equal(h.profiles.state.active,before.active);assert.equal(h.store.getItem(tools.KEY),before.stored);assert.equal(h.store.writes,before.writes);assert.equal(h.profiles.hasHeader('X-Owned'),false);assert.equal(h.profiles.state.restoreError,'Owned damaged-storage guard');
});
test('create purely inspects exact source text and changed raw tokens before storing, preserving active/working/header/browser state',async()=>{
  const h=await harness(async(route,body)=>route==='/api/config-profiles/inspect'?h.base:{entry:entry(A,'  Exact / name  ',h.base)}),before=h.snapshot(),patches=[{path:['max_body_bytes'],value_json:'9007199254740993'}];
  await h.catalog.create('  Exact / name  ',{profile_text:h.base.profile_json,patches});assert.equal(h.calls.length,2);assert.equal(h.calls[0].body.profile_text,h.base.profile_json);assert.equal(h.calls[0].body.patches,patches);assert.equal(h.calls[1].route,'/api/config-profile-catalog');assert.equal(h.calls[1].options.method,'POST');assert.equal(h.calls[1].body.name,'  Exact / name  ');assert.equal(h.calls[1].body.profile_text,h.base.profile_json);assert.match(h.calls[1].body.profile_text,/9007199254740993/);assert(!JSON.stringify(h.calls).includes('volatile-owned-secret'));sameProfile(h,before);
});
test('create from a legacy form uses pure native from-config without staging its normalized copy',async()=>{
  const h=await harness(async(route,body)=>route==='/api/config-profiles/from-config'?h.base:{entry:entry(A,'Legacy',h.base)}),before=h.snapshot();
  await h.catalog.create('Legacy',{config_text:'{"max_body_bytes":1572865}'});assert.equal(h.calls[0].route,'/api/config-profiles/from-config');assert.equal(h.calls[0].body.config_text,'{"max_body_bytes":1572865}');assert.equal(h.calls[1].body.profile_text,h.base.profile_json);sameProfile(h,before);
});
test('overwrite sends exact string revision beyond JS safe integer and SHA CAS while keeping its prepared copy inactive',async()=>{
  const original=entry(A,'Old',profile(),'9007199254740993'),h=await harness(async(route,body,options)=>route==='/api/config-profiles/inspect'?h.base:options?.method==='PUT'?{entry:entry(A,body.name,h.base,'9007199254740994')}:list([original]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot();await h.catalog.overwrite('New',{profile_text:h.base.profile_json});
  const put=h.calls.at(-1);assert.equal(put.options.method,'PUT');assert.equal(put.body.expected_revision,'9007199254740993');assert.equal(put.body.expected_sha256,original.profile_sha256);assert.equal(h.catalog.selected().revision,'9007199254740994');sameProfile(h,before);
});
test('rename and delete use selected native CAS and never rewrite the already loaded draft, active profile or secret bindings',async()=>{
  const original=entry(),renamed={...original,name:'Renamed exact',revision:'2'},h=await harness(async(route,body,options)=>route.endsWith('/rename')?{entry:renamed}:options?.method==='DELETE'?{deleted_id:A}:list([original]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot();await h.catalog.rename('Renamed exact');sameProfile(h,before);assert.equal(h.calls.at(-1).body.expected_revision,'1');
  await h.catalog.remove();assert.equal(h.calls.at(-1).options.method,'DELETE');assert.equal(h.calls.at(-1).body.expected_revision,'2');assert.equal(h.calls.at(-1).body.expected_sha256,original.profile_sha256);assert.equal(h.catalog.state.entries.length,0);assert.equal(h.catalog.state.selectedId,'');sameProfile(h,before);
});
test('409 overwrite, rename and delete preserve every prior profile and metadata fact',async()=>{
  for(const method of ['overwrite','rename','remove']){
    let armed=false;const h=await harness(async(route)=>{if(!armed)return list([entry()]);if(route==='/api/config-profiles/inspect')return h.base;throw Object.assign(Error('Owned CAS conflict'),{status:409});});await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),metadata=JSON.stringify(h.catalog.state.entries);armed=true;
    if(method==='overwrite')await h.catalog.overwrite('Name',{profile_text:h.base.profile_json});else if(method==='rename')await h.catalog.rename('Name');else await h.catalog.remove();
    sameProfile(h,before);assert.equal(JSON.stringify(h.catalog.state.entries),metadata);assert.equal(h.catalog.state.selectedId,A);assert.match(h.catalog.state.error,/Konflikt/);assert.equal(h.catalog.state.busy,false);
  }
});
test('a duplicate-name create failure does not adopt the inspected copy or replace the working draft',async()=>{
  const copy=profile('copy'),h=await harness(async route=>{if(route==='/api/config-profiles/inspect')return copy;throw Object.assign(Error('Owned exact-name conflict'),{status:409});}),before=h.snapshot();await h.catalog.create('Duplicate',{profile_text:copy.profile_json});sameProfile(h,before);assert.equal(h.catalog.state.entries.length,0);assert.match(h.catalog.state.error,/Konflikt/);
});
test('missing and unavailable native catalog reads leave profile/headers and prior metadata unchanged',async()=>{
  for(const status of [404,503]){let armed=false;const h=await harness(async()=>{if(armed)throw Object.assign(Error('Owned unavailable '+status),{status});return list([entry()]);});await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),metadata=JSON.stringify(h.catalog.state.entries);armed=true;await h.catalog.load();sameProfile(h,before);assert.equal(JSON.stringify(h.catalog.state.entries),metadata);assert.match(h.catalog.state.error,/Owned unavailable/);}
});
test('malformed load metadata, wrong ID/bytes/SHA and conflicting known names are atomic refusals before draft/header changes',async()=>{
  const good=profile('loaded'),metadata=entry(A,'A',good),other=entry(B,'B');const mutations=[reply=>{reply.entry.id=B;},reply=>{reply.entry.profile_bytes++;},reply=>{reply.entry.profile_sha256='0'.repeat(64);},reply=>{reply.entry.revision=1;},reply=>{reply.profile.engine='wrong';},reply=>{reply.entry.name='B';}];
  for(const mutate of mutations){const reply={entry:{...metadata},profile:{...good}};mutate(reply);const h=await harness(async route=>route.endsWith(A)?reply:list([metadata,other]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),entries=JSON.stringify(h.catalog.state.entries);assert.equal(await h.catalog.load(),null);sameProfile(h,before);assert.equal(JSON.stringify(h.catalog.state.entries),entries);assert.notEqual(h.catalog.state.error,'');}
});
test('new selection and load B discard a late valid load A without releasing busy ownership or transferring headers',async()=>{
  const a=profile('A'),b=profile('B'),ea=entry(A,'A',a),eb=entry(B,'B',b),first=deferred(),second=deferred();const h=await harness(async route=>route.endsWith(A)?first.promise:route.endsWith(B)?second.promise:list([ea,eb]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),old=h.catalog.load();h.catalog.select(B);const current=h.catalog.load();
  first.resolve({entry:ea,profile:a});await old;assert.equal(h.catalog.state.busy,true);sameProfile(h,before);second.resolve({entry:eb,profile:b});await current;assert.equal(h.profiles.state.working.profile_json,b.profile_json);assert.equal(h.profiles.state.active,before.active);assert.equal(h.catalog.state.busy,false);
});
test('old list failure cannot overwrite a newer pending list or clear its busy flag',async()=>{
  const a=deferred(),b=deferred();let count=0;const h=await harness(async()=>++count===1?a.promise:b.promise),old=h.catalog.list(),current=h.catalog.list();a.reject(Error('Owned old list error'));await old;assert.equal(h.catalog.state.busy,true);assert.equal(h.catalog.state.error,'');b.resolve(list([entry()]));await current;assert.equal(h.catalog.state.entries.length,1);assert.equal(h.catalog.state.busy,false);
});
test('closing Settings invalidates pending catalog load and does not stage its late native result',async()=>{
  const wait=deferred(),target=profile('late'),metadata=entry(A,'Late',target),h=await harness(async route=>route.endsWith(A)?wait.promise:list([metadata]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),pending=h.catalog.load();h.profiles.close();wait.resolve({entry:metadata,profile:target});await pending;assert.equal(h.profiles.state.working,null);assert.equal(h.profiles.state.active,before.active);assert.equal(h.store.getItem(tools.KEY),before.stored);assert.equal(h.profiles.headerValue('X-Owned'),before.secret);assert.equal(h.catalog.state.busy,false);
});
test('actual profile editing invalidates a pending catalog result while retaining the newer raw draft',async()=>{
  const wait=deferred(),target=profile('late'),metadata=entry(A,'Late',target),h=await harness(async route=>route.endsWith(A)?wait.promise:list([metadata]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),pending=h.catalog.load(),newer=profile('newer');h.profiles.edited(newer.profile_json);wait.resolve({entry:metadata,profile:target});await pending;assert.equal(h.profiles.state.text,newer.profile_json);assert.equal(h.profiles.state.active,before.active);assert.equal(h.store.getItem(tools.KEY),before.stored);assert.equal(h.catalog.state.busy,false);
});
test('an explicit catalog load supersedes an older pending profile preview before either native reply completes',async()=>{
  const old=deferred(),next=deferred(),base=profile(),stale=profile('older-preview'),loaded=profile('catalog-load'),metadata=entry(A,'Catalog',loaded);let catalog,hold=false;
  const profiles=tools.createController({storage:{setItem(){},getItem:()=>null},transport:async()=>hold?old.promise:base,invalidated:()=>catalog?.invalidate()});profiles.begin();await profiles.defaults();await profiles.save([]);
  catalog=tools.createCatalogController({profiles,transport:async route=>route.endsWith(A)?next.promise:list([metadata])});await catalog.list();catalog.select(A);hold=true;const pendingOld=profiles.defaults(),pendingLoad=catalog.load();old.resolve(stale);await pendingOld;assert.equal(profiles.state.working.profile_json,base.profile_json);assert.equal(catalog.state.busy,true);
  next.resolve({entry:metadata,profile:loaded});await pendingLoad;assert.equal(profiles.state.working.profile_json,loaded.profile_json);assert.equal(profiles.state.active.profile_json,base.profile_json);
});
test('editing a name while pure validation waits prevents the obsolete create mutation from being sent',async()=>{
  const wait=deferred(),h=await harness(async route=>{assert.equal(route,'/api/config-profiles/inspect');return wait.promise;}),before=h.snapshot(),pending=h.catalog.create('Old name',{profile_text:h.base.profile_json});h.catalog.edited();wait.resolve(h.base);await pending;assert.equal(h.calls.length,1);assert.equal(h.catalog.state.entries.length,0);assert.equal(h.catalog.state.busy,false);sameProfile(h,before);
});
test('a delayed rename acknowledgement after selection B cannot alter metadata, selection or draft',async()=>{
  const wait=deferred(),ea=entry(A,'A'),eb=entry(B,'B'),h=await harness(async route=>route.endsWith('/rename')?wait.promise:list([ea,eb]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),pending=h.catalog.rename('Old request');h.catalog.select(B);wait.resolve({entry:{...ea,name:'Old request',revision:'2'}});await pending;assert.equal(h.catalog.state.entries[0].name,'A');assert.equal(h.catalog.state.selectedId,B);sameProfile(h,before);
});
test('malformed metadata and advertised budgets are rejected without partially replacing a valid list',async()=>{
  const invalid=[x=>{x.engine='wrong';},x=>{x.scope='other';},x=>{x.limits.max_entries=129;},x=>{x.entries[0].revision=9007199254740993;},x=>{x.entries[0].revision='01';},x=>{x.entries[0].revision='9223372036854775808';},x=>{x.entries[0].profile_bytes=0;},x=>{x.entries[0].created_at='not a date';},x=>{x.entries[0].name='\n';},x=>{x.entries[0].extra=true;},x=>{x.entries.push({...x.entries[0]});},x=>{x.entries.push({...entry(B),name:x.entries[0].name});}];
  for(const change of invalid){const value=list([entry()]);change(value);assert.throws(()=>tools.checkedCatalogList(value));}
  const huge=list(Array.from({length:5},(_,i)=>({...entry('10000000-0000-4000-8000-'+String(i+1).padStart(12,'0'),'Owned'+i),profile_bytes:67108864})));assert.throws(()=>tools.checkedCatalogList(huge),/Gesamtbudget/);
  let next=list([entry()]);const h=await harness(async()=>next);await h.catalog.list();const previous=JSON.stringify(h.catalog.state.entries);next=list([entry(B)]);next.entries[0].revision='bad';await h.catalog.list();assert.equal(JSON.stringify(h.catalog.state.entries),previous);assert.notEqual(h.catalog.state.error,'');
});
test('name validation uses exact native UTF8/Unicode rules without interpreting paths or trimming',()=>{
  for(const name of [' /path\\name?x# ','__proto__','constructor','A','a','\ufeff', 'é'.repeat(128)])assert.equal(tools.catalogName(name),name);
  for(const name of ['', ' \t ', '\u0085', 'é'.repeat(129), 'bad\u0000name', 'bad\u007fname'])assert.throws(()=>tools.catalogName(name));
});
test('rename with mismatched immutable hash cannot update either metadata or live profile',async()=>{
  const initial=entry(),h=await harness(async route=>route.endsWith('/rename')?{entry:{...initial,name:'Changed',revision:'2',profile_sha256:'1'.repeat(64)}}:list([initial]));await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),prior=JSON.stringify(h.catalog.state.entries);assert.equal(await h.catalog.rename('Changed'),null);sameProfile(h,before);assert.equal(JSON.stringify(h.catalog.state.entries),prior);
});
test('actual shared transport forwards PUT and DELETE methods and original CAS strings',async()=>{
  const observed=[],base=profile(),initial=entry(A,'Initial',base,'9007199254740993');let revision=9007199254740993n;
  const sandbox=vm.createContext({TextEncoder,TextDecoder,AbortController,DOMException,Uint8Array,Map,Set,setTimeout,clearTimeout,crypto:webcrypto,fetch:async(route,options)=>{observed.push({route,...options});let result;if(route==='/api/config-profiles/inspect')result=base;else if(options.method==='PUT')result={entry:{...initial,name:'Updated',revision:String(++revision)}};else if(options.method==='DELETE')result={deleted_id:A};else result=list([initial]);const bytes=new TextEncoder().encode(JSON.stringify(result));let read=false;return{ok:true,status:200,headers:{get:()=>null},body:{getReader:()=>({read:async()=>read?{done:true}:(read=true,{done:false,value:bytes}),cancel:async()=>{}})}};}});
  vm.runInContext(fs.readFileSync(path.join(__dirname,'../config-profiles.js'),'utf8'),sandbox);const local=sandbox.ConfigProfiles,profiles=local.createController({storage:{setItem(){},getItem:()=>null},transport:async()=>base});profiles.begin();await profiles.defaults();const catalog=local.createCatalogController({profiles});await catalog.list();catalog.select(A);await catalog.overwrite('Updated',{profile_text:base.profile_json});await catalog.remove();
  const put=observed.find(x=>x.method==='PUT'),remove=observed.find(x=>x.method==='DELETE');assert(put);assert(remove);assert.equal(JSON.parse(put.body).expected_revision,'9007199254740993');assert.equal(JSON.parse(remove.body).expected_revision,'9007199254740994');assert.equal(JSON.parse(remove.body).expected_sha256,initial.profile_sha256);
});

// Execute the actual App preparation, projection, rendering and bound handlers.
// Omit unrelated layout/render callbacks; this remains VM proof, not a browser.
function appHarness() {
  const source=fs.readFileSync(path.join(__dirname,'../app.js'),'utf8'),ending=/  initialize\(\);\s*\}\)\(\);\s*$/;assert(ending.test(source));
  class Node {
    constructor(){this.children=[];this.dataset={};this.handlers={};this.value='';this.hidden=false;this.disabled=false;this.classList={toggle(){}};this.labels=[{textContent:''}];}
    append(...nodes){this.children.push(...nodes);}replaceChildren(...nodes){this.children=nodes;}setAttribute(key,value){this[key]=value;}removeAttribute(key){delete this[key];}addEventListener(key,fn){this.handlers[key]=fn;}closest(){return this;}querySelector(){return new Node();}querySelectorAll(){return [];}
    set innerHTML(value){throw Error('Catalog content must remain text, never HTML: '+value);}
  }
  const nodes=new Map(),get=id=>{if(!nodes.has(id))nodes.set(id,new Node());return nodes.get(id);};
  const sandbox={window:{ConfigProfiles:tools},console,Intl,URL,TextEncoder,TextDecoder,Map,Set,JSON,Number,Uint8Array,document:{getElementById:get,createElement:()=>new Node(),createTextNode:text=>({textContent:text})}};
  const replacement=`  toggleRendererSettings=()=>{};syncResourceContractSettings=()=>{};syncSitemapSourceProcessingSettings=()=>{};syncSitemapSourceArchiveSettings=()=>{};syncCrawlAdmissionSettings=()=>{};renderQuotaRuleControls=()=>{};readQuotaRuleControls=()=>[];syncQuotaSettings=()=>{};renderControls=()=>{};
  globalThis.actual={legacySettingsDefaults,projectConfigProfile,projectProfileToolbar,profileToolbarSnapshot,renderConfigProfileCatalog,configProfileCatalogPreparation,writeConfigProfileCatalog,bindConfigProfileCatalog,stageConfigProfile,attach(profiles,catalog){configProfiles=profiles;configProfileCatalog=catalog;}};
})();`;
  vm.runInNewContext(source.replace(ending,replacement),sandbox);const actual=sandbox.actual,elements=[];elements.namedItem=name=>elements.find(x=>x.name===name);
  for(const[name,value]of Object.entries(actual.legacySettingsDefaults)){const element={name,type:typeof value==='boolean'?'checkbox':'text',value:String(value),checked:value===true,removeAttribute(key){delete this[key];}};elements.push(element);elements[name]=element;}
  get('settings-form').elements=elements;return{...actual,elements,get};
}
test('actual App saves a patched catalog copy without adopting it or resetting an explicit toolbar target',async()=>{
  const app=appHarness(),prepared=profile('new-form-value'),h=await harness(async route=>route==='/api/config-profiles/inspect'?prepared:{entry:entry(A,'App copy',prepared)});app.attach(h.profiles,h.catalog);app.projectConfigProfile(h.base);app.projectProfileToolbar(h.base);app.elements.user_agent.value='new-form-value';app.get('seed-url').value='https://owned.invalid/run-only';app.get('config-catalog-name').value='App copy';const before=h.snapshot(),toolbar=app.profileToolbarSnapshot();
  await app.writeConfigProfileCatalog(false);assert.equal(h.calls[0].body.profile_text,h.base.profile_json);assert.equal(JSON.stringify(h.calls[0].body.patches),JSON.stringify([{path:['user_agent'],value_json:'"new-form-value"'}]));assert.equal(h.calls[1].body.profile_text,prepared.profile_json);assert.match(h.calls[0].body.profile_text,/9007199254740993/);assert.equal(app.profileToolbarSnapshot(),toolbar);sameProfile(h,before);
});
test('actual App renders exact hostile names as option text and keeps catalog selection metadata-only',async()=>{
  const name=' <img src=x onerror=owned> ',h=await harness(async()=>list([entry(A,name)])),app=appHarness();app.attach(h.profiles,h.catalog);app.bindConfigProfileCatalog();assert.equal(h.calls.length,0);await h.catalog.list();app.renderConfigProfileCatalog();const select=app.get('config-catalog-select');assert.equal(select.children[1].textContent,name+' · Revision 1');assert.equal(h.calls.length,1);select.value=A;select.handlers.change();app.renderConfigProfileCatalog();assert.equal(h.calls.length,1);assert.equal(app.get('config-catalog-name').value,name);assert.match(app.get('config-catalog-meta').textContent,/SHA-256 [a-f0-9]{64}/);assert.equal(h.profiles.state.working.profile_json,h.base.profile_json);
});
test('actual App delete handler requires visible matching selection/revision confirmation before DELETE',async()=>{
  const ea=entry(A,'A exact'),eb=entry(B,'B exact'),h=await harness(async(route,body,options)=>options?.method==='DELETE'?{deleted_id:body.expected_sha256===ea.profile_sha256?A:B}:list([ea,eb])),app=appHarness();app.attach(h.profiles,h.catalog);app.bindConfigProfileCatalog();await h.catalog.list();h.catalog.select(A);app.renderConfigProfileCatalog();const before=h.snapshot();app.get('config-catalog-delete').handlers.click();assert.equal(h.calls.length,1);assert.equal(app.get('config-catalog-delete-question').hidden,false);assert.match(app.get('config-catalog-delete-question-text').textContent,/A exact/);
  app.get('config-catalog-select').value=B;app.get('config-catalog-select').handlers.change();await app.get('config-catalog-delete-confirm').handlers.click();assert.equal(h.calls.length,1);
  app.get('config-catalog-select').value=A;app.get('config-catalog-select').handlers.change();app.get('config-catalog-delete').handlers.click();app.get('config-catalog-delete-cancel').handlers.click();await app.get('config-catalog-delete-confirm').handlers.click();assert.equal(h.calls.length,1);
  app.get('config-catalog-delete').handlers.click();await app.get('config-catalog-delete-confirm').handlers.click();assert.equal(h.calls.length,2);assert.equal(h.calls[1].options.method,'DELETE');sameProfile(h,before);
});
test('actual App projects a loaded catalog copy into settings while retaining original toolbar, active profile and browser storage',async()=>{
  const loaded=profile('loaded'),metadata=entry(A,'Loaded',loaded),h=await harness(async route=>route.endsWith(A)?{entry:metadata,profile:loaded}:list([metadata])),app=appHarness();app.attach(h.profiles,h.catalog);app.projectConfigProfile(h.base);app.projectProfileToolbar(h.base);await h.catalog.list();h.catalog.select(A);const before=h.snapshot(),toolbar=app.profileToolbarSnapshot();await app.stageConfigProfile(h.catalog.load());assert.equal(app.elements.user_agent.value,'loaded');assert.equal(h.profiles.state.working.profile_json,loaded.profile_json);assert.equal(h.profiles.state.active,before.active);assert.equal(h.store.getItem(tools.KEY),before.stored);assert.equal(app.profileToolbarSnapshot(),toolbar);assert.equal(h.profiles.hasHeader('X-Owned'),false);
});
