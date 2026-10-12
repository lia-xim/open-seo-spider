// Owned localhost HTTP -> CLI/API -> SQLite -> bounded cursor and full snapshot exports.
import http from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,writeFile,rm,mkdir} from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||path.join(root,'target','debug',`open-seo-spider${process.platform==='win32'?'.exe':''}`);
const data=await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-policy-'));
const api='http://127.0.0.1:18793';
const hits=new Map(),auth=[];
const fixture=http.createServer((request,response)=>{
  hits.set(request.url,(hits.get(request.url)||0)+1);auth.push({url:request.url,authorization:request.headers.authorization??null});
  if(request.url==='/robots.txt')return response.end('User-agent: *\nDisallow: /scope/old\n');
  if(request.url==='/scope/redirect'){response.writeHead(302,{location:'/scope/denied'});return response.end();}
  response.setHeader('content-type','text/html; charset=utf-8');
  if(request.url==='/scope/start')return response.end(`<html lang="de"><title>Policy fixture</title><body>${Array.from({length:501},(_,index)=>`<a href="/scope/old?utm=${index}&id=1#raw">Alias ${index}</a>`).join('')}<a href="/outside">Check</a><a href="/scope/redirect">Redirect</a></body></html>`);
  response.end(`<html><title>Captured</title><body>${request.url==='/outside'?'<a href="/scope/never-expanded">Never</a>':'Capture'}</body></html>`);
});
await new Promise(resolve=>fixture.listen(19093,'127.0.0.1',resolve));
const origin='http://127.0.0.1:19093';
const config={seed_urls:[origin+'/scope/start'],concurrency:2,delay_ms:0,discover_sitemaps:false,
  custom_headers:{Authorization:'owned fixture credential'},
  crawl_policy:{scope:{roots:[{origin,folders:['/scope/']}],outside_folder:'check_once'},query:{remove_keys:['utm']},
    rewrites:[{id:'own-alias',pattern:'/scope/old',replacement:{kind:'literal',text:'/scope/new'}}],exclude_patterns:['/scope/denied$']}};
let server;
async function execute(args){return new Promise((resolve,reject)=>{
  const child=spawn(binary,args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';
  child.stdout.on('data',chunk=>stdout+=chunk);child.stderr.on('data',chunk=>stderr+=chunk);child.on('error',reject);
  child.on('exit',code=>resolve({code,stdout,stderr}));
});}
async function request(route,body){const response=await fetch(api+route,body===undefined?{}:{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});const value=await response.json();assert.equal(response.ok,true,JSON.stringify(value));return value;}
async function until(fn){for(let i=0;i<400;i++){const value=await fn();if(value)return value;await new Promise(resolve=>setTimeout(resolve,25));}throw Error('Native policy acceptance timeout');}
const checks=[];
try{
  const configPath=path.join(data,'config.json'),database=path.join(data,'cli.sqlite'),reportPath=path.join(data,'cli.json');
  await writeFile(configPath,JSON.stringify(config));
  const crawl=await execute(['crawl','--config',configPath,'--store',database,'--output',reportPath]);assert.equal(crawl.code,0,crawl.stderr);
  const report=JSON.parse(await readFile(reportPath,'utf8'));
  // Every committed observation can be exported even when a denied final body is unknown.
  assert.equal(report.policy_ledger.complete,false);assert.equal(report.policy_ledger.incomplete_sources,1);assert.ok(report.policy_ledger.occurrences>500);
  const denied=report.pages.find(page=>page.url===origin+'/scope/redirect');assert.equal(denied.status,302);assert.equal(denied.final_status,null);assert.equal(denied.policy.candidates_complete,false);assert.equal(denied.policy.redirect_decisions[0].admission_reason,'redirect_denied');
  assert.ok(report.pages.some(page=>page.url===origin+'/scope/new?id=1'));assert.equal(report.pages.find(page=>page.url===origin+'/scope/start').links[0].url,origin+'/scope/old?utm=0&id=1');
  const csv=path.join(data,'policy.csv'),json=path.join(data,'policy.json');
  const exported=await execute(['policy-ledger',database,'--csv',csv,'--json',json]);assert.equal(exported.code,0,exported.stderr);
  const full=JSON.parse(await readFile(json,'utf8'));assert.equal(full.observations.length,report.policy_ledger.occurrences);assert.equal(full.export.all_snapshot_rows_exported,true);assert.equal(full.export.ledger_complete,false);assert.equal(full.export.row_count,full.observations.length);
  assert.equal((await readFile(csv,'utf8')).trimEnd().split('\n').length,full.observations.length+1);
  const slice=await execute(['policy-ledger',database,'--after-sequence','200','--limit','200']);assert.equal(slice.code,0,slice.stderr);const page=JSON.parse(slice.stdout);assert.equal(page.rows.length,200);assert.equal(page.rows[0].sequence,201);assert.equal(page.has_more,true);
  const refused=await execute(['policy-ledger',database,'--json',reportPath]);assert.notEqual(refused.code,0);assert.deepEqual(JSON.parse(await readFile(reportPath,'utf8')),report);
  checks.push('CLI native durable policy, >500 occurrence ledger, original links preserved, transformed tasks coalesced, bounded cursor, complete snapshot JSON/CSV, existing destination preserved');
  server=spawn(binary,['serve','--listen','127.0.0.1:18793','--data-dir',path.join(data,'api')],{cwd:root,windowsHide:true,stdio:'ignore'});
  await until(async()=>{try{return(await fetch(api+'/healthz')).ok;}catch{return false;}});
  const created=await request('/api/crawls',config);await until(async()=>{const status=await request('/api/crawls/'+created.id);if(status.status==='failed')throw Error(status.error);return status.status==='completed';});
  const rows=[];let cursor;
  do{const page=await request(`/api/crawls/${created.id}/policy?limit=200${cursor===undefined?'':`&after_sequence=${cursor}`}`);assert.equal(page.enabled,true);assert.equal(page.ledger_available,true);assert.ok(page.observations.length<=200);rows.push(...page.observations);cursor=page.has_more?page.next_after_sequence:undefined;if(!page.has_more)break;}while(true);
  assert.equal(new Set(rows.map(row=>row.sequence)).size,rows.length);assert.ok(rows.length>500);assert.equal(rows.filter(row=>row.occurrence.admission_reason==='coalesced').length,500);
  for(const extension of ['json','csv']){const response=await fetch(`${api}/api/crawls/${created.id}/policy.${extension}`);assert.equal(response.ok,true);assert.equal(Number(response.headers.get('x-policy-snapshot-occurrences')),rows.length);const text=await response.text();if(extension==='json'){const snapshot=JSON.parse(text);assert.equal(snapshot.export.row_count,rows.length);assert.deepEqual(snapshot.observations,rows);}else assert.equal(text.trimEnd().split('\n').length,rows.length+1);}
  assert.equal((await fetch(`${api}/api/crawls/${created.id}/policy?limit=1001`)).status,400);
  const bad={...config,renderer:{endpoint:'http://127.0.0.1:8790/render'}};const before=Array.from(hits.values()).reduce((a,b)=>a+b,0);assert.equal((await fetch(api+'/api/crawls',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(bad)})).status,400);assert.equal(Array.from(hits.values()).reduce((a,b)=>a+b,0),before);
  assert.equal(hits.get('/scope/old'),undefined);assert.equal(hits.get('/scope/denied'),undefined);assert.equal(hits.get('/scope/never-expanded'),undefined);assert.equal(hits.get('/scope/new?id=1'),2);
  assert.ok(auth.filter(item=>item.url==='/robots.txt').every(item=>item.authorization===null));
  checks.push('API native policy, pages of200 without gaps, 500 coalesced aliases, streamed complete JSON/CSV and snapshot headers, invalid pagination and renderer rejected before HTTP, actual robots anonymous, denied redirect/check-only not expanded');
  const evidence={tested_at:new Date().toISOString(),platform:process.platform,result:'PASS',checks,observations:rows.length,all_snapshot_rows_exported:true,ledger_complete:false,incomplete_sources:1,unknown_final_body:'policy-denied redirect is not inferred captured',website_targets:'owned localhost fixtures only'};console.log(JSON.stringify(evidence,null,2));
  if(process.env.SPIDER_EVIDENCE){await mkdir(path.dirname(process.env.SPIDER_EVIDENCE),{recursive:true});await writeFile(process.env.SPIDER_EVIDENCE,JSON.stringify(evidence,null,2)+'\n');}
}finally{
  if(server?.exitCode===null){const stopped=new Promise(resolve=>server.once('exit',resolve));server.kill();await stopped;}
  await new Promise(resolve=>fixture.close(resolve));assert.equal(path.dirname(path.resolve(data)),path.resolve(os.tmpdir()));assert.ok(path.basename(data).startsWith('open-seo-spider-policy-'));await rm(data,{recursive:true,force:true});
}
