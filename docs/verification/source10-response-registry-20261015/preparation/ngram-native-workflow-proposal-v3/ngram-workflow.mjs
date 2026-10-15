// Source-only native acceptance helper. The owning wrapper supplies already bound
// HTTP/CLI/browser capabilities and owns every process, deadline, pipe and close.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

const sha=b=>createHash('sha256').update(b).digest('hex');
const phrase=['a','b','c','d','e','f'];
const key=tokens=>JSON.stringify(tokens);
const requestBase={source_kind:'raw_http_html',options:{max_n:6},view:'totals',limit:200};
const save=(p,v)=>fs.writeFileSync(p,typeof v==='string'||Buffer.isBuffer(v)?v:JSON.stringify(v,null,2)+'\n',{flag:'wx'});

// Independent reference for the fixed Latin/German fixture, without importing
// product code or using any returned aggregate as the expected answer.
function reference(project,stopwords=[]){
  const totals=Array.from({length:6},()=>new Map()),sources=[];
  for(const page of project.pages){
    const captures=page.content_captures?.filter(c=>c.source_kind==='raw_http_html');
    assert.equal(captures?.length,1);const capture=captures[0],r=capture.result;
    assert(r&&['complete','empty'].includes(r.status));assert.equal(r.source_complete,true);
    assert.equal(sha(Buffer.from(r.text)),r.text_sha256);
    // Fixture has no leading/standalone combining marks or Unicode expansions.
    const tokens=(r.text.match(/[\p{L}\p{N}\p{M}]+/gu)||[]).map(t=>t.toLowerCase());
    assert(!tokens.some(t=>/^\p{M}/u.test(t)));
    const own=Array.from({length:6},()=>new Map());
    for(let n=1;n<=6;n++)for(let at=0;at+n<=tokens.length;at++){
      const window=tokens.slice(at,at+n);if(window.some(t=>stopwords.includes(t)))continue;
      const k=key(window),row=own[n-1].get(k)||{tokens:window,occurrences:0,positions:[]};row.occurrences++;row.positions.push(at+1);own[n-1].set(k,row);
    }
    for(let n=1;n<=6;n++)for(const[k,row]of own[n-1]){
      const t=totals[n-1].get(k)||{tokens:row.tokens,occurrences:0,document_frequency:0};t.occurrences+=row.occurrences;t.document_frequency++;totals[n-1].set(k,t);
    }
    sources.push({url:page.url,capture,own});
  }
  return{totals,sources};
}
function parseCsv(bytes){
  const rows=[];let row=[],value='',quoted=false;const s=bytes.toString('utf8');
  for(let i=0;i<s.length;i++){const c=s[i];if(quoted){if(c==='"'&&s[i+1]==='"'){value+='"';i++;}else if(c==='"')quoted=false;else value+=c;}else if(c==='"')quoted=true;else if(c===','){row.push(value);value='';}else if(c==='\n'){row.push(value.replace(/\r$/,''));rows.push(row);row=[];value='';}else value+=c;}
  assert.equal(quoted,false);assert.equal(value,'');assert.equal(row.length,0);const header=rows.shift();assert(header&&new Set(header).size===header.length);return rows.map(cells=>{assert.equal(cells.length,header.length);return Object.fromEntries(header.map((h,i)=>[h,cells[i]]));});
}
function verifyDescriptor(response){assert.equal(Number(response.headers['content-length']),response.bytes.length);assert.equal(response.headers['x-content-sha256'],sha(response.bytes));}
async function previewAll(request,id,initial){
  const pages=[],rows=[];let q={...initial};let first=null;
  for(;;){const response=await request('/api/crawls/'+id+'/ngrams/preview',q);verifyDescriptor(response);const v=response.value();first??=v;assert.equal(v.snapshot.snapshot_token,first.snapshot.snapshot_token);assert.equal(v.website_requests,0);assert.equal(v.whole_website_coverage_attested,false);assert.equal(v.scope,'all_saved_requested_page_rows_selected_text');assert.equal(v.after,rows.length);pages.push(v.rows.length);rows.push(...v.rows);assert(rows.length<=v.total_rows);if(!v.has_more){assert.equal(v.next_after,null);assert.equal(rows.length,v.total_rows);break;}assert.equal(v.next_after,rows.length);q={...initial,after:v.next_after,expected_snapshot_token:first.snapshot.snapshot_token,expected_generation:first.snapshot.caller.generation};}
  return{first,rows,pages};
}
function compareCsv(bytes,expected){
  const rows=parseCsv(bytes);assert.equal(rows.length,205);assert.equal(new Set(rows.map(r=>r.requested_url)).size,205);
  const byUrl=new Map(expected.rows.map(r=>[r.source.requested_url,r]));
  for(const r of rows){const want=byUrl.get(r.requested_url);assert(want);assert.equal(r.view,'occurrences');assert.equal(r.n,'6');assert.deepEqual(JSON.parse(r.tokens_json),phrase);assert.equal(r.occurrences,'2');assert.equal(r.website_requests,'0');assert.equal(r.text_sha256,want.source.text_sha256);assert.equal(r.source_revision,want.source.source_revision);assert.deepEqual(JSON.parse(r.first_positions_json),want.first_positions);assert.equal(r.positions_sampled,'false');}
  return rows;
}

export async function runNgramWorkflow({request,job,project,output,unchanged,cli,browser,api}){
  const cases=[],record=(name,evidence)=>cases.push({name,result:'PASS_ACTUAL',evidence}),ref=reference(project);
  assert.equal(ref.sources.length,206);const controlled=ref.totals[5].get(key(phrase));assert.deepEqual(controlled,{tokens:phrase,occurrences:410,document_frequency:205});
  const q={...requestBase,expected_generation:job.generation};
  const totals=[];for(let n=1;n<=6;n++){const all=await previewAll(request,job.id,{...q,n});assert.equal(all.first.saved_documents,206);assert.equal(all.first.all_supplied_sources_complete,true);assert.equal(all.rows.length,ref.totals[n-1].size);const actualKeys=all.rows.map(row=>key(row.tokens));assert.equal(new Set(actualKeys).size,actualKeys.length);assert.deepEqual(actualKeys.slice().sort(),[...ref.totals[n-1].keys()].sort());for(const row of all.rows){const want=ref.totals[n-1].get(key(row.tokens));assert(want);assert.equal(row.n,n);assert.equal(row.occurrences,want.occurrences);assert.equal(row.document_frequency,want.document_frequency);assert.equal(row.source_memberships_complete,true);}totals.push({n,rows:all.rows.length,pages:all.pages,snapshot:all.first.snapshot});}
  unchanged('All actual1..6 Ngram totals');record('genuine206_saved_captures_all_six_ngram_totals_match_independent_reference',{totals,known_sixgram:controlled});
  const occurrenceRequest={...q,view:'occurrences',n:6,exact_tokens:phrase};const occurrence=await previewAll(request,job.id,occurrenceRequest);assert.deepEqual(occurrence.pages,[200,5]);
  const expectedSources=ref.sources.filter(s=>s.own[5].has(key(phrase)));assert.equal(expectedSources.length,205);const byUrl=new Map(expectedSources.map(s=>[s.url,s]));
  for(const r of occurrence.rows){const s=byUrl.get(r.source.requested_url);assert(s);assert.equal(r.occurrences,2);assert.equal(r.source.counted,true);assert.equal(r.source.text_sha256,s.capture.result.text_sha256);assert.equal(r.source.source_revision,s.capture.source_revision);assert.deepEqual(r.first_positions,s.own[5].get(key(phrase)).positions);assert.equal(r.positions_sampled,false);}
  const csvQ={...occurrenceRequest,expected_snapshot_token:occurrence.first.snapshot.snapshot_token};const csv=await request('/api/crawls/'+job.id+'/ngrams/csv',csvQ,200,true);verifyDescriptor(csv);assert.equal(csv.headers['x-ngram-snapshot'],csvQ.expected_snapshot_token);compareCsv(csv.bytes,occurrence);save(path.join(output,'ngram-api-full.csv'),csv.bytes);unchanged('Complete205 source Ngram export');record('actual205_occurrences_200_plus5_two_positions_each_and_full_csv',{pages:occurrence.pages,rows:205,occurrences:410,csv_bytes:csv.bytes.length,csv_sha256:sha(csv.bytes)});
  for(const changed of [{query:'changed'},{limit:199},{n:5,exact_tokens:null},{options:{max_n:5},n:5,exact_tokens:null},{expected_generation:job.generation+1}])await request('/api/crawls/'+job.id+'/ngrams/preview',{...csvQ,...changed},409);
  await request('/api/crawls/'+job.id+'/ngrams/preview',{...occurrenceRequest,after:200},422);
  await request('/api/crawls/'+job.id+'/ngrams/csv',occurrenceRequest,400,true);
  await request('/api/crawls/'+job.id+'/ngrams/preview',{...q,n:7},422);
  const dom=await previewAll(request,job.id,{...q,source_kind:'rendered_dom_html',view:'sources'});assert.deepEqual(dom.pages,[200,6]);assert.equal(dom.first.all_supplied_sources_complete,false);assert(dom.rows.every(r=>r.source.counted===false));
  const domTotals=await previewAll(request,job.id,{...q,source_kind:'rendered_dom_html',n:6});assert.equal(domTotals.rows.length,0);assert.equal(domTotals.first.all_supplied_sources_complete,false);
  const stopped=await previewAll(request,job.id,{...q,n:6,exact_tokens:phrase,options:{max_n:6,stopwords:['b']}});assert.equal(stopped.rows.length,0);unchanged('Stale filters, omitted DOM, stopword nonbridging');record('strict_snapshot_filters_generation_range_unknown_dom_and_stopword_boundaries',{rejected_stale:5,missing_cursor:422,missing_csv_snapshot:400,invalid_n:422,unknown_dom_sources:206,stopword_phrase_rows:0});
  const reportPath=path.join(output,'genuine-project.json'),optionsPath=path.join(output,'ngram-cli-request.json'),jsonPath=path.join(output,'ngram-cli-page.json'),csvPath=path.join(output,'ngram-cli-full.csv');const reportBefore=fs.readFileSync(reportPath);assert.deepEqual(JSON.parse(reportBefore),project);const cliQ={...requestBase,view:'occurrences',n:6,exact_tokens:phrase};save(optionsPath,cliQ);const optionsBefore=fs.readFileSync(optionsPath);
  const cliClose=await cli('cli-ngram',['saved-ngrams',reportPath,'--options',optionsPath,'--output',jsonPath,'--csv',csvPath],0);const cliPage=JSON.parse(fs.readFileSync(jsonPath));assert.equal(cliPage.total_rows,205);assert.equal(cliPage.rows.length,200);assert.equal(cliPage.website_requests,0);assert.deepEqual(cliPage.rows,occurrence.first.rows);compareCsv(fs.readFileSync(csvPath),occurrence);
  const refused=path.join(output,'ngram-protected-new.csv'),protectedClose=await cli('cli-ngram-protected',['saved-ngrams',reportPath,'--options',optionsPath,'--output',optionsPath,'--csv',refused],1);assert(!fs.existsSync(refused));assert(fs.readFileSync(reportPath).equals(reportBefore));assert(fs.readFileSync(optionsPath).equals(optionsBefore));unchanged('Actual compiled Ngram CLI and protected input refusal');record('actual_cli_full205_csv_and_200_json_preserves_report_and_options',{cli_close:cliClose,protected_close:protectedClose,csv_sha256:sha(fs.readFileSync(csvPath))});
  const ui=await browserWorkflow({browser,api,job,output,occurrence,unchanged});record('actual_ngram_browser_two_widths_complete_sources_csv_json_and_unknown_dom',ui);
  return{cases,expected_response_producers:{warm:3,cold:1,basis:'Warm one positive API CSV plus two actual UI CSVs; cold one positive API CSV. Missing-token400 is refused before registration; CLI CSV uses separate process output and JSON/downloads add none.'},cold_expected:{job_id:job.id,request:occurrenceRequest,rows:occurrence.rows,typed_report_sha256:occurrence.first.snapshot.caller.typed_report_sha256,csv_rows:205},qualification:'Actual supplied saved requested rows and fixed fixture. No title/headings/inlink/alt/unlinked channels, word cloud, cross-request performance or manufacturer parity claim.'};
}

async function browserWorkflow({browser,api,job,output,occurrence,unchanged}){
  const context=await browser.newContext({acceptDownloads:true,viewport:{width:1440,height:1000}}),page=await context.newPage(),errors=[],posts=[],external=[],widths=[];page.setDefaultTimeout(30000);
  page.on('pageerror',e=>errors.push(String(e)));page.on('request',r=>{if(!r.url().startsWith(api+'/'))external.push(r.url());if(r.method()==='POST')posts.push({url:r.url(),body:r.postDataJSON()});});
  const ready=()=>page.waitForFunction(()=>!document.querySelector('#preview').disabled&&!document.querySelector('#results').hidden);
  const bounds=async()=>{const b=await page.evaluate(()=>({width:innerWidth,doc:document.documentElement.scrollWidth,body:document.body.scrollWidth,bad:[...document.querySelectorAll('main button,main input,main select,main textarea,main article,main h1,main dl')].filter(e=>e.getClientRects().length).map(e=>({id:e.id,tag:e.tagName,b:e.getBoundingClientRect().toJSON()})).filter(x=>x.b.left< -1||x.b.right>innerWidth+1)}));assert(b.doc>0&&b.doc<=b.width+1,'Ngram document horizontal overflow');assert(b.body>0&&b.body<=b.width+1,'Ngram body horizontal overflow');assert.deepEqual(b.bad,[]);return b;};
  try{for(const width of[1440,320]){
    await page.setViewportSize({width,height:1000});await page.goto(api+'/saved-ngrams',{waitUntil:'networkidle'});await page.waitForFunction(id=>[...document.querySelector('#crawl').options].some(o=>o.value===id)&&!document.querySelector('#preview').disabled,job.id);await page.locator('#crawl').selectOption(job.id);await page.locator('#n').selectOption('6');await page.locator('#query').fill('a b c d e f');await page.locator('#preview').click();await ready();assert.equal(await page.locator('#rows article').count(),1);assert((await page.locator('#rows article dl').innerText()).includes('410'));assert((await page.locator('#rows article dl').innerText()).includes('205'));await bounds();await page.locator('#rows article button').click();await ready();
    const urls=[],pages=[];for(const count of[100,100,5]){assert.equal(await page.locator('#rows article').count(),count);const details=await page.locator('#rows article details pre').allTextContents();for(const s of details){const r=JSON.parse(s);assert.equal(r.occurrences,2);assert.deepEqual(r.tokens,phrase);urls.push(r.source.requested_url);}pages.push(count);await bounds();if(count!==5){await page.locator('#next').click();await ready();}}
    assert.deepEqual(urls,occurrence.rows.map(r=>r.source.requested_url));assert(await page.locator('#next').isHidden());
    const downloadEvent=page.waitForEvent('download');await page.locator('#csv').click();const download=await downloadEvent;assert.equal(await download.failure(),null);const csvPath=path.join(output,'ngram-ui-'+width+'.csv');await download.saveAs(csvPath);compareCsv(fs.readFileSync(csvPath),occurrence);
    const jsonEvent=page.waitForEvent('download');await page.locator('#json').click();const jsonDownload=await jsonEvent;assert.equal(await jsonDownload.failure(),null);const jsonPath=path.join(output,'ngram-ui-'+width+'.json');await jsonDownload.saveAs(jsonPath);const json=JSON.parse(fs.readFileSync(jsonPath));assert.equal(json.rows.length,5);assert.equal(json.total_rows,205);assert.equal(json.after,200);
    await page.locator('#source').selectOption('rendered_dom_html');await page.locator('#view').selectOption('sources');await page.locator('#preview').click();await ready();assert.equal(await page.locator('#rows article').count(),100);const unknown=await page.locator('#rows article details pre').allTextContents();assert(unknown.every(s=>JSON.parse(s).source.counted===false));assert((await page.locator('#coverage').innerText()).includes('ausgeschlossen'));await bounds();await page.screenshot({path:path.join(output,'ngram-unknown-'+width+'.png'),fullPage:false});unchanged('Ngram UI actual saved views '+width);widths.push({width,pages,urls:205,csv_sha256:sha(fs.readFileSync(csvPath)),json_after:json.after,unknown_dom_sources_shown:100});
  }assert.deepEqual(errors,[]);assert.deepEqual(external,[]);assert(posts.length>0);assert(posts.every(p=>p.url.startsWith(api+'/api/crawls/'+job.id+'/ngrams/')));return{widths,page_errors:errors,external_requests:external,actual_api_posts:posts,crawl_start_posts:0};}finally{await context.close();}
}

export async function runColdNgramWorkflow({request,job,expected,output,unchanged}){
  assert.equal(job.id,expected.job_id);const all=await previewAll(request,job.id,{...expected.request,expected_generation:job.generation});assert.deepEqual(all.pages,[200,5]);assert.deepEqual(all.rows,expected.rows);assert.equal(all.first.snapshot.caller.typed_report_sha256,expected.typed_report_sha256);
  const csv=await request('/api/crawls/'+job.id+'/ngrams/csv',{...expected.request,expected_generation:job.generation,expected_snapshot_token:all.first.snapshot.snapshot_token},200,true);verifyDescriptor(csv);compareCsv(csv.bytes,all);save(path.join(output,'ngram-cold-full.csv'),csv.bytes);unchanged('Cold Ngram205rows and fullCSV without website IO');return{name:'cold_saved_ngram_reopen_same206_sources_205_memberships_and_full_csv_zero_website_requests',result:'PASS_ACTUAL',evidence:{pages:all.pages,rows:205,typed_report_sha256:all.first.snapshot.caller.typed_report_sha256,csv_sha256:sha(csv.bytes)}};
}
