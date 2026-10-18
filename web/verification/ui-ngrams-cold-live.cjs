'use strict';
// Actual saved text -> browser analysis/export -> two native processes, one owned store.
// Browser form state is document-local; this does not claim server/localStorage options.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const {spawn}=require('node:child_process'),{createHash,randomBytes}=require('node:crypto');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const artifactPath=require('./artifact-path.cjs');
const root=path.resolve(__dirname,'../..');
assert(process.env.SPIDER_BINARY,'Explicit SPIDER_BINARY is required; this test never discovers, builds or installs a native app');
const binary=path.resolve(process.env.SPIDER_BINARY);
const sha=bytes=>createHash('sha256').update(bytes).digest('hex'),binarySha=sha(fs.readFileSync(binary));
if(process.env.SPIDER_EXPECTED_BINARY_SHA)assert.equal(binarySha,process.env.SPIDER_EXPECTED_BINARY_SHA,'Reviewed binary identity');
const output=path.resolve(process.env.SPIDER_NGRAM_COLD_ARTIFACT_DIR||path.join(path.dirname(artifactPath('ngrams-cold-live.json')),`ngrams-cold-${Date.now()}`));
assert(!fs.existsSync(output),'Use a fresh cold-test output directory');fs.mkdirSync(output,{recursive:true});
const store=path.join(output,'native-store'),checks=[],errors=[],exportProof=[],native=[],readOracles=[],mutations=[],browserRequests=[],external=[];
const csvHeaderControls=[],controlledHeaders=new Set(),sourceInventoryControls=[];
const sourceFiles=['web/saved-ngrams.js','web/saved-ngrams.html','web/saved-ngrams.css','web/verification/ui-ngrams-cold-live.cjs','web/verification/artifact-path.cjs'];
const sourceHashes=Object.fromEntries(sourceFiles.map(name=>[name,sha(fs.readFileSync(path.join(root,name)))]));
const words=Array.from({length:100},(_,i)=>'term'+String(i).padStart(3,'0'));
const texts={a:'Alpha beta Alpha beta and gamma delta '+words.join(' '),b:'alpha beta and gamma',empty:'',partial:'ErrorPhrase is genuinely incomplete',budget:'BudgetPhrase must never be counted after truncation. '.repeat(20),metadata:'Body capture is deliberately disabled'};
const initialHits=[],offlineHits=[],controlHits=[],nonce=randomBytes(16).toString('hex');let offline=false,app='',activeHost,browser,page;
const website=http.createServer((req,res)=>{
  if(offline){const record={method:req.method,path:req.url};(req.headers['x-owned-witness-control']===nonce?controlHits:offlineHits).push(record);res.writeHead(410,{'content-type':'text/plain'});return res.end('Owned offline witness');}
  initialHits.push({method:req.method,path:req.url});const name=req.url.slice(1);
  if(name==='plain'){res.setHeader('content-type','text/plain');return res.end('Non-HTML capture unavailable');}
  if(!Object.hasOwn(texts,name)){res.statusCode=404;return res.end('Unknown fixture');}
  if(name==='partial')res.statusCode=404;
  res.setHeader('content-type','text/html;charset=utf-8');const title=name==='metadata'?'Saved Channel Title':'Saved '+name+' Title';
  res.end(`<html lang="en"><head><title>${title}</title><meta name="description" content="Meta Alpha Beta"></head><body><nav>ExcludedNavigation</nav><main>${texts[name]}</main><h1>Heading Alpha</h1></body></html>`);
});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(work,label){for(let i=0;i<1000;i++){if(activeHost&&(activeHost.child.exitCode!==null||activeHost.child.signalCode!==null))throw Error('Owned native process exited during '+label);const value=await work();if(value)return value;await sleep(30);}throw Error('Timeout: '+label);}
function check(label){checks.push(label);console.log('PASS '+label);}
function verifyBinary(){assert.equal(sha(fs.readFileSync(binary)),binarySha,'Native binary changed');}
async function launch(phase,listen='127.0.0.1:0'){
  verifyBinary();const env={...process.env};delete env.OPEN_SEO_PAGESPEED_API_KEY;
  const child=spawn(binary,['serve','--listen',listen,'--data-dir',store,'--shutdown-on-stdin-eof'],{cwd:root,windowsHide:true,stdio:['pipe','pipe','pipe'],env});
  let origin='',pending='',log='',spawnError;const closed=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));child.once('error',error=>spawnError=error);
  for(const stream of [child.stdout,child.stderr])stream.on('data',bytes=>log+=bytes);
  child.stdout.on('data',bytes=>{pending+=bytes;let end;while((end=pending.indexOf('\n'))>=0){const line=pending.slice(0,end).replace(/\r$/,'');pending=pending.slice(end+1);const match=/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)$/.exec(line);if(match){assert.equal(origin,'');origin=match[1];}}});
  let stopped=false;
  const host={child,stop:async()=>{if(stopped)return;stopped=true;child.stdin.end();const timer=setTimeout(()=>child.kill(),20000);const shutdown=await closed;clearTimeout(timer);fs.writeFileSync(path.join(output,`native-${phase}.log`),log);native.push({phase,origin,binary_sha256:binarySha,shutdown});if(activeHost===host)activeHost=null;assert.deepEqual(shutdown,{code:0,signal:null},'Native must close naturally through stdin EOF');verifyBinary();}};
  activeHost=host;
  try{await until(()=>{if(spawnError)throw spawnError;return origin;},'post-bind readiness line');assert(!['8787','8790'].includes(new URL(origin).port));assert.equal((await fetch(origin+'/healthz')).status,200);host.origin=origin;return host;}catch(error){await host.stop();throw error;}
}
async function fetchNative(route,body){const response=await fetch(app+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'content-type':'application/json'},body:body===undefined?undefined:JSON.stringify(body),cache:'no-store'});const bytes=Buffer.from(await response.arrayBuffer());assert.equal(response.status,200,bytes.toString());return {response,bytes,value:JSON.parse(bytes)};}
async function crawl(origin,paths,extra={}){
  const config={seed_urls:paths.map(p=>origin+'/'+p),crawl_links:false,discover_sitemaps:false,obey_robots:false,delay_ms:0,concurrency:1,content_capture:{sources:['raw_http_html'],include_selectors:['main']},...extra};
  const result=await fetchNative('/api/crawls',config);mutations.push({method:'POST',path:'/api/crawls',id:result.value.id});
  await until(async()=>{const job=(await fetchNative('/api/crawls/'+result.value.id)).value;assert.notEqual(job.status,'failed',job.error);return job.status==='completed';},'genuine capture completion');return result.value.id;
}
function headersMatch(headers,bytes){assert.equal(Number(headers['content-length']),bytes.length);assert.equal(headers['x-content-sha256'],sha(bytes));assert.equal(headers['cache-control'],'no-store');}
async function preview(trigger='#preview'){
  const pending=page.waitForResponse(r=>r.request().method()==='POST'&&r.url().startsWith(app+'/api/crawls/')&&r.url().endsWith('/ngrams/preview'));
  await page.click(trigger);const response=await pending;assert.equal(response.status(),200);
  // Always use the independent read-only native POST oracle. CDP bodies can be unavailable.
  const route=new URL(response.url()).pathname;assert(route.endsWith('/ngrams/preview'));const request=response.request().postDataJSON();
  const oracle=await fetchNative(route,request);readOracles.push({route,body:request,sha256:sha(oracle.bytes)});headersMatch(response.headers(),oracle.bytes);headersMatch(Object.fromEntries(oracle.response.headers),oracle.bytes);
  const value=oracle.value;assert.equal(value.website_requests,0);assert.equal(value.whole_website_coverage_attested,false);
  await exactDOM(value);return {page:value,request,bytes:oracle.bytes};
}
const shown=value=>value==null?'Nicht gespeichert':String(value);
function expectedArticle(row){
  let pairs;
  if(row.view==='totals')pairs=[['Wörter je Folge',row.n],['Vollständige Häufigkeit',row.occurrences],['Verschiedene Quellseiten',row.document_frequency]];
  else{const s=row.source;pairs=[['Quell-URL',s.requested_url],['Originalzeile im Rohbericht',s.raw_page_ordinal],['Dichter Korpusindex',s.ordinal],['Status / gezählt',`${s.state} / ${s.counted?'ja':'nein'}`],['Herkunft / Generation',`${s.basis} / ${s.observed_generation??'unbekannt'}`],['Quellenrevision',s.source_revision],['Text SHA-256',s.text_sha256],['Capture-Konfiguration',s.capture_config_sha256],['Kanalrevision (nur abgeleitete Analyse)',s.channel?.channel_revision],['Metadatenrevision',s.channel?.metadata_source?.revision],['Metadatenquelle',s.channel?.metadata_source?.source],['Original-Capturestatus (optional)',s.channel?.capture_status],['Capture-HTML SHA-256 (optional, kein HTML-Archiv)',s.channel?.capture_html_sha256],['Kanal-Dokument-URL',s.channel?.source_document_url],['Elemente / Tokenspannen',s.channel?.elements==null?null:JSON.stringify(s.channel.elements)],['Unbekannt / ausgeschlossen',s.channel?.unavailable_reason],['Tokens im Quelltext',s.tokens],['Vorkommen dieser Wortfolge',row.occurrences],['Tokenpositionen',row.first_positions?.join(', ')],['Positionen begrenzt',row.positions_sampled==null?null:row.positions_sampled?'Ja, vollständige Anzahl separat':'Nein']];}
  return {heading:row.tokens?row.tokens.join(' '):row.source.requested_url,cells:pairs.flatMap(([label,value])=>[label,shown(value)]),row};
}
async function exactDOM(value){
  const expected={summary:`${value.after+1>value.total_rows?0:value.after+1}–${value.after+value.rows.length} von ${value.total_rows} Zeilen in der Auswahl · ${value.total_tokens} Tokens · ${value.total_text_bytes} analysierte Textbytes.`,articles:value.rows.map(expectedArticle)};
  await page.waitForFunction(expected=>{
    if(document.getElementById('csv').disabled||document.getElementById('results').hidden||document.getElementById('summary').textContent!==expected.summary)return false;
    try{const articles=[...document.querySelectorAll('#rows > article')].map(article=>({heading:article.querySelector('h3').textContent,cells:[...article.querySelectorAll('dl > dt,dl > dd')].map(e=>e.textContent),row:JSON.parse(article.querySelector('details pre').textContent)}));return JSON.stringify(articles)===JSON.stringify(expected.articles);}catch{return false;}
  },expected);
  assert.equal(await page.locator('#rows > article').count(),value.rows.length);assert.match(await page.locator('#status').innerText(),/0 Website-Anfragen/);
  assert((await page.locator('#basis').innerText()).includes(value.snapshot.caller.typed_report_sha256));
}
async function complete(first){let current=first,rows=[...first.page.rows],pages=[first];while(current.page.has_more){const next=await preview('#next');assert.equal(next.page.after,rows.length);assert.equal(next.page.snapshot.snapshot_token,first.page.snapshot.snapshot_token);rows.push(...next.page.rows);pages.push(next);current=next;}assert.equal(rows.length,first.page.total_rows);assert(await page.locator('#next').isHidden());return {first,pages,rows,last:current};}
async function form(id,{view='totals',n='1',query='',caseBehavior='unicode_lowercase',stopwords='',source='raw_http_html',channel='',captureOnly=false}={}){
  if(await page.locator('#clear-phrase').isVisible())await page.click('#clear-phrase');
  await page.selectOption('#crawl',id);await page.selectOption('#source',source);await page.selectOption('#channel',channel);await page.selectOption('#view',view);await page.selectOption('#n',n);await page.selectOption('#case',caseBehavior);await page.fill('#query',query);await page.fill('#stopwords',stopwords);
  await page.locator('.channel-advanced').evaluate(element=>element.open=true);await page.setChecked('#channel-capture-only',captureOnly);assert(await page.isDisabled('#csv'));
}
function totalsOracle(n,{preserve=false,stopwords=[],query=''}={}){
  const map=new Map();for(const text of [texts.a,texts.b]){const tokens=text.split(' ').map(token=>preserve?token:token.toLowerCase()),seen=new Set();for(let i=0;i+n<=tokens.length;i++){const slice=tokens.slice(i,i+n);if(slice.some(t=>stopwords.includes(t)))continue;const key=JSON.stringify(slice);let row=map.get(key);if(!row){row={tokens:slice,occurrences:0,document_frequency:0};map.set(key,row);}row.occurrences++;if(!seen.has(key)){row.document_frequency++;seen.add(key);}}}
  return [...map.values()].filter(r=>r.tokens.join(' ').toLowerCase().includes(query.toLowerCase())).sort((a,b)=>b.occurrences-a.occurrences||(JSON.stringify(a.tokens)<JSON.stringify(b.tokens)?-1:JSON.stringify(a.tokens)>JSON.stringify(b.tokens)?1:0));
}
function verifyTotals(saved,n,options){assert.deepEqual(saved.rows.map(r=>({tokens:r.tokens,occurrences:r.occurrences,document_frequency:r.document_frequency})),totalsOracle(n,options));assert(saved.rows.every(r=>r.n===n&&r.source_memberships_complete===true));}
function csvRows(text){const rows=[];let row=[],cell='',quoted=false;for(let i=0;i<text.length;i++){const c=text[i];if(c==='"'){if(quoted&&text[i+1]==='"'){cell+='"';i++;}else quoted=!quoted;}else if(c===','&&!quoted){row.push(cell);cell='';}else if(c==='\n'&&!quoted){row.push(cell.replace(/\r$/,''));rows.push(row);row=[];cell='';}else cell+=c;}assert.equal(quoted,false);if(cell||row.length){row.push(cell);rows.push(row);}return rows;}
async function downloaded(button,label){const pending=page.waitForEvent('download');await page.click(button);const download=await pending,filename=path.join(output,label);await download.saveAs(filename);assert.equal(await download.failure(),null);return fs.readFileSync(filename);}
async function jsonDownload(saved,label){const before=browserRequests.length,bytes=await downloaded('#json',label+'.json');assert.equal(browserRequests.length,before,'JSON downloads already received preview bytes');assert.deepEqual(bytes,saved.bytes);assert.deepEqual(JSON.parse(bytes),saved.page);exportProof.push({kind:'page_json',label,bytes:bytes.length,sha256:sha(bytes)});return bytes;}
function expectedCSVHeader(saved){
  const header=['snapshot_token','view','n','tokens_json','occurrences','document_frequency','source_ordinal','requested_url','source_revision','capture_config_sha256','text_sha256','basis','observed_generation','source_state','source_counted','first_positions_json','positions_sampled','website_requests'];
  assert(!(saved.first.request.html_text_channel&&saved.first.request.retained_metadata_channel),'Exactly one supported channel profile');
  if(saved.first.request.html_text_channel)header.push('html_text_channel_json','channel_source_json');
  if(saved.first.request.retained_metadata_channel)header.push('retained_metadata_channel_json','channel_source_json');
  if(saved.first.page.snapshot.reporting)header.push('reporting_json','raw_page_ordinal');
  return header;
}
function validateCSV(bytes,saved){const records=csvRows(bytes.toString('utf8')),header=records.shift();assert.deepEqual(header,expectedCSVHeader(saved),'CSV must have the complete ordered supported header');let rows=records;if(saved.first.page.snapshot.reporting){const metadata=records.shift();assert.equal(metadata[header.indexOf('view')],'reporting');assert.deepEqual(JSON.parse(metadata[header.indexOf('reporting_json')]),saved.first.page.snapshot.reporting);rows=records;}assert.equal(rows.length,saved.rows.length);
  rows.forEach((cells,i)=>{assert.equal(cells.length,header.length);const row=saved.rows[i],s=row.source||{},scalar=value=>value==null?'':String(value),value={snapshot_token:saved.first.page.snapshot.snapshot_token,view:scalar(row.view),n:scalar(row.n),tokens_json:row.tokens??null,occurrences:scalar(row.occurrences),document_frequency:scalar(row.document_frequency),source_ordinal:scalar(s.ordinal),requested_url:scalar(s.requested_url),source_revision:scalar(s.source_revision),capture_config_sha256:scalar(s.capture_config_sha256),text_sha256:scalar(s.text_sha256),basis:scalar(s.basis),observed_generation:scalar(s.observed_generation),source_state:scalar(s.state),source_counted:scalar(s.counted),first_positions_json:row.first_positions??null,positions_sampled:scalar(row.positions_sampled),website_requests:'0',html_text_channel_json:saved.first.request.html_text_channel??null,retained_metadata_channel_json:saved.first.request.retained_metadata_channel??null,channel_source_json:s.channel??null,reporting_json:'',raw_page_ordinal:scalar(s.raw_page_ordinal)};header.forEach((name,c)=>{assert(Object.hasOwn(value,name),'Unexpected CSV field '+name);if(name.endsWith('_json')&&name!=='reporting_json')assert.deepEqual(JSON.parse(cells[c]),value[name],name);else assert.equal(cells[c],value[name],name);});});
}
function csvHeaderNegativeControls(bytes,saved){
  const table=csvRows(bytes.toString('utf8')),header=table[0],key=JSON.stringify(header);if(controlledHeaders.has(key))return;controlledHeaders.add(key);
  const encode=rows=>Buffer.from(rows.map(row=>row.map(cell=>'"'+cell.replaceAll('"','""')+'"').join(',')).join('\r\n')+'\r\n');
  const omit=table.map(row=>[row[header.indexOf('view')],row[header.indexOf('occurrences')]]);
  const reordered=table.map(row=>{const copy=[...row];[copy[0],copy[1]]=[copy[1],copy[0]];return copy;});
  const duplicate=table.map(row=>[...row,row[0]]);
  for(const [control,rows] of [['omitted_required_columns',omit],['reordered_columns',reordered],['duplicate_columns',duplicate]]){
    assert.throws(()=>validateCSV(encode(rows),saved),error=>error.message.includes('CSV must have the complete ordered supported header'),control);
    csvHeaderControls.push({control,supported_header:header,actual_validator:'validateCSV',basis:'mutated actual native/browser-verified CSV bytes',rejected:true});
  }
}
async function csvDownload(saved,label){const pending=page.waitForResponse(r=>r.url().startsWith(app+'/api/crawls/')&&r.url().endsWith('/ngrams/csv')&&r.request().method()==='POST');const bytes=await downloaded('#csv',label+'.csv'),response=await pending;assert.equal(response.status(),200);const route=new URL(response.url()).pathname;assert(route.endsWith('/ngrams/csv'));const payload=response.request().postDataJSON();
  const oracle=await fetch(app+route,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload),cache:'no-store'}),nativeBytes=Buffer.from(await oracle.arrayBuffer());assert.equal(oracle.status,200,nativeBytes.toString());assert.deepEqual(bytes,nativeBytes);headersMatch(response.headers(),bytes);headersMatch(Object.fromEntries(oracle.headers),bytes);assert.equal(response.headers()['x-ngram-snapshot'],saved.first.page.snapshot.snapshot_token);assert.equal(oracle.headers.get('x-ngram-snapshot'),saved.first.page.snapshot.snapshot_token);validateCSV(bytes,saved);csvHeaderNegativeControls(bytes,saved);readOracles.push({route,body:payload,sha256:sha(bytes)});await until(async()=>(await page.locator('#status').innerText()).includes('SHA-256 bestätigt'),'CSV verified UI');exportProof.push({kind:'full_csv',label,rows:saved.rows.length,bytes:bytes.length,sha256:sha(bytes)});return bytes;
}
function validateSourceInventory(saved,report){
  const actual=saved.rows.map(row=>({url:row.source.requested_url,ordinal:row.source.ordinal}));
  const expected=report.pages.map((record,ordinal)=>({url:record.url,ordinal}));
  assert.deepEqual(actual,expected,'Source inventory must retain every saved URL and ordinal exactly once in source order');
}
function sourceInventoryNegativeControls(saved,report){
  const a=saved.rows.findIndex(row=>row.source.requested_url.endsWith('/a')),b=saved.rows.findIndex(row=>row.source.requested_url.endsWith('/b'));assert(a>=0&&b>=0);
  const duplicate=structuredClone(saved.rows);duplicate[b]=structuredClone(duplicate[a]);
  const omitted=structuredClone(saved.rows);omitted.splice(b,1);
  const wrongOrder=structuredClone(saved.rows);[wrongOrder[a],wrongOrder[b]]=[wrongOrder[b],wrongOrder[a]];
  const wrongOrdinal=structuredClone(saved.rows);wrongOrdinal[b].source.ordinal=wrongOrdinal[a].source.ordinal;
  for(const [control,rows] of [['duplicate_a_omits_b',duplicate],['omitted_source',omitted],['wrong_source_order',wrongOrder],['duplicate_source_ordinal',wrongOrdinal]]){
    assert.throws(()=>validateSourceInventory({...saved,rows},report),error=>error.message.includes('Source inventory must retain every saved URL and ordinal exactly once in source order'),control);
    sourceInventoryControls.push({control,actual_validator:'validateSourceInventory',basis:'mutated actual native source-inventory rows',rejected:true});
  }
}
function sourceBindings(saved,report){for(const row of saved.rows){const s=row.source;if(!s)continue;const record=report.pages.find(p=>p.url===s.requested_url);assert(record);const capture=record.content_captures?.find(c=>c.source_kind==='raw_http_html');if(capture){assert.equal(s.source_revision,capture.source_revision);assert.equal(s.text_sha256,capture.result.text_sha256);assert.equal(s.capture_config_sha256,capture.config_sha256);assert.equal(s.observed_generation,capture.observed_generation);}else assert.equal(s.counted,false);}}
async function scenarios(phase,ids,reports){const results={};
  await form(ids.main);results.totals=await complete(await preview());verifyTotals(results.totals,1);assert.equal(results.totals.rows.length,105);assert.deepEqual(results.totals.pages.map(p=>p.page.rows.length),[100,5]);assert.equal(results.totals.first.page.total_tokens,111);assert.deepEqual(results.totals.first.page.source_states,{complete:2,complete_empty:1,incomplete_excluded:1,missing_or_unavailable:1});assert.equal(results.totals.first.page.all_supplied_sources_complete,false);await jsonDownload(results.totals.last,phase+'-tail-page');results.totals.csv=await csvDownload(results.totals,phase+'-totals');
  await form(ids.main,{n:'2',query:'alpha beta'});results.phrase=await complete(await preview());verifyTotals(results.phrase,2,{query:'alpha beta'});assert.equal(results.phrase.rows[0].occurrences,3);results.occurrences=await complete(await preview('#rows article button'));assert.equal(results.occurrences.rows.length,2);assert.deepEqual(results.occurrences.rows.map(r=>[new URL(r.source.requested_url).pathname,r.occurrences,r.first_positions]).sort(),[['/a',2,[1,3]],['/b',1,[1]]]);sourceBindings(results.occurrences,reports.main);results.occurrences.csv=await csvDownload(results.occurrences,phase+'-occurrences');await jsonDownload(results.occurrences.last,phase+'-occurrences-page');
  await form(ids.main,{n:'2',stopwords:'and'});results.stopwords=await complete(await preview());verifyTotals(results.stopwords,2,{stopwords:['and']});assert(results.stopwords.first.page.excluded_stopword_windows[1]>0);assert(!results.stopwords.rows.some(r=>r.tokens.join(' ')==='beta gamma'||r.tokens.includes('and')));results.stopwords.csv=await csvDownload(results.stopwords,phase+'-stopwords');
  await form(ids.main,{n:'1',query:'alpha',caseBehavior:'preserve'});results.preserve=await complete(await preview());verifyTotals(results.preserve,1,{preserve:true,query:'alpha'});assert.deepEqual(results.preserve.rows.map(r=>[r.tokens[0],r.occurrences]),[['Alpha',2],['alpha',1]]);
  await form(ids.main,{n:'6',query:'term099'});results.six=await complete(await preview());verifyTotals(results.six,6,{query:'term099'});assert.deepEqual(results.six.rows[0].tokens,words.slice(94));
  await form(ids.main,{view:'sources'});results.sources=await complete(await preview());assert.equal(results.sources.rows.length,5);sourceBindings(results.sources,reports.main);validateSourceInventory(results.sources,reports.main);if(phase==='warm')sourceInventoryNegativeControls(results.sources,reports.main);assert.equal(results.sources.rows.find(r=>r.source.requested_url.endsWith('/empty')).source.state,'complete_empty');assert.equal(results.sources.rows.find(r=>r.source.requested_url.endsWith('/partial')).source.state,'incomplete_excluded');assert.equal(results.sources.rows.find(r=>r.source.requested_url.endsWith('/plain')).source.state,'missing_or_unavailable');results.sources.csv=await csvDownload(results.sources,phase+'-sources');
  await form(ids.main,{view:'sources',source:'rendered_dom_html'});results.missingDOM=await complete(await preview());assert.equal(results.missingDOM.first.page.source_states.missing_or_unavailable,5);assert.equal(results.missingDOM.rows.length,5);validateSourceInventory(results.missingDOM,reports.main);assert.deepEqual(results.missingDOM.rows.map(r=>r.source.requested_url).sort(),reports.main.pages.map(p=>p.url).sort());assert(results.missingDOM.rows.every(r=>r.source.counted===false));
  await form(ids.budget,{view:'sources'});results.budget=await complete(await preview());assert.equal(results.budget.first.page.source_states.incomplete_excluded,1);assert.equal(results.budget.rows[0].source.counted,false);sourceBindings(results.budget,reports.budget);results.budget.csv=await csvDownload(results.budget,phase+'-budget');
  await form(ids.budget);results.budgetTotals=await complete(await preview());assert.equal(results.budgetTotals.rows.length,0);assert.equal(results.budgetTotals.first.page.total_tokens,0);assert.equal(results.budgetTotals.first.page.all_supplied_sources_complete,false);assert.equal(results.budgetTotals.first.page.source_states.incomplete_excluded,1);
  await form(ids.metadata);results.noCapture=await complete(await preview());assert.equal(results.noCapture.first.page.source_states.missing_or_unavailable,1);assert.equal(results.noCapture.rows.length,0);assert.equal(results.noCapture.first.page.all_supplied_sources_complete,false);
  await form(ids.metadata,{channel:'title',n:'2'});results.title=await complete(await preview());assert.deepEqual(results.title.rows.map(r=>r.tokens),[['channel','title'],['saved','channel']]);assert.equal(results.title.first.page.all_supplied_sources_complete,true);results.title.csv=await csvDownload(results.title,phase+'-title');await jsonDownload(results.title.last,phase+'-title-page');
  await form(ids.metadata,{channel:'title',n:'2',view:'sources'});results.titleSources=await complete(await preview());assert.equal(results.titleSources.rows.length,1);const titleSource=results.titleSources.rows[0].source,metadataRecord=reports.metadata.pages[0],metadataIdentity=titleSource.channel.metadata_source;assert.equal(titleSource.counted,true);assert.equal(metadataIdentity.authority_profile,'retained_html_metadata_v1');assert.equal(metadataIdentity.fetched_at,metadataRecord.fetched_at);assert.equal(metadataIdentity.engine,metadataRecord.dom_metadata.engine);assert.equal(metadataIdentity.source,metadataRecord.dom_metadata.source);assert.match(metadataIdentity.revision,/^[a-f0-9]{64}$/);assert.match(titleSource.channel.channel_revision,/^[a-f0-9]{64}$/);assert.equal(titleSource.channel.source_document_url,metadataRecord.final_url);results.titleSources.csv=await csvDownload(results.titleSources,phase+'-title-sources');await jsonDownload(results.titleSources.last,phase+'-title-sources-page');
  await form(ids.metadata,{channel:'title',n:'2',captureOnly:true});results.captureOnly=await complete(await preview());assert.equal(results.captureOnly.rows.length,0);assert.equal(results.captureOnly.first.page.all_supplied_sources_complete,false);results.captureOnly.csv=await csvDownload(results.captureOnly,phase+'-capture-only-empty');
  // Retain non-default form values in this same browser document through native EOF/restart.
  await form(ids.main,{n:'2',query:'alpha',caseBehavior:'preserve',stopwords:'and'});results.retained=await complete(await preview());verifyTotals(results.retained,2,{preserve:true,stopwords:['and'],query:'alpha'});results.retained.csv=await csvDownload(results.retained,phase+'-retained-options');
  assert.equal(offlineHits.length,0);return results;
}
async function probe(origin){const response=await fetch(origin+'/owned-witness-probe',{headers:{'x-owned-witness-control':nonce}});assert.equal(response.status,410);assert.equal(await response.text(),'Owned offline witness');}
(async()=>{
  let failure;
  try{
    await new Promise(resolve=>website.listen(0,'127.0.0.1',resolve));const origin=`http://127.0.0.1:${website.address().port}`;assert(!['8787','8790'].includes(new URL(origin).port));
    const first=await launch('warm');app=first.origin;
    const ids={main:await crawl(origin,['a','b','empty','partial','plain']),budget:await crawl(origin,['budget'],{content_capture:{sources:['raw_http_html'],include_selectors:['main'],limits:{max_text_utf8_bytes:8}}}),metadata:await crawl(origin,['metadata'],{content_capture:null})};
    const projects={},reports={};for(const [name,id] of Object.entries(ids)){projects[name]=await fetchNative(`/api/crawls/${id}/project.json`);reports[name]=projects[name].value;}
    for(const name of ['a','b','empty']){const capture=reports.main.pages.find(p=>p.url===origin+'/'+name).content_captures.find(c=>c.source_kind==='raw_http_html');assert.equal(capture.result.text,texts[name]);assert.equal(capture.result.text_sha256,sha(Buffer.from(texts[name])));assert.equal(capture.observed_generation,1);}
    assert.equal(reports.budget.pages[0].content_captures[0].result.evidence.text_truncated,true,'Actual capture limit must truncate');
    assert.equal(initialHits.length,7);offline=true;await probe(origin);
    const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();browser=await chromium.launch({headless:true,executablePath});page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:900}});page.on('pageerror',error=>errors.push(error.message));page.on('request',request=>{const url=request.url();if(url.startsWith(app+'/api/'))browserRequests.push({method:request.method(),path:new URL(url).pathname});else if(!url.startsWith(app+'/')&&!url.startsWith('blob:')&&!url.startsWith('data:'))external.push(url);});
    const served=[];for(const name of ['saved-ngrams.js','saved-ngrams.html','saved-ngrams.css']){const response=await fetch(app+'/'+(name==='saved-ngrams.html'?'saved-ngrams':name));assert.equal(response.status,200);const bytes=Buffer.from(await response.arrayBuffer());assert.equal(sha(bytes),sourceHashes['web/'+name]);served.push({path:name,sha256:sha(bytes)});}
    await page.goto(app+'/saved-ngrams');await until(()=>page.isEnabled('#crawl'),'native crawl selector');const warm=await scenarios('warm',ids,reports);
    check('Genuine saved text yields exact105 unigrams over100+5 DOM pages; independent corpus, source/revision/generation, fullCSV and pageJSON oracles pass');
    check('Real complete-empty/HTTP-incomplete/missing/truncated sources, raw versus absentDOM, retained-title channel, n1/2/6, case/filter and non-bridging stopwords stay distinct');
    const retainedOptions=await page.evaluate(()=>Object.fromEntries(['crawl','source','channel','view','n','query','case','stopwords'].map(id=>[id,document.getElementById(id).value])));
    const localStorageBefore=await page.evaluate(()=>({...localStorage}));const oldOrigin=app;await first.stop();assert.equal(offlineHits.length,0);const second=await launch('cold','127.0.0.1:'+new URL(oldOrigin).port);app=second.origin;assert.equal(app,oldOrigin,'Same owned origin across actual native restart');
    assert.deepEqual(await page.evaluate(()=>Object.fromEntries(['crawl','source','channel','view','n','query','case','stopwords'].map(id=>[id,document.getElementById(id).value]))),retainedOptions,'Existing browser document retains its own form state');
    const retainedCold=await complete(await preview());assert.deepEqual(retainedCold.first.bytes,warm.retained.first.bytes);assert.deepEqual(retainedCold.rows,warm.retained.rows);assert.deepEqual(await csvDownload(retainedCold,'cold-existing-document-options'),warm.retained.csv);assert.deepEqual(await page.evaluate(()=>({...localStorage})),localStorageBefore);
    check('Native process closesEOF0 and restarts same store/origin/binary; retained browser-document options reanalyse identical saved snapshot without website requests');
    await page.reload();await until(()=>page.isEnabled('#crawl'),'cold page crawl selector');for(const [id,value] of Object.entries({crawl:'',n:'',query:'',case:'unicode_lowercase',stopwords:'',channel:'',source:'raw_http_html'}))assert.equal(await page.inputValue('#'+id),value,'Reload defaults are document-local, not server option persistence');assert(await page.isDisabled('#csv'));
    const cold=await scenarios('cold',ids,reports);
    for(const name of Object.keys(warm)){assert.deepEqual(cold[name].rows,warm[name].rows,name+' cold rows');assert.deepEqual(cold[name].first.bytes,warm[name].first.bytes,name+' exact cold snapshot/header page');if(warm[name].csv)assert.deepEqual(cold[name].csv,warm[name].csv,name+' exact full cold CSV');}
    for(const [name,id] of Object.entries(ids)){const reopened=await fetchNative(`/api/crawls/${id}/project.json`);assert.deepEqual(reopened.bytes,projects[name].bytes,name+' raw saved project stays byte-identical');}
    check('After reload defaults, explicitly reapplied channel/options reproduce every saved cold result and fullCSV byte-for-byte; browser-local form values are not claimed as persisted server configuration');
    for(const width of [1440,320]){await page.setViewportSize({width,height:900});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'populated overflow '+width);await page.locator('#csv').scrollIntoViewIfNeeded();const bounds=await page.locator('#csv').boundingBox();assert(bounds.y>=0&&bounds.y+bounds.height<=900);await page.screenshot({path:path.join(output,`ngrams-cold-${width}.png`),fullPage:true});}
    await browser.close();browser=null;await second.stop();await probe(origin);await new Promise(resolve=>website.close(resolve));assert.equal(offlineHits.length,0);assert.equal(controlHits.length,2);assert.deepEqual(errors,[]);assert.deepEqual(external,[]);assert.equal(native.length,2);assert.equal(sourceInventoryControls.length,4);assert.equal(mutations.length,3);assert(browserRequests.every(r=>r.method==='GET'&&r.path==='/api/crawls'||r.method==='POST'&&/^\/api\/crawls\/[a-f0-9-]+\/ngrams\/(preview|csv)$/.test(r.path)),'Browser only reads native saved-data routes');assert(readOracles.every(r=>/\/ngrams\/(preview|csv)$/.test(r.route)),'Oracles only repeat explicitly read-only analysis routes');verifyBinary();for(const [name,digest] of Object.entries(sourceHashes))assert.equal(sha(fs.readFileSync(path.join(root,name))),digest,'Owned n-gram source changed: '+name);
    check('Active410 witness remains through browser and both natural child closes; zero product refetches, exact immutable reports and reachable1440/320 exports');
    fs.writeFileSync(path.join(output,'ngrams-cold-live.json'),JSON.stringify({result:'PASS',checks,binary_sha256:binarySha,test_driver_sha256:sourceHashes['web/verification/ui-ngrams-cold-live.cjs'],source_sha256:sourceHashes,served_assets:served,native,app_origin:app,crawl_ids:ids,initial_website_requests:initialHits,offline_product_requests:offlineHits,witness_control_requests:controlHits,witness_retained_through_both_child_closes_and_drained:true,browser_document_options:retainedOptions,option_persistence_scope:'No n-gram localStorage/server-option persistence claimed. Same browser document retains form values across same-origin native restart; reload restores defaults and test explicitly reapplies options.',csv_header_negative_controls:csvHeaderControls,source_inventory_negative_controls:sourceInventoryControls,saved_projects:Object.fromEntries(Object.entries(projects).map(([name,p])=>[name,{sha256:sha(p.bytes),bytes:p.bytes.length}])),source_snapshot:{warm:warm.sources.first.page.snapshot,cold:cold.sources.first.page.snapshot},exports:exportProof,read_only_native_oracles:readOracles,actual_crawl_mutations:mutations,errors,external_browser_requests:external,scope:'Two actual ordinary native processes; real crawled raw text and metadata, true truncation/error/missing capture, exact saved corpus/options/DOM/export/cold reanalysis. No seeded success reports, route mocks, whole-site coverage, rendered capture success, reference tokenization parity or broader archive/concurrency claim.'},null,2)+'\n');
    console.log('Proof: '+path.join(output,'ngrams-cold-live.json'));
  }catch(error){failure=error;try{if(page&&!page.isClosed())await page.screenshot({path:path.join(output,'failure.png'),fullPage:true});}catch{}fs.writeFileSync(path.join(output,'ngrams-cold-failure.json'),JSON.stringify({result:'FAIL',error:error.stack,checks,native,offlineHits,initialHits,errors},null,2)+'\n');throw error;}
  finally{if(browser)await browser.close();if(activeHost)await activeHost.stop().catch(error=>{if(!failure)throw error;});if(website.listening){website.closeAllConnections();await new Promise(resolve=>website.close(resolve));}}
})().catch(error=>{console.error(error);process.exitCode=1;});
