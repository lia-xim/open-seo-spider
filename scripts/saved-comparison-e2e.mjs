import {assertDefaultMcpInventory} from './mcp-inventory-contract.mjs';
// Owned native crawl -> saved reports -> API/CLI/official MCP -> offline restart.
import http from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp, readFile, writeFile, mkdir, rm} from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {Client, StreamableHTTPClientTransport} from '@modelcontextprotocol/client';
import {StdioClientTransport} from '@modelcontextprotocol/client/stdio';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||path.join(root,'target/debug',`open-seo-spider${process.platform==='win32'?'.exe':''}`);
const binarySha=createHash('sha256').update(await readFile(binary)).digest('hex');
const data=await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-saved-compare-'));
const proofPath=process.env.SPIDER_COMPARISON_PROOF||path.join(root,'docs/verification/saved-comparison-e2e.json');
const engine='open-seo-spider/saved-comparison-v1';
const hits=[], checks=[];
let revision=0, api, child, output='', launchError, offlineTrap, offlineAttemptCount=0;
const fixture=http.createServer((req,res)=>{
  hits.push(req.url);
  res.setHeader('content-type','text/html; charset=utf-8');
  const changed=req.url==='/changed';
  const version=changed?revision:0;
  res.end(`<html lang="en"><head><title>${changed?'Changed '+version:'Stable '+req.url}</title><meta name="description" content="Saved description for ${req.url} version ${version}"><meta name="robots" content="index,follow${version?',max-snippet:120':''}"><script type="application/ld+json">${JSON.stringify({'@type':'Thing',name:changed?'Version '+version:'Stable'})}</script></head><body><main><h1>Heading ${req.url} ${version}</h1><h2>Subheading ${req.url} ${version}</h2>Actual saved source for ${req.url}. Version ${version}. <a href="/${changed?'target-'+version:'constant'}">Saved link</a></main></body></html>`);
});
await new Promise((resolve,reject)=>{fixture.once('error',reject);fixture.listen(0,'127.0.0.1',resolve);});
const origin=`http://127.0.0.1:${fixture.address().port}`;
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function alive(){if(launchError||!child||child.exitCode!==null||child.signalCode!==null)throw launchError||Error('Owned comparison App exited: '+output.slice(-3000));}
async function until(fn,label){for(let i=0;i<600;i++){alive();const result=await fn();if(result)return result;await delay(25);}throw Error('Timed out: '+label+'\n'+output.slice(-3000));}
async function launch(){
  output='';launchError=null;
  child=spawn(binary,['serve','--listen','127.0.0.1:0','--data-dir',data],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
  child.on('error',error=>launchError=error);
  child.stdout.on('data',chunk=>output+=chunk);
  child.stderr.on('data',chunk=>output+=chunk);
  api=await until(()=>output.match(/(?:^|\n)Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/)?.[1],'owned complete bind line');
  await until(async()=>{try{return (await fetch(api+'/healthz')).ok;}catch{return false;}},'owned health');
}
async function stop(){const owned=child;if(!owned)return;child=null;if(owned.exitCode===null&&owned.signalCode===null){const closed=new Promise(resolve=>owned.once('close',resolve));owned.kill();await closed;}}
async function request(route,body){alive();const response=await fetch(api+route,body===undefined?{}:{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});const text=await response.text();alive();return {status:response.status,body:JSON.parse(text)};}
async function call(route,body){const response=await request(route,body);assert.equal(response.status,200,JSON.stringify(response));return response.body;}
async function crawl(urls,extra={}){const started=await call('/api/crawls',{seed_urls:urls.map(p=>origin+p),crawl_links:false,discover_sitemaps:false,obey_robots:false,delay_ms:0,concurrency:2,content_capture:{sources:['raw_http_html','rendered_dom_html'],include_selectors:['main']},...extra});const status=await until(async()=>{const job=await call('/api/crawls/'+started.id);if(job.status==='failed')throw Error(job.error);return ['completed','limited'].includes(job.status)&&job;},'crawl completed');return {id:started.id,generation:status.generation,report:await call(`/api/crawls/${started.id}/project.json`)};}
const compare=(before,after,config={})=>call('/api/compare/saved',{before,after,config});
async function execute(args){return await new Promise((resolve,reject)=>{const process=spawn(binary,args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';process.on('error',reject);process.stdout.on('data',chunk=>stdout+=chunk);process.stderr.on('data',chunk=>stderr+=chunk);process.on('close',code=>resolve({code,stdout,stderr}));});}
const row=(result,suffix)=>result.comparison.pages.find(p=>p.comparison_key.endsWith(suffix));
const field=(page,name)=>page.fields.find(f=>f.field===name);
async function imported(report){return (await call('/api/crawls/import',report)).id;}
let before,after,result,offlineHits;
try {
  await launch();
  before=await crawl(['/changed','/same','/removed']);
  revision=1;
  after=await crawl(['/changed','/same','/added']);
  const baseHits=hits.length;
  const config={include_selected_text:true};
  result=await compare(before.id,after.id,config);
  assert.equal(result.website_requests,0);assert.equal(result.comparison.website_requests,0);assert.equal(hits.length,baseHits);
  assert.equal(result.comparison.engine,engine);
  for(const [key,value]of Object.entries({added:1,removed:1,changed:1,unchanged:1}))assert.equal(result.comparison.counts[key],value,key);
  assert.equal(row(result,'/same').kind,'unchanged');assert.equal(row(result,'/removed').kind,'removed');assert.equal(row(result,'/added').kind,'added');
  checks.push('Two actual native crawls jointly produce added/removed/changed/unchanged with zero comparison website requests');
  const changed=row(result,'/changed');
  assert.deepEqual(field(changed,'titles').before.value,['Changed 0']);assert.deepEqual(field(changed,'titles').after.value,['Changed 1']);
  for(const key of ['meta_descriptions','h1','h2','meta_robots','links','structured_data','text_sources.raw_http_html.selected_text'])assert.equal(field(changed,key).outcome,'changed',key);
  assert.equal(before.report.issues.length,after.report.issues.length,'changed facts must remain detectable when issue counts are equal');
  assert.equal(field(changed,'text_sources.rendered_dom_html.selected_text').before.state,'unavailable');
  assert.equal(field(changed,'text_sources.rendered_dom_html.selected_text').outcome,'unknown');
  assert.equal(result.comparison.before_snapshot.caller_binding_attested_by_core,false);
  checks.push('Actual title/description/H1/H2/directive/link/JSON-LD/selected raw text deltas retain concrete values despite equal issue counts; unavailable rendered source is unknown, cached App binding stays declared');

  const beforePath=path.join(data,'before-report.json'),afterPath=path.join(data,'after-report.json'),configPath=path.join(data,'comparison-config.json');
  await writeFile(beforePath,JSON.stringify(before.report));await writeFile(afterPath,JSON.stringify(after.report));await writeFile(configPath,JSON.stringify(config));
  const jsonPath=path.join(data,'comparison.json'),csvPath=path.join(data,'comparison.csv');
  const cli=await execute(['compare-saved',beforePath,afterPath,'--config',configPath,'--output',jsonPath,'--csv',csvPath]);assert.equal(cli.code,0,cli.stderr);
  const cliResult=JSON.parse(await readFile(jsonPath,'utf8'));
  assert.deepEqual(cliResult.comparison.pages,result.comparison.pages);assert.deepEqual(cliResult.comparison.counts,result.comparison.counts);
  for(const side of ['before_snapshot','after_snapshot'])assert.equal(cliResult.comparison[side].typed_report_sha256,result.comparison[side].typed_report_sha256);
  const csv=await readFile(csvPath,'utf8');assert.ok(csv.startsWith('"page_kind","comparison_key"'));assert.ok(csv.includes('Changed 0'));assert.ok(csv.includes('Changed 1'));assert.ok(csv.includes('"unknown"'));
  const originalBytes=await readFile(jsonPath);
  const refused=await execute(['compare-saved',beforePath,afterPath,'--output',jsonPath]);assert.equal(refused.code,2);assert.equal(JSON.parse(refused.stderr).error.code,'output_destination_exists');assert.deepEqual(await readFile(jsonPath),originalBytes);assert.equal(hits.length,baseHits);
  checks.push('CLI JSON/CSV retains identical deltas and typed snapshot hashes; existing destinations refuse with exit2 and original bytes preserved');

  for(const kind of ['stdio','streamable_http']){
    const client=new Client({name:'saved-comparison-acceptance',version:'0.1.0'});
    const transport=kind==='stdio'?new StdioClientTransport({command:binary,args:['mcp','--server',api],cwd:root,stderr:'pipe'}):new StreamableHTTPClientTransport(new URL(api+'/mcp'));
    try{
      alive();await client.connect(transport);alive();
      const tools=(await client.listTools()).tools;assertDefaultMcpInventory(tools);assert.ok(tools.some(t=>t.name==='spider_compare_saved'&&t.annotations.readOnlyHint===true&&t.annotations.openWorldHint===false));
      const saved=await client.callTool({name:'spider_compare_saved',arguments:{before:before.id,after:after.id,config}});assert.equal(saved.isError,false,JSON.stringify(saved));assert.deepEqual(saved.structuredContent,result);
      const conflict=await client.callTool({name:'spider_compare_saved',arguments:{before:before.id,after:after.id,expected_after_generation:0}});assert.equal(conflict.isError,true);assert.equal(conflict.structuredContent.error.code,'generation_conflict');assert.equal(conflict.structuredContent.context.observed_generation,after.generation);assert.equal(conflict.structuredContent.website_requests,0);alive();assert.equal(hits.length,baseHits);
    }finally{await client.close();}
    checks.push(`${kind}: official SDK23 exposes read-only saved comparison with exact API result and structured generation-conflict error`);
  }

  const shifted=structuredClone(before.report);
  for(const page of shifted.pages){page.fetched_at='2040-01-01T00:00:00Z';for(const capture of page.content_captures||[])capture.fetched_at=page.fetched_at;}
  shifted.summary.started_at='2040-01-01T00:00:00Z';shifted.summary.finished_at='2040-01-01T00:00:01Z';
  const invalidShiftedId=await imported(shifted);
  const invalidShifted=await request('/api/compare/saved',{before:before.id,after:invalidShiftedId,config});
  assert.equal(invalidShifted.status,400);assert.equal(invalidShifted.body.error.code,'invalid_saved_text_binding');
  revision=0;
  const laterSame=await crawl(['/changed','/same','/removed']);
  assert.notEqual(laterSame.report.pages[0].fetched_at,before.report.pages[0].fetched_at);
  const timeOnly=await compare(before.id,laterSame.id,config);assert.equal(timeOnly.comparison.counts.changed,0);assert.equal(timeOnly.comparison.counts.unchanged,3);
  const shiftedId=await imported(before.report);
  const importedSame=await compare(before.id,shiftedId,config);assert.equal(importedSame.comparison.counts.changed,0);assert.equal(importedSame.comparison.after_snapshot.caller_binding.generation,null);
  checks.push('Actual later identical crawl ignores timestamps; imports invent no generation; tampered capture timestamp/revision binding rejects explicitly');

  const legacy=structuredClone(before.report);
  for(const page of legacy.pages){delete page.content_captures;delete page.dom_metadata;delete page.raw_metadata;delete page.native_responses;}
  const legacyId=await imported(legacy),legacyResult=await compare(legacyId,before.id,config);
  assert.equal(field(row(legacyResult,'/changed'),'text_sources.raw_http_html.selected_text').before.state,'missing');
  assert.equal(field(row(legacyResult,'/changed'),'text_sources.raw_http_html.selected_text').comparable_complete_values,false);
  const partial=await crawl(['/changed'],{content_capture:{sources:['raw_http_html'],include_selectors:['main'],limits:{max_text_utf8_bytes:8}}});
  const partialResult=await compare(partial.id,partial.id,config);
  assert.equal(field(row(partialResult,'/changed'),'text_sources.raw_http_html.selected_text').before.state,'partial');assert.equal(field(row(partialResult,'/changed'),'text_sources.raw_http_html.selected_text').outcome,'unknown');
  const afterPartialHits=hits.length;
  checks.push('Legacy absent sources are missing; actual bounded selected text stays partial and its equal prefix is unknown');

  const collisionReport=structuredClone(before.report);
  collisionReport.pages=[structuredClone(before.report.pages[0]),structuredClone(before.report.pages[0])];
  const collisionId=await imported(collisionReport);
  const collision=await request('/api/compare/saved',{before:collisionId,after:after.id});assert.equal(collision.status,400);assert.equal(collision.body.error.code,'ambiguous_snapshot_identity');assert.equal(collision.body.error.collisions[0].occurrences.length,2);
  const mappings=[{side:'before',from_origin:origin,from_path_prefix:'/',to_origin:'https://production.fixture',to_path_prefix:'/site'}];
  const mapped=await compare(before.id,after.id,{mappings});
  assert.equal(mapped.comparison.counts.added,3);assert.equal(mapped.comparison.counts.removed,3);
  const mappedBefore=mapped.comparison.pages.find(p=>p.kind==='removed'&&p.before.original_requested_url.endsWith('/changed'));assert.equal(mappedBefore.comparison_key,'https://production.fixture/site/changed');assert.equal(mappedBefore.before.original_requested_url,origin+'/changed');assert.equal(field(mappedBefore,'final_url').before.value,origin+'/changed');
  for(const body of [{before:before.id,after:after.id,extra:true},{before:before.id,after:after.id,config:{unexpected:true}},{before:before.id,after:after.id,config:{limits:{max_pages_per_snapshot:1}}},{before:before.id,after:after.id,config:{mappings:[...mappings,...mappings]}}])assert.equal((await request('/api/compare/saved',body)).status,400);
  const zeroExpected=await call('/api/compare/saved',{before:shiftedId,after:legacyId,expected_before_generation:0,expected_after_generation:0});assert.equal(zeroExpected.website_requests,0);assert.equal(hits.length,afterPartialHits);
  checks.push('Mapping changes join keys only; duplicate identities, overlapping mapping, strict arguments and reduced page budgets reject without truncating or website fetches');

  const snapshotsBefore=await call(`/api/crawls/${before.id}/project.json`),snapshotsAfter=await call(`/api/crawls/${after.id}/project.json`);
  assert.deepEqual(snapshotsBefore,before.report);assert.deepEqual(snapshotsAfter,after.report);
  offlineHits=hits.length;
  await new Promise(resolve=>fixture.close(resolve));fixture.closeAllConnections();
  // The website responder is closed. A distinct owned witness on its former
  // endpoint observes attempted HTTP requests instead of trusting a dead counter.
  offlineTrap=http.createServer((req,res)=>{offlineAttemptCount++;res.writeHead(410);res.end('Fixture website is offline');});
  await new Promise((resolve,reject)=>{offlineTrap.once('error',reject);offlineTrap.listen(Number(new URL(origin).port),'127.0.0.1',resolve);});
  await stop();await launch();
  const restarted=await compare(before.id,after.id,config);assert.deepEqual(restarted.comparison.pages,result.comparison.pages);assert.deepEqual(restarted.comparison.counts,result.comparison.counts);
  for(const side of ['before_snapshot','after_snapshot'])assert.equal(restarted.comparison[side].typed_report_sha256,result.comparison[side].typed_report_sha256);
  const stdout=await execute(['compare-saved',beforePath,afterPath,'--config',configPath]);assert.equal(stdout.code,0,stdout.stderr);assert.deepEqual(JSON.parse(stdout.stdout).comparison.pages,result.comparison.pages);
  for(const kind of ['stdio','streamable_http']){
    const client=new Client({name:'saved-comparison-cold-restart',version:'0.1.0'});
    const transport=kind==='stdio'?new StdioClientTransport({command:binary,args:['mcp','--server',api],cwd:root,stderr:'pipe'}):new StreamableHTTPClientTransport(new URL(api+'/mcp'));
    try{alive();await client.connect(transport);const response=await client.callTool({name:'spider_compare_saved',arguments:{before:before.id,after:after.id,config}});assert.equal(response.isError,false);assert.deepEqual(response.structuredContent,restarted);alive();}finally{await client.close();}
  }
  assert.equal(hits.length,offlineHits);assert.equal(offlineAttemptCount,0);
  checks.push('After website shutdown and App cold restart, API/CLI/both SDK transports retain exact saved deltas/hashes and unchanged reports; a separate endpoint witness observes zero HTTP attempts');
  const proof={tested_at:new Date().toISOString(),result:'PASS',engine,binary_sha256:binarySha,checks,counts:result.comparison.counts,typed_before_sha256:result.comparison.before_snapshot.typed_report_sha256,typed_after_sha256:result.comparison.after_snapshot.typed_report_sha256,owned_origin:origin,owned_api:api,website_requests_for_read_only_phases:0,offline_endpoint_attempts:offlineAttemptCount,total_fixture_requests:offlineHits,limits:['Own bounded native/API/CLI/official-SDK contract; actual browser UI and release are separate gates.','Comparison JSON contains hash-bound snapshot descriptors and field projections; a combined full-original-report bundle remains a separate open feature acceptance.','SQL authenticity is not attested by the comparison caller binding; generation0 imports are explicitly undeclared.','No full website or vendor parity claim.']};
  await mkdir(path.dirname(proofPath),{recursive:true});await writeFile(proofPath,JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify({result:'PASS',checks:checks.length,proof:proofPath,counts:proof.counts},null,2));
}finally{await stop();if(fixture.listening){await new Promise(resolve=>fixture.close(resolve));fixture.closeAllConnections();}if(offlineTrap?.listening){await new Promise(resolve=>offlineTrap.close(resolve));offlineTrap.closeAllConnections();}await rm(data,{recursive:true,force:true});}
