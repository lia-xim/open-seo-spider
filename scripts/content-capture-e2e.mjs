// Native HTTP -> persisted source captures -> explicit offline cohort/API/CLI.
import http from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,writeFile,mkdir,rm} from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||path.join(root,'target/debug',`open-seo-spider${process.platform==='win32'?'.exe':''}`);
const data=await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-content-capture-'));
const api='http://127.0.0.1:18797',origin='http://127.0.0.1:19097';
let revision=0;const hits=[];
const words='These authors explain each observed fact with clear words. The text describes a small garden with tall trees and bright flowers. We can compare these complete saved samples without fetching a website again.';
const html=url=>`<html lang="en"><title>${url}</title><body><nav>Excluded navigation</nav><main>${words} Revision${revision}.</main><footer>Excluded footer</footer></body></html>`;
const fixture=http.createServer((request,response)=>{
  hits.push(request.url);response.setHeader('content-type','text/html; charset=utf-8');
  if(request.url==='/robots.txt')return response.end('User-agent: *\nAllow: /');
  if(request.url==='/empty')return response.end('<html lang="en"><main></main></html>');
  if(request.url==='/nonhtml'){response.setHeader('content-type','application/json');return response.end('{"html":"<main>not a document</main>"}');}
  if(request.url==='/lossy')return response.end(Buffer.concat([Buffer.from('<html lang="en"><main>Observed '),Buffer.from([0xff]),Buffer.from(' text.</main></html>')]));
  if(request.url==='/missing')response.statusCode=404;
  if(request.url==='/a'&&revision===2)response.statusCode=500;
  response.end(html(request.url));
});
// An owned renderer protocol fixture, explicitly not an actual Chromium proof.
const worker=http.createServer(async(request,response)=>{let body='';for await(const chunk of request)body+=chunk;const input=JSON.parse(body);response.setHeader('content-type','application/json');response.end(JSON.stringify({html:'<html lang="de"><main>Gerenderter eigener Beispieltext mit einer getrennten Sprache.</main></html>',final_url:input.url,status:200,headers:{'content-type':'text/html; charset=utf-8'},rendered_at:'2026-10-12T12:00:00Z',browser_version:'owned-protocol-fixture',console_errors:[],network_requests:[]}));});
await new Promise(resolve=>fixture.listen(19097,'127.0.0.1',resolve));await new Promise(resolve=>worker.listen(19098,'127.0.0.1',resolve));
let server;const checks=[];const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(fn){for(let i=0;i<600;i++){const value=await fn();if(value)return value;await wait(25);}throw Error('Native content acceptance timeout');}
async function request(route,body){return fetch(api+route,body===undefined?undefined:{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});}
async function get(route,body){const response=await request(route,body);assert.equal(response.status,200,await response.clone().text());return response.json();}
async function completed(id){return until(async()=>{const result=await get('/api/crawls/'+id);if(result.status==='failed')throw Error(result.error);return ['completed','limited','paused'].includes(result.status)&&result;});}
async function start(config){const result=await get('/api/crawls',config);await completed(result.id);return result.id;}
async function execute(args){return new Promise((resolve,reject)=>{const child=spawn(binary,args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';child.stdout.on('data',chunk=>stdout+=chunk);child.stderr.on('data',chunk=>stderr+=chunk);child.on('error',reject);child.on('exit',code=>resolve({code,stdout,stderr}));});}
try{
  server=spawn(binary,['serve','--listen','127.0.0.1:18797','--data-dir',data],{cwd:root,windowsHide:true,stdio:'ignore'});
  await until(async()=>{try{return(await fetch(api+'/healthz')).ok;}catch{return false;}});
  const base={crawl_links:false,discover_sitemaps:false,obey_robots:false,delay_ms:0,concurrency:2};
  const beforeInvalid=hits.length;
  assert.equal((await request('/api/crawls',{...base,seed_urls:[origin+'/a'],content_capture:{include_selectors:[':not(']}})).status,400);assert.equal(hits.length,beforeInvalid);
  checks.push('Malformed capture selectors rejected before HTTP; opt-in configuration is typed');
  const capture={sources:['raw_http_html','rendered_dom_html'],include_selectors:['main']};
  const id=await start({...base,seed_urls:['/a','/b','/empty','/nonhtml','/lossy','/missing'].map(url=>origin+url),content_capture:capture});
  const report=await get(`/api/crawls/${id}/project.json`),a=report.pages.find(page=>page.url===origin+'/a');
  assert.equal(a.content_captures.length,2);const raw=a.content_captures[0];assert.equal(raw.status,'complete');assert.equal(raw.result.text,words+' Revision0.');assert.equal(raw.result.html_language,'en');assert.equal(raw.result.html_utf8_sha256,hash(html('/a')));assert.equal(raw.encoded_body_sha256,hash(html('/a')));assert.equal(raw.observed_generation,1);
  assert.equal(a.content_captures[1].status,'unavailable');assert.equal(a.content_captures[1].result.text,null);assert.equal(a.content_captures[1].source_html_utf8_sha256,null);
  assert.equal(report.pages.find(page=>page.url.endsWith('/empty')).content_captures[0].status,'empty');assert.equal(report.pages.find(page=>page.url.endsWith('/empty')).content_captures[0].result.text,'');assert.equal(report.pages.find(page=>page.url.endsWith('/nonhtml')).content_captures[0].status,'unavailable');assert.equal(report.pages.find(page=>page.url.endsWith('/lossy')).content_captures[0].status,'incomplete');assert.equal(report.pages.find(page=>page.url.endsWith('/missing')).content_captures[0].status,'incomplete');
  checks.push('Real native raw bytes, selected text, source-specific root language and hashes persist; empty, missing rendered, nonHTML, lossy and404 stay distinct');
  const first=await get(`/api/crawls/${id}/content?limit=2`),second=await get(`/api/crawls/${id}/content?after_ordinal=${first.next_after_ordinal}&limit=2`);
  assert.equal(first.rows.length,2);assert.equal(first.text_included,false);assert.equal(first.total_saved_pages,6);assert.equal(second.rows.length,2);assert.notEqual(first.rows[1].requested_url,second.rows[0].requested_url);assert.equal(first.snapshot.origin,'native_saved_report');assert.ok(first.rows.every(row=>!Object.hasOwn(row,'text')));
  const source=await get(`/api/crawls/${id}/content/document?url=${encodeURIComponent(origin+'/a')}`);assert.deepEqual(source.capture,raw);
  const beforeAnalyze=hits.length,selection={urls:[origin+'/a',origin+'/b',origin+'/missing',origin+'/nonhtml'],source_kind:'raw_http_html',expected_generation:1};
  const analysis=await get(`/api/crawls/${id}/content/analyze`,selection);assert.equal(analysis.included_count,4);assert.equal(analysis.analysis.pairs[0].near_duplicate,true);assert.equal(analysis.analysis.pairs[1].near_duplicate,null);assert.equal(analysis.analysis.documents[3].status,'missing');assert.equal(analysis.website_requests,0);assert.equal(analysis.site_wide_coverage,false);assert.equal(hits.length,beforeAnalyze);
  const filtered=await get(`/api/crawls/${id}/content/analyze`,{...selection,complete_only:true});assert.equal(filtered.excluded.length,2);assert.equal(filtered.included_count,2);
  const empty=await get(`/api/crawls/${id}/content/analyze`,{urls:[origin+'/nonhtml'],complete_only:true});assert.equal(empty.status,'empty_filtered_cohort');assert.equal(empty.analysis,null);assert.equal(empty.excluded[0].row.requested_url,origin+'/nonhtml');
  for(const invalid of [{...selection,expected_generation:0},{...selection,urls:[origin+'/a',origin+'/a']},{...selection,urls:[origin+'/not-saved']},{...selection,unexpected:true},{...selection,config:{similarity:{threshold:2}}}])assert.ok([400,422].includes((await request(`/api/crawls/${id}/content/analyze`,invalid)).status));
  assert.equal((await request(`/api/crawls/${id}/content?limit=201`)).status,400);assert.equal(hits.length,beforeAnalyze);
  checks.push('Metadata inventory cursors, exact source inspector, duplicate/incomplete/missing cohorts and explicit filter exclusions use one saved snapshot; invalid selection/config/generation never fetch');
  const snapshotFile=path.join(data,'owned-snapshot-input.json'),selectionFile=path.join(data,'owned-selection-input.json'),output=path.join(data,'owned-analysis-output.json');
  const snapshotBytes=Buffer.from(JSON.stringify(report,null,2)+'\n');await writeFile(snapshotFile,snapshotBytes);const cliSelection={...selection};delete cliSelection.expected_generation;await writeFile(selectionFile,JSON.stringify(cliSelection));
  const cli=await execute(['analyze-crawl-content',snapshotFile,'--selection',selectionFile,'--output',output]);assert.equal(cli.code,0,cli.stderr);const cliResult=JSON.parse(await readFile(output,'utf8'));assert.deepEqual(cliResult.analysis,analysis.analysis);assert.equal(cliResult.snapshot.generation,null);assert.equal(cliResult.snapshot.origin,'declared_imported_report');assert.deepEqual(await readFile(snapshotFile),snapshotBytes);
  const outputBytes=await readFile(output);assert.notEqual((await execute(['analyze-crawl-content',snapshotFile,'--selection',selectionFile,'--output',output])).code,0);assert.deepEqual(await readFile(output),outputBytes);assert.equal(hits.length,beforeAnalyze);
  checks.push('Offline CLI analysis equals the native API engine result with declared file origin and unknown generation; input and existing output bytes preserved');
  const renderedId=await start({...base,seed_urls:[origin+'/a'],content_capture:capture,renderer:{endpoint:'http://127.0.0.1:19098/render',wait_ms:0}});const renderedReport=await get(`/api/crawls/${renderedId}/project.json`);const rendered=renderedReport.pages[0];assert.equal(rendered.content_captures[0].result.html_language,'en');assert.equal(rendered.content_captures[1].result.html_language,'de');assert.equal(rendered.content_captures[1].encoded_body_sha256,null);assert.notEqual(rendered.content_captures[0].source_revision,rendered.content_captures[1].source_revision);
  checks.push('Owned renderer protocol output retains independent raw/DOM text and language with no raw-body hash substitution; actual Chromium is a separate gate');
  const errorId=await start({...base,seed_urls:[origin+'/a'],content_capture:{include_selectors:['main'],limits:{max_html_utf8_bytes:32}}});const errorReport=await get(`/api/crawls/${errorId}/project.json`);assert.equal(errorReport.pages[0].final_status,200);assert.equal(errorReport.pages[0].content_captures[0].status,'error');assert.equal(errorReport.pages[0].content_captures[0].result,null);assert.ok(errorReport.pages[0].content_captures[0].failure);const errorAnalysis=await get(`/api/crawls/${errorId}/content/analyze`,{urls:[origin+'/a']});assert.equal(errorAnalysis.analysis.documents[0].status,'missing');
  const disabledId=await start({...base,seed_urls:[origin+'/a']});const disabledReport=await get(`/api/crawls/${disabledId}/project.json`);assert.equal(Object.hasOwn(disabledReport.config,'content_capture'),false);assert.equal(Object.hasOwn(disabledReport.pages[0],'content_captures'),false);assert.equal((await get(`/api/crawls/${disabledId}/content`)).rows[0].status,'missing');
  checks.push('Capture budget error remains an explicit missing analytical sample while HTTP200 survives; disabled capture preserves legacy config/page serialization');
  const parentBytes=await (await request(`/api/crawls/${id}/project.json`)).text();revision=2;const child=await get(`/api/crawls/${id}/recrawl`,{urls:[origin+'/a'],max_attempts:0});await completed(child.id);const childReport=await get(`/api/crawls/${child.id}/project.json`);assert.deepEqual(childReport.pages.find(page=>page.url===origin+'/b'),report.pages.find(page=>page.url===origin+'/b'));assert.equal(childReport.pages.find(page=>page.url===origin+'/a').content_captures[0].status,'incomplete');
  const mixed=await get(`/api/crawls/${child.id}/content/analyze`,{urls:[origin+'/a',origin+'/b']});assert.equal(mixed.documents[0].row.basis,'refreshed_selected');assert.equal(mixed.documents[1].row.basis,'inherited_parent');assert.equal(mixed.analysis.pairs[0].near_duplicate,null);assert.equal(await(await request(`/api/crawls/${id}/project.json`)).text(),parentBytes);
  checks.push('Selected500 refresh replaces old complete text with a new incomplete sample; unselected captures and exact parent bytes remain inherited and unchanged');
  const corruptId='11111111-1111-4111-8111-111111111111',corruptJSON=Buffer.from(JSON.stringify(disabledReport,null,2)+'\n');
  const stopped=new Promise(resolve=>server.once('exit',resolve));server.kill();await stopped;
  await writeFile(path.join(data,corruptId+'.json'),corruptJSON);await writeFile(path.join(data,corruptId+'.sqlite'),'owned deliberately invalid SQLite');
  server=spawn(binary,['serve','--listen','127.0.0.1:18797','--data-dir',data],{cwd:root,windowsHide:true,stdio:'ignore'});
  await until(async()=>{try{return(await fetch(api+'/healthz')).ok;}catch{return false;}});
  const beforeRecoveryRead=hits.length,declared=await get(`/api/crawls/${corruptId}/content`);
  assert.equal(declared.snapshot.origin,'declared_imported_report');assert.equal(declared.snapshot.generation,null);assert.equal(hits.length,beforeRecoveryRead);assert.deepEqual(await readFile(path.join(data,corruptId+'.json')),corruptJSON);
  const recoveredNative=await get(`/api/crawls/${id}/content`);assert.equal(recoveredNative.snapshot.origin,'native_saved_report');assert.equal(recoveredNative.snapshot.generation,1);
  checks.push('Read-only restart keeps valid native checkpoint binding; retained JSON beside malformed SQLite stays declared with unknown generation and unchanged bytes');
  const evidence={tested_at:new Date().toISOString(),platform:process.platform,result:'PASS',checks,website_targets:'owned localhost only',analysis_website_requests:0,actual_chromium:'separate explicit Core/UI gate',full_vendor_parity:'NOT_PROVEN'};
  await mkdir(path.join(root,'docs/verification'),{recursive:true});await writeFile(process.env.SPIDER_EVIDENCE||path.join(root,'docs/verification/content-capture-e2e.json'),JSON.stringify(evidence,null,2)+'\n');console.log(JSON.stringify(evidence,null,2));
} finally {
  if(server?.exitCode===null){const stopped=new Promise(resolve=>server.once('exit',resolve));server.kill();await stopped;}
  fixture.closeAllConnections();worker.closeAllConnections();await new Promise(resolve=>fixture.close(resolve));await new Promise(resolve=>worker.close(resolve));
  const owned=path.resolve(data);assert.equal(path.dirname(owned),path.resolve(os.tmpdir()));assert.ok(path.basename(owned).startsWith('open-seo-spider-content-capture-'));await rm(owned,{recursive:true,force:true});
}
