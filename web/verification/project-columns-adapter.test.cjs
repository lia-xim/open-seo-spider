'use strict';
// Actual TableExplorerUI closure, with inert DOM and storage sinks.
// These are pure transaction controls, not native or browser execution proof.
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const root=path.resolve(__dirname,'../..'),source=fs.readFileSync(path.join(__dirname,'../table-explorer-ui.js'),'utf8');
const declaredSchema=[{key:'url',type:'text',mandatory:true,label:'URL',value:r=>r.url},{key:'status',type:'number',label:'Status',value:r=>r.status},{key:'title',type:'text',label:'Title',value:r=>r.title}];
const rows=[{url:'https://owned.invalid/a',status:200,title:'Alpha'},{url:'https://owned.invalid/b',status:404,title:'Beta'},{url:'https://owned.invalid/c',status:200,title:'Beta'}];
function harness(){
 const nodes=new Map(),writes=[],listeners=new Map();
 function node(){return{value:'',hidden:false,dataset:{},children:[],options:[],listeners:new Map(),addEventListener(type,fn){this.listeners.set(type,fn);},replaceChildren(...values){this.children=[...values];this.options=this.children;},append(...values){this.children.push(...values);this.options=this.children;},setAttribute(){},querySelectorAll(){return[];}};}
 const $=id=>{if(!nodes.has(id))nodes.set(id,node());return nodes.get(id);};
 const context=vm.createContext({console,window:null,TextEncoder,TextDecoder,structuredClone});context.window=context;
 context.document={getElementById:id=>['table-expression-editor','table-expression-target'].includes(id)?null:$(id),createElement:node,createTextNode:text=>({textContent:text})};context.localStorage={getItem:()=>null,setItem:(...args)=>writes.push(['set',...args]),removeItem:(...args)=>writes.push(['remove',...args])};context.addEventListener=(event,fn)=>listeners.set(event,fn);
 vm.runInContext(fs.readFileSync(path.join(root,'web/table-explorer.js'),'utf8'),context,{filename:'actual-table-explorer.js'});
 context.TableExpressionEditor={copy:structuredClone};
 vm.runInContext(source,context,{filename:'actual-table-explorer-ui.js'});
 let sort=vm.runInContext('({key:"url",direction:1})',context),search='',filter='all',edits=0,changes=0;
 const hooks={owner:()=>({projectId:'owned-crawl',section:'titles'}),sort:()=>sort,setSort:next=>sort=next,identity:vm.runInContext('({identity:row=>row.url})',context),queryEdited:()=>edits++,changed:()=>changes++};
 const ui=context.TableExplorerUI.create(hooks);ui.synchronize(declaredSchema);
 const plain=value=>JSON.parse(JSON.stringify(value));
 const snapshot=()=>({draft:plain(ui.draft()),mode:ui.queryVersion(),search,filter,sort:{...sort},writes:[...writes],edits,changes});
 function install(version){search='Beta';filter='all';sort=vm.runInContext('({key:"url",direction:-1})',context);const common={search,sort,visibleKeys:['url','status','title']};let result;
  if(version===1)result=ui.installPrepared({...common,rules:[{key:'status',op:'min',min:200}]});
  else{context.declaredView=JSON.stringify({...common,expression:{kind:'any',children:[{kind:'rule',rule:{key:'status',op:'min',min:400}},{kind:'rule',rule:{key:'title',op:'equals',text:'Alpha'}}]}});result=ui.installPrepared(vm.runInContext('JSON.parse(declaredView)',context));}
  assert.equal(result.ok,true,JSON.stringify(result));return result;
 }
 return{ui,$,writes,plain,snapshot,install,rows:()=>plain(ui.rows(rows,declaredSchema,search,sort)),setSearch:value=>search=value};
}
test('actual proposed adapter preserves accepted V1 AND, search, sort, row membership and browser storage',()=>{
 const h=harness();assert.equal(h.install(1).ok,true);const before=h.snapshot(),memberBefore=h.rows();assert.deepEqual(memberBefore.map(r=>r.url),['https://owned.invalid/c','https://owned.invalid/b']);
 assert.equal(h.ui.installColumns(['url','title'],()=>true).ok,true);const after=h.snapshot();
 assert.deepEqual(after.draft.value.rules,before.draft.value.rules);assert.deepEqual(after.sort,before.sort);assert.equal(after.search,before.search);assert.equal(after.filter,before.filter);assert.equal(after.mode,1);assert.deepEqual(after.draft.value.visibleKeys,['url','title']);assert.deepEqual(h.rows(),memberBefore);assert.deepEqual(h.writes,[]);assert.equal(after.edits,before.edits+1);assert.equal(after.changes,before.changes+1);
});
test('actual proposed adapter preserves grouped V2 OR AST and complete membership',()=>{
 const h=harness();assert.equal(h.install(2).ok,true);const before=h.snapshot(),members=h.rows();assert.deepEqual(members.map(r=>r.url),['https://owned.invalid/b']);
 assert.equal(h.ui.installColumns(['url','title','status'],()=>true).ok,true);const after=h.snapshot();assert.equal(after.mode,2);assert.deepEqual(after.draft.value.expression,before.draft.value.expression);assert.deepEqual(after.draft.value.visibleKeys,['url','title','status']);assert.deepEqual(after.sort,before.sort);assert.equal(after.search,before.search);assert.equal(after.filter,before.filter);assert.deepEqual(h.rows(),members);assert.deepEqual(h.writes,[]);
});
for(const keys of[[],['status','url'],['url','url'],['url','absent'],['url','status','title','status']])test('actual adapter refuses invalid projection atomically '+JSON.stringify(keys),()=>{const h=harness();assert.equal(h.install(1).ok,true);const before=h.snapshot();assert.equal(h.ui.installColumns(keys,()=>true).ok,false);assert.deepEqual(h.snapshot(),before);});
test('actual adapter refuses both preflight and final synchronous stale guards without mutation',()=>{
 for(const results of[[false],[true,false]]){const h=harness();assert.equal(h.install(2).ok,true);const before=h.snapshot();let calls=0;const result=h.ui.installColumns(['url','title'],()=>results[calls++]);assert.equal(result.ok,false);assert.equal(result.error.code,'stale_preferences');assert.equal(calls,results.length);assert.deepEqual(h.snapshot(),before);}
});
test('actual adapter keeps a valid query accepted when a native load invokes query invalidation synchronously',async()=>{
 const P=require('../project-columns.js'),h=harness();assert.equal(h.install(2).ok,true);const id='11111111-1111-4111-8111-111111111111',revision='22222222-2222-4222-8222-222222222222',entry={section:'titles',schema:declaredSchema.map(c=>({key:c.key,type:c.type,mandatory:Boolean(c.mandatory)})),visible_keys:['url','title']};
 let query=0,controller;const before=h.snapshot();controller=P.create({owner:()=>({projectId:id,section:'titles',generation:1}),queryRevision:()=>query,schema:()=>entry.schema,visibleKeys:()=>h.plain(h.ui.draft().value.visibleKeys),apply:(keys,guard)=>{const applied=h.ui.installColumns(keys,guard);if(applied.ok){query++;controller.invalidate();}return applied;},fetch:async()=>new Response(JSON.stringify({project_id:id,fingerprint:{state:'present',revision,sha256:'a'.repeat(64)},document:{engine:P.ENGINE,version:1,project_id:id,revision,entries:[entry]},error:null}),{headers:{'content-type':'application/json'}})});
 assert.equal((await controller.load()).ok,true);assert.equal(controller.state().status,'present');assert.equal(controller.canSave(),true);assert.equal(h.ui.queryVersion(),2);assert.deepEqual(h.snapshot().draft.value.expression,before.draft.value.expression);assert.deepEqual(h.plain(h.ui.draft().value.visibleKeys),['url','title']);assert.deepEqual(h.writes,[]);
});
