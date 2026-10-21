'use strict';
// Actual production App closure; only automatic startup is replaced. Declared
// states below are regression controls, not native capture or browser evidence.
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const source=fs.readFileSync("E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\mcp-file-tools-v1\\artifacts\\extraction-source-trust-next-slice-20261021\\pre-fix-app-aef8cd.js",'utf8'),ending=/  initialize\(\);\s*\}\)\(\);\s*$/;
assert(ending.test(source),'Expected one final App startup');
function harness(pages=[],config={custom_extractions:[{name:'Owned missing',selector:'.missing',method:'css'}]}){
 const context={console,Intl,URL,Blob,TextEncoder,TextDecoder,Uint8Array,Set,Map,setTimeout,clearTimeout,document:{getElementById(){throw Error('No DOM call authorized in pure state test');}},fetch(){throw Error('Pure extraction regression forbids HTTP');}};
 vm.runInNewContext(source.replace(ending,`  globalThis.actual={state,getRows,columns,matchesFilter,normalPage,tableSourceAvailability,sourceState:typeof extractionSourceState==='function'?extractionSourceState:null};\n})();`),context);
 const app=context.actual;app.state.section='extractions';app.state.crawl={id:'declared-owner',status:'completed',generation:7,config};app.state.generation=1;app.state.pages=pages.map(page=>app.normalPage(page));app.state.pageMap=new Map(app.state.pages.map(page=>[page.url,page]));return app;
}
const clone=value=>JSON.parse(JSON.stringify(value)),baseURL='https://owned.invalid/source';
function extraction(changes={}){return{name:'Owned missing',selector:'.missing',attribute:null,method:'css',output:'auto',engine:'scraper-html5-css',result_type:'text',values:[],matched_count:0,truncated:false,error:null,source:'raw_html',...changes};}
function observation(changes={}){return{engine:'open-seo-spider/extraction-source-v1',source:'raw_html',status:'complete',evaluation:'evaluated',decode_status:'exact',document_url:baseURL,observed_generation:7,observed_at:'2026-10-21T10:00:00Z',input_sha256:'a'.repeat(64),input_utf8_bytes:28,source_revision:'b'.repeat(64),reasons:[],...changes};}
function page(result,changes={}){return{url:baseURL,final_url:baseURL,final_status:200,internal:true,truncated:false,error:null,renderer_error:null,custom_extractions:[result],...changes};}
function row(app){assert.equal(app.getRows().length,1);return app.getRows()[0];}
function state(app,value=row(app)){assert.equal(typeof app.sourceState,'function','App must expose actual extractionSourceState');return clone(app.sourceState(value));}
function includes(app,filter,value=row(app)){app.state.filter=filter;return app.matchesFilter(value);}
test('App has one source classifier and preserves complete-source zero independently from result limit',()=>{
 const app=harness([page(extraction({source_observation:observation()}))]),value=state(app);assert.equal(value.status,'complete');assert.equal(value.evaluation,'evaluated');assert.equal(value.decode,'exact');assert.equal(value.exactCount,0);assert.equal(value.scope,'complete_source');assert.equal(includes(app,'empty'),true);
});
for(const [label,record]of[
 ['legacy',extraction()],
 ['truncated source',extraction({source_observation:observation({status:'incomplete',reasons:['native_response_truncated']})})],
 ['lossy source',extraction({source_observation:observation({status:'incomplete',decode_status:'lossy',reasons:['source_decode_lossy']})})],
 ['unknown charset',extraction({source_observation:observation({status:'incomplete',decode_status:'unknown',reasons:['source_decode_unknown']})})],
 ['evaluation error',extraction({matched_count:null,error:'Declared evaluation failure',source_observation:observation({evaluation:'error'})})]
])test(label+' never enters confirmed-empty or acquires an exact count',()=>{
 const app=harness([page(record,{truncated:label==='truncated source'})]);assert.equal(includes(app,'empty'),false);assert.equal(state(app).exactCount,null);
});
test('complete input with result-budget truncation keeps exact count3 and retained values2',()=>{
 const app=harness([page(extraction({values:['α','β'],matched_count:3,truncated:true,source_observation:observation()}))]);assert.equal(state(app).exactCount,3);assert.equal(includes(app,'truncated'),true);assert.equal(includes(app,'incomplete'),false);assert.equal(includes(app,'present'),true);assert.equal(includes(app,'empty'),false);
});
test('partial input values are evaluated-input-only, not a complete count or generic lower bound',()=>{
 const app=harness([page(extraction({values:['prefix:last-child'],matched_count:1,source_observation:observation({status:'incomplete',reasons:['native_response_truncated']})}),{truncated:true})]);assert.equal(state(app).exactCount,null);assert.equal(state(app).scope,'evaluated_input_only');assert.equal(includes(app,'incomplete'),true);assert.equal(includes(app,'present'),true);assert.equal(includes(app,'truncated'),false);
});
for(const [value,type]of[['0','number'],['false','boolean'],['','string']])test('XPath scalar '+JSON.stringify(value)+' is one retained value, distinct from empty nodeset',()=>{
 const result=extraction({method:'xpath',selector:'count(//p)',output:'function_value',engine:'sxd-xpath-1.0-html-dom',result_type:type,values:[value],matched_count:1,source_observation:observation()}),app=harness([page(result)],{custom_extractions:[{name:result.name,selector:result.selector,method:result.method,output:result.output}]});assert.equal(state(app).exactCount,1);assert.equal(includes(app,'present'),true);assert.equal(includes(app,'empty'),false);assert.deepEqual(clone(row(app).values),[value]);
});
test('optional renderer failure does not revoke complete raw-source evidence',()=>{
 const app=harness([page(extraction({source_observation:observation()}),{renderer_error:'Owned worker navigation failed'})]);assert.equal(state(app).status,'complete');assert.equal(state(app).exactCount,0);assert.equal(row(app).source,'raw_html');
});
test('unavailable not-run source is visible and never becomes an evaluated zero',()=>{
 const app=harness([page(extraction({matched_count:null,source_observation:observation({status:'unavailable',evaluation:'not_run',decode_status:'unknown',document_url:null,input_sha256:null,input_utf8_bytes:null,reasons:['native_non_html']})}))]);assert.equal(state(app).status,'unavailable');assert.equal(state(app).scope,'not_evaluated');assert.equal(includes(app,'unavailable'),true);assert.equal(includes(app,'empty'),false);assert.equal(state(app).exactCount,null);
});
for(const [label,change]of[
 ['wrong contract engine',{engine:'other-engine'}],['wrong source binding',{source:'rendered_html'}],['malformed hash',{input_sha256:'bad'}],['invalid byte count',{input_utf8_bytes:-1}],['unknown status',{status:'invented'}],['nonarray reasons',{reasons:{}}]
])test('malformed '+label+' remains unknown with null exact count',()=>{
 const app=harness([page(extraction({source_observation:observation(change)}))]);assert.equal(state(app).status,'unknown');assert.equal(state(app).exactCount,null);assert.equal(includes(app,'unknown'),true);assert.equal(includes(app,'empty'),false);assert(state(app).reasons.includes('invalid_source_observation'));
});
test('missing configured result remains an explicit unknown row without synthesized matches',()=>{
 const config={custom_extractions:[{name:'A',selector:'.a',method:'css'},{name:'B',selector:'.b',method:'css'}]},app=harness([page(extraction({name:'A',selector:'.a',source_observation:observation()}))],config),rows=app.getRows();assert.equal(rows.length,2);const missing=rows.find(value=>value.name==='B');assert(missing);assert.equal(state(app,missing).status,'unknown');assert.equal(state(app,missing).exactCount,null);assert.equal(missing.matched_count??null,null);assert.deepEqual(clone(missing.values),[]);assert.equal(includes(app,'empty',missing),false);
});
test('unconfigured no-observation extraction source refuses header-only export',()=>{
 const app=harness([page(extraction(),{custom_extractions:[]})],{custom_extractions:[]});assert.equal(app.getRows().length,0);assert.equal(app.tableSourceAvailability().ok,false);assert.equal(app.tableSourceAvailability().error.code,'table_source_unavailable');
});
test('historical present observations remain readable even when configuration is absent',()=>{
 const app=harness([page(extraction({values:['retained legacy value'],matched_count:1}))],{});assert.equal(app.getRows().length,1);assert.equal(app.tableSourceAvailability().ok,true);assert.equal(state(app).status,'unknown');assert.equal(includes(app,'present'),true);
});
test('actual columns carry typed trust fields and keep zero/empty/null scalars separate',()=>{
 const app=harness([page(extraction({source_observation:observation()}))]),columns=app.columns(),keys=columns.map(value=>value.key);for(const key of['source_status','evaluation_status','decode_status','exact_count','source_reasons','input_sha256','input_utf8_bytes','source_revision','document_url','observed_generation','observed_at','source_engine','count_scope'])assert(keys.includes(key),key);
 const value=row(app),get=key=>{const c=columns.find(value=>value.key===key);return(c.exportValue||c.rawValue||c.value)(value);};assert.equal(get('exact_count'),0);assert.equal(get('input_utf8_bytes'),28);assert.equal(get('observed_generation'),7);assert.equal(get('source_status'),'complete');assert.equal(get('count_scope'),'complete_source');
 const before=JSON.stringify(app.state.pages);for(const c of columns){c.value(value);if(c.exportValue)c.exportValue(value);}state(app);assert.equal(JSON.stringify(app.state.pages),before,'Classification/projection must not mutate saved source');
});
