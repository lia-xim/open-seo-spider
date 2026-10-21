'use strict';
// Actual native saved captures -> complete paired CSV/XLSX main-table downloads.
// Every expected population is independently enumerated from declared fixture indices
// and original PageRecords; production compiler/evaluator is never an oracle import.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const {spawn}=require('node:child_process'),{createHash,randomUUID}=require('node:crypto');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const csvRecords=require('../../web/verification/csv-records.cjs');
const root=path.resolve(__dirname,'../..'),sha=bytes=>createHash('sha256').update(bytes).digest('hex');
assert(process.env.SPIDER_BINARY&&process.env.SPIDER_BINARY_SHA256,'Explicit reviewed normal binary/SHA required; never build or install');
const binary=fs.realpathSync(process.env.SPIDER_BINARY),binarySha=sha(fs.readFileSync(binary));assert.equal(binarySha,process.env.SPIDER_BINARY_SHA256);
const chrome=process.env.OPEN_SEO_BROWSER_PATH||process.env.SPIDER_CHROME;assert(chrome&&fs.statSync(chrome).isFile(),'Explicit pinned Chromium required');
const output=path.resolve(process.env.SPIDER_TABLE_XLSX_OUTPUT||path.join(process.env.SPIDER_TEST_ARTIFACT_DIR||path.join(root,'artifacts'),'table-xlsx-'+Date.now()+'-'+randomUUID().slice(0,8)));
assert(!fs.existsSync(output),'Fresh owned output required');fs.mkdirSync(output,{recursive:true});process.env.SPIDER_TEST_ARTIFACT_DIR=output;
const artifactPath=require('../../web/verification/artifact-path.cjs'),store=path.join(output,'store'),nonce=randomUUID();
const assets=[...new Set(['index.html','styles.css',...Array.from(fs.readFileSync(path.join(root,'web/index.html'),'utf8').matchAll(/<script\s+src="\/([^"?#]+)"/g),match=>match[1])])];
const pins=[__filename,path.join(root,'web/verification/artifact-path.cjs'),path.join(root,'web/verification/csv-records.cjs'),path.join(__dirname,'xlsx-package-oracle.cjs'),binary,chrome,...assets.map(name=>path.join(root,'web',name))].map(file=>({file,sha256:sha(fs.readFileSync(file))}));
const checks=[],owners=[],served=[],nativeReads=[],downloads=[],errors=[],external=[],siteHits=[],offlineHits=[],witnesses=[],views=[],geometry=[],negativeControls=[];
const titleKeys=['url','title_text','title_length','title_pixels','title_count','duplicates','status'];
const statusKeys=['url','initial_status','status','final_url','redirects','response_ms','indexable','error'];
const labels={url:'URL',title_text:'Seitentitel',title_length:'Zeichen',title_pixels:'Pixelprofil 20px',title_count:'Title-Tags',duplicates:'Vorkommen',status:'HTTP final',initial_status:'HTTP initial',final_url:'Finale URL',redirects:'Redirects',response_ms:'Antwortzeit',indexable:'Indexierbarkeit',error:'Abruffehler',requested_url:'Anfrage-URL',response_index:'Antwortposition'};
const paths=Array.from({length:151},(_,index)=>'/items/'+String(index).padStart(4,'0'));
const metadataLabels=['Crawl Coverage','Observation Basis','Selected State','Parent Snapshot SHA256','Fetched At','Observation Requested URL'];
const reportingLabels=['Captured Pages','Visible Pages','Hidden Pages'];
const collator=new Intl.Collator('de-DE',{numeric:true,sensitivity:'base'}),number=value=>Number(value).toLocaleString('de-DE');
let website,siteOrigin,app,browser,page,warmContext,coldContext,offline=false,failure,jobA,jobB,capturedSiteRequests,storageState;
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function check(label){checks.push(label);console.log('PASS '+label);}
function pinsUnchanged(){for(const pin of pins)assert.equal(sha(fs.readFileSync(pin.file)),pin.sha256,'Protected source/binary changed: '+pin.file);}
function deadline(promise,ms,label){let timer;return Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Timeout: '+label)),ms);})]).finally(()=>clearTimeout(timer));}
async function until(work,label,ms=45000){const end=Date.now()+ms;while(Date.now()<end){if(app)assert.equal(app.child.exitCode,null,'Native exited: '+label);const value=await work();if(value)return value;await sleep(25);}throw Error('Timeout: '+label);}
async function launch(phase,listen='127.0.0.1:0'){
  pinsUnchanged();const env={...process.env,OPEN_SEO_COORDINATED_EXECUTION_WINDOW:'confirmed'};delete env.OPEN_SEO_PAGESPEED_API_KEY;
  const child=spawn(binary,['serve','--listen',listen,'--data-dir',store,'--shutdown-on-stdin-eof'],{cwd:root,env,windowsHide:true,stdio:['pipe','pipe','pipe']});
  const owner={phase,child,log:'',origin:null,close:null,natural_eof:false,forced:false};owner.closed=new Promise(resolve=>child.once('close',(code,signal)=>{owner.close={code,signal};resolve(owner.close);}));child.once('error',error=>owner.error=error);
  for(const stream of[child.stdout,child.stderr])stream.on('data',bytes=>owner.log+=bytes.toString());owners.push(owner);app=owner;
  owner.origin=await until(()=>{if(owner.error)throw owner.error;return /(?:^|\n)Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/.exec(owner.log)?.[1];},'complete post-bind line '+phase);
  assert(!['8787','8790'].includes(new URL(owner.origin).port));assert.equal((await native('/healthz')).response.status,200);return owner;
}
async function stop(owner){owner.child.stdin.end();assert.deepEqual(await deadline(owner.closed,20000,'natural stdin EOF '+owner.phase),{code:0,signal:null});owner.natural_eof=true;fs.writeFileSync(artifactPath('native-'+owner.phase+'.log'),owner.log);if(app===owner)app=null;pinsUnchanged();}
async function native(route,body,status=200){const response=await fetch(app.origin+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'content-type':'application/json'},body:body===undefined?undefined:JSON.stringify(body),redirect:'error',cache:'no-store',signal:AbortSignal.timeout(30000)});const bytes=Buffer.from(await response.arrayBuffer());assert.equal(response.status,status,'Native '+route+': '+bytes.toString().slice(0,300));nativeReads.push({phase:app.phase,path:route,method:body===undefined?'GET':'POST',status,bytes:bytes.length,sha256:sha(bytes)});return{response,bytes};}
async function api(route,body,status){const value=await native(route,body,status);return{...value,value:JSON.parse(value.bytes)};}
async function bindAssets(){for(const name of assets){const value=await native(name==='index.html'?'/':'/'+name);assert.deepEqual(value.bytes,fs.readFileSync(path.join(root,'web',name)),'Embedded '+name);served.push({phase:app.phase,path:name,sha256:sha(value.bytes)});}}
function titleFor(index){return index===0?'':index===2?'Cafe\u0301 zero':index===3?'Café overlap':['=SUM(1,2) Café cohort 0 & <literal>','+Café cohort 1, "quoted"','_x0041_ Café cohort 2','@Café cohort 3','-Café cohort 4','Café cohort 5','Café cohort 6'][index%7];}
function titlesFor(index){return index===1||index>1&&index%5===0?[]:index>=3&&index%3===0?[titleFor(index),'Second tag '+index]:[titleFor(index)];}
async function ownedWebsite(){
  const server=http.createServer((request,response)=>{
    if(request.headers['x-owned-offline-witness']===nonce){assert(offline);witnesses.push({path:request.url,status:410});response.writeHead(410,{'content-type':'text/plain'});return response.end('Owned positive offline witness');}
    const entry={path:request.url,method:request.method};siteHits.push(entry);if(offline){offlineHits.push(entry);response.writeHead(410);return response.end('Owned offline website');}
    if(request.url==='/overflow/'){response.writeHead(200,{'content-type':'text/html; charset=utf-8','cache-control':'no-store'});return response.end('<!doctype html><html><head><title>'+ 'X'.repeat(32768) +'</title></head><body>Owned actual oversize-cell negative only</body></html>');}
    const match=/^\/(a|b)\/(?:items\/(\d{4})|(transport-error))?$/.exec(request.url);
    if(!match){response.writeHead(404);return response.end();}
    const group=match[1],index=match[2]===undefined?-1:Number(match[2]);
    if(match[3]){entry.transport='socket_closed_before_http_headers';request.socket.destroy();return;}
    if(index===150){response.writeHead(302,{location:'/'+group+paths[149]});return response.end();}
    if(index>150){response.writeHead(404);return response.end();}
    response.writeHead(200,{'content-type':'text/html; charset=utf-8','cache-control':'no-store'});
    if(index<0)return response.end('<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Owned '+group+' index</title></head><body><h1>Owned index</h1>'+[...paths.map(value=>'/'+group+value),'/'+group+'/transport-error'].map(value=>'<a href="'+value+'">'+value+'</a>').join('')+'</body></html>');
    response.end('<!doctype html><html lang="en"><head><meta charset="utf-8">'+titlesFor(index).map(value=>'<title>'+escapeHTML(value)+'</title>').join('')+'</head><body>'+(index===0?'':'<h1>Owned '+group+' row '+index+'</h1><p>Deterministic captured '+group+' HTML.</p>')+'</body></html>');
  });
  website=server;await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});assert(![8787,8790].includes(server.address().port));siteOrigin='http://127.0.0.1:'+server.address().port;
}
async function captureJob(seed){
  const start=(await api('/api/crawls',{seed_urls:[seed],crawl_links:true,discover_sitemaps:false,obey_robots:false,delay_ms:0,concurrency:4,request_timeout_secs:5,max_pages:0,max_depth:0})).value;
  assert(start.id);await until(async()=>{const job=(await api('/api/crawls/'+start.id)).value;assert.notEqual(job.status,'failed',job.error);return job.status==='completed';},'genuine native capture');
  const raw=await native('/api/crawls/'+start.id+'/project.json'),report=JSON.parse(raw.bytes),detail=(await api('/api/crawls/'+start.id)).value;
  const savedPath=path.join(store,start.id+'.json'),saved=fs.readFileSync(savedPath);assert.deepEqual(JSON.parse(saved),report);fs.writeFileSync(artifactPath(start.id+'-original-project.json'),raw.bytes);
  return{id:start.id,report,project:raw.bytes,saved,savedPath,detail};
}
function nativeControls(){for(const[job,group]of[[jobA,'a'],[jobB,'b']]){const byURL=new Map(job.report.pages.map(value=>[value.url,value]));assert.equal(job.report.pages.length,153);assert.equal(byURL.size,153);assert.deepEqual([...byURL.keys()].sort(),['/'+group+'/',...paths.map(value=>'/'+group+value),'/'+group+'/transport-error'].map(value=>siteOrigin+value).sort());for(let index=0;index<151;index++){const row=byURL.get(siteOrigin+'/'+group+paths[index]),observed=index===150?149:index;assert.equal(row.internal,true);assert.equal(row.final_status,200);assert.equal(row.error,null);assert.deepEqual(row.titles,titlesFor(observed));}const empty=byURL.get(siteOrigin+'/'+group+paths[0]),absent=byURL.get(siteOrigin+'/'+group+paths[1]),two=byURL.get(siteOrigin+'/'+group+paths[3]),nfd=byURL.get(siteOrigin+'/'+group+paths[2]),failed=byURL.get(siteOrigin+'/'+group+'/transport-error');assert.deepEqual(empty.titles,['']);assert.deepEqual(empty.title_pixel_widths,[0]);assert.deepEqual(absent.titles,[]);assert.equal(two.titles.length,2);assert.deepEqual(nfd.title_pixel_widths,[null]);assert.equal(failed.status,null);assert.equal(failed.final_status,null);assert(failed.error);negativeControls.push({kind:'genuine_native_zero_empty_unknown_NFD_two_titles',id:job.id,pages:153,empty_width:0,absent_count:0,multiple_count:2,unsupported_width:null,failed_transport_status:null});}}
async function context(storage){const owner=app,origin=owner.origin,created=await browser.newContext({acceptDownloads:true,viewport:{width:1440,height:1000},...(storage?{storageState:storage}:{})});await created.route('**/*',route=>{const url=route.request().url();if(url.startsWith(origin+'/')||/^(about:|blob:|data:)/.test(url))return route.continue();external.push(url);return route.abort();});created.on('page',owned=>{owned.on('pageerror',error=>errors.push(error.message));owned.on('download',download=>downloads.push({source:'browser-event',phase:owner.phase,suggested:download.suggestedFilename()}));});return created;}
async function pageIn(created){const owned=await created.newPage();await owned.goto(app.origin);await owned.locator('#connection-state.connected').waitFor();return owned;}
const ownerReadiness=[];
async function selectJob(id){
  await page.locator('#crawl-history option[value="'+id+'"]').waitFor({state:'attached'});
  const origin=new URL(page.url()).origin,phase=app.phase,route=origin+'/api/crawls/'+encodeURIComponent(id)+'/fragments';assert(origin===app.origin,'Selected owner must belong to an owned native or measured negative-platform origin');
  // History already contains a terminal summary before selectCrawl's async
  // refresh has loaded any pages. Bind the actual final refresh dependency,
  // then the data-committed UI; status text alone is not data readiness.
  const pending=page.waitForResponse(response=>response.url()===route&&response.request().method()==='GET',{timeout:30000});
  const [response]=await Promise.all([pending,page.selectOption('#crawl-history',id)]);
  assert.equal(response.status(),200,'Selected owner final refresh dependency');
  assert.equal(await deadline(response.finished(),30000,'selected owner final refresh request terminal'),null,'Selected owner final response completed');
  // Both owned fixture captures are nonempty. Export is disabled while the
  // selected summary has no loaded pages/issues, including on a same-ID reset.
  await page.waitForFunction(id=>document.getElementById('crawl-history').value===id&&document.getElementById('crawl-state').textContent.includes('abgeschlossen')&&!document.getElementById('export-csv').disabled,id,{timeout:30000});
  ownerReadiness.push({id,origin,phase,route,method:'GET',status:200,request_finished:true,DOM_owner_completed:true,export_enabled:true,timeout_ms:30000,source:'actual_browser_final_refresh_dependency_and_committed_data'});
}
async function section(name){await page.click('[data-section="'+name+'"]');await page.locator('#table-options').waitFor({state:'attached'});await options();await page.locator('#table-sort-column').waitFor({state:'visible'});}
async function options(){const details=page.locator('#table-options');if(!await details.evaluate(element=>element.open))await details.locator(':scope > summary').click();}
async function columnOptions(){await options();const details=page.locator('#table-options details.table-columns');if(!await details.evaluate(element=>element.open))await details.locator(':scope > summary').click();}
async function headers(){return page.locator('#results-table thead [data-sort]').evaluateAll(nodes=>nodes.map(node=>node.dataset.sort));}
async function firstPage(){while(!await page.locator('#page-prev').isDisabled())await page.click('#page-prev');}
async function rule(key,op,values={}){await options();await page.selectOption('#table-rule-column',key);await page.selectOption('#table-rule-op',op);if(Object.hasOwn(values,'text'))await page.fill('#table-rule-text',values.text);if(Object.hasOwn(values,'min'))await page.fill('#table-rule-min',String(values.min));if(Object.hasOwn(values,'max'))await page.fill('#table-rule-max',String(values.max));await page.click('#table-rule-add');}
async function sort(key,direction){await options();await page.selectOption('#table-sort-column',key);await page.selectOption('#table-sort-direction',String(direction));}
const titleRows=job=>{
  const source=job.report.pages.map((value,ordinal)=>({value,ordinal})).filter(({value})=>value.internal&&['text/html','application/xhtml+xml'].includes(value.content_type.split(';')[0]));
  const counts=new Map();for(const {value}of source){const text=(value.titles[0]||'').trim();if(text)counts.set(text,(counts.get(text)||0)+1);}
  return source.map(({value,ordinal})=>({url:value.url,title_text:value.titles.length?value.titles[0]:null,title_length:Array.from(value.titles[0]||'').length,title_pixels:value.title_pixel_widths[0],title_count:value.titles.length,duplicates:counts.get((value.titles[0]||'').trim())||0,status:value.final_status??value.status,page:value,ordinal}));
};
const statusRows=job=>job.report.pages.filter(value=>value.internal).map((value,ordinal)=>({url:value.url,initial_status:value.status,status:value.final_status??value.status,final_url:value.final_url,redirects:value.redirect_chain.length,response_ms:value.response_ms,indexable:value.indexable==null?null:value.indexable?'Indexierbar':'Nicht indexierbar',error:value.error,page:value,ordinal}));
function ordered(rows,key,direction=1){return [...rows].sort((a,b)=>{const first=a[key],second=b[key],rank=value=>value==null?2:typeof value==='string'&&!value.trim()?1:0;const placement=rank(first)-rank(second);if(placement)return placement;const value=rank(first)?0:typeof first==='number'&&typeof second==='number'?(first<second?-1:first>second?1:0)*direction:collator.compare(String(first).normalize('NFC'),String(second).normalize('NFC'))*direction;return value||(a.url<b.url?-1:a.url>b.url?1:0)||(a.ordinal-b.ordinal);});}
function rendered(row,key){const value=row[key];if(key==='title_text')return value??'';if(key==='response_ms')return Math.round(value).toLocaleString('de-DE')+' ms';if(key==='status'||key==='initial_status')return value==null?'—':String(value);return value==null?'—':typeof value==='number'?number(value):String(value);}
async function exact(rows,keys,label){
  await page.selectOption('#page-size','50');await page.waitForFunction(expected=>document.getElementById('result-count').textContent===expected,number(rows.length)+' Ergebnisse');
  await firstPage();const collected=[];
  for(let offset=0;offset<Math.max(rows.length,1);offset+=50){
    const expected=rows.slice(offset,offset+50).map(row=>keys.map(key=>rendered(row,key))),expectedLabel=rows.length?number(offset+1)+'–'+number(Math.min(offset+50,rows.length))+' von '+number(rows.length):'Keine Ergebnisse';
    await page.waitForFunction(({expected,keys,label})=>JSON.stringify([...document.querySelectorAll('#results-table thead [data-sort]')].map(node=>node.dataset.sort))===JSON.stringify(keys)&&document.getElementById('page-label').textContent===label&&JSON.stringify([...document.querySelectorAll('#results-table tbody tr[data-row]')].map(node=>[...node.querySelectorAll('td')].map(cell=>cell.textContent.trim())))===JSON.stringify(expected),{expected,keys,label:expectedLabel});
    const actual=await page.locator('#results-table tbody tr[data-row]').evaluateAll(nodes=>nodes.map(node=>[...node.querySelectorAll('td')].map(cell=>cell.textContent.trim())));assert.deepEqual(actual,expected,label+' exact rows/cells '+offset);assert.deepEqual(await headers(),keys);assert.equal(await page.locator('#page-next').isDisabled(),offset+50>=rows.length);collected.push(...actual);if(offset+50<rows.length)await page.click('#page-next');
  }
  assert.equal(collected.length,rows.length);assert.deepEqual(collected.map(row=>row[keys.indexOf('url')]),rows.map(row=>row.url));views.push({phase:app.phase,label,rows:rows.length,columns:keys,ordered_cells_sha256:sha(Buffer.from(JSON.stringify(collected))),pages:Array.from({length:Math.ceil(rows.length/50)},(_,index)=>Math.min(50,rows.length-index*50))});return collected;
}
async function columns(desired){await columnOptions();const all=await page.locator('#table-column-list input[data-table-column]').evaluateAll(nodes=>nodes.map(node=>({key:node.value,disabled:node.disabled})));for(const item of all){if(item.disabled){assert(desired.includes(item.key));continue;}await columnOptions();const input=page.locator('#table-column-list input[data-table-column][value="'+item.key+'"]');if(await input.isChecked()!==desired.includes(item.key))await input.setChecked(desired.includes(item.key));}
  for(let index=1;index<desired.length;index++){for(let guard=0;(await headers()).indexOf(desired[index])>index;guard++){assert(guard<128);await columnOptions();const before=await headers();await page.click('#table-column-list button[data-table-column-move="up"][data-column-key="'+desired[index]+'"]');assert.notDeepEqual(await headers(),before);}}assert.deepEqual(await headers(),desired);
}
function protectedCell(value){let text=String(value??'');if(typeof value!=='number'&&typeof value!=='boolean'&&/^[\s\uFEFF]*[=+@-]/u.test(text))text="'"+text;return text;}
function decodedCSV(bytes,expectedHeader){const text=bytes.toString('utf8').replace(/^\uFEFF/,''),firstEnd=text.indexOf('\r\n');assert.equal(firstEnd<0?text:text.slice(0,firstEnd),expectedHeader.map(label=>'"'+label.replace(/"/g,'""')+'"').join(','),'Exact ordered CSV header including duplicate rejection');return csvRecords(bytes.subarray(-2).equals(Buffer.from('\r\n'))?bytes:Buffer.concat([bytes,Buffer.from('\r\n')]));}
async function csv(rows,keys,job,label,sectionName='titles'){
  const pending=page.waitForEvent('download');await page.click('#export-csv');const actual=await pending,file=artifactPath(label+'.csv');await actual.saveAs(file);const bytes=fs.readFileSync(file);assert.equal(bytes.subarray(0,3).toString('hex'),'efbbbf');assert(actual.suggestedFilename().includes(job.id));
  const visibility=job.detail.reporting_visibility,extra=visibility?reportingLabels:[],expectedHeader=[...keys.map(key=>sectionName==='responses'&&key==='status'?'HTTP beobachtet':labels[key]),...metadataLabels,...extra],parsed=decodedCSV(bytes,expectedHeader);assert.equal(parsed.length,rows.length);
  if(rows.length)assert.deepEqual(Object.keys(parsed[0]),expectedHeader);
  for(let index=0;index<rows.length;index++){const row=rows[index],expected={};for(const key of keys)expected[sectionName==='responses'&&key==='status'?'HTTP beobachtet':labels[key]]=protectedCell(row[key]);const provenance=sectionName==='responses'?['crawl_snapshot','unknown_or_mixed_row_identity','','','','']:['crawl_snapshot','captured','','',row.page.fetched_at,row.page.url];for(let column=0;column<metadataLabels.length;column++)expected[metadataLabels[column]]=provenance[column];if(visibility)for(const[column,key]of ['captured_pages','visible_pages','hidden_pages'].entries())expected[reportingLabels[column]]=String(visibility[key]);assert.deepEqual(parsed[index],expected,label+' complete CSV cells '+index);}
  downloads.push({source:'actual-file',phase:app.phase,label,path:file,bytes:bytes.length,sha256:sha(bytes),rows:rows.length,columns:expectedHeader,verification:'Actual browser file; every ordered cell independently mapped from saved native PageRecords'});return bytes;
}
async function witness(){const response=await fetch(siteOrigin+'/offline-positive-control',{headers:{'x-owned-offline-witness':nonce},cache:'no-store',signal:AbortSignal.timeout(5000)});assert.equal(response.status,410);assert.equal(await response.text(),'Owned positive offline witness');}
async function sameSaved(){for(const job of[jobA,jobB]){assert.deepEqual((await native('/api/crawls/'+job.id+'/project.json')).bytes,job.project,'Native project bytes remain unchanged');assert.deepEqual(fs.readFileSync(job.savedPath),job.saved,'Native saved JSON bytes remain unchanged');}}

// Ignored preparation candidate. No native/Chrome run occurred during authoring.
// The owner must review and freeze the complete source/normal binary closure
// before scheduling this gate; the driver never builds or installs its runtime.
const {inspectWorkbook,packageParts}=require('./xlsx-package-oracle.cjs');
const workbookFiles=[],csvPairs=[],staleExports=[],retained={};
const mainColumns=['url','title_count','title_text','title_pixels','status'];
const responseColumns=['url','requested_url','response_index','status'];
const currentScope=['titles','status','responses'];
let overflowJob;
function escapeHTML(value){return value.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}
async function mode(value){await options();const expected=value===1?'simple':'grouped';if(await page.inputValue('#table-rule-mode')!==expected){await clearQuery();await page.selectOption('#table-rule-mode',expected);}assert.equal(await page.inputValue('#table-rule-mode'),expected);}
async function clearQuery(){
  await options();const active=await page.inputValue('#table-rule-mode'),button=page.locator('#table-rules-clear');
  if(await button.isEnabled())await button.click();assert.equal(await button.isDisabled(),true);assert.equal(await page.inputValue('#table-rule-mode'),active);
  assert.equal(await page.locator('#table-rules li[data-table-rule-index]').count(),0);
  if(active==='simple')assert.equal(await page.locator('#table-expression-editor').isHidden(),true);
  else{assert.equal(await page.locator('#table-expression-editor').isVisible(),true);assert.equal(await page.locator('#table-expression-editor [data-expression-kind]').count(),0);}
  for(const name of['table-explorer-error','table-expression-error']){assert.equal(await page.locator('#'+name).isHidden(),true);assert.equal((await page.locator('#'+name).textContent()).trim(),'');}
  await page.fill('#table-search','');await page.selectOption('#table-filter','all');
}
async function domSnapshot(){return page.evaluate(()=>({owner:document.getElementById('crawl-history').value,section:document.querySelector('[data-section][aria-current="page"]')?.dataset.section,mode:document.getElementById('table-rule-mode').value,filter:document.getElementById('table-filter').value,search:document.getElementById('table-search').value,sort:[document.getElementById('table-sort-column').value,document.getElementById('table-sort-direction').value],columns:[...document.querySelectorAll('#results-table thead [data-sort]')].map(n=>n.dataset.sort),page:document.getElementById('page-label').textContent,rules:[...document.querySelectorAll('#table-rules li[data-table-rule-index]')].map(n=>n.textContent),expression:[...document.querySelectorAll('#table-expression-editor [data-expression-kind]')].map(n=>({path:n.dataset.expressionPath,kind:n.dataset.expressionKind,text:n.textContent})),rows:[...document.querySelectorAll('#results-table tbody tr[data-row]')].map(n=>[...n.querySelectorAll('td')].map(c=>c.textContent.trim()))}));}
async function storedBytes(){return page.evaluate(()=>Object.fromEntries(Object.keys(localStorage).sort().map(key=>[key,localStorage.getItem(key)])));}
function allTitles(job){const rows=titleRows(job);assert.equal(rows.length,152);return ordered(rows,'url',1);}
function selectedTitles(job){
  // Declared fixture truth table, independent of product query/evaluator output.
  const members=new Set(Array.from({length:151},(_,index)=>index).filter(index=>{const observed=index===150?149:index;return observed>=4&&observed%5!==0;}).map(index=>'/items/'+String(index).padStart(4,'0')));
  const rows=titleRows(job).filter(row=>[...members].some(suffix=>row.url.endsWith(suffix)));assert.equal(rows.length,118);return ordered(rows,'title_text',-1);
}
function allResponses(job){
  const rows=[];
  for(const[p,ordinal]of job.report.pages.map((p,index)=>[p,index])){
    if(p.error){assert.equal(p.status,null);assert.equal(p.redirect_chain.length,0);continue;}
    for(const[index,hop]of p.redirect_chain.entries())rows.push({url:hop.url,requested_url:p.url,response_index:index+1,status:hop.status,page:p,ordinal});
    rows.push({url:p.final_url||p.url,requested_url:p.url,response_index:p.redirect_chain.length+1,status:p.final_status??p.status,page:p,ordinal});
  }
  assert.equal(rows.length,153);
  return rows.sort((a,b)=>(a.url<b.url?-1:a.url>b.url?1:0)||(a.requested_url<b.requested_url?-1:a.requested_url>b.requested_url?1:0)||a.response_index-b.response_index);
}
function responseTwins(job){const all=allResponses(job),target=job.report.pages.find(p=>p.url.endsWith('/items/0149')).url,rows=all.filter(row=>row.url===target);assert.equal(rows.length,2);assert.equal(rows[0].requested_url,target);assert(rows[1].requested_url.endsWith('/items/0150'));assert.deepEqual(rows.map(row=>row.response_index),[1,2]);assert.deepEqual(rows.map(row=>row.status),[200,200]);return rows;}
async function configureAll(sectionName,keys){await section(sectionName);await mode(1);await clearQuery();await sort('url',1);await columns(keys);}
async function configureMain(version){
  await section('titles');await mode(version);await clearQuery();
  if(version===1)await rule('title_count','min',{min:1});
  else{await rule('title_count','min',{min:2});await page.selectOption('#table-expression-target','root');await page.click('#table-expression-wrap-any');await page.selectOption('#table-expression-target','root');await rule('title_text','contains',{text:'CAFE\u0301'});}
  await page.selectOption('#table-filter','duplicate');await page.fill('#table-search','CAFÉ');await sort('title_text',-1);await columns(mainColumns);
}
function expectedWorkbook(rows,keys,job,sectionName){
  const visibility=job.detail.reporting_visibility;
  const header=[...keys.map(key=>sectionName==='responses'&&key==='status'?'HTTP beobachtet':labels[key]),...metadataLabels,...visibility?reportingLabels:[]];
  return{header,rows:rows.map(row=>{
    const values=keys.map(key=>row[key]??null),provenance=sectionName==='responses'?['crawl_snapshot','unknown_or_mixed_row_identity','','','','']:['crawl_snapshot','captured','','',row.page.fetched_at,row.page.url];
    return[...values,...provenance,...visibility?['captured_pages','visible_pages','hidden_pages'].map(key=>visibility[key]):[]];
  })};
}
async function xlsx(rows,keys,job,label,sectionName){
  assert(currentScope.includes(sectionName),'Native deep scope is explicit; generic main-table product support remains broader');
  await page.locator('#table-view').waitFor({state:'visible'});assert.equal(await page.locator('#export-xlsx').isEnabled(),true);
  const before=await domSnapshot(),stored=await storedBytes(),count=downloads.filter(d=>d.source==='browser-event').length;
  const pending=page.waitForEvent('download');await page.click('#export-xlsx');const actual=await pending,file=artifactPath(label+'.xlsx');
  assert.equal(actual.suggestedFilename(),'open-seo-spider-'+sectionName+'-'+job.id+'.xlsx');await actual.saveAs(file);
  const bytes=fs.readFileSync(file),expected=expectedWorkbook(rows,keys,job,sectionName);
  const expectationFile=artifactPath(label+'.independent-expected.json');fs.writeFileSync(expectationFile,JSON.stringify({source:'Declared fixture truth table + immutable original native PageRecords; no product planner/compiler/writer or CSV-derived expectation',project_sha256:sha(job.project),section:sectionName,keys,...expected},null,2)+'\n');
  const inspected=await inspectWorkbook(browser,bytes,expected);fs.writeFileSync(artifactPath(label+'.reader.json'),JSON.stringify(inspected,null,2)+'\n');
  await page.waitForFunction(()=>document.getElementById('table-export-status').dataset.phase!=='prepare'&&!document.getElementById('export-xlsx').disabled);
  assert.equal(downloads.filter(d=>d.source==='browser-event').length,count+1);assert.deepEqual(await domSnapshot(),before,'Complete export preserves current DOM page/query/projection');assert.deepEqual(await storedBytes(),stored,'Complete export changes no saved columns or query catalog bytes');
  workbookFiles.push({phase:app.phase,label,path:file,bytes:bytes.length,sha256:sha(bytes),matrix_sha256:inspected.matrix_sha256,rows:rows.length,columns:expected.header,keys,project_sha256:sha(job.project),reader:inspected.reader,stats:inspected.stats,package:inspected.package});
  check(label+': actual complete ordered XLSX file, independent ZIP/CRC/XML/types/provenance oracle');return{bytes,matrix_sha256:inspected.matrix_sha256,expected};
}
async function pair(rows,keys,job,label,sectionName){
  const before=await domSnapshot(),csvBytes=await csv(rows,keys,job,label,sectionName);assert.deepEqual(await domSnapshot(),before,'CSV preserves current DOM page and query');
  const workbook=await xlsx(rows,keys,job,label,sectionName);
  csvPairs.push({label,phase:app.phase,section:sectionName,rows:rows.length,csv_sha256:sha(csvBytes),xlsx_sha256:sha(workbook.bytes),matrix_sha256:workbook.matrix_sha256,representation:'CSV existing null-to-empty/formula apostrophe; XLSX raw typed values/null/literal inline string'});
  return{csvBytes,...workbook};
}
async function exportWhileSecondPage(rows,keys,job,label,sectionName){
  await exact(rows,keys,label+' full paginated DOM oracle');await firstPage();if(rows.length>50){await page.click('#page-next');assert.equal(await page.locator('#page-prev').isEnabled(),true);assert((await page.locator('#page-label').textContent()).startsWith('51–'));}
  return pair(rows,keys,job,label,sectionName);
}
async function positiveCases(job,label){
  await selectJob(job.id);await configureAll('titles',titleKeys);const titles=allTitles(job);
  const full=await exportWhileSecondPage(titles,titleKeys,job,label+'-titles152','titles');
  const emptyRow=full.expected.rows.find(row=>row[0].endsWith('/items/0000')),missingRow=full.expected.rows.find(row=>row[0].endsWith('/items/0001')),nfdRow=full.expected.rows.find(row=>row[0].endsWith('/items/0002'));
  assert.equal(emptyRow[1],'');assert.equal(emptyRow[3],0);assert.equal(missingRow[1],null);assert.equal(missingRow[3],null);assert.equal(nfdRow[1],'Cafe\u0301 zero');assert.equal(nfdRow[3],null);
  assert(full.expected.rows.some(row=>typeof row[1]==='string'&&row[1].startsWith('=SUM')));assert(full.expected.rows.some(row=>typeof row[1]==='string'&&row[1].startsWith('_x0041_')));
  const output={full};
  for(const version of[1,2]){await configureMain(version);output['v'+version]=await exportWhileSecondPage(selectedTitles(job),mainColumns,job,label+'-v'+version+'-118','titles');}
  assert.equal(output.v1.matrix_sha256,output.v2.matrix_sha256,'Overlapping OR chooses same118 original rows once, in identical current order');
  await configureAll('status',statusKeys);output.status=await exportWhileSecondPage(ordered(statusRows(job),'url',1),statusKeys,job,label+'-status153','status');
  assert.equal(output.status.expected.rows.filter(row=>row[1]===null&&row[2]===null&&typeof row[7]==='string'&&row[7]).length,1,'Real transport failure remains unknown HTTP + actual error text');
  await configureAll('responses',responseColumns);output.responses=await exportWhileSecondPage(allResponses(job),responseColumns,job,label+'-responses153','responses');
  const target=responseTwins(job)[0].url;await rule('url','equals',{text:target});output.twins=await exportWhileSecondPage(responseTwins(job),responseColumns,job,label+'-response-twins2','responses');
  await configureAll('titles',mainColumns);await page.fill('#table-search','owned-XLSX-no-result-'+nonce);output.empty=await exportWhileSecondPage([],mainColumns,job,label+'-empty0','titles');assert.equal(output.empty.expected.rows.length,0);
  return output;
}
async function zeroDownloadsAfter(label,before){
  const observed=[],start=Date.now();
  for(const target of[0,50,200]){const remaining=target-(Date.now()-start);if(remaining>0)await sleep(remaining);const count=downloads.filter(d=>d.source==='browser-event').length;assert.equal(count,before,label+' no download at measured+'+target+'ms');observed.push({target_ms:target,elapsed_ms:Date.now()-start,count});}
  return observed;
}
async function staleRace(kind){
  await selectJob(jobA.id);await configureMain(kind==='mode'?1:2);await firstPage();const original=await domSnapshot(),stored=await storedBytes(),before=downloads.filter(d=>d.source==='browser-event').length;
  // Listener runs after the product click handler, before its genuine next-task
  // continuation. It dispatches normal DOM controls; it never accesses App state,
  // installs fake records, or replaces Promise/RAF/timer/Blob/download methods.
  await page.evaluate(({kind,id})=>{
    window.__xlsxRaceReceipt=null;
    document.getElementById('export-xlsx').addEventListener('click',()=>{
      const status=document.getElementById('table-export-status'),receipt={kind,phase:status.dataset.phase,visible:!status.hidden&&status.getClientRects().length>0,code:status.dataset.code||'',at:performance.now()};
      const change=(selector,value,event='change')=>{const el=document.querySelector(selector);el.value=value;el.dispatchEvent(new Event(event,{bubbles:true}));};
      if(kind==='owner')change('#crawl-history',id);
      else if(kind==='section')document.querySelector('[data-section="status"]').click();
      else if(kind==='search')change('#table-search','Cafe altered XLSX race','input');
      else if(kind==='filter')change('#table-filter','all');
      else if(kind==='sort')change('#table-sort-direction','1');
      else if(kind==='mode')change('#table-rule-mode','grouped');
      else if(kind==='columns'){const input=document.querySelector('#table-column-list input[data-table-column][value="title_text"]');input.checked=false;input.dispatchEvent(new Event('change',{bubbles:true}));}
      else if(kind==='cancel')document.getElementById('table-export-cancel').click();
      else throw Error('Unknown race');
      window.__xlsxRaceReceipt=receipt;
    },{once:true});
  },{kind,id:jobB.id});
  await page.click('#export-xlsx');const receipt=await page.evaluate(()=>window.__xlsxRaceReceipt);assert(receipt);assert.equal(receipt.phase,'prepare','Race starts during actual preparation');assert.equal(receipt.visible,true,'Preparation phase was truly visible');
  await page.waitForFunction(()=>document.getElementById('table-export-status').dataset.phase!=='prepare');
  if(kind==='owner')await page.waitForFunction(id=>document.getElementById('crawl-history').value===id&&!document.getElementById('export-csv').disabled,jobB.id);
  const observations=await zeroDownloadsAfter(kind,before),after=await domSnapshot(),terminal=await page.locator('#table-export-status').evaluate(el=>({phase:el.dataset.phase,code:el.dataset.code||'',message:el.textContent,visible:!el.hidden&&el.getClientRects().length>0}));
  assert.deepEqual(await storedBytes(),stored,'Stale/cancel export never writes saved catalogs or columns');
  assert.equal(terminal.visible,true,'Genuine guard result is visibly reported');assert(terminal.message.trim(),'Genuine guard result has a meaningful message');
  if(kind==='cancel'){assert.equal(terminal.phase,'cancelled');assert.equal(terminal.code,'cancelled');assert.deepEqual(after,original,'Cancel changes no accepted query/rows');}
  else{assert.equal(terminal.phase,'refused');assert(['stale','unavailable'].includes(terminal.code),'Stale guard result, not an unrelated failure: '+terminal.code);assert.notDeepEqual(after,original,'Actual DOM control changed the owner/query/projection');}
  staleExports.push({kind,source:'Scripted real DOM race after product handler and before unmodified setTimeout(0) task continuation; no App internals',receipt,terminal,observations,original_sha256:sha(Buffer.from(JSON.stringify(original))),after_sha256:sha(Buffer.from(JSON.stringify(after)))});
  await sameSaved();
}
async function actualOversizeRefusal(){
  await selectJob(overflowJob.id);await configureAll('titles',titleKeys);assert.equal(overflowJob.report.pages.length,1);const nativeTitle=overflowJob.report.pages[0].titles[0];assert.equal(nativeTitle.length,32768,'Oversized title genuinely survived native capture; no fabricated PageRecord');
  const before=downloads.filter(d=>d.source==='browser-event').length,query=await domSnapshot(),stored=await storedBytes();await page.click('#export-xlsx');
  await page.waitForFunction(()=>document.getElementById('table-export-status').dataset.code==='cell_limit'&&document.getElementById('table-export-status').dataset.phase!=='prepare');
  const status=await page.locator('#table-export-status').evaluate(el=>({phase:el.dataset.phase,code:el.dataset.code,message:el.textContent,visible:!el.hidden&&el.getClientRects().length>0}));assert(status.visible&&status.message);await zeroDownloadsAfter('actual32768-codeunit title whole-export refusal',before);
  assert.deepEqual(await domSnapshot(),query);assert.deepEqual(await storedBytes(),stored);assert.deepEqual((await native('/api/crawls/'+overflowJob.id+'/project.json')).bytes,overflowJob.project);assert.deepEqual(fs.readFileSync(overflowJob.savedPath),overflowJob.saved);
  negativeControls.push({kind:'actual_native_oversized_cell_whole_export_refusal',id:overflowJob.id,title_code_units:nativeTitle.length,project_sha256:sha(overflowJob.project),status,no_partial_file:true,query_storage_report_unchanged:true});
}
async function invalidQueryRefusal(){
  await selectJob(jobA.id);await configureAll('titles',mainColumns);await rule('title_length','min',{min:3});
  await page.selectOption('#table-rule-column','title_length');await page.selectOption('#table-rule-op','range');await page.fill('#table-rule-min','10');await page.fill('#table-rule-max','2');await page.click('#table-rule-add');
  await page.locator('#table-explorer-error').waitFor({state:'visible'});assert.equal(await page.locator('#export-xlsx').isDisabled(),true);
  const before=downloads.filter(d=>d.source==='browser-event').length;await zeroDownloadsAfter('invalid range query',before);negativeControls.push({kind:'actual_invalid_query_disables_xlsx',source:'Real range10..2 editor controls',no_header_only_false_success:true});
}
async function geometryCases(){
  await selectJob(jobB.id);await configureMain(2);await exact(selectedTitles(jobB),mainColumns,'Geometry full118 projection');
  for(const width of[1440,320]){
    await page.setViewportSize({width,height:1000});await options();
    for(const selector of['#export-csv','#export-xlsx','#table-rule-mode','#table-sort-column','#table-sort-direction','#page-next']){
      const element=page.locator(selector);await element.scrollIntoViewIfNeeded();const box=await element.boundingBox();assert(box&&box.x>=-1&&box.x+box.width<=width+1&&box.y>=-1&&box.y+box.height<=1001,selector+' fully reachable at '+width);geometry.push({width,selector,...box});
    }
    const file=await exportWhileSecondPage(selectedTitles(jobB),mainColumns,jobB,'warm-b-responsive'+width,'titles');assert.equal(file.matrix_sha256,retained.b.v2.matrix_sha256);assert.deepEqual(file.csvBytes,retained.b.v2.csvBytes);
    assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'No horizontal document overflow at '+width);await page.screenshot({path:artifactPath('xlsx-responsive-'+width+'.png'),fullPage:true});
  }
  await page.setViewportSize({width:1440,height:1000});
}
function corruptReaderControl(){
  assert(workbookFiles.length);const source=fs.readFileSync(workbookFiles[0].path),bad=Buffer.from(source),archive=packageParts(source),needle=Buffer.from('<worksheet');const at=bad.indexOf(needle);assert(at>0);bad[at+1]^=1;
  const file=artifactPath('declared-negative-corrupt-crc.xlsx');fs.writeFileSync(file,bad);assert.throws(()=>packageParts(bad),/CRC/);
  negativeControls.push({kind:'independent_reader_rejects_declared_corrupt_copy',source_sha256:archive.sha256,negative_sha256:sha(bad),path:file,scope:'Reader negative control only; no product acceptance claim'});
}
async function savedAll(){await sameSaved();if(overflowJob){assert.deepEqual(fs.readFileSync(overflowJob.savedPath),overflowJob.saved);assert.deepEqual((await native('/api/crawls/'+overflowJob.id+'/project.json')).bytes,overflowJob.project);}}

(async()=>{
  try{
    assert(assets.includes('table-xlsx.js')&&assets.includes('table-export-ui.js'),'Actual embedded XLSX and controller asset closure required');
    await ownedWebsite();const warm=await launch('warm');await bindAssets();jobA=await captureJob(siteOrigin+'/a/');jobB=await captureJob(siteOrigin+'/b/');overflowJob=await captureJob(siteOrigin+'/overflow/');nativeControls();capturedSiteRequests=siteHits.length;
    browser=await chromium.launch({headless:true,executablePath:chrome});warmContext=await context();page=await pageIn(warmContext);assert.deepEqual(await page.evaluate(()=>Object.keys(localStorage)),[]);
    for(const[job,label]of[[jobA,'a'],[jobB,'b']])retained[label]=await positiveCases(job,'warm-'+label);
    check('Native captures retain all152 HTML/153status/153response observations per owner, duplicate118 cohorts, raw null vs empty/measured0/NFD, literal unsafe strings and original redirect twins');
    await invalidQueryRefusal();for(const kind of['owner','section','search','filter','sort','mode','columns','cancel'])await staleRace(kind);await actualOversizeRefusal();await geometryCases();corruptReaderControl();await savedAll();storageState=await warmContext.storageState();
    offline=true;await witness();await page.goto('about:blank');await warmContext.close();warmContext=null;await stop(warm);const cold=await launch('cold',new URL(warm.origin).host);assert.equal(cold.origin,warm.origin);assert.notEqual(cold.child.pid,warm.child.pid);await bindAssets();coldContext=await context(storageState);page=await pageIn(coldContext);
    for(const[job,label]of[[jobA,'a'],[jobB,'b']]){
      const repeated=await positiveCases(job,'cold-'+label);
      for(const key of Object.keys(retained[label])){assert.deepEqual(repeated[key].csvBytes,retained[label][key].csvBytes,'Actual warm/cold CSV bytes unchanged');assert.equal(repeated[key].matrix_sha256,retained[label][key].matrix_sha256,'Actual warm/cold complete typed workbook matrix unchanged');assert.deepEqual(repeated[key].bytes,retained[label][key].bytes,'Reviewed deterministic writer yields exact workbook bytes');}
    }
    await savedAll();await page.goto('about:blank');await coldContext.close();coldContext=null;await stop(cold);await witness();assert.deepEqual(offlineHits,[]);assert.equal(siteHits.length,capturedSiteRequests);assert.equal(witnesses.length,2);assert.equal(owners.length,2);assert(owners.every(owner=>owner.natural_eof&&!owner.forced&&owner.close.code===0&&owner.close.signal===null));assert.deepEqual(errors,[]);assert.deepEqual(external,[]);assert.equal(served.length,assets.length*2);assert.equal(staleExports.length,8);assert.equal(workbookFiles.length,30);assert.equal(csvPairs.length,30);assert.equal(downloads.filter(d=>d.source==='browser-event').length,60);assert.equal(geometry.length,12);pinsUnchanged();
    check('Actual warm/cold downloaded workbooks and CSV bytes retain complete independent populations, ordered cells/types/provenance and original occurrence identity, with1440/320 reachable controls,8 real-DOM prepare races, genuine cell-limit refusal, unchanged reports, natural native EOF0 and zero offline product refetches');
  }catch(error){failure=error;}
  finally{
    if(page&&!page.isClosed()&&failure)try{await page.screenshot({path:artifactPath('table-xlsx-failure.png'),fullPage:true});fs.writeFileSync(artifactPath('table-xlsx-failure.html'),await page.content());}catch{}
    if(browser)try{await deadline(browser.close(),10000,'owned Chromium cleanup');}catch(error){failure||=error;}
    for(const owner of owners)if(!owner.natural_eof&&owner.child.exitCode===null){try{await stop(owner);}catch(error){failure||=error;if(owner.child.exitCode===null){owner.forced=true;owner.child.kill();try{await deadline(owner.closed,10000,'owned native forced-close witness');}catch(closeError){failure||=closeError;}}}}
    if(website?.listening){website.closeAllConnections();await new Promise(resolve=>website.close(resolve));}for(const owner of owners)fs.writeFileSync(artifactPath('native-'+owner.phase+'.log'),owner.log);try{pinsUnchanged();}catch(error){failure||=error;}
    fs.writeFileSync(artifactPath('ui-table-xlsx-live.json'),JSON.stringify({result:failure?'FAIL':'PASS',failure:failure?.stack,checks,binary_sha256:binarySha,pins,served_assets:served,native:owners.map(({phase,origin,child,natural_eof,forced,close})=>({phase,origin,pid:child.pid,natural_eof,forced,close})),native_captures:[jobA,jobB,overflowJob].filter(Boolean).map(job=>({id:job.id,pages:job.report.pages.length,project_sha256:sha(job.project),saved_sha256:sha(job.saved)})),native_reads:nativeReads,website_capture_requests:capturedSiteRequests,site_hits:siteHits,offline_product_requests:offlineHits,positive_410_controls:witnesses,owner_readiness:ownerReadiness,views,workbooks:workbookFiles,csv_pairs:csvPairs,prepare_races:staleExports,negative_controls:negativeControls,downloads,geometry,errors,external_requests:external,not_proven:['Other main tables, segmented/selected-recrawl provenance, resource/policy/content/sitemap specialist exports, server cursor and bulk/XLSX integration remain open backlog.','Default row/column/cell/payload/package boundaries are product pure proof;153-page native corpus does not establish those limits or large-corpus performance.','Corrupted copy proves independent reader refusal only.','Python/openpyxl and actual Excel/LibreOffice GUI interoperability are separate proofs; this CI reader uses Node ZIP/CRC and fresh Chromium DOMParser only.','Browser download-event MIME is not independently exposed; MIME is covered by controller/core pure contract and served-source pinning.','Scripted DOM prepare races cover measured ordering and later0/50/200ms observations; arbitrary scheduling permutations and all browsers remain unproven.'],scope:'Two genuine API-started owned website captures preserve original R2 153page/152HTML/118overlap structure with repeated literal formula/XML/Xstring strings. Extra genuine one-page32768-title capture is negative only. Positive exact whole-file cells/types/order come from immutable original saved PageRecords + declared fixture truth; never product compiler/projection/writer or CSV-only oracle. No success HTTP mocks, App internals, renderer substitutes, Promise/RAF/timer overrides, fake native records or product download interception.'},null,2)+'\n');
  }
  if(failure)throw failure;console.log('Evidence: '+output);
})().catch(error=>{console.error(error.stack);process.exitCode=1;});

