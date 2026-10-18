'use strict';
// Actual production closure functions, with only automatic DOM startup replaced.
// These are state controls; the independent live suite supplies native axe proof.
const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs'),path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../app.js'),'utf8');
const ending=/  initialize\(\);\s*\}\)\(\);\s*$/;
assert(ending.test(source),'Expected one final application startup');
class Element{
 constructor(tag){this.tag=tag;this.children=[];this.dataset={};this.textContent='';this.classList={toggle(){}};}
 append(...nodes){this.children.push(...nodes);} replaceChildren(...nodes){this.children=nodes;} addEventListener(){}
}
function harness(){
 const elements=new Map(),pending=[];const get=id=>{if(!elements.has(id))elements.set(id,new Element('div'));return elements.get(id);};
 const context={console,Intl,URL,Set,Map,Number,JSON,Blob,TextEncoder,Uint8Array,document:{getElementById:get,createElement:tag=>new Element(tag)},fetch:()=>new Promise((resolve,reject)=>pending.push({resolve,reject}))};
 vm.runInNewContext(source.replace(ending,'  globalThis.actual={state,getRows,columns,accessibilityObservation,renderAccessibilityInspector,serverDownload};\n})();'),context);
 return {...context.actual,get,pending};
}
function zero(){return {engine:'axe-core',version:'4.14.0',automated_only:true,violations:[],incomplete:[],passes_count:4,inapplicable_count:8};}
const snapshot=value=>JSON.parse(JSON.stringify(value));
test('malformed imported metadata objects cannot crash real accessibility table or inspector',()=>{
 const app=harness(),hostile={toString:null,valueOf:null};
 for(const field of ['version','engine','error']){
  const raw={...zero(),[field]:hostile},page={url:'https://owned.invalid/',accessibility:raw};
  app.state.section='accessibility';app.state.pages=[page];
  const row=app.getRows()[0];assert.equal(row.violations,null);assert.equal(row.incomplete,null);
  for(const column of app.columns())assert.doesNotThrow(()=>column.render(row));
  const inspector=new Element('div');assert.doesNotThrow(()=>app.renderAccessibilityInspector(page,inspector));
  assert(['malformed','error'].includes(inspector.dataset.accessibilityStatus));
  assert.deepEqual(snapshot(raw[field]),hostile,'Stored original remains unchanged');
 }
});
test('actual classifier preserves observed0 and unknown instead of using empty fallbacks',()=>{
 const app=harness();assert.equal(app.accessibilityObservation({accessibility:zero()}).violations,0);
 for(const raw of [undefined,null,{},[],{...zero(),automated_only:false},{...zero(),incomplete:null}])assert.equal(app.accessibilityObservation({accessibility:raw}).violations,null);
});
test('late A accessibility CSV network failure cannot overwrite newly selected B message',async()=>{
 const app=harness();app.state.crawl={id:'A',generation:1};app.state.generation=7;
 const operation=app.serverDownload('accessibility.csv?generation=1','A.csv');assert.equal(app.pending.length,1);
 app.state.crawl={id:'B',generation:2};app.state.generation=8;app.get('message').textContent='B selected';
 app.pending[0].reject(Error('late A failure'));await operation;assert.equal(app.get('message').textContent,'B selected');
});
test('late A metadata refusal stays scoped while current export errors remain visible',async()=>{
 const app=harness();app.state.crawl={id:'A',generation:1};app.state.generation=7;
 const operation=app.serverDownload('accessibility.csv?generation=1','A.csv');app.state.crawl={id:'B',generation:2};app.state.generation=8;app.get('message').textContent='B selected';
 app.pending[0].resolve({ok:true,headers:{get(){return null;}}});await operation;assert.equal(app.get('message').textContent,'B selected');
 const current=app.serverDownload('accessibility.csv?generation=2','B.csv');app.pending[1].reject(Error('current B failure'));await current;assert.match(app.get('message').textContent,/current B failure/);
});
