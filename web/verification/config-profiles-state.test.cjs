'use strict';
// Controller/representation controls only. Native parsing and browser/file-picker
// acceptance have separate actual fixtures; injected replies prove neither.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const context = vm.createContext({TextEncoder, TextDecoder, AbortController, DOMException, Uint8Array, Map, Set, setTimeout, clearTimeout});
vm.runInContext(fs.readFileSync(path.join(__dirname, '../config-profiles.js'), 'utf8'), context);
const tools = context.ConfigProfiles;
const encode = text => new TextEncoder().encode(text);
function envelope(config = '{}', headers = []) {
  return {engine:'open-seo-spider/crawl-config-profile-v1',version:1,profile_json:'{"version":1,"config":'+config+',"required_headers":'+JSON.stringify(headers)+'}',
    config_json:config,effective_config_json:config,normalization:[],missing_headers:headers,requires_targets:false,max_bytes:tools.MAX_BYTES};
}
function storage() {
  const values = new Map();
  return {writes:0,fail:false,getItem:key=>values.get(key)??null,setItem(key,value){if(this.fail)throw Error('Owned quota refusal');values.set(key,value);this.writes++;}};
}
function file(text) {
  const bytes=typeof text==='string'?encode(text):text;
  return {size:bytes.length,slice(){return {arrayBuffer:async()=>bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength)};}};
}
function deferred() { let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject}; }
test('raw field views retain unsafe integers, nested arrays, strings and absence',()=>{
  const text='{"delay_ms":9007199254740993,"renderer":{"max_response_bytes":18446744073709551615},"seeds":["{x}","a\\\"b"],"empty":null}';
  assert.equal(tools.at(text,['delay_ms']),'9007199254740993');
  assert.equal(tools.at(text,['renderer','max_response_bytes']),'18446744073709551615');
  assert.equal(tools.at(text,['missing']),undefined);assert.equal(tools.at(text,['empty']),'null');
  assert.equal(tools.fields(text).get('seeds'),'["{x}","a\\\"b"]');
  assert.throws(()=>tools.fields('{"x":1,"x":2}'),/Doppelter/);
});
test('MiB projections roundtrip exact byte budgets including beyond JS safe integer',()=>{
  for(const value of ['1','1572865','5242880','9007199254740993','18446744073709551615'])assert.equal(tools.fromMiB(tools.toMiB(value)),value);
  assert.throws(()=>tools.fromMiB('0'),/positive/);assert.throws(()=>tools.fromMiB('0.0000001'),/Bytezahl/);
});
test('malformed UTF8 file never reaches native inspection or mutates accepted state/storage',async()=>{
  const saved=storage(),base=envelope('{"delay_ms":1}');let requests=0;
  const controller=tools.createController({storage:saved,transport:async()=>{requests++;return base;}});
  controller.begin();await controller.defaults();await controller.save([]);
  const before=saved.getItem(tools.KEY),text=controller.state.text,active=controller.state.active,count=requests;
  assert.equal(await controller.importFile(file(new Uint8Array([0xc3,0x28]))),null);
  assert.equal(requests,count);assert.equal(saved.getItem(tools.KEY),before);assert.equal(controller.state.text,text);assert.equal(controller.state.active,active);
});
test('raw duplicate keys reach native validator without parse/stringify collapse',async()=>{
  const saved=storage(),bad='{"version":1,"config":{"delay_ms":1,"delay_ms":2},"required_headers":[]}';let observed;
  const controller=tools.createController({storage:saved,transport:async(route,body)=>{observed=body.profile_text;throw Error('Owned duplicate refusal');}});
  controller.begin();assert.equal(await controller.importFile(file(bad)),null);assert.equal(observed,bad);assert.equal(saved.writes,0);assert.equal(controller.state.active,null);assert.equal(controller.state.working,null);
});
test('late file A cannot issue native inspection after B owns the draft',async()=>{
  const wait=deferred(),a=envelope('{"user_agent":"A"}'),b=envelope('{"user_agent":"B"}'),calls=[];
  const controller=tools.createController({storage:storage(),transport:async(route,body)=>{calls.push(body.profile_text);return b;}});
  controller.begin();const delayed={size:encode(a.profile_json).length,slice:()=>({arrayBuffer:()=>wait.promise})};
  const pending=controller.importFile(delayed);await controller.importFile(file(b.profile_json));wait.resolve(encode(a.profile_json).buffer);await pending;
  assert.equal(calls.length,1);assert.equal(calls[0],b.profile_json);assert.equal(controller.state.text,b.profile_json);assert.equal(controller.state.busy,false);
});
test('late native response cannot overwrite B or release B busy ownership',async()=>{
  const first=deferred(),second=deferred(),a=envelope('{"user_agent":"A"}'),b=envelope('{"user_agent":"B"}');let count=0,oldSignal;
  const controller=tools.createController({storage:storage(),transport:(route,body,options)=>{count++;if(count===1){oldSignal=options.signal;return first.promise;}return second.promise;}});
  controller.begin();const old=controller.defaults(),newer=controller.defaults();assert.equal(oldSignal.aborted,true);
  first.resolve(a);await old;assert.equal(controller.state.busy,true);assert.equal(controller.state.working,null);
  second.resolve(b);await newer;assert.equal(controller.state.text,b.profile_json);assert.equal(controller.state.busy,false);
});
test('stale selected-crawl owner refuses a valid late reply',async()=>{
  const wait=deferred();let selected='A';const controller=tools.createController({storage:storage(),transport:()=>wait.promise});
  controller.begin();const pending=controller.selected('A',()=>selected==='A');selected='B';wait.resolve(envelope());await pending;
  assert.equal(controller.state.working,null);assert.equal(controller.state.busy,false);assert.equal(controller.state.error,'');
});
test('storage quota refusal leaves active profile, staged text and prior bytes unchanged',async()=>{
  const saved=storage(),a=envelope('{"delay_ms":1}'),b=envelope('{"delay_ms":2}');let next=a;
  const controller=tools.createController({storage:saved,transport:async()=>next});controller.begin();await controller.defaults();await controller.save([]);
  const before=saved.getItem(tools.KEY),active=controller.state.active;controller.edited(b.profile_json);saved.fail=true;next=b;
  assert.equal(await controller.save([]),null);assert.equal(controller.state.active,active);assert.equal(controller.state.text,b.profile_json);assert.equal(saved.getItem(tools.KEY),before);assert.match(controller.state.error,/quota/);
});
test('case-distinct and prototype-like header names retain exact volatile bindings only',async()=>{
  const names=['X-Exact','x-exact','__proto__','constructor'],value=envelope('{"custom_headers":{}}',names),saved=storage();
  const controller=tools.createController({storage:saved,transport:async()=>value});controller.begin();await controller.defaults();await controller.save([]);
  assert.throws(()=>controller.headerValues(),/fehlen/);
  for(const [index,name]of names.entries())controller.header(name,'owned-secret-'+index);
  const bindings=controller.headerValues();for(const[index,name]of names.entries())assert.equal(bindings[name],'owned-secret-'+index);
  assert.equal(Object.getPrototypeOf(bindings),vm.runInContext('Object.prototype',context));assert.equal(Object.prototype['owned-secret'],undefined);
  assert(!saved.getItem(tools.KEY).includes('owned-secret'));assert(saved.getItem(tools.KEY).includes('__proto__'));
});
test('explicit empty header differs from unresolved and is not persisted',async()=>{
  const saved=storage(),value=envelope('{}',['X-Empty']);const controller=tools.createController({storage:saved,transport:async()=>value});controller.begin();await controller.defaults();await controller.save([]);
  assert.throws(()=>controller.headerValues());controller.header('X-Empty','',true);assert.equal(controller.headerValues()['X-Empty'],'');controller.header('X-Empty','',false);assert.throws(()=>controller.headerValues());
});
test('native response containing header values cannot reach working draft or storage',async()=>{
  const controller=tools.createController({storage:storage(),transport:async()=>envelope('{"custom_headers":{"Authorization":"never-save"}}')});controller.begin();assert.equal(await controller.defaults(),null);assert.equal(controller.state.working,null);assert.equal(controller.state.active,null);
});
test('an incompatible native engine cannot stage or persist a profile',async()=>{
  const saved=storage(),value={...envelope(),engine:'other-profile-v1'},controller=tools.createController({storage:saved,transport:async()=>value});controller.begin();assert.equal(await controller.defaults(),null);assert.equal(controller.state.working,null);assert.equal(saved.writes,0);
});
test('valid export leaves active/staged profile and storage unchanged',async()=>{
  const saved=storage(),value=envelope('{"delay_ms":9007199254740993}');
  const controller=tools.createController({storage:saved,transport:async(route)=>route.endsWith('/export')?{text:value.profile_json,bytes:encode(value.profile_json)}:value});
  controller.begin();await controller.defaults();await controller.save([]);const before=saved.getItem(tools.KEY),active=controller.state.active,writes=saved.writes;
  const result=await controller.exportProfile([]);assert.equal(result.text,value.profile_json);assert.equal(controller.state.active,active);assert.equal(saved.getItem(tools.KEY),before);assert.equal(saved.writes,writes);
});
test('restore validates stored JSON text without rounding or rewriting the storage key',async()=>{
  const saved=storage(),value=envelope('{"delay_ms":9007199254740993}');saved.setItem(tools.KEY,JSON.stringify({version:1,profile_text:value.profile_json}));let observed;
  const controller=tools.createController({storage:saved,transport:async(route,body)=>{observed=body.profile_text;return value;}});const writes=saved.writes;await controller.restore();
  assert.equal(observed,value.profile_json);assert.equal(controller.state.active.profile_json,value.profile_json);assert.equal(saved.writes,writes);
});
test('closing dialog invalidates pending native result and keeps active storage unchanged',async()=>{
  const saved=storage(),wait=deferred(),controller=tools.createController({storage:saved,transport:()=>wait.promise});controller.begin();const pending=controller.defaults();controller.close();wait.resolve(envelope());await pending;
  assert.equal(controller.state.working,null);assert.equal(controller.state.active,null);assert.equal(controller.state.open,false);assert.equal(saved.writes,0);
});
test('explicit source replacement clears all header values while retaining exact requirements',async()=>{
  const value=envelope('{}',['X-Exact','x-exact']),controller=tools.createController({storage:storage(),transport:async()=>value});controller.begin();await controller.defaults();await controller.save([]);
  controller.header('X-Exact','one');controller.header('x-exact','two');controller.clearHeaders();assert.equal(controller.unresolved().length,2);assert.throws(()=>controller.headerValues(),/fehlen/);
});
test('canceling another staged profile cannot transfer its header secrets to the active profile',async()=>{
  const a=envelope('{"user_agent":"A"}',['X-Exact']),b=envelope('{"user_agent":"B"}',['X-Exact']);let next=a;
  const controller=tools.createController({storage:storage(),transport:async()=>next});controller.begin();await controller.defaults();await controller.save([]);
  next=b;await controller.defaults();controller.header('X-Exact','for-B-only');controller.close();assert.equal(controller.state.active.profile_json,a.profile_json);assert.throws(()=>controller.headerValues(),/fehlen/);
});
test('saving then closing retains only the explicitly resolved active-session header values',async()=>{
  const value=envelope('{}',['X-Exact']),controller=tools.createController({storage:storage(),transport:async()=>value});controller.begin();await controller.defaults();controller.header('X-Exact','owned-current');await controller.save([]);controller.close();assert.equal(controller.headerValues()['X-Exact'],'owned-current');
});
test('corrupt, incompatible and native-refused restores remain blocked through dialog/edit/import until a validated atomic Save',async()=>{
  const cases=['{bad',JSON.stringify({version:2,profile_text:'{}'}),JSON.stringify({version:1,profile_text:[]}),JSON.stringify({version:1,profile_text:'native-refused'})];
  for(const bytes of cases){
    const saved=storage(),value=envelope('{"seed_urls":["https://owned.invalid/"]}');saved.setItem(tools.KEY,bytes);
    const controller=tools.createController({storage:saved,transport:async(route,body)=>{if(body?.profile_text==='native-refused')throw Error('Owned native refusal');return value;}});
    assert.equal(await controller.restore(),null);assert.equal(typeof controller.state.restoreError,'string');assert.notEqual(controller.state.restoreError,'');const failure=controller.state.restoreError;
    controller.begin();assert.equal(controller.state.restoreError,failure);controller.edited();controller.close();assert.equal(controller.state.restoreError,failure);controller.begin();
    await controller.importFile(file(value.profile_json));await controller.inspect([]);assert.equal(controller.state.restoreError,failure);assert.equal(saved.getItem(tools.KEY),bytes);
    saved.fail=true;assert.equal(await controller.save([]),null);assert.equal(controller.state.restoreError,failure);assert.equal(saved.getItem(tools.KEY),bytes);assert.equal(controller.state.active,null);
    saved.fail=false;await controller.save([]);assert.equal(controller.state.restoreError,'');assert.equal(controller.state.active.profile_json,value.profile_json);assert.notEqual(saved.getItem(tools.KEY),bytes);
  }
});
test('storage read refusal blocks restore without deleting or writing a profile',async()=>{
  let writes=0,calls=0;const controller=tools.createController({storage:{getItem(){throw Error('Owned storage access refusal');},setItem(){writes++;}},transport:async()=>{calls++;return envelope();}});
  assert.equal(await controller.restore(),null);assert.match(controller.state.restoreError,/Owned storage access refusal/);controller.begin();assert.match(controller.state.restoreError,/Owned storage access refusal/);assert.equal(writes,0);assert.equal(calls,0);
});
test('a superseded late restore refusal cannot block a newer explicitly saved profile',async()=>{
  const saved=storage(),old=deferred(),value=envelope('{"user_agent":"newer"}');saved.setItem(tools.KEY,JSON.stringify({version:1,profile_text:'owned-old'}));
  const controller=tools.createController({storage:saved,transport:async(route,body)=>body?.profile_text==='owned-old'?old.promise:value});const pending=controller.restore();
  controller.begin();await controller.defaults();await controller.save([]);const bytes=saved.getItem(tools.KEY);old.reject(Error('Owned stale restore refusal'));await pending;
  assert.equal(controller.state.restoreError,'');assert.equal(controller.state.active.profile_json,value.profile_json);assert.equal(saved.getItem(tools.KEY),bytes);assert.equal(controller.state.error,'');
});
test('opening Settings during pending restore cannot enable a fallback Start before an explicit valid Save',async()=>{
  const app=appHarness(),saved=storage(),old=deferred(),value=appProfile();saved.setItem(tools.KEY,JSON.stringify({version:1,profile_text:'owned-pending'}));const bytes=saved.getItem(tools.KEY);
  const controller=tools.createController({storage:saved,transport:async(route,body)=>body?.profile_text==='owned-pending'?old.promise:value,changed:()=>app.renderConfigProfile()});app.attachController(controller);
  const pending=controller.restore();assert.equal(controller.state.restorePending,true);controller.begin();await controller.defaults();controller.close();app.renderControls();assert.equal(app.get('start-button').disabled,true);
  app.get('crawl-mode').value='spider';app.get('seed-url').value='https://owned.invalid/';await app.startCrawl({preventDefault(){}});assert.equal(app.fixtureStarts.length,0);assert.equal(saved.getItem(tools.KEY),bytes);
  controller.begin();await controller.defaults();app.projectConfigProfile(value);await app.saveConfigProfile();assert.equal(controller.state.restorePending,false);app.renderControls();assert.equal(app.get('start-button').disabled,false);
  old.reject(Error('Owned superseded restore'));await pending;assert.equal(controller.state.restorePending,false);assert.equal(controller.state.restoreError,'');
});

// Actual App projection/patch functions; DOM layout/render-only callbacks are
// omitted here. Real native/browser tests independently exercise those controls.
function appHarness() {
  const source=fs.readFileSync(path.join(__dirname,'../app.js'),'utf8'),ending=/  initialize\(\);\s*\}\)\(\);\s*$/;
  assert(ending.test(source));
  class Node {
    constructor(tag){this.tag=tag;this.children=[];this.dataset={};this.handlers={};this.hidden=false;this.value='';this.disabled=false;this.classList={toggle(){}};}
    append(...values){this.children.push(...values);}replaceChildren(...values){this.children=values;}
    removeAttribute(key){delete this[key];}setAttribute(key,value){this[key]=value;}addEventListener(key,fn){this.handlers[key]=fn;}
    querySelectorAll(selector){const found=[];for(const child of this.children){if(selector==='[data-profile-header]'&&child.dataset?.profileHeader!==undefined||selector==='[data-profile-header-resolve]'&&child.dataset?.profileHeaderResolve!==undefined)found.push(child);if(child.querySelectorAll)found.push(...child.querySelectorAll(selector));}return found;}
    querySelector(){return new Node('button');}closest(){return this;}close(){this.open=false;this.handlers.close?.();}
  }
  const nodes=new Map(),get=id=>{if(!nodes.has(id))nodes.set(id,new Node('div'));return nodes.get(id);};
  const sandbox={window:{ConfigProfiles:tools},console,Intl,URL,TextEncoder,TextDecoder,Map,Set,JSON,Number,Uint8Array,document:{getElementById:get,createElement:tag=>new Node(tag),createTextNode:text=>({textContent:text})}};
  const substituted=source.replace(ending,`  let fixtureQuotaRows=[];toggleRendererSettings=()=>{};syncResourceContractSettings=()=>{};syncSitemapSourceProcessingSettings=()=>{};syncSitemapSourceArchiveSettings=()=>{};syncCrawlAdmissionSettings=()=>{};renderQuotaRuleControls=rows=>{fixtureQuotaRows=(rows||[]).map(row=>({...row,max_urls:String(row.max_urls)}));};readQuotaRuleControls=()=>fixtureQuotaRows;syncQuotaSettings=()=>{};
  const fixtureStarts=[],fixtureRenderControls=renderControls;renderControls=()=>{};renderDashboardSegments=()=>{};renderSitemapBundleControls=()=>{};renderQuotaSummary=()=>{};syncDirectiveReportingProfileSource=()=>{};invalidateContentRequests=()=>{};renderHistory=()=>{};selectCrawl=async()=>{};request=async(route,options)=>{fixtureStarts.push({route,body:JSON.parse(options.body)});return{id:'owned-profile-run'};};
  globalThis.actual={state,legacySettingsDefaults,projectConfigProfile,configProfilePatches,renderConfigProfile,renderControls:fixtureRenderControls,projectProfileToolbar,profileToolbarSnapshot,saveConfigProfile,startCrawl,startProfileCrawl,fixtureStarts,toolbarBaseline:()=>profileToolbarBaseline,attachController(value){configProfiles=value;},editQuotaRows(rows){fixtureQuotaRows=rows;},attach(value){configProfiles={state:{working:value,text:value.profile_json}};}};
})();`);
  vm.runInNewContext(substituted,sandbox);
  const actual=sandbox.actual,elements=[];
  elements.namedItem=name=>elements.find(element=>element.name===name);
  for(const[name,value]of Object.entries(actual.legacySettingsDefaults)){
    const element={name,type:typeof value==='boolean'?'checkbox':'text',value:String(value),checked:value===true,removeAttribute(key){delete this[key];}};
    elements.push(element);elements[name]=element;
  }
  get('settings-form').elements=elements;
  get('seed-url').labels=[{textContent:''}];get('settings-dialog').open=true;
  return {...actual,get,elements};
}
function appProfile() {
  const config=JSON.stringify({concurrency:128,request_timeout_secs:900,max_pages:null,max_depth:null,delay_ms:'BIG',max_body_bytes:1572865,
    user_agent:'Owned native profile',indexability_agent:'googlebot',obey_robots:false,check_external:false,validate_fragments:false,include_subdomains:false,
    seed_urls:['https://owned.invalid/one','https://owned.invalid/two'],sitemap_urls:[],custom_headers:{},
    renderer:{endpoint:'http://127.0.0.1:9876/render',wait_ms:1,accessibility:false,screenshot:false,collect_cookies:false,custom_javascript:[],max_response_bytes:1234567},
    crawl_policy:{scope:{roots:[{origin:'https://owned.invalid',folders:['/']}]}}}).replace('"delay_ms":"BIG"','"delay_ms":9007199254740993');
  return envelope(config);
}
test('actual App projection preserves exact non-MiB/unsafe values without committing or dirty patches',()=>{
  const app=appHarness(),value=appProfile(),prior=app.state.settings;app.attach(value);app.projectConfigProfile(value);
  assert.equal(app.elements.concurrency.value,'128');assert.equal(app.elements.timeout_seconds.value,'900');assert.equal(app.elements.delay_ms.value,'9007199254740993');
  assert.equal(tools.fromMiB(app.elements.max_body_mb.value),'1572865');assert.equal(app.state.settings,prior);assert.equal(app.configProfilePatches().length,0);assert.equal(app.elements.concurrency.max,'1024');
});
test('actual unrelated form edit emits only its path and preserves all native-only declarations',()=>{
  const app=appHarness(),value=appProfile();app.attach(value);app.projectConfigProfile(value);app.elements.user_agent.value='Edited UA';
  assert.equal(JSON.stringify(app.configProfilePatches()),JSON.stringify([{path:['user_agent'],value_json:'"Edited UA"'}]));
  assert(value.config_json.includes('9007199254740993'));assert(value.config_json.includes('"max_response_bytes":1234567'));
});
test('actual renderer edit patches one leaf instead of replacing native max_response_bytes',()=>{
  const app=appHarness(),value=appProfile();app.attach(value);app.projectConfigProfile(value);app.elements.renderer_wait_ms.value='2';
  assert.equal(JSON.stringify(app.configProfilePatches()),JSON.stringify([{path:['renderer','wait_ms'],value_json:'2'}]));
});
test('actual body edit maps decimal MiB to an exact integer JSON token',()=>{
  const app=appHarness(),value=appProfile();app.attach(value);app.projectConfigProfile(value);app.elements.max_body_mb.value=tools.toMiB('9007199254740993');
  assert.equal(JSON.stringify(app.configProfilePatches()),JSON.stringify([{path:['max_body_bytes'],value_json:'9007199254740993'}]));
});
test('actual optional disable removes only a declared key, never absent linked limits',()=>{
  const app=appHarness(),value=appProfile();app.attach(value);app.projectConfigProfile(value);app.elements.crawl_policy_enabled.checked=false;
  assert.equal(JSON.stringify(app.configProfilePatches()),JSON.stringify([{path:['crawl_policy'],remove:true}]));
});
test('actual imported quota projection and unrelated quota edit retain untouched original pattern rows',()=>{
  const app=appHarness(),base=appProfile(),raw=base.config_json.replace(tools.fields(base.config_json).get('renderer'),'null').slice(0,-1)+',"crawl_quotas":{"version":1,"max_urls_per_depth":5,"url_pattern_quotas":[{"id":"owned","pattern":"/one","max_urls":2}]}}',value=envelope(raw);
  app.attach(value);app.projectConfigProfile(value);assert.equal(app.configProfilePatches().length,0);app.elements.crawl_quota_per_depth.value='6';
  assert.equal(JSON.stringify(app.configProfilePatches()),JSON.stringify([{path:['crawl_quotas','max_urls_per_depth'],value_json:'6'}]));
});
test('actual quota row edit is detected independently of hidden JSON formatting',()=>{
  const app=appHarness(),base=appProfile(),raw=base.config_json.replace(tools.fields(base.config_json).get('renderer'),'null').slice(0,-1)+',"crawl_quotas":{"version":1,"url_pattern_quotas":[{"id":"owned","pattern":"/one","max_urls":2}]}}',value=envelope(raw);
  app.attach(value);app.projectConfigProfile(value);app.editQuotaRows([{id:'owned',pattern:'/two',max_urls:'3'}]);
  assert.equal(JSON.stringify(app.configProfilePatches()),JSON.stringify([{path:['crawl_quotas','url_pattern_quotas'],value_json:'[{"id":"owned","pattern":"/two","max_urls":3}]'}]));
});
test('actual header input and unresolve listeners survive synchronous rerender without losing typed values',async()=>{
  const app=appHarness(),value=envelope('{}',['constructor']),controller=tools.createController({storage:storage(),transport:async()=>value,changed:()=>app.renderConfigProfile()});
  app.attachController(controller);controller.begin();await controller.defaults();
  const host=app.get('config-profile-required-headers'),input=host.querySelectorAll('[data-profile-header]')[0],resolve=host.querySelectorAll('[data-profile-header-resolve]')[0];
  input.value='owned-typed-secret';input.handlers.input();assert.equal(controller.headerValue('constructor'),'owned-typed-secret');assert.equal(input.value,'owned-typed-secret');assert.equal(resolve.checked,true);
  resolve.checked=false;resolve.handlers.change();assert.equal(controller.hasHeader('constructor'),false);assert.equal(resolve.checked,false);
});
test('actual Save preserves an explicit toolbar target after header reentry and Start emits original-source replacement patches',async()=>{
  const app=appHarness(),raw=appProfile().config_json.slice(0,-1)+',"crawl_links":false,"input_imports":[],"explicit_sitemap_inputs":[]}',value=envelope(raw,['X-Exact','x-exact']);
  const controller=tools.createController({storage:storage(),transport:async()=>value,changed:()=>app.renderConfigProfile()});
  app.attachController(controller);controller.begin();await controller.defaults();await controller.save([]);app.projectConfigProfile(value,true);app.projectProfileToolbar(value);
  const baseline=app.toolbarBaseline();app.get('seed-url').value='https://owned.invalid/replacement';controller.edited();controller.clearHeaders();assert.throws(()=>controller.headerValues(),/fehlen/);
  const host=app.get('config-profile-required-headers');for(const input of host.querySelectorAll('[data-profile-header]')){input.value='owned-'+input.dataset.profileHeader;input.handlers.input();}
  await app.saveConfigProfile();assert.equal(app.get('seed-url').value,'https://owned.invalid/replacement');assert.equal(app.toolbarBaseline(),baseline);assert.notEqual(app.profileToolbarSnapshot(),baseline);
  await app.startProfileCrawl();assert.equal(app.fixtureStarts.length,1);const actual=app.fixtureStarts[0];assert.equal(actual.route,'/api/crawls/from-profile');
  assert.equal(JSON.stringify(actual.body.patches),JSON.stringify([{path:['seed_urls'],value_json:'["https://owned.invalid/replacement"]'},{path:['sitemap_urls'],value_json:'[]'},{path:['input_imports'],value_json:'[]'},{path:['explicit_sitemap_inputs'],remove:true}]));
  assert.equal(JSON.stringify(actual.body.header_values),JSON.stringify({'X-Exact':'owned-X-Exact','x-exact':'owned-x-exact'}));assert.equal(actual.body.profile_text,value.profile_json);assert.match(actual.body.profile_text,/"crawl_links":false/);
});
test('actual Save deliberately projects a changed profile source instead of retaining an old toolbar override',async()=>{
  const app=appHarness(),value=appProfile(),replacement=envelope(value.config_json.replace('https://owned.invalid/one','https://owned.invalid/imported'));let next=value;
  const controller=tools.createController({storage:storage(),transport:async()=>next,changed:()=>app.renderConfigProfile()});
  app.attachController(controller);controller.begin();await controller.defaults();await controller.save([]);app.projectConfigProfile(value,true);app.projectProfileToolbar(value);
  app.get('seed-url').value='https://owned.invalid/old-override';controller.edited(replacement.profile_json);next=replacement;await controller.inspect([]);app.projectConfigProfile(replacement);
  await app.saveConfigProfile();assert.equal(app.get('seed-url').value,'https://owned.invalid/imported');assert.equal(app.profileToolbarSnapshot(),app.toolbarBaseline());
});
test('actual App blocks button and both programmatic Start paths after corrupt restore while keeping recovery reachable',async()=>{
  const app=appHarness(),saved=storage(),value=appProfile(),controller=tools.createController({storage:saved,transport:async()=>value,changed:()=>app.renderConfigProfile()});app.attachController(controller);
  saved.setItem(tools.KEY,'{bad');await controller.restore();app.get('crawl-mode').value='spider';app.get('seed-url').value='https://owned.invalid/';
  app.renderControls();assert.equal(app.get('start-button').disabled,true);assert.match(app.get('settings-summary').textContent,/gesperrt/);assert.equal(app.get('config-profile-restore-error').hidden,false);const warning=app.get('config-profile-restore-error').textContent;assert(warning.length);
  controller.begin();assert.equal(app.get('config-profile-error').hidden,false);controller.close();assert.equal(app.get('config-profile-restore-error').textContent,warning);
  await app.startCrawl({preventDefault(){}});await app.startProfileCrawl();assert.equal(app.fixtureStarts.length,0);assert.equal(app.state.starting,false);assert.equal(saved.getItem(tools.KEY),'{bad');
  controller.begin();await controller.defaults();app.projectConfigProfile(value);app.renderControls();assert.equal(app.get('start-button').disabled,true);assert.equal(saved.getItem(tools.KEY),'{bad');
  await app.saveConfigProfile();app.renderControls();assert.equal(app.get('start-button').disabled,false);assert.equal(app.get('config-profile-restore-error').hidden,true);assert.equal(controller.state.restoreError,'');
  await app.startCrawl({preventDefault(){}});assert.equal(app.fixtureStarts.length,1);assert.equal(app.fixtureStarts[0].route,'/api/crawls/from-profile');
});
