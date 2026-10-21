'use strict';
// Offline actual-App state controls. No browser, native capture, writer or fetch.
// Declared prefix/lossy/legacy records are negative controls, not website evidence.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'../..'),file=path.join(root,'web/app.js'),source=fs.readFileSync(file,'utf8');
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function part(start,end){const a=source.indexOf(start),b=source.indexOf(end,a);assert(a>=0&&b>a);return source.slice(a,b);}
const actual=part('  function getRows()','  function filterOptions()')+'\n'+part('  function matchesFilter(row)','  function tableOwner()');
const records=[
 {name:'complete-source-zero-control',truncated:false,error:null,sourceNotes:'Declared complete source only; current result has no actual per-result completion marker.'},
 {name:'body-prefix-zero-control',truncated:true,error:null,sourceNotes:'Declared body limit; current result count0 is only evaluated-prefix count.'},
 {name:'partial-body-read-zero-control',truncated:false,error:'Response body read failed: declared negative transport control',sourceNotes:'Declared body-read failure after prefix.'},
 {name:'lossy-source-zero-control',truncated:false,error:null,warnings:['Response text contains invalid bytes for the declared character encoding'],sourceNotes:'Declared lossy decode; no per-extraction decode state.'},
 {name:'legacy-source-zero-control',truncated:false,error:null,sourceNotes:'Legacy result with matched_count0 but no source-observation member.'}
].map(({name,...page})=>({...page,url:'https://owned.invalid/'+name,custom_extractions:[{name:'Missing selector',selector:'.missing',attribute:null,values:[],source:'raw_html',method:'css',output:'auto',engine:'scraper-html5-css',result_type:'text',matched_count:0,truncated:false,error:null}]}));
const context=vm.createContext({console,URL,Intl,TextEncoder,fetch(){throw Error('Offline probe forbids all fetches');}});
context.input=records;
const domDeclaration=source.split(/\r?\n/).find(line=>line.trim().startsWith('const domSections ='));assert(domDeclaration);
vm.runInContext(`const state={section:'extractions',pages:input,filter:'all'};const array=v=>Array.isArray(v)?v:v==null?[]:[v],internals=()=>state.pages,escape=v=>String(v??''),number=v=>String(v??0),effectiveStatus=p=>p.final_status??p.status??null;`+domDeclaration+'\n'+actual+'\nglobalThis.probe={getRows,columns,matchesFilter,state};',context);
const app=context.probe,rows=app.getRows(),cols=app.columns();assert.equal(rows.length,5);
const result=rows.map(row=>{app.state.filter='empty';const entersEmpty=app.matchesFilter(row);app.state.filter='truncated';const entersResultTruncation=app.matchesFilter(row);return{url:row.url,page_truncated:row._page.truncated,page_error:row._page.error,page_warning:row._page.warnings??[],enters_empty_filter:entersEmpty,enters_result_truncation_filter:entersResultTruncation,projection:Object.fromEntries(cols.map(c=>[c.key,(c.exportValue||c.rawValue||c.value)(row)??null]))};});
assert(result.every(v=>v.enters_empty_filter===true));assert(result.every(v=>v.enters_result_truncation_filter===false));
const withoutURL=result.map(v=>Object.fromEntries(Object.entries(v.projection).filter(([k])=>k!=='url')));for(const item of withoutURL)assert.deepEqual(item,withoutURL[0]);
const proof={result:'REPRODUCED_OFFLINE_STATE_AMBIGUITY',scope:'Actual unchanged App getRows/columns/matchesFilter on five declared negative/control states. No native/browser/Cargo/worker/network proof.',app_sha256:sha(fs.readFileSync(file)),app_bytes:fs.statSync(file).size,rows:result,assertions:['all five enter empty filter','none enters result truncation filter','all exported source/count/result scalars are identical except URL','page truncation/error/decode warning are omitted from main projection'],native_runs:0,browser_runs:0};
fs.writeFileSync(path.join(__dirname,'offline-state-proof.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify({result:proof.result,app_sha256:proof.app_sha256,controls:result.length,native_runs:0,browser_runs:0}));
