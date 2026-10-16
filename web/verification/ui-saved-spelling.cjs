'use strict';
// Source-only VM fixture definition. No browser, engine or product execution by author.
const fs=require('node:fs'),assert=require('node:assert/strict'),crypto=require('node:crypto'),vm=require('node:vm');
const targets=[{"path":"E:/CodexWork/open-seo-spider-01a12ac3-20261013/source28-spelling-root-composition-v3/source/web/spelling.js","bytes":14485,"sha256":"e7c125c42d42a4f8727a45182eaae7425a7a25a70ca7ecd95000d8d0bbb258ff"},{"path":"E:/CodexWork/open-seo-spider-01a12ac3-20261013/source28-spelling-root-composition-v3/source/web/spelling.html","bytes":5657,"sha256":"f0af645d582e5e20d8e40c6794ca3cb6add0bd80ad6a6879ee8163c5f0cc0255"}];assert.equal(process.argv.length,4,'Explicit current spelling.js and spelling.html required');
for(let i=0;i<2;i++){assert.equal(process.argv[i+2].replace(/\\/g,'/'),targets[i].path);const b=fs.readFileSync(process.argv[i+2]);assert.equal(b.length,targets[i].bytes);assert.equal(crypto.createHash('sha256').update(b).digest('hex'),targets[i].sha256);}
const source=fs.readFileSync(process.argv[2],'utf8'),html=fs.readFileSync(process.argv[3],'utf8');
const a=source.indexOf('// SOURCE28_PURE_BEGIN'),b=source.indexOf('// SOURCE28_PURE_END');assert(a>=0&&b>a);
const pure=vm.runInNewContext(source.slice(a,b)+';({spellingWords,spellingDictionaryText,spellingValidateReply})',{});
const request={version:1,selection:{urls:['https://saved.invalid/one'],source_kind:'raw_http_html',reporting_mode:'reported',expected_generation:7,expected_snapshot_sha256:'a'.repeat(64),expected_reporting_identity:'b'.repeat(64)},config:{version:1,enabled:false},expected_dictionary_sha256:'c'.repeat(64)};
function reply(){return{crawl_id:'job',report:{engine:'open-seo-spider/saved-spelling-v1',crawl_id:'job',website_requests:0,offset_unit:'utf16_code_units',request:structuredClone(request),saved_capture:{requested_urls:['https://saved.invalid/one'],source_kind:'raw_http_html',snapshot:{sha256:'a'.repeat(64),generation:7},reporting:{identity_sha256:'b'.repeat(64),identity:{mode:'reported'}}},dictionary_sha256:'c'.repeat(64),documents:[],engine_requests:0,complete:false,accepted_count:null}};}
const rows=[];function group(name,variants,run){run();rows.push({name,variants});}
group('word_editor_preserves_case_and_unicode',3,()=>{const words=pure.spellingWords('en | CrawlFoundry\nde-DE | Eigenwort\nen | e\u0301');assert.equal(words.length,3);assert.equal(words[0].word,'CrawlFoundry');assert.equal(words[2].word,'e\u0301');});
group('word_editor_refuses_ambiguous_lines',4,()=>{for(const text of ['word','en |','| word','en | word | extra'])assert.throws(()=>pure.spellingWords(text));});
group('dictionary_editor_roundtrip_is_explicit',2,()=>{const words=pure.spellingWords('de | Eigenwort');assert.equal(pure.spellingDictionaryText(words),'de | Eigenwort');assert.equal(pure.spellingWords(pure.spellingDictionaryText(words))[0].language,'de');});
group('exact_reply_accepts_key_reordering',2,()=>{assert.equal(pure.spellingValidateReply(reply(),request,'job').accepted_count,null);const v=reply();v.report.request={config:v.report.request.config,selection:v.report.request.selection,version:1,expected_dictionary_sha256:'c'.repeat(64)};assert(pure.spellingValidateReply(v,request,'job'));});
group('requested_mode_and_identity_cannot_be_dropped_or_substituted',4,()=>{for(const mutate of [v=>delete v.report.request.selection.reporting_mode,v=>v.report.request.selection.reporting_mode='raw',v=>delete v.report.saved_capture.reporting,v=>v.report.saved_capture.reporting.identity_sha256='d'.repeat(64)]){const v=reply();mutate(v);assert.throws(()=>pure.spellingValidateReply(v,request,'job'));}});
group('generation_snapshot_dictionary_and_job_are_bound',4,()=>{for(const mutate of [v=>v.report.saved_capture.snapshot.generation=8,v=>v.report.saved_capture.snapshot.sha256='d'.repeat(64),v=>v.report.dictionary_sha256='d'.repeat(64),v=>v.crawl_id='other']){const v=reply();mutate(v);assert.throws(()=>pure.spellingValidateReply(v,request,'job'));}});
group('incomplete_result_cannot_claim_zero',2,()=>{const v=reply();v.report.accepted_count=0;assert.throws(()=>pure.spellingValidateReply(v,request,'job'));v.report.accepted_count=null;assert(pure.spellingValidateReply(v,request,'job'));});
function dom(){
  class Element{constructor(){this.value='';this.checked=false;this.disabled=false;this.hidden=false;this.textContent='';this.listeners={};this.children=[];}addEventListener(type,fn){(this.listeners[type]??=[]).push(fn);}replaceChildren(...v){this.children=v;}append(...v){this.children.push(...v);}click(){for(const f of this.listeners.click||[])f({target:this});}}
  const elements=new Map();const get=id=>{if(!elements.has(id))elements.set(id,new Element());return elements.get(id);};
  get('crawl').value='old-job';get('source').value='raw_http_html';get('mode').value='';get('manual').value='draft-language';
  const pending=[];const fetch=(url,options)=>new Promise(resolve=>pending.push({url,options,resolve}));
  const document={getElementById:get,createElement:()=>new Element()};
  vm.runInNewContext(source,{document,fetch,AbortController,AbortSignal,URL,URLSearchParams,TextDecoder,Uint8Array,Promise,setTimeout,crypto:{subtle:{}},Blob,Option:function(t,v){this.text=t;this.value=v;},console});
  return{get,pending};
}
const turn=()=>new Promise(resolve=>setImmediate(resolve));
function jsonResponse(value){let done=false;const bytes=new TextEncoder().encode(JSON.stringify(value));return{ok:true,status:200,body:{getReader:()=>({read:async()=>done?{done:true}:(done=true,{done:false,value:bytes}),cancel:async()=>{}})}};}
(async()=>{
  const d=dom();d.get('inventory').click();await turn();assert.equal(d.pending.length,1);const first=d.pending[0];
  d.get('crawl').value='new-job';for(const fn of d.get('crawl').listeners.change)fn();assert(first.options.signal.aborted);
  first.resolve(jsonResponse({snapshot:{sha256:'a'.repeat(64)},rows:[{requested_url:'old-url'}],total_saved_pages:1,has_more:false}));await turn();await turn();
  assert.equal(d.get('captures').children.length,0);assert.notEqual(d.get('status').textContent,'Inventar geladen.');
  rows.push({name:'late_inventory_reply_cannot_repopulate_changed_crawl',variants:3});
  const p=dom();p.get('load-profile').click();await turn();p.get('crawl').value='new-job';for(const fn of p.get('crawl').listeners.change)fn();
  p.pending[0].resolve(jsonResponse({profile:{config:{manual_language:'stale-profile'}},sha256:'x'}));await turn();await turn();
  assert.equal(p.get('manual').value,'draft-language');assert.equal(p.get('save-profile').disabled,true);assert(p.pending[0].options.signal.aborted);
  rows.push({name:'late_profile_load_does_not_overwrite_draft_on_selection_change',variants:3});
  assert(html.includes('Nur für diesen Crawl')&&html.includes('Anwendungswörterbuch'));
  console.log(JSON.stringify({kind:'source28_saved_spelling_ui_VM_fixture_result',groups:rows.length,variants:rows.reduce((n,r)=>n+r.variants,0),rows,actual_browser:false,actual_engine:false}));
})().catch(error=>{console.error(error);process.exitCode=1;});