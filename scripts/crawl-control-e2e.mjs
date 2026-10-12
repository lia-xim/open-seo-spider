// Owned barriers prove API admission/drain, SQLite-only recovery and immutable history.
import assert from 'node:assert/strict';
import http from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,writeFile,rm,readdir,mkdir,rename} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {DatabaseSync} from 'node:sqlite';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||join(root,'target','debug',process.platform==='win32'?'open-seo-spider.exe':'open-seo-spider');
const data=await mkdtemp(join(tmpdir(),'open-seo-spider-control-'));
const api='http://127.0.0.1:18784';
const requests=new Map(),held=new Map(),released=new Set(),checks=[];
const delay=ms=>new Promise(done=>setTimeout(done,ms));
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
let child,log='';
const html=(name,links=[])=>`<title>${name}</title><h1>${name}</h1>${links.map(url=>`<a href="${url}">${url}</a>`).join('')}`;
const fixture=http.createServer((req,res)=>{
 requests.set(req.url,(requests.get(req.url)||0)+1);
 const send=()=>{
  if(req.url==='/robots.txt')return res.end('User-agent: *\nAllow: /\n');
  if(req.url==='/pause-index.xml'){res.setHeader('content-type','application/xml');return res.end(`<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>${origin}/discovery-a.xml</loc></sitemap><sitemap><loc>${origin}/discovery-b.xml</loc></sitemap></sitemapindex>`);}
  if(req.url.startsWith('/discovery-')&&req.url.endsWith('.xml')){res.setHeader('content-type','application/xml');return res.end(`<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>${origin}${req.url.slice(0,-4)}</loc></url></urlset>`);}
  res.setHeader('content-type','text/html');
  if(req.url==='/pause-home')return res.end(html('Pause home',['/hold/first','/hold/second','/later/one','/later/two']));
  if(req.url==='/crash-home')return res.end(html('Crash home',['/crash-held','/crash-later']));
  if(req.url==='/limit-home')return res.end(html('Limit home',['/limit-child-a','/limit-child-b']));
  res.end(html(req.url,req.url==='/hold/first'?['/discovered-in-drain']:[]));
 };
 if(['/hold/first','/hold/second','/crash-held','/pause-index.xml'].includes(req.url)&&!released.has(req.url))held.set(req.url,{send,res});else send();
});
await new Promise(done=>fixture.listen(0,'127.0.0.1',done));
const origin=`http://127.0.0.1:${fixture.address().port}`;
const release=url=>{released.add(url);const entry=held.get(url);if(entry){held.delete(url);entry.send();}};
async function call(route,body){const response=await fetch(api+route,body===undefined?{}:{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});const text=await response.text();assert.equal(response.status,200,`${route}: ${text.slice(0,1000)}`);return JSON.parse(text);}
async function until(fn,label){for(let i=0;i<900;i++){const result=await fn();if(result)return result;await delay(40);}throw Error(`Timeout ${label}; ${log.slice(-2000)}`);}
async function launch(){log='';child=spawn(binary,['serve','--listen','127.0.0.1:18784','--data-dir',data],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});child.stdout.on('data',x=>{log+=x});child.stderr.on('data',x=>{log+=x});child.once('error',x=>{log+=x.message});await until(async()=>{try{return(await call('/healthz')).ok}catch{return false}},'ready');}
async function stop(){if(!child||child.exitCode!==null)return;const ended=new Promise(done=>child.once('exit',done));child.kill();await ended;child=null;}
const status=id=>call(`/api/crawls/${id}`);
const terminal=(id,state)=>until(async()=>{const current=await status(id);if(current.status==='failed')throw Error(current.error);return current.status===state&&current},state);
const config=url=>({seed_urls:[origin+url],concurrency:2,delay_ms:0,discover_sitemaps:false});
const conflict=async route=>assert.equal((await fetch(api+route,{method:'POST',headers:{'content-type':'application/json'},body:'{}'})).status,409);
try{
 await launch();
 const started=await call('/api/crawls',config('/pause-home'));
 await until(()=>held.has('/hold/first')&&held.has('/hold/second'),'two admitted documents');
 assert.equal((await call(`/api/crawls/${started.id}/pause`,{})).status,'pausing');
 assert.equal((await call(`/api/crawls/${started.id}/pause`,{})).status,'pausing');
 await conflict(`/api/crawls/${started.id}/resume`);
 await delay(100);assert.equal((await status(started.id)).status,'pausing');assert.equal(requests.get('/later/one'),undefined);assert.equal(requests.get('/later/two'),undefined);
 release('/hold/first');release('/hold/second');
 const paused=await terminal(started.id,'paused');assert.equal(paused.stats.fetched,3);assert.equal(paused.stats.in_flight,0);assert.equal(paused.stats.queued,3);assert.equal(paused.checkpoint.outcome,'paused');assert.equal(paused.checkpoint.generation,1);assert.equal(paused.checkpoint.discovery_complete,true);
 const pausedReport=await call(`/api/crawls/${started.id}/project.json`);assert.equal(pausedReport.summary.cancelled,false);assert.equal(pausedReport.pages.length,3);
 checks.push('Pause is idempotent, refuses concurrent resume, stops new admissions, drains two held requests and durably retains three discovered pending pages');
 const reportPath=join(data,`${started.id}.json`),manifestPath=join(data,`${started.id}.job.json`);
 const originalRevision=await readFile(join(data,'snapshots',started.id,'generation-1.json'));
 const decorated=Buffer.from(JSON.stringify({...JSON.parse(await readFile(reportPath,'utf8')),future_extension:{owned:'preserve exact bytes'}},null,3)+'\n');await writeFile(reportPath,decorated);
 const beforeRestart=new Map(requests);await stop();await launch();assert.equal((await status(started.id)).status,'paused');assert.ok(decorated.equals(await readFile(reportPath)));assert.deepEqual(requests,beforeRestart);
 checks.push('Paused startup verifies SQLite checkpoint while preserving existing JSON whitespace/future fields exactly and performing no HTTP');
 const pausedDatabase=join(data,`${started.id}.sqlite`);await stop();await rename(pausedDatabase,pausedDatabase+'.owned-backup');await launch();const unavailable=await status(started.id);assert.equal(unavailable.status,'failed');assert.equal(unavailable.resume_available,false);assert.match(unavailable.error,/no valid SQLite frontier/);assert.equal((await call(`/api/crawls/${started.id}/project.json`)).pages.length,3);assert.ok(decorated.equals(await readFile(reportPath)));await conflict(`/api/crawls/${started.id}/resume`);assert.deepEqual(requests,beforeRestart);await stop();await rename(pausedDatabase+'.owned-backup',pausedDatabase);await launch();assert.equal((await status(started.id)).status,'paused');
 checks.push('A paused report/manifest with missing SQLite never becomes completed or resumable; its original snapshot remains available and byte-identical');
 await stop();await rm(reportPath);await rm(manifestPath);await launch();assert.equal((await status(started.id)).status,'paused');assert.equal((await call(`/api/crawls/${started.id}/project.json`)).pages.length,3);assert.deepEqual(requests,beforeRestart);
 checks.push('A paused SQLite-only project with no report or manifest recovers its checkpoint and observations without requests');
 await writeFile(reportPath,decorated);await call(`/api/crawls/${started.id}/resume`,{});
 const completed=await terminal(started.id,'completed');assert.equal(completed.generation,2);assert.equal(completed.checkpoint.revision,2);assert.equal(completed.stats.fetched,6);for(const url of ['/pause-home','/hold/first','/hold/second','/later/one','/later/two','/discovered-in-drain'])assert.equal(requests.get(url),1,url);
 const completedReport=await call(`/api/crawls/${started.id}/project.json`);assert.equal(completedReport.pages.length,6);assert.deepEqual(completedReport.pages.map(page=>page.url).sort(),['/pause-home','/hold/first','/hold/second','/later/one','/later/two','/discovered-in-drain'].map(url=>origin+url).sort());for(const page of pausedReport.pages)assert.deepEqual(completedReport.pages.find(current=>current.url===page.url),page);
 const snapshots=join(data,'snapshots',started.id);assert.ok(originalRevision.equals(await readFile(join(snapshots,'generation-1.json'))));const archives=(await readdir(snapshots)).filter(name=>name.startsWith('before-resume-'));assert.equal(archives.length,1);assert.ok(decorated.equals(await readFile(join(snapshots,archives[0]))));
 checks.push('Resume increments generation/revision, finishes six unique pages without refetching committed rows, and archives exact prior bytes with immutable earlier revision');
 await conflict(`/api/crawls/${started.id}/pause`);
 const database=join(data,`${started.id}.sqlite`);await rename(database,database+'.owned-backup');await conflict(`/api/crawls/${started.id}/resume`);await rename(database+'.owned-backup',database);
 checks.push('Completed pause and manifest-only resume are explicit conflicts, never invented saved-frontier work');
 const crash=await call('/api/crawls',{...config('/crash-home'),concurrency:1});await until(()=>held.has('/crash-held'),'crash admitted document');await stop();release('/crash-held');await rm(join(data,`${crash.id}.job.json`));await launch();const interrupted=await status(crash.id);assert.equal(interrupted.status,'interrupted');assert.equal(interrupted.stats.fetched,1);assert.equal(interrupted.stats.in_flight,0);assert.equal(requests.get('/crash-home'),1);
 await call(`/api/crawls/${crash.id}/resume`,{});await terminal(crash.id,'completed');assert.equal(requests.get('/crash-home'),1);assert.equal(requests.get('/crash-held'),2);assert.equal(requests.get('/crash-later'),1);
 checks.push('An interrupted SQLite-only project retains committed seed and retries uncommitted admitted URL, explicitly providing at-least-once request recovery');
 const sitemap=await call('/api/crawls',{seed_urls:[],sitemap_urls:[origin+'/pause-index.xml'],discover_sitemaps:false,concurrency:2,delay_ms:0});await until(()=>held.has('/pause-index.xml'),'sitemap admitted unit');await call(`/api/crawls/${sitemap.id}/pause`,{});release('/pause-index.xml');const discovery=await terminal(sitemap.id,'paused');assert.equal(discovery.stats.fetched,0);assert.equal(discovery.checkpoint.discovery_complete,false);assert.equal(discovery.checkpoint.discovery_pending,2);assert.equal(discovery.stats.discovery_pending,2);assert.equal(requests.get('/discovery-a.xml'),undefined);assert.equal(requests.get('/discovery-b.xml'),undefined);
 await stop();const beforeDiscoveryRestart=new Map(requests);await launch();assert.equal((await status(sitemap.id)).status,'paused');assert.deepEqual(requests,beforeDiscoveryRestart);await call(`/api/crawls/${sitemap.id}/resume`,{});const sitemapCompleted=await terminal(sitemap.id,'completed');assert.equal(sitemapCompleted.stats.fetched,2);assert.equal(sitemapCompleted.checkpoint.discovery_complete,true);for(const url of ['/pause-index.xml','/discovery-a.xml','/discovery-b.xml','/discovery-a','/discovery-b'])assert.equal(requests.get(url),1,url);
 checks.push('Pause drains the admitted sitemap unit, preserves two child discovery units, survives restart and resumes discovery without falsely marking completion or refetching committed index');
 const limitedJob=await call('/api/crawls',{...config('/limit-home'),max_pages:1});const limited=await terminal(limitedJob.id,'limited');assert.equal(limited.stats.fetched,1);assert.equal(limited.stats.queued,2);assert.equal(limited.resume_available,true);const limitedBytes=await readFile(join(data,`${limitedJob.id}.json`));const beforeInvalidResume=new Map(requests);assert.equal((await fetch(api+`/api/crawls/${limitedJob.id}/resume`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({concurrency:0})})).status,400);assert.equal((await status(limitedJob.id)).status,'limited');assert.ok(limitedBytes.equals(await readFile(join(data,`${limitedJob.id}.json`))));assert.deepEqual(requests,beforeInvalidResume);await call(`/api/crawls/${limitedJob.id}/resume`,{max_pages:0,concurrency:1});const limitCompleted=await terminal(limitedJob.id,'completed');assert.equal(limitCompleted.config.max_pages,null);assert.equal(limitCompleted.config.concurrency,1);assert.equal(limitCompleted.stats.fetched,3);for(const url of ['/limit-home','/limit-child-a','/limit-child-b'])assert.equal(requests.get(url),1,url);
 checks.push('A real operator-limited checkpoint can explicitly remove the limit on resume; invalid operational settings preserve status/bytes and cause zero requests');
 const legacyJob=await call('/api/crawls',{...config('/limit-home'),max_pages:1});await terminal(legacyJob.id,'limited');const legacyPath=join(data,`${legacyJob.id}.json`),legacyBytes=await readFile(legacyPath),legacyHits=new Map(requests);await stop();const legacyDb=new DatabaseSync(join(data,`${legacyJob.id}.sqlite`));try{legacyDb.exec("DELETE FROM metadata WHERE key IN ('run_state','checkpoint')");}finally{legacyDb.close()}await launch();const legacyState=await status(legacyJob.id);assert.equal(legacyState.status,'limited');assert.equal(legacyState.stats.queued,2);assert.equal(legacyState.resume_available,true);assert.ok(legacyBytes.equals(await readFile(legacyPath)));assert.deepEqual(requests,legacyHits);await stop();await rm(join(data,`${legacyJob.id}.job.json`));await launch();assert.equal((await status(legacyJob.id)).status,'limited');assert.ok(legacyBytes.equals(await readFile(legacyPath)));assert.deepEqual(requests,legacyHits);
 checks.push('Legacy limited report/frontier retains limited status and pending count with or without manifest, preserving bytes and making zero startup requests');
 const policyRequest={config:{scope:{roots:[{origin,folders:['/allowed/']}]},query:{remove_keys:['tracking'],key_match:'percent_decoded_utf8',order:'preserve'}},inputs:[{occurrence_id:'a',source_kind:'anchor',raw_value:origin+'/allowed/page?tracking=1&b=2&a=1#part'},{occurrence_id:'outside',source_kind:'anchor',raw_value:origin+'/allowed-extra/page'}]};
 const beforePolicy=new Map(requests),preview=await call('/api/policy/preview',policyRequest);assert.equal(preview.website_requests,0);assert.equal(preview.decisions[0].decision,'fetch_and_traverse');assert.equal(preview.decisions[0].fetch_candidate_url,origin+'/allowed/page?b=2&a=1');assert.equal(preview.decisions[1].decision,'deny');assert.deepEqual(requests,beforePolicy);
 const policyPath=join(data,'policy.json'),policyOutput=join(data,'preview.json');await writeFile(policyPath,JSON.stringify(policyRequest));const cli=spawn(binary,['preview-policy',policyPath,'--output',policyOutput],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let cliLog='';cli.stderr.on('data',x=>{cliLog+=x});const cliExit=await new Promise((done,reject)=>{cli.once('exit',done);cli.once('error',reject)});assert.equal(cliExit,0,cliLog);assert.deepEqual(JSON.parse(await readFile(policyOutput,'utf8')),preview);assert.deepEqual(requests,beforePolicy);const capabilities=await call('/api/capabilities');assert.equal(capabilities.offline_policy_preview.applied_to_crawls,true);assert.equal(capabilities.native_crawl_policy.opt_in,true);assert.equal(capabilities.native_crawl_policy.renderer_supported,false);
 checks.push('Explicit folder/query policy preview has equal API/CLI decisions with zero HTTP; native execution is separately opt-in and renderer combination is explicitly unsupported');
 const proof={tested_at:new Date().toISOString(),result:'PASS',platform:process.platform,checks,paused_revision_sha256:sha(originalRevision),archived_original_sha256:sha(decorated),limits:'Owned local fixtures; exactly-once remote requests, power-loss durability and full vendor control/workflow parity remain NOT_PROVEN. Policy preview makes zero requests; native live-policy behavior has separate acceptance gates.'};
 if(globalThis.process.env.SPIDER_EVIDENCE){await mkdir(dirname(globalThis.process.env.SPIDER_EVIDENCE),{recursive:true});await writeFile(globalThis.process.env.SPIDER_EVIDENCE,JSON.stringify(proof,null,2)+'\n');}
 console.log(JSON.stringify(proof,null,2));
}finally{
 await stop();for(const [url] of held)release(url);fixture.closeAllConnections();await new Promise(done=>fixture.close(done));assert.equal(dirname(data),tmpdir());assert.ok(data.startsWith(join(tmpdir(),'open-seo-spider-control-')));await rm(data,{recursive:true,force:true});
}
