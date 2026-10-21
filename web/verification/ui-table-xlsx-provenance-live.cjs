'use strict';
// Actual native saved captures -> complete paired CSV/XLSX main-table downloads.
// Every expected population is independently enumerated from declared fixture indices
// and original PageRecords; production compiler/evaluator is never an oracle import.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const {spawn}=require('node:child_process'),{createHash,randomUUID}=require('node:crypto');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const root=path.resolve(__dirname,'../..'),sha=bytes=>createHash('sha256').update(bytes).digest('hex');
assert(process.env.SPIDER_BINARY&&process.env.SPIDER_BINARY_SHA256,'Explicit reviewed normal binary/SHA required; never build or install');
const binary=fs.realpathSync(process.env.SPIDER_BINARY),binarySha=sha(fs.readFileSync(binary));assert.equal(binarySha,process.env.SPIDER_BINARY_SHA256);
const chrome=process.env.OPEN_SEO_BROWSER_PATH||process.env.SPIDER_CHROME;assert(chrome&&fs.statSync(chrome).isFile(),'Explicit pinned Chromium required');
const output=path.resolve(process.env.SPIDER_TABLE_XLSX_PROVENANCE_OUTPUT||path.join(process.env.SPIDER_TEST_ARTIFACT_DIR||path.join(root,'artifacts'),'table-xlsx-provenance-'+Date.now()+'-'+randomUUID().slice(0,8)));
assert(!fs.existsSync(output),'Fresh owned output required');fs.mkdirSync(output,{recursive:true});process.env.SPIDER_TEST_ARTIFACT_DIR=output;
const artifactPath=require('../../web/verification/artifact-path.cjs'),store=path.join(output,'store'),nonce=randomUUID();
const assets=[...new Set(['index.html','styles.css',...Array.from(fs.readFileSync(path.join(root,'web/index.html'),'utf8').matchAll(/<script\s+src="\/([^"?#]+)"/g),match=>match[1])])];
const pins=[__filename,path.join(root,'web/verification/artifact-path.cjs'),path.join(root,'web/verification/xlsx-package-oracle.cjs'),path.join(__dirname,'xlsx-provenance-oracle.cjs'),binary,chrome,...assets.map(name=>path.join(root,'web',name))].map(file=>({file,sha256:sha(fs.readFileSync(file))}));
const checks=[],owners=[],served=[],nativeReads=[],downloads=[],errors=[],external=[],siteHits=[],offlineHits=[],witnesses=[],views=[],geometry=[],negativeControls=[];
const titleKeys=['url','title_text','title_length','title_pixels','title_count','duplicates','status'];
const statusKeys=['url','initial_status','status','final_url','redirects','response_ms','indexable','error'];
const labels={url:'URL',title_text:'Seitentitel',title_length:'Zeichen',title_pixels:'Pixelprofil 20px',title_count:'Title-Tags',duplicates:'Vorkommen',status:'HTTP final',initial_status:'HTTP initial',final_url:'Finale URL',redirects:'Redirects',response_ms:'Antwortzeit',indexable:'Indexierbarkeit',error:'Abruffehler',requested_url:'Anfrage-URL',response_index:'Antwortposition'};
const metadataLabels=['Crawl Coverage','Observation Basis','Selected State','Parent Snapshot SHA256','Fetched At','Observation Requested URL'];
const reportingLabels=['Captured Pages','Visible Pages','Hidden Pages'];
const collator=new Intl.Collator('de-DE',{numeric:true,sensitivity:'base'}),number=value=>Number(value).toLocaleString('de-DE');
let website,siteOrigin,app,browser,page,warmContext,coldContext,offline=false,failure,parent,limited,completed,pendingCapture,editor,capturedSiteRequests,storageState;
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
async function section(name){await page.click('[data-section="'+name+'"]');await page.locator('#table-options').waitFor({state:'attached'});await options();await page.locator('#table-sort-column').waitFor({state:'visible'});}
async function options(){const details=page.locator('#table-options');if(!await details.evaluate(element=>element.open))await details.locator(':scope > summary').click();}
async function columnOptions(){await options();const details=page.locator('#table-options details.table-columns');if(!await details.evaluate(element=>element.open))await details.locator(':scope > summary').click();}
async function headers(){return page.locator('#results-table thead [data-sort]').evaluateAll(nodes=>nodes.map(node=>node.dataset.sort));}
async function firstPage(){while(!await page.locator('#page-prev').isDisabled())await page.click('#page-prev');}
async function rule(key,op,values={}){await options();await page.selectOption('#table-rule-column',key);await page.selectOption('#table-rule-op',op);if(Object.hasOwn(values,'text'))await page.fill('#table-rule-text',values.text);if(Object.hasOwn(values,'min'))await page.fill('#table-rule-min',String(values.min));if(Object.hasOwn(values,'max'))await page.fill('#table-rule-max',String(values.max));await page.click('#table-rule-add');}
async function sort(key,direction){await options();await page.selectOption('#table-sort-column',key);await page.selectOption('#table-sort-direction',String(direction));}
async function columns(desired){await columnOptions();const all=await page.locator('#table-column-list input[data-table-column]').evaluateAll(nodes=>nodes.map(node=>({key:node.value,disabled:node.disabled})));for(const item of all){if(item.disabled){assert(desired.includes(item.key));continue;}await columnOptions();const input=page.locator('#table-column-list input[data-table-column][value="'+item.key+'"]');if(await input.isChecked()!==desired.includes(item.key))await input.setChecked(desired.includes(item.key));}
  for(let index=1;index<desired.length;index++){for(let guard=0;(await headers()).indexOf(desired[index])>index;guard++){assert(guard<128);await columnOptions();const before=await headers();await page.click('#table-column-list button[data-table-column-move="up"][data-column-key="'+desired[index]+'"]');assert.notDeepEqual(await headers(),before);}}assert.deepEqual(await headers(),desired);
}
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
async function actualDownload(selector,label){
  const control=page.locator(selector);
  assert.equal(await control.isVisible(),true,label+' download control must be visible: '+selector);
  assert.equal(await control.isEnabled(),true,label+' validated export must be enabled: '+selector);
  const[download]=await Promise.all([page.waitForEvent('download'),control.click()]);
  return download;
}

// Ignored preparation candidate: no native/browser work was run while authoring.
const O=require('./xlsx-provenance-oracle.cjs');
const {inspectWorkbook}=require('../../web/verification/xlsx-package-oracle.cjs');
const editorAssets=['saved-segments.html','saved-segments.js','saved-segments.css'],editorServed=[];
for(const name of editorAssets){const file=path.join(root,'web',name);pins.push({file,sha256:sha(fs.readFileSync(file))});}
const products=Array.from({length:110},(_,index)=>'/products/'+String(index).padStart(4,'0'));
const selectedPaths=['/redirect-a?x=1&y=2','/redirect-b?x=1&y=2'];
const allPaths=['/home',...products,'/products/hidden','/outside','/untitled',...selectedPaths];
const files=[],reads=[],readPromises=[],posts=[],segmentViews=[],races=[],offlinePhases=[],captureRecords=[],snapshots=[];
let fixturePhase='OLD',holdA=false,held=null,rules,responseSequence=0;
async function bindEditorAssets(){for(const name of editorAssets){const value=await native(name==='saved-segments.html'?'/saved-segments':'/'+name);assert.deepEqual(value.bytes,fs.readFileSync(path.join(root,'web',name)));editorServed.push({phase:app.phase,path:name,sha256:sha(value.bytes)});}}
const html=(title,links=[],hidden=false)=>'<!doctype html><html lang="en"><head><meta charset="utf-8">'+(title===null?'':'<title>'+title+'</title>')+'<meta name="description" content="Genuine owned provenance fixture">'+(hidden?'<meta name="robots" content="noindex">':'')+'</head><body><h1>Owned original document</h1>'+links.map(href=>'<a href="'+href.replace(/&/g,'&amp;')+'">Owned link</a>').join('')+'</body></html>';
function hits(){const result=new Map();for(const hit of siteHits)result.set(hit.path,(result.get(hit.path)||0)+1);return result;}
function exactHitDelta(before,allowed){const now=hits(),wanted=new Map(before);for(const pathname of allowed)wanted.set(pathname,(wanted.get(pathname)||0)+1);assert.deepEqual([...now.entries()].sort(),[...wanted.entries()].sort(),'Only explicitly selected request/redirect hops were fetched');}
async function ownedWebsite(){
  website=http.createServer((request,response)=>{
    if(request.url==='/offline-positive-control'&&request.headers['x-owned-offline-witness']===nonce){assert(offline);witnesses.push({phase:app?.phase,path:request.url,status:410});response.writeHead(410,{'content-type':'text/plain'});return response.end('Owned provenance offline witness');}
    const hit={path:request.url,method:request.method,phase:app?.phase,fixture_phase:fixturePhase};siteHits.push(hit);if(offline){offlineHits.push(hit);response.writeHead(410);return response.end('Owned offline fixture');}
    const url=new URL(request.url,'http://owned'),send=()=>{
      if(['/redirect-a','/redirect-b'].includes(url.pathname)){response.writeHead(302,{location:'/shared-final'});return response.end();}
      response.writeHead(200,{'content-type':'text/html; charset=utf-8','cache-control':'no-store'});
      if(url.pathname==='/home')return response.end(html('OLD HOME',allPaths.filter(value=>value!=='/home')));
      if(url.pathname==='/shared-final')return response.end(html(fixturePhase+' SHARED FINAL',fixturePhase==='NEW'?['/never-admit-new-link']:[]));
      if(products.includes(url.pathname))return response.end(html(fixturePhase+' Product cohort '+Number(url.pathname.slice(-4))%7));
      if(url.pathname==='/products/hidden')return response.end(html(fixturePhase+' Product hidden',[],true));
      if(url.pathname==='/untitled')return response.end(html(null));
      if(url.pathname==='/outside')return response.end(html(fixturePhase+' OUTSIDE'));
      response.end(html('MUST NOT BE ADMITTED'));
    };
    if(fixturePhase==='NEW'&&holdA&&url.pathname==='/redirect-a'){holdA=false;held={response,send};return;}
    send();
  });
  await new Promise((resolve,reject)=>{website.once('error',reject);website.listen(0,'127.0.0.1',resolve);});siteOrigin='http://127.0.0.1:'+website.address().port;assert(!['8787','8790'].includes(String(website.address().port)));
}
async function capture(id,label){
  const detail=(await api('/api/crawls/'+id)).value;assert(['completed','limited'].includes(detail.status));
  const raw=await native('/api/crawls/'+id+'/project.json'),report=JSON.parse(raw.bytes),savedPath=path.join(store,id+'.json'),saved=fs.readFileSync(savedPath);
  const canonicalPath=path.join(store,'snapshots',id,'generation-'+detail.generation+'.json'),canonicalBytes=fs.readFileSync(canonicalPath);
  assert.deepEqual(JSON.parse(saved),report);assert.deepEqual(JSON.parse(canonicalBytes),report);fs.writeFileSync(artifactPath(label+'-original-project.json'),raw.bytes);fs.writeFileSync(artifactPath(label+'-canonical-generation.json'),canonicalBytes);
  const result={id,detail,report,project:raw.bytes,saved,savedPath,canonicalPath,canonicalBytes,source_basis:'Actual native stable API project, own saved JSON and immutable canonical generation bytes'};
  snapshots.push(result);captureRecords.push({id,label,status:detail.status,generation:detail.generation,pages:report.pages.length,project_sha256:sha(raw.bytes),canonical_sha256:sha(canonicalBytes)});return result;
}
async function untilStatus(id,status){return until(async()=>{const value=(await api('/api/crawls/'+id)).value;assert.notEqual(value.status,'failed',value.error);return value.status===status&&value;},'native '+id+' '+status);}
async function observeRead(response){
  const sequence=responseSequence++,body=response.request().postDataBuffer(),headers=await response.allHeaders();assert(body);const url=new URL(response.url());
  const raw=await fetch(response.url(),{method:'POST',headers:{'content-type':'application/json'},body,redirect:'error',cache:'no-store',signal:AbortSignal.timeout(30000)}),bytes=Buffer.from(await raw.arrayBuffer());
  assert.equal(response.status(),200);assert.equal(raw.status,200);for(const key of['content-type','content-length','x-content-sha256','cache-control','x-segment-snapshot-token','x-segment-generation'])assert.equal(headers[key],raw.headers.get(key));
  assert.equal(headers['x-content-sha256'],sha(bytes));assert.equal(Number(headers['content-length']),bytes.length);assert.equal(headers['cache-control'],'no-store');
  const value=JSON.parse(bytes);reads.push({sequence,phase:app.phase,path:url.pathname,request:JSON.parse(body),request_sha256:sha(body),bytes:bytes.length,sha256:sha(bytes),value,source:'Exact independently repeated native read-only POST; browser SHA headers plus full DOM/file cells bind installed content'});
}
async function context(storage){
  const owner=app,created=await browser.newContext({acceptDownloads:true,viewport:{width:1440,height:1000},...storage?{storageState:storage}:{}});
  await created.route('**/*',route=>{const url=route.request().url();if(url.startsWith(owner.origin+'/')||/^(about:|blob:|data:)/.test(url))return route.continue();external.push(url);return route.abort();});
  created.on('page',target=>{
    target.on('pageerror',error=>errors.push(error.message));target.on('download',download=>downloads.push({phase:owner.phase,suggested:download.suggestedFilename()}));
    target.on('request',request=>{if(request.method()==='POST'&&/\/(recrawl|resume)$/.test(new URL(request.url()).pathname))posts.push({phase:owner.phase,url:request.url(),body:request.postDataJSON()});});
    target.on('response',response=>{if(response.request().method()==='POST'&&/\/segments\/dashboard(?:-analysis)?$/.test(new URL(response.url()).pathname)){const pending=observeRead(response);readPromises.push(pending);pending.catch(()=>{});}});
  });return created;
}
async function pageIn(created){const target=await created.newPage();await target.goto(app.origin);await target.locator('#connection-state.connected').waitFor();return target;}
async function selectCapture(value){
  await page.locator('#crawl-history option[value="'+value.id+'"]').waitFor({state:'attached'});
  const final=page.waitForResponse(response=>response.url()===app.origin+'/api/crawls/'+value.id+'/fragments'&&response.request().method()==='GET');
  const[response]=await Promise.all([final,page.selectOption('#crawl-history',value.id)]);assert.equal(response.status(),200);assert.equal(await response.finished(),null);
  await page.waitForFunction(id=>document.getElementById('crawl-history').value===id&&!document.getElementById('export-csv').disabled,value.id);
}
async function configure(sectionName,keys=O.PROJECTION){await section(sectionName);await clearQuery();if(await page.inputValue('#table-rule-mode')!=='simple')await page.selectOption('#table-rule-mode','simple');await sort('url',1);await columns(keys);}
async function exact(capture,rows,keys,label){
  await page.selectOption('#page-size','50');await firstPage();const collected=[];
  for(let offset=0;offset<Math.max(rows.length,1);offset+=50){
    const expected=rows.slice(offset,offset+50).map(row=>({cells:keys.map(key=>O.display(row,key)),age:O.age(capture,row)})),wantedPage=rows.length?number(offset+1)+'–'+number(Math.min(offset+50,rows.length))+' von '+number(rows.length):'Keine Ergebnisse';
    await page.waitForFunction(({expected,wantedPage})=>{
      const actual=[...document.querySelectorAll('#results-table tbody tr[data-row]')].map(row=>({cells:[...row.cells].map(cell=>{const cloned=cell.cloneNode(true);cloned.querySelectorAll('.observation-age,input').forEach(node=>node.remove());return cloned.textContent.trim();}),age:row.querySelector('.observation-age')?.textContent||''}));
      return document.getElementById('page-label').textContent===wantedPage&&JSON.stringify(actual)===JSON.stringify(expected);
    },{expected,wantedPage});
    const actual=await page.locator('#results-table tbody tr[data-row]').evaluateAll(nodes=>nodes.map(row=>({cells:[...row.cells].map(cell=>{const cloned=cell.cloneNode(true);cloned.querySelectorAll('.observation-age,input').forEach(node=>node.remove());return cloned.textContent.trim();}),age:row.querySelector('.observation-age')?.textContent||''})));assert.deepEqual(actual,expected,label+' independently expected URL/text and separate exact age badges');collected.push(...actual);assert.deepEqual(await headers(),keys);if(offset+50<rows.length)await page.click('#page-next');
  }
  assert.equal(collected.length,rows.length);await firstPage();if(rows.length>50)await page.click('#page-next');views.push({label,rows:rows.length,keys,DOM_sha256:sha(Buffer.from(JSON.stringify(collected)))});
}
async function pair(capture,sectionName,label,view=null,keys=sectionName==='titles'?O.PROJECTION:O.STATUS_KEYS,indexabilityText=null){
  await configure(sectionName,keys);if(indexabilityText!==null){assert(['Indexierbar','Nicht indexierbar'].includes(indexabilityText));await rule('indexable','equals',{text:indexabilityText});}
  const population=O.rows(capture,sectionName,view),rows=O.order(indexabilityText===null?population:population.filter(row=>row.indexable===(indexabilityText==='Indexierbar')),'url',1);await exact(capture,rows,keys,label);
  const expected=O.matrix(capture,rows,keys,sectionName,view),query=await domSnapshot(),storage=await storedBytes(),count=downloads.length;
  let download=await actualDownload('#export-csv',label+' CSV');assert.equal(download.suggestedFilename(),'open-seo-spider-'+sectionName+'-'+capture.id+'.csv');const csvFile=artifactPath(label+'.csv');await download.saveAs(csvFile);const csvBytes=fs.readFileSync(csvFile);assert.deepEqual(csvBytes,O.csvBytes(expected),'Actual complete CSV bytes independently expected from captured facts/provenance');
  download=await actualDownload('#export-xlsx',label+' XLSX');assert.equal(download.suggestedFilename(),'open-seo-spider-'+sectionName+'-'+capture.id+'.xlsx');const file=artifactPath(label+'.xlsx');await download.saveAs(file);const bytes=fs.readFileSync(file),decoded=await inspectWorkbook(browser,bytes,expected);
  await page.waitForFunction(()=>document.getElementById('table-export-status').dataset.phase==='complete');assert.equal(downloads.length,count+2);assert.deepEqual(await domSnapshot(),query);assert.deepEqual(await storedBytes(),storage);
  fs.writeFileSync(artifactPath(label+'.expected.json'),JSON.stringify({source:capture.source_basis,project_sha256:capture.project?sha(capture.project):null,canonical_sha256:capture.canonicalBytes?sha(capture.canonicalBytes):null,section:sectionName,keys,...expected},null,2)+'\n');fs.writeFileSync(artifactPath(label+'.reader.json'),JSON.stringify(decoded,null,2)+'\n');
  const result={phase:app.phase,label,section:sectionName,path:file,rows:rows.length,columns:expected.header,bytes:bytes.length,sha256:sha(bytes),csv_sha256:sha(csvBytes),matrix_sha256:decoded.matrix_sha256,context:view?.context??null,stats:decoded.stats};files.push(result);return{...result,bytesValue:bytes,csvBytes};
}
async function noSegment(){await page.waitForFunction(()=>document.getElementById('dashboard-segment-json').disabled&&document.getElementById('dashboard-segments').getAttribute('aria-busy')==='false');assert.equal(await page.locator('#dashboard-segment-scope').isHidden(),true);}
async function authorRules(){
  editor=await warmContext.newPage();await editor.goto(app.origin+'/saved-segments');
  for(const definition of rules.definitions){await editor.click('#add');const fieldset=editor.locator('#definitions fieldset').last();await fieldset.getByLabel('Name',{exact:true}).fill(definition.name);const condition=definition.conditions[0];await fieldset.locator('.condition select').nth(0).selectOption(condition.field);await fieldset.locator('.condition select').nth(1).selectOption(condition.operator);await fieldset.getByLabel('Wert',{exact:true}).fill(condition.value);}
  await editor.click('#save');assert.match(await editor.locator('#message').textContent(),/gespeichert/);assert.deepEqual(await editor.evaluate(()=>JSON.parse(localStorage.getItem('open-seo-saved-segments-v1'))),rules);await editor.close();editor=null;
}
async function apply(capture,selectionValue='segment:s1',mode='reported'){
  await page.click('#dashboard-segment-load');await page.selectOption('#dashboard-segment-selection',selectionValue);await page.selectOption('#dashboard-segment-mode',mode);assert.equal(await page.locator('#dashboard-segment-json').isDisabled(),true);
  const start=reads.length;await page.click('#dashboard-segment-apply');await page.waitForFunction(()=>!document.getElementById('dashboard-segment-json').disabled&&document.getElementById('dashboard-segments').getAttribute('aria-busy')==='false');await Promise.all(readPromises);
  const current=reads.slice(start).sort((a,b)=>a.sequence-b.sequence),pageReads=current.filter(read=>read.path.endsWith('/dashboard')&&read.request.view==='pages'),issueReads=current.filter(read=>read.path.endsWith('/dashboard')&&read.request.view==='issues'),analysis=current.find(read=>read.path.endsWith('/dashboard-analysis'));assert(analysis);
  const selection=O.declaration(selectionValue),view=O.checkSegment(capture,rules,selection,mode,pageReads.map(read=>read.value),issueReads.map(read=>read.value),analysis.value);
  for(const group of[pageReads,issueReads])for(let index=0;index<group.length;index++){assert.equal(group[index].request.limit,200);assert.deepEqual(group[index].request.cursor,index?group[index-1].value.next:null);assert.equal(group[index].value.next?.offset??group[index].value.total,group.slice(0,index+1).reduce((sum,item)=>sum+item.value.rows.length,0));}
  segmentViews.push({phase:app.phase,capture:capture.id,generation:capture.detail.generation,selection,mode,rows:view.ordinals.length,context:view.context,reporting:view.cohort.reporting});return{...view,request:pageReads[0].request};
}
async function stableCases(capture,label){
  await selectCapture(capture);await noSegment();const result={normal:await pair(capture,'titles',label+'-ordinary'),status:await pair(capture,'status',label+'-status')};
  for(const mode of['reported','raw']){const view=await apply(capture,'segment:s1',mode);result[mode]=await pair(capture,'titles',label+'-segment-'+mode,view);result[mode+'View']=view;if(mode==='raw'){const rawStatus=O.rows(capture,'status',view);assert(rawStatus.some(row=>row.indexable===true)&&rawStatus.some(row=>row.indexable===false),'Genuine noindex source supplies true and false raw booleans');result.rawStatus=await pair(capture,'status',label+'-raw-status-bools',view);result.falseFilter=await pair(capture,'status',label+'-raw-text-filter-false',view,O.STATUS_KEYS,'Nicht indexierbar');assert.equal(result.falseFilter.rows,rawStatus.filter(row=>row.indexable===false).length);assert.equal(rawStatus.find(row=>new URL(row.url).pathname==='/products/hidden').indexable,false);result.trueFilter=await pair(capture,'status',label+'-raw-text-filter-true',view,O.STATUS_KEYS,'Indexierbar');assert.equal(result.trueFilter.rows,rawStatus.filter(row=>row.indexable===true).length);}}
  assert.notEqual(result.reported.context.snapshot_token,result.raw.context.snapshot_token);assert.equal(result.raw.rows,result.reported.rows+1,'Raw view includes genuine profile-hidden observation');
  const empty=await apply(capture,'segment:s3','reported');result.empty=await pair(capture,'titles',label+'-empty-segment',empty);assert.equal(result.empty.rows,0);await page.click('#dashboard-segment-clear');await noSegment();return result;
}
async function unchangedParent(){assert.deepEqual((await native('/api/crawls/'+parent.id+'/project.json')).bytes,parent.project);assert.deepEqual(fs.readFileSync(parent.savedPath),parent.saved);assert.deepEqual(fs.readFileSync(parent.canonicalPath),parent.canonicalBytes);}
async function unchangedGeneration(capture){assert.deepEqual(fs.readFileSync(capture.canonicalPath),capture.canonicalBytes);}
function sameFiles(before,after){for(const key of['normal','status','reported','raw','rawStatus','trueFilter','falseFilter','empty']){assert.deepEqual(after[key].csvBytes,before[key].csvBytes);assert.deepEqual(after[key].bytesValue,before[key].bytesValue);assert.equal(after[key].matrix_sha256,before[key].matrix_sha256);}}
async function offlineWitness(label){const response=await fetch(siteOrigin+'/offline-positive-control',{headers:{'x-owned-offline-witness':nonce},cache:'no-store',signal:AbortSignal.timeout(5000)});assert.equal(response.status,410);assert.equal(await response.text(),'Owned provenance offline witness');offlinePhases.push({label,phase:app?.phase,site_requests:siteHits.length,product_offline_requests:offlineHits.length,witness:witnesses.length});assert.deepEqual(offlineHits,[]);}
async function guard(capture,view,field,value,label){
  const body={...view.request,cursor:null,expected_generation:view.context.app_generation,expected_snapshot_token:view.context.snapshot_token,[field]:value},response=await api('/api/crawls/'+capture.id+'/segments/dashboard.json',body,409);
  assert.equal(response.response.headers.get('content-disposition'),null);assert.equal(response.response.headers.get('x-content-sha256'),null);assert(response.value.code&&response.value.message);negativeControls.push({label,id:capture.id,field,status:409,code:response.value.code,request:body});
}
async function noDownloads(label,before){const start=Date.now(),observations=[];for(const target of[0,50,200]){const delay=target-(Date.now()-start);if(delay>0)await sleep(delay);assert.equal(downloads.length,before,label+' no download at measured observation');observations.push({target_ms:target,elapsed_ms:Date.now()-start,downloads:downloads.length});}return observations;}
async function segmentRace(capture,kind){
  await selectCapture(capture);const view=await apply(capture,'segment:s1','reported');await configure('titles');const before=downloads.length,stored=await storedBytes(),query=await domSnapshot();
  await page.evaluate(({kind,parentID,id})=>{window.__xlsxProvenanceRace=null;document.getElementById('export-xlsx').addEventListener('click',()=>{
    const element=document.getElementById('table-export-status'),receipt={phase:element.dataset.phase,visible:!element.hidden&&element.getClientRects().length>0};
    const change=(selector,value,event='change')=>{const control=document.querySelector(selector);control.value=value;control.dispatchEvent(new Event(event,{bubbles:true}));};
    if(kind==='segment-selection')change('#dashboard-segment-selection','segment:s2');else if(kind==='reporting-mode')change('#dashboard-segment-mode','raw');else if(kind==='segment-clear')document.getElementById('dashboard-segment-clear').click();else if(kind==='query')change('#table-search','Product changed query','input');else if(kind==='owner')change('#crawl-history',parentID);else if(kind==='same-owner-reload')change('#crawl-history',id);else throw Error('Undeclared race');window.__xlsxProvenanceRace=receipt;
  },{once:true});},{kind,parentID:parent.id,id:capture.id});
  await page.click('#export-xlsx');const receipt=await page.evaluate(()=>window.__xlsxProvenanceRace);assert.equal(receipt.phase,'prepare');assert.equal(receipt.visible,true);
  await page.waitForFunction(()=>document.getElementById('table-export-status').dataset.phase!=='prepare');const terminal=await page.locator('#table-export-status').evaluate(element=>({phase:element.dataset.phase,code:element.dataset.code,message:element.textContent,visible:!element.hidden&&element.getClientRects().length>0}));
  assert.equal(terminal.phase,'refused');assert(['stale','unavailable'].includes(terminal.code));assert(terminal.visible&&terminal.message.trim());const observations=await noDownloads(kind,before);assert.deepEqual(await storedBytes(),stored);await unchangedParent();await unchangedGeneration(capture);
  races.push({kind,id:capture.id,context:view.context,receipt,terminal,observations,query_before_sha256:sha(Buffer.from(JSON.stringify(query))),query_after_sha256:sha(Buffer.from(JSON.stringify(await domSnapshot()))),source:'Real DOM controls after product export handler, before genuine unmodified setTimeout(0) continuation; no App access or timer overrides'});
}
async function selectTargets(){
  await selectCapture(parent);await configure('titles');for(const pathname of selectedPaths){const url=siteOrigin+pathname;await page.fill('#table-search',url);await page.waitForFunction(url=>document.querySelectorAll('#results-table tbody tr[data-row]').length===1&&document.querySelector('[data-recrawl-url]')?.dataset.recrawlUrl===url,url);await page.locator('[data-recrawl-url]').check();}
  await page.fill('#table-search','');await page.click('#recrawl-selected');assert.deepEqual(await page.locator('#selected-recrawl-urls li').allTextContents(),selectedPaths.map(pathname=>siteOrigin+pathname));
}
async function heldPending(childID){
  const detail=(await api('/api/crawls/'+childID)).value,actual=(await api('/api/crawls/'+childID+'/pages?limit=1000')).value;assert.equal(detail.operation.pending_count,2);assert.equal(detail.operation.refreshed_count,0);assert.equal(detail.operation.retained_count,114);assert.equal(actual.total,113);
  const urls=new Set(selectedPaths.map(pathname=>siteOrigin+pathname)),rawPages=parent.report.pages.filter(page=>!urls.has(page.url));assert.equal(rawPages.length,114);
  const visible=rawPages.filter(page=>new URL(page.url).pathname!=='/products/hidden');assert.deepEqual(actual.pages.map(row=>Object.fromEntries(Object.keys(visible.find(page=>page.url===row.url)).map(key=>[key,row[key]]))),visible);
  assert(!actual.pages.some(row=>urls.has(row.url)));assert.equal(await page.locator('#dashboard-segment-apply').isDisabled(),true,'No unstable segment snapshot is invented');
  pendingCapture={id:childID,detail,report:{config:parent.report.config,pages:rawPages,operation:detail.operation},source_basis:'Expected retained population from immutable genuine parent PageRecords minus exact selected requested URLs, all visible original fields independently matched against actual child pages API; running child has no persisted full report'};
  const refused=await native('/api/crawls/'+childID+'/project.json',undefined,409);negativeControls.push({kind:'running_child_has_no_stable_project',id:childID,status:refused.response.status,pending_urls:[...urls]});
  const result=await pair(pendingCapture,'titles','warm-pending-retained-only');assert.equal(result.rows,113);negativeControls.push({kind:'genuine_pending_target_absence',id:childID,pending_targets:2,current_rows:113,pending_PageRecords:0,not_proven:'selected_pending_not_refreshed positive row is deliberately unreachable: native excludes selected historical observations until fresh commit'});return result;
}
async function beginChild(){
  const before=hits(),postCount=posts.length;await selectTargets();await page.click('#selected-recrawl-cancel');assert.equal(posts.length,postCount);exactHitDelta(before,[]);
  fixturePhase='NEW';holdA=true;await selectTargets();await page.fill('#recrawl-max-attempts','1');const[response]=await Promise.all([page.waitForResponse(response=>response.url()===app.origin+'/api/crawls/'+parent.id+'/recrawl'&&response.request().method()==='POST'),page.locator('#selected-recrawl-form button[type=submit]').click()]);assert.equal(response.status(),200);const childID=(await response.json()).id;assert.notEqual(childID,parent.id);await until(()=>held,'real first selected-target HTTP barrier');
  assert.deepEqual(posts.at(-1).body,{urls:selectedPaths.map(pathname=>siteOrigin+pathname),max_attempts:1});await page.waitForFunction(id=>document.getElementById('crawl-history').value===id&&document.getElementById('recrawl-operation-summary').textContent.includes('Ausstehend: 2'),childID);
  await heldPending(childID);const barrier=held;held=null;barrier.send();await untilStatus(childID,'limited');limited=await capture(childID,'limited');assert.equal(limited.report.pages.length,115);assert.equal(limited.report.operation.refreshed_count,1);assert.equal(limited.report.operation.pending_count,1);assert.equal(limited.report.operation.retained_count,114);assert.equal(limited.report.operation.max_attempts,1);assert.equal(limited.report.config.max_pages,999);exactHitDelta(before,[selectedPaths[0],'/shared-final']);
  const byURL=new Map(limited.report.pages.map(page=>[page.url,page]));assert.equal(byURL.get(siteOrigin+selectedPaths[0]).titles[0],'NEW SHARED FINAL');assert(!byURL.has(siteOrigin+selectedPaths[1]));assert.equal(byURL.get(siteOrigin+'/outside').titles[0],'OLD OUTSIDE');await unchangedParent();
  await page.waitForFunction(()=>document.getElementById('recrawl-operation-summary').textContent.includes('Neu geprüft: 1')&&document.getElementById('crawl-state').textContent.includes('Grenze'));
  const download=await actualDownload('#recrawl-parent-snapshot','actual child immutable parent'),file=artifactPath('actual-child-immutable-parent.json');await download.saveAs(file);const bytes=fs.readFileSync(file);assert.equal(sha(parent.saved),limited.report.operation.parent.snapshot_sha256,'Declared parent SHA independently equals original exact saved bytes');assert.deepEqual(bytes,parent.saved);assert.equal(sha(bytes),limited.report.operation.parent.snapshot_sha256);assert.deepEqual(JSON.parse(bytes),parent.report);captureRecords.push({kind:'actual_child_parent_snapshot_download',path:file,bytes:bytes.length,sha256:sha(bytes)});return before;
}
async function resumeChild(){
  await selectCapture(limited);const before=hits();await page.setViewportSize({width:320,height:900});await page.click('#resume-button');assert.match(await page.locator('#resume-limit-heading').textContent(),/Abrufbudget/);await page.fill('#resume-max-pages','0');const button=page.locator('#resume-limit-form button[type=submit]');await button.scrollIntoViewIfNeeded();const box=await button.boundingBox();assert(box&&box.x>=0&&box.x+box.width<=320&&box.y>=0&&box.y+box.height<=900);geometry.push({label:'actual-selected-resume-budget320',width:320,...box});
  const[response]=await Promise.all([page.waitForResponse(response=>response.url()===app.origin+'/api/crawls/'+limited.id+'/resume'&&response.request().method()==='POST'),button.click()]);assert.equal(response.status(),200);assert.deepEqual(response.request().postDataJSON(),{max_attempts:0});await untilStatus(limited.id,'completed');completed=await capture(limited.id,'completed');assert.equal(completed.report.pages.length,116);assert.equal(completed.report.operation.refreshed_count,2);assert.equal(completed.report.operation.pending_count,0);exactHitDelta(before,[selectedPaths[1],'/shared-final']);await unchangedParent();await unchangedGeneration(limited);await page.setViewportSize({width:1440,height:1000});
}
async function viewportFiles(capture,label){
  await selectCapture(capture);const view=await apply(capture,'segment:s1','raw');await page.setViewportSize({width:320,height:900});for(const selector of['#dashboard-segment-selection','#dashboard-segment-mode','#dashboard-segment-apply','#export-csv','#export-xlsx','#page-next']){const control=page.locator(selector);await control.scrollIntoViewIfNeeded();const box=await control.boundingBox();assert(box&&box.x>=-1&&box.x+box.width<=321&&box.y>=-1&&box.y+box.height<=901,selector+' reachable320');geometry.push({label,width:320,selector,...box});}
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));const result=await pair(capture,'titles',label+'-raw320',view);await page.screenshot({path:artifactPath(label+'-320.png'),fullPage:true});await page.setViewportSize({width:1440,height:1000});return result;
}
(async()=>{
  let limitedWarm,limitedCold,completeWarm,completeCold;
  try{
    assert.equal(assets.length,17);await ownedWebsite();const warm=await launch('warm-limited');await bindAssets();await bindEditorAssets();const start=(await api('/api/crawls',{seed_urls:[siteOrigin+'/home'],crawl_links:true,discover_sitemaps:false,obey_robots:false,concurrency:1,delay_ms:0,max_pages:999,request_timeout_secs:30,directive_reporting:{...O.PROFILE}})).value;await untilStatus(start.id,'completed');parent=await capture(start.id,'parent');assert.equal(parent.report.pages.length,116);assert.deepEqual(new Set(parent.report.pages.map(page=>new URL(page.url).pathname+new URL(page.url).search)),new Set(allPaths));assert.deepEqual(O.profileCounts(parent.report),{captured:116,visible:115,hidden:1});
    rules={version:1,definitions:[{id:'s1',name:'Products and selected redirect sources',join:'all',conditions:[{field:'requested_url',operator:'regex',value:'/(?:products/(?:[0-9]{4}|hidden)|redirect-[ab])(?:\\?|$)',case_insensitive:false}]},{id:'s2',name:'Stored Product title overlap',join:'all',conditions:[{field:'stored_title',operator:'contains',value:'Product',case_insensitive:false}]},{id:'s3',name:'Known empty uncaptured target',join:'all',conditions:[{field:'requested_url',operator:'equals',value:siteOrigin+'/never-captured',case_insensitive:false}]}]};
    browser=await chromium.launch({headless:true,executablePath:chrome});warmContext=await context();page=await pageIn(warmContext);await selectCapture(parent);await authorRules();await beginChild();limitedWarm=await stableCases(limited,'warm-limited');assert.equal(limitedWarm.normal.rows,114);assert.equal(limitedWarm.reported.rows,111);assert.equal(limitedWarm.raw.rows,112);await viewportFiles(limited,'limited');
    for(const[field,value]of[['expected_snapshot_token','0'.repeat(64)],['expected_generation',limited.detail.generation+1],['reporting_mode','raw'],['rules',{...rules,definitions:rules.definitions.map((value,index)=>index?value:{...value,name:'Changed bound definition'})}]])await guard(limited,limitedWarm.reportedView,field,value,'stable-'+field);
    for(const kind of['segment-selection','reporting-mode','segment-clear','query','owner','same-owner-reload'])await segmentRace(limited,kind);await unchangedParent();storageState=await warmContext.storageState();offline=true;const beforeLimitedOffline=siteHits.length;await offlineWitness('limited-before-cold');await page.goto('about:blank');await warmContext.close();warmContext=null;await stop(warm);
    const coldLimited=await launch('cold-limited',new URL(warm.origin).host);assert.notEqual(coldLimited.child.pid,warm.child.pid);await bindAssets();coldContext=await context(storageState);page=await pageIn(coldContext);const actualLimited=await capture(limited.id,'cold-limited');assert.deepEqual(actualLimited.project,limited.project);limitedCold=await stableCases(actualLimited,'cold-limited');sameFiles(limitedWarm,limitedCold);await unchangedParent();await offlineWitness('limited-after-cold');assert.equal(siteHits.length,beforeLimitedOffline,'Saved limited read stage never refetches origin');
    // A new explicit user-confirmed resume is a distinct authorized capture stage,
    // outside both qualified offline saved-read windows.
    offline=false;await resumeChild();await guard(completed,limitedWarm.reportedView,'expected_generation',limited.detail.generation,'real-data-generation-change-after-resume');completeWarm=await stableCases(completed,'completed-before-cold');assert.equal(completeWarm.normal.rows,115);assert.equal(completeWarm.reported.rows,112);assert.equal(completeWarm.raw.rows,113);await viewportFiles(completed,'completed');await unchangedParent();storageState=await coldContext.storageState();offline=true;const beforeCompletedOffline=siteHits.length;await offlineWitness('completed-before-cold');await page.goto('about:blank');await coldContext.close();coldContext=null;await stop(coldLimited);
    const coldCompleted=await launch('cold-completed',new URL(warm.origin).host);assert.notEqual(coldCompleted.child.pid,coldLimited.child.pid);await bindAssets();coldContext=await context(storageState);page=await pageIn(coldContext);const actualComplete=await capture(completed.id,'cold-completed');assert.deepEqual(actualComplete.project,completed.project);completeCold=await stableCases(actualComplete,'cold-completed');sameFiles(completeWarm,completeCold);await unchangedParent();await unchangedGeneration(limited);await offlineWitness('completed-after-cold');assert.equal(siteHits.length,beforeCompletedOffline);assert.deepEqual(offlineHits,[]);assert.equal(witnesses.length,4);assert.equal(posts.filter(value=>value.url.endsWith('/recrawl')).length,1);assert.equal(posts.filter(value=>value.url.endsWith('/resume')).length,1);assert(!siteHits.some(hit=>hit.path==='/never-admit-new-link'));await page.goto('about:blank');await coldContext.close();coldContext=null;await stop(coldCompleted);assert.equal(owners.length,3);assert(owners.every(owner=>owner.natural_eof&&!owner.forced&&owner.close.code===0));assert.deepEqual(errors,[]);assert.deepEqual(external,[]);assert.equal(served.length,51);assert.equal(editorServed.length,3);assert.equal(races.length,6);assert.equal(files.length,35);assert.equal(downloads.length,71);pinsUnchanged();check('Actual selected parent/pending/limited/resumed provenance + complete reported/raw segment matrices survive both cold restarts, exact independent CSV/XLSX cells/types/ordinals, three EOF0 processes, four qualified410 witnesses and zero offline website refetches');
  }catch(error){failure=error;}
  finally{
    if(held){held.response.destroy();held=null;}if(page&&!page.isClosed()&&failure)try{await page.screenshot({path:artifactPath('provenance-failure.png'),fullPage:true});fs.writeFileSync(artifactPath('provenance-failure.html'),await page.content());}catch{}
    if(browser)try{await deadline(browser.close(),10000,'owned Chromium cleanup');}catch(error){failure||=error;}
    for(const owner of owners)if(!owner.natural_eof&&owner.child.exitCode===null){try{await stop(owner);}catch(error){failure||=error;if(owner.child.exitCode===null){owner.forced=true;owner.child.kill();try{await deadline(owner.closed,10000,'owned forced close');}catch(closeError){failure||=closeError;}}}}
    if(website?.listening){website.closeAllConnections();await new Promise(resolve=>website.close(resolve));}try{pinsUnchanged();}catch(error){failure||=error;}
    fs.writeFileSync(artifactPath('ui-table-xlsx-provenance-live.json'),JSON.stringify({result:failure?'FAIL':'PASS',failure:failure?.stack,checks,pins,served_assets:served,editor_served_assets:editorServed,native:owners.map(({phase,origin,child,natural_eof,forced,close})=>({phase,origin,pid:child.pid,natural_eof,forced,close})),captures:captureRecords,native_reads:nativeReads,segments:segmentViews,segment_native_reads:reads,posts,files,downloads,races,negative_controls:negativeControls,geometry,site_hits:siteHits,offline_product_requests:offlineHits,offline_qualified_observations:offlinePhases,positive_410_witnesses:witnesses,errors,external_requests:external,not_proven:['Pending selected historical rows intentionally have no current PageRecord; positive selected_pending_not_refreshed XLSX cells are not reachable and not fabricated.','Segment/raw modes change eligible/selected membership; generic Captured/Visible/Hidden columns remain saved-profile diagnostics, independently checked against raw records and native cohort.reporting profile fields.','Snapshot origin/generation are App declarations, not SQL/report-generation attestation; canonical generation files are witnessed own native output bytes.','Other segment operators, specialist/cursor/bulk formats, every schema, arbitrary scheduling, large corpus and vendor parity remain open.'],scope:'All positive observations come from genuine owned native parent/selected recrawl and saved generation reports. No product projection/compiler/writer is used as the expectation oracle. Complete typed XLSX and exact original CSV/provenance arrays are read independently. Exact read-only POST replay is explicitly attributed; no success HTTP mocks, synthetic PageRecords or mutation replays. Two saved-read offline windows bracket four qualified witnesses; the deliberate one-target resume occurs only after explicitly reopening the owned fixture.'},null,2)+'\n');
  }
  if(failure)throw failure;console.log('Evidence: '+output);
})().catch(error=>{console.error(error.stack);process.exitCode=1;});
