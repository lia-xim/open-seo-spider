'use strict';
// Runs the actual tester script with deferred local API responses. Matcher and
// browser/backend/website-request proof are separate from this DOM state test.
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');
const source=fs.readFileSync(path.join(__dirname,'..','robots.js'),'utf8');
const flush=()=>new Promise(resolve=>setImmediate(resolve));
function deferred(){let resolve;const promise=new Promise(done=>{resolve=done;});return {promise,resolve};}
class Element{
  constructor(){this.value='';this.textContent='';this.children=[];this.handlers=new Map();this.attributes=new Map();this.dataset={};}
  addEventListener(name,handler){this.handlers.set(name,handler);}
  setAttribute(name,value){this.attributes.set(name,value);}
  append(child){this.children.push(child);}
  replaceChildren(){this.children=[];}
  fire(name,event={}){return this.handlers.get(name)?.(event);}
  set innerHTML(value){throw Error(`Untrusted HTML insertion: ${value}`);}
}
function harness(){
  const nodes=new Map(),requests=[],downloads=[],objects=new Map();let objectId=0;
  const get=selector=>{if(!nodes.has(selector))nodes.set(selector,new Element());return nodes.get(selector);};
  for(const [selector,value] of Object.entries({'#user-agent':'Bot','#robots-text':'User-agent: *\nDisallow: /private\n','#urls':'https://never-requested.invalid/private','#max-robots-bytes':'524288','#max-urls':'10000','#max-url-bytes':'8192','#max-input-bytes':'8388608','#max-evaluation-bytes':'67108864','#decision-filter':'all'}))get(selector).value=value;
  const context={TextEncoder,Blob,setTimeout:callback=>callback(),document:{querySelector:get,createElement(){const node=new Element();node.click=()=>downloads.push({name:node.download,blob:objects.get(node.href)});return node;}},URL:{createObjectURL(blob){const id=`blob:local-test-${++objectId}`;objects.set(id,blob);return id;},revokeObjectURL(id){objects.delete(id);}},fetch(route,options){assert.ok(['/api/robots/simulate','/api/robots/export.csv'].includes(route),`Unexpected website request: ${route}`);const pending=deferred();requests.push({route,input:JSON.parse(options.body),...pending});return pending.promise;}};
  vm.createContext(context);vm.runInContext(source,context,{filename:'web/robots.js'});
  const h={get,requests,downloads,input(selector,value){get(selector).value=value;get(selector).fire('input');},submit(){return get('#robots-form').fire('submit',{preventDefault(){}});},result(request,marker='current'){request.resolve({ok:true,json:async()=>({engine:marker,rules_sha256:'exact-test-hash',rules_bytes:request.input.robots_text.length,user_agent:request.input.user_agent,website_requests:0,ignored_directives:['sitemap'],limitations:['No matcher trace asserted'],rows:request.input.urls.map((input_url,index)=>({source_row:index+1,input_url,normalized_url:input_url,path_query:'/test',fragment:null,allowed:input_url==='invalid'?null:false,error:input_url==='invalid'?'Invalid URL':null}))})});},error(request,message){request.resolve({ok:false,status:400,json:async()=>({error:message})});}};
  return h;
}
test('empty non-ASCII and control-containing agent values are rejected before API',async()=>{
  for(const agent of ['', 'éBot','Bot\tOther','B'.repeat(257)]){const h=harness();h.input('#user-agent',agent);await h.submit();assert.equal(h.requests.length,0);assert.equal(h.get('#status').attributes.get('role'),'alert');}
});
test('all positive bounded numeric fields reject zero negative fractional missing and excessive values',async()=>{
  const fields=['#max-robots-bytes','#max-urls','#max-url-bytes','#max-input-bytes','#max-evaluation-bytes'];
  for(const selector of fields)for(const value of ['0','-1','1.5','','999999999']){const h=harness();h.input(selector,value);await h.submit();assert.equal(h.requests.length,0,`${selector}: ${value}`);assert.equal(h.get('#status').attributes.get('role'),'alert');}
});
test('limits count UTF-8 bytes field rows total input and parser work before API',async()=>{
  for(const [selector,value,setup] of [
    ['#max-robots-bytes','3',h=>h.input('#robots-text','éé')],
    ['#max-url-bytes','3',h=>h.input('#urls','éé')],
    ['#max-urls','1',h=>h.input('#urls','a\nb')],
    ['#max-input-bytes','1',()=>{}],['#max-evaluation-bytes','1',()=>{}],
  ]){const h=harness();h.input(selector,value);setup(h);await h.submit();assert.equal(h.requests.length,0,selector);}
});
test('exact URL order duplicates and interior empty rows survive local submission and JSON export',async()=>{
  const h=harness();const urls=['https://never-requested.invalid/private','','invalid','https://never-requested.invalid/private'];
  h.input('#urls',urls.join('\r\n')+'\r\n');const pending=h.submit();const request=h.requests[0];assert.deepEqual(request.input.urls,urls);h.result(request);await pending;
  h.get('#export-json').fire('click');const saved=JSON.parse(await h.downloads[0].blob.text());assert.deepEqual(saved.input.urls,urls);assert.deepEqual(saved.result.rows.map(row=>row.source_row),[1,2,3,4]);assert.equal(saved.result.rows[2].allowed,null);assert.equal(h.get('#results-table tbody').children.length,4);
});
test('reverse responses cannot replace newer submitted rules agent or results',async()=>{
  const h=harness(),first=h.submit();h.input('#user-agent','NewBot');h.input('#robots-text','User-agent: NewBot\nAllow: /\n');const second=h.submit();
  h.result(h.requests[1],'new');await second;const currentStatus=h.get('#status').textContent;h.result(h.requests[0],'old');await first;
  assert.match(h.get('#facts').textContent,/NewBot.*new/);assert.equal(h.get('#status').textContent,currentStatus);
});
test('an unsent edit invalidates old success error and CSV export responses',async()=>{
  const h=harness(),first=h.submit();h.input('#urls','https://never-requested.invalid/new');h.result(h.requests[0]);await first;assert.equal(h.get('#result-card').hidden,true);
  const second=h.submit();h.error(h.requests[1],'old failure');h.input('#urls','https://never-requested.invalid/newer');await second;assert.match(h.get('#status').textContent,/Eingabe geändert/);
  const ready=h.submit();h.result(h.requests[2]);await ready;const exporting=h.get('#export-csv').fire('click');h.input('#user-agent','LaterBot');h.requests[3].resolve({ok:true,text:async()=>'old export'});await exporting;
  assert.equal(h.downloads.length,0);assert.equal(h.get('#export-json').disabled,true);
});
test('preset selection invalidates prior results and every result cell uses safe text',async()=>{
  const h=harness();h.input('#urls','<img src=x onerror=alert(1)>');const pending=h.submit();h.result(h.requests[0]);await pending;
  assert.equal(h.get('#results-table tbody').children[0].children[1].textContent,'<img src=x onerror=alert(1)>');
  h.get('#agent-preset').value='Googlebot';h.get('#agent-preset').fire('change');assert.equal(h.get('#user-agent').value,'Googlebot');assert.equal(h.get('#result-card').hidden,true);
});
