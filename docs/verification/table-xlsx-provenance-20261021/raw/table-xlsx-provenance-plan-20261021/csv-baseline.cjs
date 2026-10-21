'use strict';
// Read-only Git/source comparison using extracted actual App functions and fake sinks.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{execFileSync}=require('node:child_process'),{createHash}=require('node:crypto');
const root=path.resolve(__dirname,'../..'),hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const baseline=execFileSync('git',['show','22fc522:web/app.js'],{cwd:root}),current=fs.readFileSync(path.join(root,'web/app.js'));
assert.equal(hash(baseline),'b985eba3a91774ca7f8c738f4dce82fe69d89e580182f464bf6deb081b64861c');
fs.writeFileSync(path.join(__dirname,'app-before-22fc522.js'),baseline);
const testFile=path.join(root,'web/verification/table-source-availability.test.cjs'),testSource=fs.readFileSync(testFile,'utf8');
function loadHarness(source){
  const localRequire=id=>id==='node:test'?()=>{}:id==='node:fs'?{...fs,readFileSync(file,...args){return path.resolve(file)===path.join(root,'web/app.js')?source.toString('utf8'):fs.readFileSync(file,...args);}}:require(id);
  const module={exports:{}};new Function('require','module','__dirname',testSource+'\nmodule.exports={harness,nativeImports};')(localRequire,module,path.dirname(testFile));return module.exports;
}
const before=loadHarness(baseline),after=loadHarness(current),comparisons=[];
(async()=>{
  for(const section of ['inputs','linkmetrics','structured','robots'])for(const metadata of [false,true])for(const narrow of [false,true])for(const emptyQuery of [false,true]){
    const pages=[{url:'https://owned.invalid/a',indexable:true,status:200,meta_robots:[],x_robots_tag:[],indexability_reasons:[],fetched_at:'2026-10-21T12:00:00Z',_segmentOrdinal:0},{url:'https://owned.invalid/b',indexable:false,status:200,meta_robots:['noindex'],x_robots_tag:[],indexability_reasons:['noindex'],fetched_at:'2026-10-20T12:00:00Z',_segmentOrdinal:7}];
    const changes={pages,analysis:{graph:{nodes:pages.map((page,i)=>({...page,link_score:i,in_degree:i,out_degree:1,inlink_occurrences:2,outlink_occurrences:1,crawl_depth:i,shortest_link_depth:i}))},structured_data:pages.map((page,i)=>({url:page.url,parsed_blocks:i,invalid_json_blocks:0,types:i?{Article:1}:{},typed_nodes:[]}))},crawl:{id:'declared-owner',status:'completed',config:{input_imports:[after.nativeImports.csv]}}};
    if(metadata){changes.segmentView={context:{snapshot_token:'declared-token',rules_sha256:'a'.repeat(64),typed_report_sha256:'b'.repeat(64),app_generation:3,reporting_view:{mode:'raw'}},selection:{kind:'all'}};changes.crawl.reporting_visibility={captured_pages:2,visible_pages:1,hidden_pages:1};changes.crawl.operation={coverage:'mixed_age_selected_recrawl',parent:{snapshot_sha256:'c'.repeat(64)},targets:[{requested_url:pages[0].url,state:'committed'}]};}
    const old=before.harness(section,JSON.parse(JSON.stringify(changes))),now=after.harness(section,JSON.parse(JSON.stringify(changes)));
    for(const h of [old,now]){if(narrow)h.run('visibleKeys=tableSchema().filter((c,i)=>c.mandatory||i===1).map(c=>c.key);');if(emptyQuery)h.run("state.search='sentinel with no matches';");h.run('exportCSV()');assert.equal(h.downloads.length,1);}
    const a=Buffer.from(await old.downloads[0].blob.arrayBuffer()),b=Buffer.from(await now.downloads[0].blob.arrayBuffer());assert.deepEqual(b,a);assert.equal(now.downloads[0].filename,old.downloads[0].filename);
    comparisons.push({section,metadata,narrow,emptyQuery,bytes:b.length,sha256:hash(b)});
  }
  const result={result:'PASS_PURE_EXTRACTED_APP',baseline:{revision:'22fc522',bytes:baseline.length,sha256:hash(baseline)},current:{bytes:current.length,sha256:hash(current)},test:{bytes:Buffer.byteLength(testSource),sha256:hash(Buffer.from(testSource))},comparisons:comparisons.length,details:comparisons,boundary:'Declared scalar/source fixtures, real App functions and query compiler, fake DOM/Blob sinks. No native/browser/transport claim. This is byte-exact CSV comparison, including BOM, labels, formula protection, projection and metadata, for four changed families only.'};
  fs.writeFileSync(path.join(__dirname,'csv-baseline-proof.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({...result,details:undefined}));
})().catch(error=>{console.error(error);process.exitCode=1;});
