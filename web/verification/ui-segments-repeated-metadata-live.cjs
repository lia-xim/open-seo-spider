'use strict';
// A real repeated-description crawl, not seeded positive report JSON. Imported
// duplicate observations are a separately labelled control. All replayed POSTs
// below are pure native reads; the import mutation is observed exactly once.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawn} = require('node:child_process');
const {createHash, randomUUID} = require('node:crypto');
const {createRequire} = require('node:module');
const {pathToFileURL} = require('node:url');
const {chromium} = require('../../renderer/node_modules/playwright-core');
const root = path.resolve(__dirname, '../..');
assert(process.env.SPIDER_BINARY, 'Explicit normal SPIDER_BINARY required');
const binary = fs.realpathSync(process.env.SPIDER_BINARY), sha = bytes => createHash('sha256').update(bytes).digest('hex');
const binarySha = sha(fs.readFileSync(binary));
if (process.env.SPIDER_BINARY_SHA256) assert.equal(binarySha, process.env.SPIDER_BINARY_SHA256);
const output = path.resolve(process.env.SPIDER_REPEATED_METADATA_OUTPUT || path.join(process.env.SPIDER_TEST_ARTIFACT_DIR || path.join(root,'artifacts'), `segments-repeated-metadata-${Date.now()}-${randomUUID().slice(0,8)}`));
assert(!fs.existsSync(output)); fs.mkdirSync(output,{recursive:true}); process.env.SPIDER_TEST_ARTIFACT_DIR=output;
const artifactPath = require('./artifact-path.cjs'), store=path.join(output,'native-store');
const pins=[__filename,path.join(__dirname,'artifact-path.cjs')].map(file=>({path:file,sha256:sha(fs.readFileSync(file))}));
const CAP=64*1024*1024, PAGE_CAP=32*1024*1024;
const executionStarted=Date.now();let completedPhases=0;
const description='Independent owned fixture with captured source facts and enough distinct document text to inspect saved membership accurately.';
const productPaths=Array.from({length:1005},(_,i)=>'/products/'+String(i).padStart(4,'0'));
const websitePaths=[...productPaths,'/products/hidden','/outside','/untitled','/unassigned','/missing'];
const checks=[],owners=[],errors=[],external=[],reads=[],readPromises=[],downloads=[],projections=[],assets=[],guards=[],negativeControls=[];
const websiteHits=new Map(),witnessHits=[],witnessControls=[],sourceIds=new WeakMap(),nonce=randomUUID();
const sourceGenerations=new WeakMap(),consumerProofs=[],consumerChildren=[],standaloneProofs=[];
const collator=new Intl.Collator('de',{numeric:true,sensitivity:'base'}), number=n=>Number(n).toLocaleString('de-DE');
let website,siteOrigin,origin,app,browser,context,page,editor,report,rawAnalysis,nativeId,tail,rules,expectation,offline=false,capturedHits,failure;
let rawProjectProof,storedReportProof,firstLargeProof,firstIssueCSVProof,validatedCSVControl;
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function bounded(promise,label,ms=30000){let timer;return Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Timeout: '+label)),ms);})]).finally(()=>clearTimeout(timer));}
async function until(work,label){for(let attempt=0;attempt<2400;attempt++){if(app)assert.equal(app.child.exitCode,null,app.log);const value=await work();if(value)return value;await sleep(25);}throw Error('Timeout: '+label);}
function verifyPins(){assert.equal(sha(fs.readFileSync(binary)),binarySha);for(const pin of pins)assert.equal(sha(fs.readFileSync(pin.path)),pin.sha256);}
function recordPhase(name){
  assert(reads.every(Boolean),'A verified phase cannot retain unresolved browser-native read oracles');
  const sequence=++completedPhases,receipt={result:'IN_PROGRESS',completed_phase:name,sequence,checked_utc:new Date().toISOString(),elapsed_ms:Date.now()-executionStarted,binary_sha256:binarySha,pins,
    native:owners.map(o=>({phase:o.phase,pid:o.child.pid,origin:o.origin,shutdown_observed:!!o.shutdown,...(o.shutdown?{shutdown:o.shutdown}:{})})),
    consumer_children:consumerChildren.map(o=>({phase:o.phase,pid:o.child.pid,shutdown_observed:!!o.shutdown,...(o.shutdown?{shutdown:o.shutdown}:{})})),
    native_crawl_id:nativeId,native_pages:report?.pages.length,native_issues:report?.issues.length,raw_project:rawProjectProof,stored_report:storedReportProof,
    projections,negative_controls:negativeControls,source_guards:guards,exports:downloads,standalone_api_cursors:standaloneProofs,actual_consumers:consumerProofs,
    read_only_native_oracles:reads,offline_product_requests:witnessHits,witness_controls:witnessControls,errors,external_requests:external,
    scope:'Checkpoint only after the named phase assertions return. Later phases and final whole-suite PASS are not established; shutdown is recorded only after the actual close witness has been awaited. No report, parsed response rows, source issue arrays or binary response buffers retained.'};
  fs.writeFileSync(artifactPath('phase-'+String(sequence).padStart(2,'0')+'-'+name+'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
}
async function launch(phase,listen='127.0.0.1:0'){
  verifyPins();const env={...process.env,OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed'};delete env.OPEN_SEO_PAGESPEED_API_KEY;
  const child=spawn(binary,['serve','--listen',listen,'--data-dir',store,'--shutdown-on-stdin-eof'],{cwd:root,windowsHide:true,stdio:['pipe','pipe','pipe'],env});
  const owner={phase,child,log:''};owner.closed=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));child.once('error',error=>{owner.spawnError=error;});
  for(const stream of[child.stdout,child.stderr])stream.on('data',bytes=>{owner.log+=bytes;});owners.push(owner);app=owner;
  owner.origin=await until(()=>{if(owner.spawnError)throw owner.spawnError;return owner.log.match(/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/m)?.[1];},'native '+phase);
  assert(!['8787','8790'].includes(new URL(owner.origin).port));origin=owner.origin;assert.equal((await fetch(origin+'/healthz')).status,200);return owner;
}
async function stop(owner){owner.child.stdin.end();owner.shutdown=await bounded(owner.closed,'native EOF '+owner.phase);assert.deepEqual(owner.shutdown,{code:0,signal:null},owner.log);if(app===owner)app=null;verifyPins();}
async function raw(route,body,status=200){const response=await fetch(origin+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'content-type':'application/json'},body:body===undefined?undefined:JSON.stringify(body),redirect:'error',cache:'no-store'});const bytes=Buffer.from(await response.arrayBuffer());assert.equal(response.status,status,bytes.subarray(0,2000).toString());return{response,bytes};}
async function api(route,body,status=200){const value=await raw(route,body,status);return{...value,value:JSON.parse(value.bytes)};}
async function assetPins(phase){for(const name of['app.js','dashboard-segments.js','index.html','styles.css','saved-segments.js','saved-segments.css']){const {bytes}=await raw(name==='index.html'?'/':'/'+name);const digest=sha(bytes);assert.equal(digest,sha(fs.readFileSync(path.join(root,'web',name))),name+' must match the new normal build');assets.push({phase,path:name,sha256:digest});}}
function sourceId(source){const id=sourceIds.get(source);assert(id);return id;}
function members(source,ordinals){return ordinals.map(page_ordinal=>({page_ordinal,page:source.pages[page_ordinal]}));}
function issuesFor(source,ordinals){const urls=new Set(ordinals.map(i=>source.pages[i].url));return source.issues.filter(issue=>urls.has(issue.url));}
function groupURLs(source){return source.pages.filter(p=>p.internal&&p.indexable&&p.meta_descriptions?.[0]?.trim().toLowerCase()===description.toLowerCase()).map(p=>p.url);}
function validateRelated(issue,source){assert.equal(issue.code,'description_duplicate');assert.equal(issue.severity,'warning');const group=groupURLs(source);assert.equal(group.length,1008);assert(group.includes(issue.url));assert.equal(issue.message,'Same meta description on 1008 indexable URLs');assert.deepEqual(issue.related_urls,group.filter(url=>url!==issue.url),'Every related URL retains original group page order, without self or substitution');}
function validateSource(){
  assert.equal(report.pages.length,1010);assert.equal(report.issues.length,7068);assert.deepEqual(new Set(report.pages.map(p=>p.url)),new Set(websitePaths.map(s=>siteOrigin+s)));
  const duplicates=report.issues.filter(issue=>issue.code==='description_duplicate');assert.equal(duplicates.length,1008);
  assert.deepEqual(new Set(duplicates.map(issue=>issue.url)),new Set(groupURLs(report)));for(const issue of duplicates)validateRelated(issue,report);
  assert.equal(duplicates.reduce((sum,issue)=>sum+issue.related_urls.length,0),1015056);assert(!report.issues.some(issue=>['title_duplicate','content_duplicate'].includes(issue.code)),'Only description metadata repeats; title and document text remain distinct');
}
function validateContext(value,expected){assert.equal(value.context.job_id,sourceId(expected.source));assert.equal(value.context.app_generation,sourceGenerations.get(expected.source));assert.match(value.context.snapshot_token,/^[a-f0-9]{64}$/);assert.deepEqual(value.cohort.rules,expected.rules);assert.deepEqual(value.cohort.selection,expected.selection);assert.deepEqual(value.captured_crawl_summary,expected.source.summary);assert.equal(value.cohort.summary.selected_pages,expected.ordinals.length);assert.equal(value.cohort.summary.selected_issues,expected.issues.length);}
function validateFullJSON(value,expected){
  validateContext(value,expected);assert.equal(value.kind,'segment_dashboard_cohort_not_raw_crawl');assert.deepEqual(value.pages.map(({page_ordinal,page})=>({page_ordinal,page})),members(expected.source,expected.ordinals));assert.deepEqual(value.issues,expected.issues);assert.deepEqual(value.captured_config,expected.source.config);
  if(expected.source===report)for(const issue of value.issues.filter(issue=>issue.code==='description_duplicate'))validateRelated(issue,report);
}
function extraColumns(view){return[
 ['Segment Snapshot Token',view.context.snapshot_token],['Segment Native Generation',String(view.context.app_generation)],['Segment Typed Report SHA256',view.context.typed_report_sha256],['Segment Rules SHA256',view.context.rules_sha256],['Segment Selection JSON',JSON.stringify(view.cohort.selection)],['Segment Reporting View JSON',JSON.stringify(view.context.reporting_view??null)],['Segment Captured Pages',String(view.source.pages.length)],['Segment Eligible Pages',String(view.cohort.summary.total_pages)],['Segment Selected Pages',String(view.ordinals.length)],['Segment Captured Issues',String(view.source.issues.length)],['Segment Selected Issues',String(view.issues.length)],['Segment Orphan Issues',String(view.cohort.summary.orphan_issues)],['Segment Scope','selected saved observations; crawl activity and relationship targets unchanged; no SQL or origin attestation']];}
function* csvRows(bytes){
  const text=bytes.toString('utf8').replace(/^\uFEFF/,''),row=[];let field='',quoted=false;
  for(let i=0;i<text.length;i++){const char=text[i];if(char==='"'&&quoted&&text[i+1]==='"'){field+='"';i++;}else if(char==='"')quoted=!quoted;else if(char===','&&!quoted){row.push(field);field='';}else if(char==='\n'&&!quoted){row.push(field.replace(/\r$/,''));yield row.splice(0);field='';}else field+=char;}
  assert.equal(quoted,false);if(field||row.length){row.push(field);yield row;}
}
function validateIssueCSVRow(row,issue,view){
  const expected=[issue.severity[0].toUpperCase()+issue.severity.slice(1),issue.code,issue.url,issue.message,issue.related_urls.join(' | '),...extraColumns(view).map(pair=>pair[1])];
  assert.deepEqual(row,expected,'Every native issue CSV cell and context value must remain complete and ordered');
}
function validateIssueCSV(bytes,view){
  assert(bytes.length<=CAP);const iterator=csvRows(bytes);assert.deepEqual(iterator.next().value,['Severity','Code','URL','Message','Related URLs',...extraColumns(view).map(pair=>pair[0])]);let count=0;
  for(const row of iterator){assert(count<view.issues.length);const issue=view.issues[count++];validateIssueCSVRow(row,issue,view);if(issue.code==='description_duplicate'&&view.source===report){validateRelated(issue,report);validatedCSVControl??={row:[...row],issue,view};}}
  assert.equal(count,view.issues.length);return count;
}
async function observedRead(response,expected,index){
  assert(expected,'Every browser native read has an explicit source oracle');const request=response.request(),body=request.postDataBuffer(),url=new URL(response.url());assert(body);assert.equal(url.origin,origin);
  assert.match(url.pathname,/^\/api\/crawls\/[a-f0-9-]+\/segments\/dashboard(?:-analysis|\.json|\/[a-z-]+\.csv)?$/i);
  const oracle=await fetch(response.url(),{method:'POST',headers:{'content-type':'application/json'},body,redirect:'error',cache:'no-store'}),bytes=Buffer.from(await oracle.arrayBuffer()),headers=await response.allHeaders();
  assert.equal(response.status(),200);assert.equal(oracle.status,200,bytes.subarray(0,2000).toString());
  for(const key of['content-type','content-length','x-content-sha256','cache-control','x-segment-snapshot-token','x-segment-generation'])assert.equal(headers[key],oracle.headers.get(key),key);
  assert.equal(Number(headers['content-length']),bytes.length);assert.equal(headers['x-content-sha256'],sha(bytes));assert.equal(headers['cache-control'],'no-store');
  const requestValue=JSON.parse(body),entry={index,path:url.pathname,request:requestValue,request_sha256:sha(body),bytes:bytes.length,sha256:sha(bytes),context:null,label:expected.label,body_source:'unconditional_exact_read_only_native_post',browser_content_binding:'product_SHA_validation_and_exact_DOM_or_actual_download'};
  if(url.pathname.endsWith('.csv')){
    assert(expected.view);assert.equal(headers['content-type'].split(';')[0],'text/csv');validateIssueCSV(bytes,expected.view);entry.rows=expected.issues.length;entry.context=expected.view.context;
  }else{
    assert.equal(headers['content-type'].split(';')[0],'application/json');const value=JSON.parse(bytes);validateContext(value,expected);entry.context=value.context;entry.cohort=value.cohort;
    if(url.pathname.endsWith('/dashboard')){
      assert(bytes.length<=PAGE_CAP);const offset=requestValue.cursor?.offset??0,sourceRows=requestValue.view==='pages'?members(expected.source,expected.ordinals):expected.issues;
      assert.equal(value.view,requestValue.view);assert.equal(requestValue.limit,200);assert.equal(value.total,sourceRows.length);assert.deepEqual(value.rows.map(row=>requestValue.view==='pages'?{page_ordinal:row.page_ordinal,page:row.page}:row),sourceRows.slice(offset,offset+200));
      const end=Math.min(offset+200,sourceRows.length);assert.deepEqual(value.next,end<sourceRows.length?{offset:end,view:requestValue.view,snapshot_token:value.context.snapshot_token}:null);entry.rows=value.rows.length;entry.next=value.next;entry.total=value.total;
      if(requestValue.view==='issues'&&expected.source===report)for(const issue of value.rows.filter(issue=>issue.code==='description_duplicate'))validateRelated(issue,report);
    }else if(url.pathname.endsWith('/dashboard-analysis')){
      assert(bytes.length<=PAGE_CAP);assert.equal(value.target_lookup.length,expected.source.pages.length);const urls=new Set(expected.ordinals.map(i=>expected.source.pages[i].url));
      if(expected.source===report){assert.deepEqual(value.analysis.graph.nodes,rawAnalysis.graph.nodes.filter(node=>urls.has(node.url)));assert.deepEqual(value.analysis.graph.edges,rawAnalysis.graph.edges.filter(edge=>urls.has(edge.source)));assert.deepEqual(value.analysis.graph.orphan_candidates,rawAnalysis.graph.orphan_candidates.filter(row=>urls.has(row.url)));assert.deepEqual(value.analysis.coverage,rawAnalysis.coverage);}
      entry.analysis_nodes=value.analysis.graph.nodes.length;entry.analysis_edges=value.analysis.graph.edges.length;
    }else{assert(bytes.length<=CAP);validateFullJSON(value,expected);}
  }
  assert.equal(headers['x-segment-snapshot-token'],entry.context.snapshot_token);assert.equal(headers['x-segment-generation'],String(entry.context.app_generation));
  if(url.pathname.endsWith('.json')||url.pathname.endsWith('.csv')){entry.oracle_file=artifactPath('oracle-'+String(index).padStart(3,'0')+(url.pathname.endsWith('.csv')?'.csv':'.json'));fs.writeFileSync(entry.oracle_file,bytes);}
  // No parsed response, large byte buffer, or issue/related-URL array is kept in
  // the receipt. Source comparisons finish before this promise is released.
  reads[index]=entry;
}
function observe(target){target.on('pageerror',error=>errors.push(error.message));target.on('response',response=>{if(response.request().method()==='POST'&&/\/segments\/dashboard(?:-analysis|\.json|\/[a-z-]+\.csv)?$/.test(new URL(response.url()).pathname)){const index=reads.length;reads.push(null);const pending=observedRead(response,expectation,index);readPromises.push(pending);pending.catch(()=>{});}});}
async function drainReads(){await Promise.all(readPromises);assert(reads.every(Boolean));}
async function selectCrawl(id){await page.locator(`#crawl-history option[value="${id}"]`).waitFor({state:'attached'});await page.selectOption('#crawl-history',id);await page.waitForFunction(()=>!document.getElementById('dashboard-segment-apply').disabled);await page.waitForFunction(()=>document.getElementById('crawl-state').textContent.toLowerCase().includes('abgeschlossen'));}
async function noApplied(){await page.waitForFunction(()=>document.getElementById('dashboard-segment-json').disabled&&document.getElementById('dashboard-segments').getAttribute('aria-busy')==='false');assert.equal(await page.locator('#dashboard-segment-csv').isDisabled(),true);assert.equal(await page.locator('#dashboard-segment-export-kind').isDisabled(),true);}
async function authorRules(definitions){
  if(!editor||editor.isClosed()){editor=await context.newPage();observe(editor);}await editor.goto(origin+'/saved-segments');
  for(const definition of definitions){await editor.click('#add');const fieldset=editor.locator('#definitions fieldset').last();await fieldset.getByLabel('Name',{exact:true}).fill(definition.name);
    for(let i=0;i<definition.conditions.length;i++){if(i)await fieldset.getByRole('button',{name:'Bedingung hinzufügen',exact:true}).click();const row=fieldset.locator('.condition').nth(i),condition=definition.conditions[i];await row.locator('select').nth(0).selectOption(condition.field);await row.locator('select').nth(1).selectOption(condition.operator);await row.getByLabel('Wert',{exact:true}).fill(condition.value);if(condition.case_insensitive)await row.locator('input[type=checkbox]').check();}}
  await editor.click('#save');assert.match(await editor.locator('#message').textContent(),/gespeichert/);assert.deepEqual(await editor.evaluate(()=>JSON.parse(localStorage.getItem('open-seo-saved-segments-v1'))),{version:1,definitions});
}
async function loadRules(){await page.click('#dashboard-segment-load');await noApplied();}
async function apply(chosen,source,ordinals,label,activeRules=rules){
  const independent=(await api('/api/crawls/'+sourceId(source))).value;assert.equal(independent.generation,sourceGenerations.get(source),'Native metadata independently binds imported0 versus captured1');
  await page.selectOption('#dashboard-segment-selection',chosen);await page.selectOption('#dashboard-segment-mode','reported');assert.equal(await page.locator('#dashboard-segment-json').isDisabled(),true);
  const expected={source,ordinals,issues:issuesFor(source,ordinals),rules:activeRules,selection:{kind:'segment',id:chosen.slice(8)},label};expectation=expected;
  const start=reads.length,pending=page.waitForResponse(response=>response.url()===`${origin}/api/crawls/${sourceId(source)}/segments/dashboard-analysis`&&response.request().method()==='POST',{timeout:420000});
  await page.click('#dashboard-segment-apply');await pending;await page.waitForFunction(()=>!document.getElementById('dashboard-segment-json').disabled,null,{timeout:180000});await drainReads();
  const current=reads.slice(start),pageReads=current.filter(r=>r.path.endsWith('/dashboard')&&r.request.view==='pages'),issueReads=current.filter(r=>r.path.endsWith('/dashboard')&&r.request.view==='issues');assert(pageReads.length&&issueReads.length);
  const first=pageReads[0];for(const entry of current){assert.deepEqual(entry.context,first.context);assert.deepEqual(entry.cohort,first.cohort);if(entry.request.cursor)assert.equal(entry.request.expected_snapshot_token,first.context.snapshot_token);}
  for(const entries of[pageReads,issueReads])for(let i=0;i<entries.length;i++){assert.deepEqual(entries[i].request.cursor,i?entries[i-1].next:null);assert.equal(entries[i].next?.offset??entries[i].total,entries.slice(0,i+1).reduce((sum,entry)=>sum+entry.rows,0));}
  assert.equal(await page.locator('#metric-fetched').textContent(),number(ordinals.length));assert.equal(await page.locator('#metric-issues').textContent(),number(expected.issues.length));
  const view={...expected,context:first.context,cohort:first.cohort,request:first.request};expected.view=view;expectation=expected;
  projections.push({label,source:sourceId(source),pages:ordinals.length,issues:expected.issues.length,related_urls:expected.issues.reduce((sum,i)=>sum+i.related_urls.length,0),page_sizes:pageReads.map(r=>r.rows),issue_sizes:issueReads.map(r=>r.rows),snapshot:first.context.snapshot_token});return view;
}
function issueTitle(code){return code==='canonical_missing'?'Canonical fehlt':String(code).replace(/[_-]/g,' ');}
async function exactDOM(view){
  const titleExpected=members(view.source,view.ordinals).sort((a,b)=>collator.compare(a.page.url,b.page.url)).map(({page:p})=>[p.url,p.titles[0],number(Array.from(p.titles[0]).length),p.title_pixel_widths?.[0]==null?'—':number(p.title_pixel_widths[0]),number(p.titles.length),'1',String(p.final_status??p.status)]);
  const issueExpected=[...view.issues].sort((a,b)=>collator.compare(a.url,b.url)).map(issue=>[issue.url,({error:'Fehler',warning:'Warnung',notice:'Hinweis'})[issue.severity],issueTitle(issue.code),issue.message,number(issue.related_urls.length)]);
  for(const[section,expected]of[['titles',titleExpected],['issues',issueExpected]]){
    await page.click(`[data-section="${section}"]`);await page.selectOption('#page-size','100');const seen=[];
    for(let offset=0;offset<expected.length;offset+=100){const slice=expected.slice(offset,offset+100);await page.waitForFunction(rows=>JSON.stringify([...document.querySelectorAll('#results-table tbody tr[data-row]')].map(row=>[...row.cells].map(c=>c.textContent)))===JSON.stringify(rows),slice);const actual=await page.locator('#results-table tbody tr[data-row]').evaluateAll(rows=>rows.map(row=>[...row.cells].map(c=>c.textContent)));assert.deepEqual(actual,slice);seen.push(...actual);if(offset+100<expected.length){assert.equal(await page.locator('#page-next').isDisabled(),false);await page.click('#page-next');}}
    assert.deepEqual(seen,expected);assert.equal(await page.locator('#page-next').isDisabled(),true);
  }
}
async function attachment(kind,label,view){
  expectation={source:view.source,ordinals:view.ordinals,issues:view.issues,rules:view.rules,selection:view.selection,label,view};
  if(kind==='issues'){await page.selectOption('#dashboard-segment-export-kind','issues');assert.equal(await page.locator('#dashboard-segment-json').isDisabled(),false,'Changing export kind preserves the confirmed cohort');}
  const start=reads.length,pending=page.waitForEvent('download',{timeout:180000});await page.click(kind==='json'?'#dashboard-segment-json':'#dashboard-segment-csv');const download=await pending;assert.equal(await download.failure(),null);const file=await download.path();await drainReads();
  const read=reads.slice(start).find(r=>r.path.endsWith(kind==='json'?'/dashboard.json':'/dashboard/issues.csv'));assert(read);const actual=fs.readFileSync(file);assert(actual.equals(fs.readFileSync(read.oracle_file)),'Actual browser download equals the independently read native response byte for byte');assert(actual.length<=CAP);
  const saved=artifactPath(label+(kind==='json'?'.json':'.csv'));fs.copyFileSync(file,saved);if(kind==='json')validateFullJSON(JSON.parse(actual),expectation);else validateIssueCSV(actual,view);
  const proof={label,kind,bytes:actual.length,sha256:sha(actual),path:saved,native_oracle:read.index};downloads.push(proof);return proof;
}
function validatorControls(view){
  const original=view.issues.find(issue=>issue.code==='description_duplicate');assert(original);validateRelated(original,report);
  const missing={...original,related_urls:original.related_urls.slice(0,-1)};assert.throws(()=>validateRelated(missing,report),assert.AssertionError);negativeControls.push('actual_related_validator_rejects_missing_last_url');
  const duplicate={...original,related_urls:[...original.related_urls]};duplicate.related_urls[duplicate.related_urls.length-1]=duplicate.related_urls[0];assert.throws(()=>validateRelated(duplicate,report),assert.AssertionError);negativeControls.push('actual_related_validator_rejects_duplicate_replacing_last_url_at_equal_count');
  const reordered={...original,related_urls:[...original.related_urls]};[reordered.related_urls[0],reordered.related_urls[1]]=[reordered.related_urls[1],reordered.related_urls[0]];assert.throws(()=>validateRelated(reordered,report),assert.AssertionError);negativeControls.push('actual_related_validator_rejects_reordered_urls');
  assert(validatedCSVControl);const wrong=[...validatedCSVControl.row];wrong[wrong.length-1]+=' forged';assert.throws(()=>validateIssueCSVRow(wrong,validatedCSVControl.issue,validatedCSVControl.view),assert.AssertionError);negativeControls.push('actual_csv_row_validator_rejects_changed_final_context_cell');
}
async function tailInspector(view){
  await page.click('[data-section="titles"]');await page.locator('#results-table tbody tr[data-row]').click();assert.equal(await page.locator('#inspector-url').textContent(),tail.page.url);
  assert.equal(await page.locator('#inspector-content dt').filter({hasText:/^Originale Seitenbeobachtung \(ab 0\)$/}).evaluate(e=>e.nextElementSibling.textContent),String(view.ordinals[0]));
  await page.click('#inspector-tabs [data-tab="issues"]');const expected=view.issues.flatMap(issue=>issue.related_urls);assert.equal(expected.length,1007);assert.deepEqual(await page.locator('#inspector-content .detail-links a').evaluateAll(links=>links.map(link=>[link.textContent,link.href])),expected.map(url=>[url,url]));
}
function standaloneRows(view,kind){return kind==='issues'?view.issues:view.ordinals.map(page_ordinal=>{const p=view.source.pages[page_ordinal];return{page_ordinal,url:p.url,document_url:p.final_url,status:p.status,depth:p.depth,stored_title:p.titles[0],matched_indices:[0,...(p.url===tail.page.url?[1]:[])],unknown_indices:[]};});}
async function standaloneRead(view,request){
  const result=await raw(`/api/crawls/${sourceId(view.source)}/segments`,request),headers=result.response.headers;
  assert(result.bytes.length<=PAGE_CAP);assert.equal(headers.get('content-type').split(';')[0],'application/json');assert.equal(headers.get('cache-control'),'no-store');assert.equal(Number(headers.get('content-length')),result.bytes.length);assert.equal(headers.get('x-content-sha256'),sha(result.bytes));
  const value=JSON.parse(result.bytes),sourceRows=standaloneRows(view,request.view),offset=request.cursor?.offset??0,end=Math.min(offset+request.limit,sourceRows.length);
  assert.deepEqual(value.context,view.context);assert.equal(value.view,request.view);assert.equal(value.total,sourceRows.length);assert.deepEqual(value.rows,sourceRows.slice(offset,end));
  assert.deepEqual(value.next,end<sourceRows.length?{offset:end,view:request.view,snapshot_token:view.context.snapshot_token}:null);
  for(const key of['total_pages','selected_pages','selected_issues','multiple_matches','unassigned_pages','unknown_pages','orphan_issues'])assert.equal(value.summary[key],view.cohort.summary[key]);
  if(request.view==='issues')for(const issue of value.rows.filter(i=>i.code==='description_duplicate'))validateRelated(issue,report);
  const proof={kind:request.view,offset,rows:value.rows.length,total:value.total,bytes:result.bytes.length,sha256:sha(result.bytes),request_sha256:sha(JSON.stringify(request)),snapshot:view.context.snapshot_token};
  return{value,proof};
}
async function standaloneCoverage(view){
  for(const kind of['pages','issues']){let cursor=null,seen=0;do{const request={...view.request,view:kind,limit:200,cursor,expected_snapshot_token:view.context.snapshot_token},read=await standaloneRead(view,request);standaloneProofs.push(read.proof);seen+=read.value.rows.length;cursor=read.value.next;}while(cursor);assert.equal(seen,standaloneRows(view,kind).length);}
  for(const suffix of['segments.json','segments.csv']){const refused=await raw(`/api/crawls/${sourceId(view.source)}/${suffix}`,{...view.request,cursor:null,expected_snapshot_token:view.context.snapshot_token},400),error=JSON.parse(refused.bytes);assert.equal(error.code,'segments_refused');assert.match(error.message,/Segment output exceeds byte ceiling/);assert.equal(refused.response.headers.get('content-disposition'),null);assert.equal(refused.response.headers.get('x-content-sha256'),null);consumerProofs.push({consumer:'native_standalone_api',suffix,status:400,error,successful_prefix:false});}
}
async function cliRun(label,request,csv,success){
  verifyPins();const folder=artifactPath('cli-'+label);fs.mkdirSync(folder);const requestPath=path.join(folder,'request.json'),destination=path.join(folder,csv?'output.csv':'output.json');fs.writeFileSync(requestPath,JSON.stringify(request));
  const args=['saved-segments',storedReportProof.path,'--request',requestPath,'--output',destination,...(csv?['--csv']:[])];
  const child=spawn(binary,args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed'}}),owner={phase:'cli-'+label,child,stdout:'',stderr:''};
  consumerChildren.push(owner);owner.closed=new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',(code,signal)=>resolve({code,signal}));});child.stdout.on('data',b=>{owner.stdout+=b;});child.stderr.on('data',b=>{owner.stderr+=b;});owner.shutdown=await bounded(owner.closed,label,120000);
  assert.equal(owner.shutdown.signal,null);if(success){assert.equal(owner.shutdown.code,0,owner.stderr);assert.deepEqual(fs.readdirSync(folder).sort(),['request.json',path.basename(destination)].sort());}else{assert.notEqual(owner.shutdown.code,0);assert.match(owner.stderr,/Segment output exceeds byte ceiling/);assert.equal(fs.existsSync(destination),false);assert.deepEqual(fs.readdirSync(folder),['request.json'],'Refused CLI leaves no output, temporary file or successful prefix');}
  consumerProofs.push({consumer:'actual_normal_cli',label,args,shutdown:owner.shutdown,output_exists:fs.existsSync(destination),stdout:owner.stdout,stderr:owner.stderr});verifyPins();assert.equal(sha(fs.readFileSync(storedReportProof.path)),storedReportProof.sha256);return destination;
}
async function cliCoverage(tailView,large){
  const request={...tailView.request,expected_generation:null,expected_snapshot_token:null,cursor:null},jsonPath=await cliRun('tail-json',request,false,true),value=JSON.parse(fs.readFileSync(jsonPath));
  assert.equal(value.context.job_id,null);assert.equal(value.context.app_generation,null);assert.equal(value.context.origin,'declared_offline_report_file');assert.equal(value.context.sql_attested,false);assert.equal(value.context.report_generation_attested,false);assert.equal(value.context.typed_report_sha256,tailView.context.typed_report_sha256);assert.equal(value.context.rules_sha256,tailView.context.rules_sha256);assert.deepEqual(value.context.reporting_view,tailView.context.reporting_view);
  assert.deepEqual(value.projection.pages,standaloneRows(tailView,'pages'));assert.deepEqual(value.projection.issues,tailView.issues);for(const issue of value.projection.issues.filter(i=>i.code==='description_duplicate'))validateRelated(issue,report);
  const csvPath=await cliRun('tail-csv',request,true,true),rows=[...csvRows(fs.readFileSync(csvPath))];assert.deepEqual(rows.shift(),['row_type','page_ordinal','url','segment_indices_json','unknown_indices_json','data_json']);
  assert.deepEqual(rows.map(r=>r[0]),['context','summary','reporting','page',...tailView.issues.map(()=>'issue')]);
  assert.deepEqual(rows[0].slice(0,5),['context','','','','']);assert.deepEqual(JSON.parse(rows[0][5]),value.context);
  const summary=JSON.parse(rows[1][5]);for(const key of['engine','depth_policy','rules','selection','total_pages','selected_pages','multiple_matches','unassigned_pages','unknown_pages','orphan_issues','counts','charts'])assert.deepEqual(summary[key],value.projection[key]);assert.deepEqual(rows[1].slice(0,5),['summary','','','','']);
  assert.deepEqual(rows[2].slice(0,5),['reporting','','','','']);assert.deepEqual(JSON.parse(rows[2][5]),value.projection.reporting);
  const p=value.projection.pages[0];assert.deepEqual(rows[3].slice(0,5),['page',String(p.page_ordinal),p.url,JSON.stringify(p.matched_indices),JSON.stringify(p.unknown_indices)]);assert.deepEqual(JSON.parse(rows[3][5]),p);
  for(let i=0;i<tailView.issues.length;i++){assert.deepEqual(rows[i+4].slice(0,5),['issue','',tailView.issues[i].url,'','']);assert.deepEqual(JSON.parse(rows[i+4][5]),tailView.issues[i]);}
  consumerProofs.push({consumer:'actual_normal_cli',scope:'tail_full_json_and_6column_csv_complete_fields_and1007relatedURLs',json:{bytes:fs.statSync(jsonPath).size,sha256:sha(fs.readFileSync(jsonPath))},csv:{bytes:fs.statSync(csvPath).size,sha256:sha(fs.readFileSync(csvPath))}});
  const full={...large.request,selection:{kind:'all'},expected_generation:null,expected_snapshot_token:null,cursor:null};await cliRun('large-json-refused',full,false,false);await cliRun('large-csv-refused',full,true,false);
}
async function mcpCoverage(view){
  const requireSdk=createRequire(path.join(root,'scripts/package.json')),sdkPackage=path.join(root,'scripts/node_modules/@modelcontextprotocol/client/package.json'),clientPath=requireSdk.resolve('@modelcontextprotocol/client'),stdioPath=requireSdk.resolve('@modelcontextprotocol/client/stdio');
  assert.equal(JSON.parse(fs.readFileSync(sdkPackage)).version,'2.3.1');for(const file of[sdkPackage,clientPath,stdioPath])pins.push({path:file,sha256:sha(fs.readFileSync(file))});
  const {Client}=await import(pathToFileURL(clientPath).href),{StdioClientTransport}=await import(pathToFileURL(stdioPath).href);let owner;
  class OwnedStdio extends StdioClientTransport{async start(){verifyPins();const started=super.start();const child=this._process;assert(child&&child.stdin,'Pinned SDK2.3.1 owns its observed native child');owner={phase:'mcp-stdio',child,stderr:''};consumerChildren.push(owner);owner.closed=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));if(this.stderr)this.stderr.on('data',b=>{owner.stderr+=b;});await started;}}
  const transport=new OwnedStdio({command:binary,args:['mcp','--server',origin],cwd:root,stderr:'pipe',maxBufferSize:4*1024*1024,env:{OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed'}}),client=new Client({name:'repeated-metadata-owned-stdio',version:'0.1.0'});
  try{
    await client.connect(transport);const tool=(await client.listTools()).tools.find(t=>t.name==='spider_segments');assert(tool);assert.equal(tool.annotations.readOnlyHint,true);assert.equal(tool.annotations.openWorldHint,false);
    const firstDup=view.issues.findIndex(i=>i.code==='description_duplicate');assert(firstDup>=0);assert(view.issues.slice(firstDup,firstDup+200).every(i=>i.code==='description_duplicate'));
    const cases=[['first200_pages','pages',0,200],['last200_pages','pages',view.ordinals.length-200,200],['first8_issues','issues',0,8],['last8_issues','issues',view.issues.length-8,8],['eight_duplicate_issues','issues',firstDup,8]];
    for(const[label,kind,offset,limit]of cases){const request={...view.request,view:kind,limit,cursor:offset?{offset,view:kind,snapshot_token:view.context.snapshot_token}:null,expected_snapshot_token:view.context.snapshot_token},oracle=await standaloneRead(view,request);assert(oracle.proof.bytes<1024*1024);const result=await client.callTool({name:'spider_segments',arguments:{id:sourceId(view.source),action:'preview',request}});assert.equal(result.isError,false,JSON.stringify(result));assert.deepEqual(result.structuredContent,oracle.value);assert.equal(result.content.length,1);assert.equal(result.content[0].type,'text');assert.deepEqual(JSON.parse(result.content[0].text),oracle.value);consumerProofs.push({consumer:'actual_normal_mcp_stdio',label,tool:'spider_segments',...oracle.proof});}
    const oversized={...view.request,view:'issues',limit:200,cursor:{offset:firstDup,view:'issues',snapshot_token:view.context.snapshot_token},expected_snapshot_token:view.context.snapshot_token},oracle=await standaloneRead(view,oversized);assert(oracle.proof.bytes>1024*1024&&oracle.proof.bytes<PAGE_CAP);
    const refused=await client.callTool({name:'spider_segments',arguments:{id:sourceId(view.source),action:'preview',request:oversized}});assert.equal(refused.isError,true);assert.equal(refused.structuredContent,undefined);assert.equal(refused.content.length,1);assert.match(refused.content[0].text,/MCP segment result exceeds1MiB/);consumerProofs.push({consumer:'actual_normal_mcp_stdio',label:'200_duplicate_issues_refused_at_unchanged1MiB',isError:true,native_oracle:oracle.proof,error:refused.content[0].text});
  }finally{
    if(owner&&owner.child.exitCode===null){owner.child.stdin.end();owner.shutdown=await bounded(owner.closed,'native MCP stdin EOF');assert.deepEqual(owner.shutdown,{code:0,signal:null},owner.stderr);}else if(owner&&!owner.shutdown)owner.shutdown=await owner.closed;
    await client.close();if(owner)assert.deepEqual(owner.shutdown,{code:0,signal:null},owner.stderr);verifyPins();
  }
}
async function probeWitness(){const response=await fetch(siteOrigin+'/owned-witness-probe',{headers:{'x-owned-witness-control':nonce},redirect:'error'});assert.equal(response.status,410);assert.equal(await response.text(),'Owned offline witness');}

(async()=>{
  try{
    website=http.createServer((request,response)=>{
      if(offline){(request.url==='/owned-witness-probe'&&request.headers['x-owned-witness-control']===nonce?witnessControls:witnessHits).push(request.url);response.writeHead(410,{'content-type':'text/plain'});response.end('Owned offline witness');return;}
      websiteHits.set(request.url,(websiteHits.get(request.url)||0)+1);const product=productPaths.includes(request.url),valid=websitePaths.includes(request.url),title=product||request.url==='/products/hidden'?'Owned product '+request.url.slice(-4)+' – independent captured document':'Outside selected cohort '+request.url;
      response.writeHead(request.url==='/missing'||!valid?404:200,{'content-type':'text/html; charset=utf-8'});response.end(`<html lang="en"><head>${request.url==='/untitled'?'':`<title>${title}</title>`}<meta name="description" content="${description}">${request.url==='/products/hidden'?'<meta name="robots" content="noindex">':''}</head><body><main><h1>${title}</h1><p>Genuinely captured owned native dashboard segmentation fixture. No provider or public website.</p>${product?'<a href="/outside">Captured target outside selected products</a>':request.url==='/outside'?'<a href="/products/0000">Genuine captured incoming link</a>':''}</main></body></html>`);
    });
    await new Promise((resolve,reject)=>{website.once('error',reject);website.listen(0,'127.0.0.1',resolve);});siteOrigin='http://127.0.0.1:'+website.address().port;
    const first=await launch('first');await assetPins('first');nativeId=(await api('/api/crawls',{seed_urls:websitePaths.map(s=>siteOrigin+s),crawl_links:false,discover_sitemaps:false,obey_robots:false,delay_ms:0,concurrency:16,directive_reporting:{version:1,respect_noindex:true,respect_canonical:false,respect_next_prev:false}})).value.id;
    await until(async()=>{const job=(await api('/api/crawls/'+nativeId)).value;assert.notEqual(job.status,'failed',job.error);return job.status==='completed';},'1010 repeated-metadata genuine captures');
    {const saved=await raw(`/api/crawls/${nativeId}/project.json`);report=JSON.parse(saved.bytes);rawProjectProof={bytes:saved.bytes.length,sha256:sha(saved.bytes)};fs.writeFileSync(artifactPath('original-raw-project.json'),saved.bytes);}
    sourceIds.set(report,nativeId);const nativeMetadata=(await api('/api/crawls/'+nativeId)).value;assert.equal(nativeMetadata.generation,1);sourceGenerations.set(report,nativeMetadata.generation);validateSource();capturedHits=[...websiteHits.values()].reduce((sum,n)=>sum+n,0);assert.equal(capturedHits,1010);assert([...websiteHits.values()].every(n=>n===1));offline=true;
    const storedPath=path.join(store,nativeId+'.json');storedReportProof={path:storedPath,bytes:fs.statSync(storedPath).size,sha256:sha(fs.readFileSync(storedPath))};assert(storedReportProof.bytes>PAGE_CAP&&storedReportProof.bytes<CAP,'Positive source genuinely exceeds32MiB but fits64MiB');
    tail=report.pages.map((page,index)=>({page,index})).filter(({page,index})=>index>1000&&productPaths.includes(new URL(page.url).pathname)).at(-1);assert(tail);rawAnalysis=(await api(`/api/crawls/${nativeId}/analysis`)).value;
    rules={version:1,definitions:[{id:'s1',name:'Alle1005Produkte mit vollständigen Duplikatbefunden',join:'all',conditions:[{field:'requested_url',operator:'regex',value:'/products/[0-9]{4}$',case_insensitive:false}]},{id:'s2',name:'Originalbeobachtung hinter1000',join:'all',conditions:[{field:'requested_url',operator:'equals',value:tail.page.url,case_insensitive:false}]}]};
    const executablePath=process.env.OPEN_SEO_BROWSER_PATH||process.env.SPIDER_CHROME;assert(executablePath);browser=await chromium.launch({headless:true,executablePath});context=await browser.newContext({acceptDownloads:true,viewport:{width:1440,height:1000}});
    await context.route('**/*',route=>{if(!['about:','data:','blob:'].includes(new URL(route.request().url()).protocol)&&new URL(route.request().url()).origin!==origin){external.push(route.request().url());return route.abort();}return route.continue();});
    page=await context.newPage();observe(page);await page.goto(origin);await page.locator('#connection-state.connected').waitFor();await selectCrawl(nativeId);await authorRules(rules.definitions);await loadRules();
    assert.equal(await page.locator('#dashboard-segment-export-kind').inputValue(),'pages');assert.equal(await page.locator('#dashboard-segment-export-kind option').count(),13);
    const productOrdinals=report.pages.map((p,i)=>({p,i})).filter(({p})=>productPaths.includes(new URL(p.url).pathname)).map(({i})=>i),large=await apply('segment:s1',report,productOrdinals,'warm-large');
    assert.equal(large.issues.length,7035);assert.equal(large.issues.reduce((sum,issue)=>sum+issue.related_urls.length,0),1012035);assert.deepEqual(projections[0].page_sizes,[200,200,200,200,200,5]);assert.deepEqual(projections[0].issue_sizes,[...Array(35).fill(200),35]);await exactDOM(large);
    firstLargeProof=await attachment('json','warm-complete-cohort',large);firstIssueCSVProof=await attachment('issues','warm-all-issues',large);assert(firstLargeProof.bytes>PAGE_CAP&&firstLargeProof.bytes<CAP);assert(firstIssueCSVProof.bytes>PAGE_CAP&&firstIssueCSVProof.bytes<CAP);validatorControls(large);recordPhase('warm-dom-full-exports-and-validator-controls');
    await standaloneCoverage(large);recordPhase('standalone-all-cursors-and-full-export-refusals');await mcpCoverage(large);recordPhase('actual-mcp-previews-refusal-and-eof');
    for(const[field,value]of[['expected_snapshot_token','0'.repeat(64)],['expected_generation',large.context.app_generation+1]]){const refused=await api(`/api/crawls/${nativeId}/segments/dashboard.json`,{...large.request,cursor:null,expected_snapshot_token:large.context.snapshot_token,expected_generation:large.context.app_generation,[field]:value},409);assert.equal(refused.response.headers.get('content-disposition'),null);assert.equal(refused.response.headers.get('x-content-sha256'),null);guards.push({field,status:409,code:refused.value.code});}
    checks.push('1010 genuine captures produce1008 repeated-description diagnoses and1015056 ordered relatedURL occurrences; all1005 selected pages and7035issues/1012035relatedURLs survive complete200-row native paging, every100-row browser page and actual fullJSON/nativeissueCSV above32MiB below64MiB');
    await page.click('#dashboard-segment-clear');await noApplied();assert.equal(await page.locator('#metric-fetched').textContent(),'1.010');const one=await apply('segment:s2',report,[tail.index],'warm-tail');await tailInspector(one);assert(tail.index>1000);await cliCoverage(one,large);recordPhase('actual-cli-full-tail-and-all-export-refusals');
    checks.push('Actual normal CLI tailJSON/CSV preserve all1007relatedURLs, full32MiB exports refuse without partialfiles; actual SDK2.3.1 stdio MCP returns exact bounded page/issue previews and explicitly refuses200duplicateissues above1MiB; standaloneAPI all42cursors remain complete with32MiBfull-export refusal');
    // Late duplicate URL control: these rows are declared imports, not new HTTP
    // observations. The omitted last duplicate must never replace ordinal1001.
    const others=report.pages.filter(p=>p.url!==tail.page.url).slice(0,1001),declared={...report,pages:[...others,{...tail.page,titles:['Declared selected tail200']},{...tail.page,status:404,final_status:404,titles:['Declared omitted tail404']}],issues:report.issues.filter(i=>i.url===tail.page.url)};
    const importedPending=page.waitForResponse(r=>r.url()===origin+'/api/crawls/import'&&r.request().method()==='POST').then(async r=>({status:r.status(),bytes:await r.body()}));await page.setInputFiles('#import-file',{name:'declared-duplicate-tail.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(declared))});const imported=await importedPending;assert.equal(imported.status,200,imported.bytes.subarray(0,2000).toString());const declaredId=JSON.parse(imported.bytes).id;
    const declaredSaved=(await api(`/api/crawls/${declaredId}/project.json`)).value;sourceIds.set(declaredSaved,declaredId);const importedMetadata=(await api('/api/crawls/'+declaredId)).value;assert.equal(importedMetadata.generation,0);sourceGenerations.set(declaredSaved,importedMetadata.generation);await page.waitForFunction(id=>document.getElementById('crawl-history').value===id,declaredId);await noApplied();
    const controlRules={version:1,definitions:[{id:'s1',name:'Deklarierte späte200Beobachtung',join:'all',conditions:[{field:'requested_url',operator:'equals',value:tail.page.url,case_insensitive:false},{field:'status',operator:'equals',value:'200',case_insensitive:false}]}]};await authorRules(controlRules.definitions);await loadRules();const control=await apply('segment:s1',declaredSaved,[1001],'declared-duplicate-tail',controlRules);await tailInspector(control);await page.click('#inspector-tabs [data-tab="details"]');assert.match(await page.locator('#inspector-content').textContent(),/Declared selected tail200/);assert(!/Declared omitted tail404/.test(await page.locator('#inspector-content').textContent()));
    const crossed=await api(`/api/crawls/${declaredId}/segments/dashboard.json`,{...control.request,cursor:null,expected_snapshot_token:large.context.snapshot_token},409);assert.equal(crossed.response.headers.get('content-disposition'),null);guards.push({field:'cross_job_snapshot',status:409,code:crossed.value.code});
    await selectCrawl(nativeId);await noApplied();await authorRules(rules.definitions);await loadRules();await editor.close();editor=null;recordPhase('declared-late-duplicate-and-source-context-guards');
    checks.push('Clear and crawl switch invalidate exports; tailoriginalordinal>1000 exposes all1007 relatedURLs in the inspector; separately imported duplicateURL at lastordinal cannot replace selected declaredordinal1001, and token/generation/cross-job exports refuse409');
    offline=true;await probeWitness();await stop(first);const second=await launch('cold',new URL(origin).host);assert.equal(second.origin,first.origin);assert.notEqual(second.child.pid,first.child.pid);await assetPins('cold');
    await page.reload();await page.locator('#connection-state.connected').waitFor();await selectCrawl(nativeId);await noApplied();await loadRules();const cold=await apply('segment:s1',report,productOrdinals,'cold-large');assert.deepEqual(cold.context,large.context);await exactDOM(cold);
    const coldJSON=await attachment('json','cold-complete-cohort',cold),coldCSV=await attachment('issues','cold-all-issues',cold);assert(fs.readFileSync(coldJSON.path).equals(fs.readFileSync(firstLargeProof.path)));assert(fs.readFileSync(coldCSV.path).equals(fs.readFileSync(firstIssueCSVProof.path)));recordPhase('cold-all-dom-and-byte-identical-full-exports');
    await page.setViewportSize({width:320,height:900});for(const selector of['#dashboard-segment-export-kind','#dashboard-segment-csv']){const element=page.locator(selector);await element.scrollIntoViewIfNeeded();const box=await element.boundingBox();assert(box&&box.x>=0&&box.x+box.width<=320,selector+' reachable at320');}assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:artifactPath('repeated-metadata-320.png')});await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:artifactPath('repeated-metadata-1440.png')});
    {const original=await raw(`/api/crawls/${nativeId}/project.json`);assert.equal(original.bytes.length,rawProjectProof.bytes);assert.equal(sha(original.bytes),rawProjectProof.sha256);assert(original.bytes.equals(fs.readFileSync(artifactPath('original-raw-project.json'))));}assert.equal(sha(fs.readFileSync(storedReportProof.path)),storedReportProof.sha256);
    await drainReads();assert.deepEqual(errors,[]);assert.deepEqual(external,[]);assert.deepEqual(witnessHits,[]);await browser.close();browser=null;await stop(second);await probeWitness();assert.deepEqual(witnessHits,[]);assert.equal(witnessControls.length,2);assert.equal([...websiteHits.values()].reduce((sum,n)=>sum+n,0),capturedHits);assert.equal(negativeControls.length,4);
    checks.push('Cold explicitlarge reapplication after two naturalEOF0 processes at the same ownedorigin/store reproduces everyrow and byte-identical fullJSON/issueCSV;410 witness covers both closes with0refetches+2controls; originalreportbytes, binary and driverpins remain unchanged');
  }catch(error){failure=error;}
  finally{
    if(page&&failure)try{await page.screenshot({path:artifactPath('repeated-metadata-failure.png')});}catch{}
    if(browser)await browser.close();for(const owner of consumerChildren)if(!owner.shutdown&&owner.child.exitCode===null){try{if(owner.child.stdin?.writable)owner.child.stdin.end();else owner.child.kill();owner.shutdown=await bounded(owner.closed,'consumer cleanup');}catch(error){failure||=error;if(owner.child.exitCode===null)owner.child.kill();}}
    for(const owner of owners)if(!owner.shutdown&&owner.child.exitCode===null){try{await stop(owner);}catch(error){failure||=error;if(owner.child.exitCode===null)owner.child.kill();}}
    if(website?.listening){website.closeAllConnections();await new Promise(resolve=>website.close(resolve));}for(const owner of owners)fs.writeFileSync(artifactPath('native-'+owner.phase+'.log'),owner.log);try{verifyPins();}catch(error){failure||=error;}
    fs.writeFileSync(artifactPath('segments-repeated-metadata-live.json'),JSON.stringify({result:failure?'FAIL':'PASS',failure:failure?.stack,checks,binary_sha256:binarySha,pins,served_assets:assets,native:owners.map(o=>({phase:o.phase,pid:o.child.pid,origin:o.origin,shutdown:o.shutdown})),consumer_children:consumerChildren.map(o=>({phase:o.phase,pid:o.child.pid,shutdown:o.shutdown})),actual_consumers:consumerProofs,standalone_api_cursors:standaloneProofs,native_crawl_id:nativeId,native_pages:report?.pages.length,native_issues:report?.issues.length,tail_original_ordinal:tail?.index,raw_project:rawProjectProof,stored_report:storedReportProof,website_capture_requests:capturedHits,offline_product_requests:witnessHits,witness_controls:witnessControls,projections,source_guards:guards,negative_controls:negativeControls,exports:downloads,read_only_native_oracles:reads,errors,external_requests:external,scope:'Actual1010-page shared-description capture, complete ordered issueevidence, bounded32MiB pages/64MiB dashboardexports and explicit two-process cold reapplication. Actual normal CLI/MCP consumers keep their distinct32MiB/1MiB limits. Imported duplicateURL control is declared. No capraise, evidence truncation, server-side rules, crash recovery, all operators, provider access, installed app, broad browser matrix, performance or manufacturer parity claim.'},null,2)+'\n');
  }
  if(failure)throw failure;for(const check of checks)console.log('PASS '+check);console.log('Evidence: '+output);
})().catch(error=>{console.error(error.stack);process.exitCode=1;});
