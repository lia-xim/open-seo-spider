// Owns the test-only native host, local HTTP provider, website trap and cold restart.
// No production provider URL override, paid credential or seeded job is used.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import {spawn} from 'node:child_process';
import {createHash, randomBytes} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const output=path.resolve(process.env.SPIDER_ENRICHMENT_ARTIFACT_DIR||path.join(root,'artifacts',`enrichment-success-${Date.now()}`));
assert(!fs.existsSync(output),'Each execution requires a fresh artifact directory');
fs.mkdirSync(path.dirname(output),{recursive:true});fs.mkdirSync(output);
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(work,label){for(let i=0;i<1200;i++){const result=await work();if(result)return result;await sleep(50);}throw Error('Timeout: '+label);}
function sources(){const rows=[];function walk(dir,all=false){for(const entry of fs.readdirSync(dir,{withFileTypes:true})){const name=path.join(dir,entry.name);if(entry.isDirectory()){if(!['verification','node_modules'].includes(entry.name))walk(name,all);}else if(all||name.endsWith('.rs')||entry.name==='Cargo.toml')rows.push([path.relative(root,name),sha(fs.readFileSync(name))]);}}walk(path.join(root,'crates'));walk(path.join(root,'web'),true);if(fs.existsSync(path.join(root,'assets')))walk(path.join(root,'assets'),true);for(const name of ['Cargo.toml','Cargo.lock'])rows.push([name,sha(fs.readFileSync(path.join(root,name)))]);return rows.sort(([a],[b])=>a.localeCompare(b));}
function drivers(){return ['scripts/run-enrichment-browser-checks.mjs','web/verification/ui-enrichment-success-live.cjs','web/verification/artifact-path.cjs'].map(name=>[name,sha(fs.readFileSync(path.join(root,name)))]);}
const driverManifest=drivers();
async function compile(){
  const before=sources(),log=fs.createWriteStream(path.join(output,'cargo-no-run.log'));let text='',errors='';
  const child=spawn('cargo',['test','-p','open-seo-spider','--bin','open-seo-spider','--no-run','--locked','--offline','--message-format=json'],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
  child.stdout.on('data',b=>{text+=b;log.write(b);});child.stderr.on('data',b=>{errors+=b;log.write(b);});
  const code=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',resolve);});log.end();assert.equal(code,0,errors);
  const artifacts=text.split('\n').filter(Boolean).map(line=>{try{return JSON.parse(line);}catch{return null;}}).filter(row=>row?.reason==='compiler-artifact'&&row.profile.test&&row.target.name==='open-seo-spider'&&row.executable);
  assert.equal(artifacts.length,1,'Exactly one app test executable');assert.deepEqual(sources(),before,'Compiled source closure changed');return {binary:artifacts[0].executable,binary_sha256:sha(fs.readFileSync(artifacts[0].executable)),source_manifest:before};
}
const manifestPath=path.join(output,'fixture.json'),handoff=path.join(output,'handoff.json');
let websiteHits=0,revision=1,initialReleased=false,heldInitial=null,heldCancel=null,cancelAborts=0;
const attempts=[],attemptCounts=new Map(),violations=[];
const website=http.createServer((req,res)=>{websiteHits++;res.end('OWNED WEBSITE TRAP: this URL must never be fetched by the app.');});
const nonce=randomBytes(16).toString('hex');let site;
const provider=http.createServer((req,res)=>{
  const url=new URL(req.url,'http://127.0.0.1');
  if(url.pathname===`/${nonce}/control/state`){res.setHeader('content-type','application/json');return res.end(JSON.stringify({websiteHits,revision,attempts,cancelAborts,heldInitial:!!heldInitial,heldCancel:!!heldCancel,violations}));}
  if(url.pathname===`/${nonce}/control/release`){initialReleased=true;if(heldInitial){const answer=heldInitial;heldInitial=null;answer();}return res.end('released');}
  if(url.pathname===`/${nonce}/control/revision2`){revision=2;return res.end('revision2');}
  if(![`/${nonce}/provider`,`/${nonce}/deny`].includes(url.pathname)){res.statusCode=404;return res.end();}
  const submitted=url.searchParams.get('url'),strategy=url.searchParams.get('strategy'),category=url.searchParams.get('category');
  if(req.method!=='GET'||[...url.searchParams.keys()].sort().join(',')!=='category,strategy,url'||strategy!=='mobile'||category!=='performance'||!submitted?.startsWith(site+'/'))violations.push(req.url);
  const index=Number(new URL(submitted).pathname.split('/').pop()),key=revision+':'+submitted;
  const attempt=(attemptCounts.get(key)||0)+1;attemptCounts.set(key,attempt);attempts.push({url:submitted,strategy,category,revision,attempt,path:url.pathname});
  if(url.pathname.endsWith('/deny')){res.statusCode=503;return res.end('Cold reads must not call provider');}
  if(submitted.endsWith('/cancel-held')){heldCancel=true;res.once('close',()=>{heldCancel=null;cancelAborts++;});return;}
  const reply=()=>{
    res.setHeader('content-type','application/json');
    if(index===3){res.statusCode=403;return res.end(JSON.stringify({error:{message:'PRIVATE_PROVIDER_SENTINEL_NEVER_SAVE'}}));}
    if(index===5&&attempt===1){res.statusCode=429;res.setHeader('Retry-After','0');return res.end('{}');}
    if(index===2)return res.end('{}');
    if(index===4)return res.end(JSON.stringify({lighthouseResult:{runtimeError:{code:'ERRORED_DOCUMENT_REQUEST',message:'PRIVATE_PROVIDER_SENTINEL_NEVER_SAVE'}}}));
    const score=index===0&&revision===1?0:Number((index/200+(revision-1)/10).toFixed(3));
    const audits={'first-contentful-paint':{numericValue:500+index},'total-blocking-time':{numericValue:index},'cumulative-layout-shift':{numericValue:index/1000}};
    if(index!==1)audits['largest-contentful-paint']={numericValue:index===0&&revision===1?0:1000+index*7+(revision-1)*200};
    res.end(JSON.stringify({lighthouseResult:{requestedUrl:submitted,finalUrl:submitted+'?provider-final=1',fetchTime:`2026-10-18T00:00:0${revision}.000Z`,lighthouseVersion:'fixture-1',categories:{performance:{score}},audits}}));
  };
  if(index===0&&!initialReleased){heldInitial=reply;return;}reply();
});
async function listen(server){await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));assert(![8787,8790].includes(server.address().port));return `http://127.0.0.1:${server.address().port}`;}
const children=[],nativeProof=[];let compiled;
async function host(phase,endpoint,store){
  const before=sha(fs.readFileSync(compiled.binary));assert.equal(before,compiled.binary_sha256,'Host must use the just-compiled executable');const env={...process.env,OPEN_SEO_TEST_ENRICHMENT_STORE:store,OPEN_SEO_TEST_ENRICHMENT_PROVIDER:endpoint};delete env.OPEN_SEO_PAGESPEED_API_KEY;
  const child=spawn(compiled.binary,['--exact','server::enrichment_browser_fixture::native_enrichment_browser_fixture','--ignored','--nocapture','--test-threads=1'],{cwd:root,windowsHide:true,stdio:['pipe','pipe','pipe'],env});children.push(child);
  let origin='',pending='',log='',spawnError;const closed=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));child.once('error',error=>spawnError=error);
  for(const stream of [child.stdout,child.stderr])stream.on('data',b=>log+=b);
  child.stdout.on('data',b=>{pending+=b;let end;while((end=pending.indexOf('\n'))>=0){const line=pending.slice(0,end).replace(/\r$/,'');pending=pending.slice(end+1);const match=/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)$/.exec(line);if(match){assert.equal(origin,'');origin=match[1];}}});
  const stop=async()=>{child.stdin.end();const timer=setTimeout(()=>child.kill(),20000);const result=await closed;clearTimeout(timer);fs.writeFileSync(path.join(output,`native-${phase}.log`),log);assert.equal(result.code,0,log);assert.equal(sha(fs.readFileSync(compiled.binary)),before);nativeProof.push({phase,origin,sha256:before,shutdown:result});};
  try{await until(()=>{if(spawnError)throw spawnError;assert.equal(child.exitCode,null,log);return origin;},'native host bind');assert(!['8787','8790'].includes(new URL(origin).port));assert.equal((await fetch(origin+'/healthz')).status,200);return {origin,stop};}catch(error){await stop();throw error;}
}
async function browser(phase,origin,control,store){
  const child=spawn(process.execPath,['web/verification/ui-enrichment-success-live.cjs'],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,SPIDER_APP_URL:origin,SPIDER_ENRICHMENT_PHASE:phase,SPIDER_ENRICHMENT_FIXTURE:manifestPath,SPIDER_ENRICHMENT_CONTROL:control,SPIDER_ENRICHMENT_HANDOFF:handoff,SPIDER_ENRICHMENT_STORE:store,SPIDER_TEST_ARTIFACT_DIR:output}});let log='';for(const stream of [child.stdout,child.stderr])stream.on('data',b=>{log+=b;process.stdout.write(b);});
  const code=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',resolve);});fs.writeFileSync(path.join(output,`browser-${phase}.log`),log);assert.equal(code,0,log);
}
try{
  compiled=await compile();console.log('Compiled cfg(test) native host '+sha(fs.readFileSync(compiled.binary)));
  site=await listen(website);const providerOrigin=await listen(provider),endpoint=`${providerOrigin}/${nonce}`;
  const urls=Array.from({length:105},(_,i)=>site+'/metric/'+String(i).padStart(3,'0'));
  fs.writeFileSync(manifestPath,JSON.stringify({site,urls,input:[...urls,urls[0],urls[0]+'#section','not-a-url'].join('\n'),cancelInput:site+'/cancel-held\n'+site+'/cancel-tail'}));
  const store=path.join(output,'native-store');const first=await host('initial',endpoint+'/provider',store);
  try{await browser('initial',first.origin,endpoint+'/control',store);}finally{await first.stop();}
  const before=attempts.length;const second=await host('cold',endpoint+'/deny',store);
  try{await browser('cold',second.origin,endpoint+'/control',store);}finally{await second.stop();}
  assert.equal(attempts.length,before,'Cold reopening must not call the provider');assert.equal(websiteHits,0);assert.deepEqual(violations,[]);assert.deepEqual(sources(),compiled.source_manifest);assert.deepEqual(drivers(),driverManifest);assert.equal(nativeProof[0].sha256,nativeProof[1].sha256);
  fs.writeFileSync(path.join(output,'enrichment-success-coordinator.json'),JSON.stringify({result:'PASS',scope:'Actual browser -> full native cfg(test) router/service -> owned HTTP provider -> durable report -> CSV -> process-cold reopen. Real Google, CLI and other providers remain unproven.',binary:compiled.binary,binary_sha256:nativeProof[0].sha256,source_manifest:compiled.source_manifest,driver_manifest:driverManifest,native:nativeProof,provider_attempts:attempts,website_requests:websiteHits,cancel_aborts:cancelAborts,violations},null,2)+'\n');
  console.log('PASS positive native enrichment and process-cold reopen: '+output);
}finally{
  for(const child of children)if(child.exitCode===null){child.stdin.end();child.kill();}
  for(const server of [provider,website]){server.close();server.closeAllConnections();}
}
