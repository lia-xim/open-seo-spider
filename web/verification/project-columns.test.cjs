'use strict';
// Pure controller protocol/state tests only. All HTTP bodies are declared test
// data; none establishes a native file, process, website or browser acceptance.
const test=require('node:test'),assert=require('node:assert/strict'),P=require('../project-columns.js');
const A='11111111-1111-4111-8111-111111111111',B='22222222-2222-4222-8222-222222222222',R1='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',R2='bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const columns=[{key:'url',type:'text',mandatory:true},{key:'title_text',type:'text',mandatory:false},{key:'count',type:'number',mandatory:false}];
const other={section:'status',schema:[{key:'url',type:'text',mandatory:true},{key:'status',type:'number',mandatory:false}],visible_keys:['url','status']};
const pref=(visible=['url','count'])=>({section:'titles',schema:structuredClone(columns),visible_keys:visible});
function reply({id=A,state='present',revision=R1,sha='a'.repeat(64),entries=[pref()]}={}){return{project_id:id,fingerprint:state==='missing'?{state}:state==='invalid'?{state,sha256:sha}:{state,revision,sha256:sha},document:state!=='present'?null:{engine:P.ENGINE,version:1,project_id:id,revision,entries},error:state==='invalid'?{code:'stored_document_invalid',message:'Declared corrupt-file control'}:null};}
const response=(body,status=200,headers={})=>new Response(JSON.stringify(body),{status,headers:{'content-type':'application/json',...headers}});
function harness(responses=[]){
 const ui={projectId:A,section:'titles',generation:1,nativeGeneration:1,segment:null,query:0,visible:['url','title_text'],schema:structuredClone(columns),available:true},calls=[],changes=[],applied=[];
 let controller;const fetch=async(url,options)=>{calls.push({url,options});assert(responses.length,'Unplanned HTTP call');const next=responses.shift();return typeof next==='function'?next(url,options):next;};
 controller=P.create({fetch,owner:()=>({...ui}),queryRevision:()=>ui.query,schema:()=>ui.schema,visibleKeys:()=>ui.visible,available:()=>ui.available,apply:(keys,guard)=>{assert(guard(),'Actual installation guard must still hold');applied.push([...keys]);ui.visible=[...keys];ui.query++;controller.invalidate();return{ok:true};},changed:value=>changes.push(value)});
 return{ui,calls,changes,applied,controller,responses};
}
function deferred(){let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return{promise,resolve,reject};}
test('cancelling a read with an existing CAS base never leaves a false loading message',async()=>{
 const pending=deferred(),h=harness([response(reply()),()=>pending.promise]);await h.controller.load();const before=h.controller.state().fingerprint,read=h.controller.load();assert.equal(h.controller.state().busy,true);h.ui.query++;h.controller.invalidate();assert.equal((await read).ok,false);assert.equal(h.controller.state().busy,false);assert.equal(h.controller.state().phase,'idle');assert.match(h.controller.state().message,/abgebrochen/);assert.deepEqual(h.controller.state().fingerprint,before);assert.equal(h.calls.length,2);
});
test('constructor never autoloads/autosaves; explicit missing load grants a missing CAS baseline',async()=>{
 const h=harness([response(reply({state:'missing'}))]);assert.equal(h.calls.length,0);assert.equal(h.controller.canSave(),false);assert.equal((await h.controller.save()).error.code,'load_required');assert.equal(h.calls.length,0);
 assert.equal((await h.controller.load()).ok,true);assert.equal(h.calls.length,1);assert.equal(h.applied.length,0);assert.equal(h.controller.state().status,'missing');assert.equal(h.controller.canSave(),true);assert.equal(h.controller.state().fingerprint.state,'missing');
});
test('known empty document remains present empty with a revision; does not apply defaults over current columns',async()=>{
 const h=harness([response(reply({entries:[]}))]);await h.controller.load();assert.equal(h.controller.state().status,'empty');assert.equal(h.controller.state().fingerprint.revision,R1);assert.equal(h.applied.length,0);assert.deepEqual(h.ui.visible,['url','title_text']);
});
test('explicit successful load validates exact schema and installs only ordered keys once',async()=>{
 const h=harness([response(reply())]);h.ui.activeQuery={version:2,expression:{kind:'any',children:[{key:'title_text',op:'contains',text:'a'},{key:'count',op:'range',min:0,max:0}]},search:'CAFÉ',filter:'duplicate',sort:{key:'count',direction:-1}};const query=structuredClone(h.ui.activeQuery);
 assert.equal((await h.controller.load()).ok,true);assert.deepEqual(h.applied,[['url','count']]);assert.deepEqual(h.ui.activeQuery,query);assert.equal(h.controller.state().status,'present');assert.equal(h.controller.canSave(),true,'Own synchronous columns apply must not invalidate its just-loaded baseline');
});
test('save merges only the current section and preserves inactive entry data and order',async()=>{
 const h=harness([response(reply({entries:[pref(),other]})),response(reply({revision:R2,sha:'b'.repeat(64),entries:[pref(['url','title_text']),other]}))]);await h.controller.load();h.ui.visible=['url','title_text'];assert.equal((await h.controller.save()).ok,true);const call=h.calls[1],body=JSON.parse(call.options.body);assert.equal(call.options.method,'PUT');assert.deepEqual(body,{expected:{state:'present',revision:R1,sha256:'a'.repeat(64)},entries:[pref(['url','title_text']),other]});assert.equal(h.controller.state().fingerprint.revision,R2);assert.equal(h.applied.length,1,'Save must not apply or reset the table');
});
test('missing first-save CAS creates a document only after an explicit read',async()=>{
 const h=harness([response(reply({state:'missing'})),response(reply({revision:R2,entries:[pref(['url','title_text'])]}))]);await h.controller.load();await h.controller.save();assert.deepEqual(JSON.parse(h.calls[1].options.body).expected,{state:'missing'});assert.equal(h.calls[1].url,'/api/crawls/'+A+'/table-preferences');
});
test('HTTP409 is surfaced and never retried/refreshed automatically',async()=>{
 const h=harness([response(reply()),response({code:'stale_preferences',error:'Actual declared CAS conflict'},409)]);await h.controller.load();const accepted=[...h.ui.visible];const result=await h.controller.save();assert.equal(result.ok,false);assert.equal(result.error.code,'stale_preferences');assert.equal(h.calls.length,2);assert.deepEqual(h.ui.visible,accepted);assert.equal(h.controller.canSave(),false);assert.equal(h.controller.state().status,'unloaded');assert.equal((await h.controller.save()).error.code,'load_required');assert.equal(h.calls.length,2);
});
test('invalid native read is distinct, refuses ordinary save and allows explicit raw-SHA reset',async()=>{
 const h=harness([response(reply({state:'invalid'})),response(reply({revision:R2,sha:'b'.repeat(64),entries:[]}))]);const accepted=[...h.ui.visible];assert.equal((await h.controller.load()).error.code,'stored_document_invalid');assert.equal(h.controller.state().status,'invalid');assert.equal(h.controller.canSave(),false);assert.equal(h.controller.canReset(),true);assert.equal((await h.controller.save()).error.code,'invalid_saved_preferences');assert.equal(h.calls.length,1);assert.equal((await h.controller.reset()).ok,true);assert.equal(h.calls[1].url,'/api/crawls/'+A+'/table-preferences/reset');assert.equal(h.calls[1].options.method,'POST');assert.deepEqual(JSON.parse(h.calls[1].options.body),{expected:{state:'invalid',sha256:'a'.repeat(64)}});assert.deepEqual(h.ui.visible,accepted);assert.equal(h.applied.length,0);assert.equal(h.controller.state().status,'empty');
});
test('reset requires a new present-empty revision and cannot accept reset ABA',async()=>{
 const h=harness([response(reply()),response(reply({entries:[]}))]);await h.controller.load();assert.equal((await h.controller.reset()).ok,false);assert.equal(h.controller.canSave(),false);assert.equal(h.controller.state().status,'unloaded');
});
test('stale A load cannot install columns or state into newly selected B',async()=>{
 const waiting=deferred(),h=harness([()=>waiting.promise]);const result=h.controller.load();h.ui.projectId=B;h.ui.generation++;h.controller.invalidate();const before=h.controller.state();waiting.resolve(response(reply()));assert.equal((await result).error.code,'stale_preferences');assert.deepEqual(h.controller.state(),before);assert.equal(h.controller.state().projectId,B);assert.equal(h.applied.length,0);assert.equal(h.calls[0].options.signal.aborted,true);
});
test('query edits while GET waits invalidate a load without altering accepted columns',async()=>{
 const waiting=deferred(),h=harness([()=>waiting.promise]);const result=h.controller.load();h.ui.query++;h.ui.visible=['url','title_text','count'];h.controller.invalidate();const before=h.controller.state();waiting.resolve(response(reply()));assert.equal((await result).error.code,'stale_preferences');assert.equal(h.applied.length,0);assert.deepEqual(h.ui.visible,['url','title_text','count']);assert.deepEqual(h.controller.state(),before);
});
test('query revision is checked even if an adapter misses explicit invalidate',async()=>{
 const waiting=deferred(),h=harness([()=>waiting.promise]);const result=h.controller.load();h.ui.query++;waiting.resolve(response(reply()));assert.equal((await result).error.code,'stale_preferences');assert.equal(h.applied.length,0);assert.equal(h.controller.state().busy,false);
});
test('a stale/aborted PUT never reports success in B and requires a fresh A read before another save',async()=>{
 const waiting=deferred(),h=harness([response(reply()),()=>waiting.promise]);await h.controller.load();const result=h.controller.save();h.ui.projectId=B;h.ui.generation++;h.controller.invalidate();const before=h.controller.state();waiting.resolve(response(reply({revision:R2,entries:[pref()]})));assert.equal((await result).error.code,'stale_preferences');assert.deepEqual(h.controller.state(),before);assert.equal(h.controller.state().projectId,B);assert.equal(h.controller.canSave(),false);
});
test('prepared request bytes do not change when caller edits input arrays during PUT',async()=>{
 const waiting=deferred(),h=harness([response(reply({state:'missing'})),()=>waiting.promise]);await h.controller.load();const result=h.controller.save();const before=h.calls[1].options.body;h.ui.visible.push('count');h.ui.schema[0].key='changed-after-snapshot';assert.equal(h.calls[1].options.body,before);waiting.resolve(response(reply({revision:R2,entries:[pref(['url','title_text'])]})));assert.equal((await result).ok,true);
});
test('incompatible saved schema refuses load atomically, without dropping unknown columns',async()=>{
 const stored=pref();stored.schema[1].type='number';const h=harness([response(reply({entries:[stored]}))]);const accepted=[...h.ui.visible];assert.equal((await h.controller.load()).ok,false);assert.deepEqual(h.ui.visible,accepted);assert.equal(h.applied.length,0);
});
test('unavailable view and in-flight double-click are explicit zero-write refusals',async()=>{
 const waiting=deferred(),h=harness([()=>waiting.promise]);h.ui.available=false;assert.equal((await h.controller.load()).error.code,'unavailable');assert.equal(h.calls.length,0);h.ui.available=true;const first=h.controller.load();assert.equal((await h.controller.load()).error.code,'preferences_busy');assert.equal(h.calls.length,1);waiting.resolve(response(reply({state:'missing'})));await first;
});
for(const [label,edit]of[
 ['duplicate key',value=>value.schema.push({...value.schema[0]})],['unknown visible',value=>value.visible_keys.push('invented')],['missing mandatory',value=>value.visible_keys=['count']],['duplicate visible',value=>value.visible_keys=['url','url']],['invalid type',value=>value.schema[1].type='boolean'],['control in key',value=>value.schema[1].key='bad\nkey'],['too many columns',value=>value.schema=Array.from({length:129},(_,i)=>({key:'c'+i,type:'text',mandatory:i===0}))]
])test('wire metadata rejects '+label,()=>{const value=pref();edit(value);assert.equal(P.validateEntry(value).ok,false);});
test('UTF8 key/section limits are actual bytes, not JavaScript code units',()=>{
 const value={section:'ä'.repeat(64),schema:[{key:'😀'.repeat(64),type:'text',mandatory:true}],visible_keys:['😀'.repeat(64)]};assert.equal(P.validateEntry(value).ok,true);value.schema[0].key+='a';assert.equal(P.validateEntry(value).ok,false);value.schema[0].key='url';value.visible_keys=['url'];value.section+='a';assert.equal(P.validateEntry(value).ok,false);
});
test('fingerprints and owner bindings reject arrays/coercion/unknown fields',()=>{
 const value=reply();value.fingerprint.revision=[R1];value.document.revision=value.fingerprint.revision;assert.equal(P.validateEnvelope(value,A).ok,false);
 assert.equal(P.validateEnvelope(reply({id:B}),A).ok,false);const unknown=reply({state:'missing'});unknown.fingerprint.sha256='a'.repeat(64);assert.equal(P.validateEnvelope(unknown,A).ok,false);
});
test('64 sections are bounded and duplicate section identity cannot be saved twice',()=>{
 const value=reply({entries:Array.from({length:65},(_,i)=>({...pref(),section:'section'+i}))});assert.equal(P.validateEnvelope(value,A).ok,false);assert.equal(P.validateEnvelope(reply({entries:[pref(),pref()]}),A).ok,false);
});
for(const [label,replyValue]of[
 ['wrong owner',()=>response(reply({id:B}))],['wrong content type',()=>response(reply(),200,{'content-type':'text/html'})],['truncated JSON',()=>new Response('{"broken":',{headers:{'content-type':'application/json'}})],['overflow',()=>new Response(' '.repeat(P.LIMITS.replyBytes+1),{headers:{'content-type':'application/json'}})],['file failure',()=>response({code:'preferences_unavailable',error:'Declared filesystem write/read failure'},503)]
])test('honest '+label+' failure never applies or emits saved success',async()=>{const h=harness([replyValue()]),before=[...h.ui.visible];assert.equal((await h.controller.load()).ok,false);assert.equal(h.applied.length,0);assert.deepEqual(h.ui.visible,before);assert.equal(h.controller.state().busy,false);assert(h.controller.state().error);assert(!/wurden nativ gespeichert/.test(h.controller.state().message));});
test('body and protocol bounds reject oversized current payload before sending a PUT',async()=>{
 const h=harness([response(reply({state:'missing'}))]);await h.controller.load();h.ui.schema=Array.from({length:129},(_,i)=>({key:'c'+i,type:'text',mandatory:i===0}));h.ui.visible=['c0'];assert.equal((await h.controller.save()).ok,false);assert.equal(h.calls.length,1);
});
test('fixed deadline aborts a hanging GET and releases busy state without retry',async t=>{
 assert.equal(P.DEADLINE_MS,15000);t.mock.timers.enable({apis:['setTimeout']});
 try{const h=harness([()=>new Promise(()=>{})]),pending=h.controller.load();t.mock.timers.tick(P.DEADLINE_MS-1);await Promise.resolve();assert.equal(h.controller.state().busy,true);t.mock.timers.tick(1);const result=await pending;assert.equal(result.error.code,'preferences_timeout');assert.equal(h.controller.state().busy,false);assert.equal(h.controller.canSave(),false);assert.equal(h.calls.length,1);assert.equal(h.calls[0].options.signal.aborted,true);t.mock.timers.tick(150000);assert.equal(h.calls.length,1);}finally{t.mock.timers.reset();}
});
test('fixed deadline cancels a hanging response body rather than waiting for EOF forever',async t=>{
 assert.equal(P.DEADLINE_MS,15000);t.mock.timers.enable({apis:['setTimeout']});let cancelled=0;
 try{const body=new ReadableStream({start(){},cancel(){cancelled++;}}),h=harness([new Response(body,{headers:{'content-type':'application/json'}})]),pending=h.controller.load();await new Promise(setImmediate);assert.equal(body.locked,true);t.mock.timers.tick(P.DEADLINE_MS);assert.equal((await pending).error.code,'preferences_timeout');await Promise.resolve();assert.equal(cancelled,1);assert.equal(h.controller.state().busy,false);}finally{t.mock.timers.reset();}
});
test('timed-out PUT is uncertain, clears its old CAS base and never retries automatically',async t=>{
 assert.equal(P.DEADLINE_MS,15000);t.mock.timers.enable({apis:['setTimeout']});
 try{const h=harness([response(reply()),()=>new Promise(()=>{})]);await h.controller.load();const pending=h.controller.save();t.mock.timers.tick(P.DEADLINE_MS);assert.equal((await pending).error.code,'preferences_timeout');assert.equal(h.controller.state().busy,false);assert.equal(h.controller.canSave(),false);assert.equal((await h.controller.save()).error.code,'load_required');assert.equal(h.calls.length,2);t.mock.timers.tick(150000);assert.equal(h.calls.length,2);}finally{t.mock.timers.reset();}
});
