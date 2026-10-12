// Work-only derivation. Original source SHA256: f18675e22a217a6c4fabf48507991945c4150ef48cc4ef283fbd982d5ad05efb
// SDK assertions below are byte-identical after reversing documented fixture URL and liveness insertions.
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import http from 'node:http';
import { mkdtemp, rm, writeFile, mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import { fileURLToPath, pathToFileURL } from 'node:url';
const root=path.resolve(process.env.SPIDER_REPO_ROOT || process.cwd());
const sourcePath=path.join(root,'scripts/mcp-e2e.mjs');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
assert.equal(hash(await readFile(sourcePath)), 'f18675e22a217a6c4fabf48507991945c4150ef48cc4ef283fbd982d5ad05efb', 'Reviewed original MCP harness identity');
const requireFromScripts=createRequire(path.join(root,'scripts/package.json'));
let sdkRoot=path.dirname(requireFromScripts.resolve('@modelcontextprotocol/client'));
let sdkPackage;
while(true){
  try{const candidate=JSON.parse(await readFile(path.join(sdkRoot,'package.json'),'utf8'));if(candidate.name==='@modelcontextprotocol/client'){sdkPackage=candidate;break;}}catch(error){if(error.code!=='ENOENT')throw error;}
  const parent=path.dirname(sdkRoot);assert.notEqual(parent,sdkRoot,'Installed SDK package manifest required');sdkRoot=parent;
}
function sdkEsmEntry(key){
  const entry=sdkPackage.exports[key]?.import?.default;
  assert.equal(typeof entry,'string','Installed SDK conditional ESM export required: '+key);
  assert.ok(entry.startsWith('./'),'SDK ESM export must remain package-relative');
  const target=path.resolve(sdkRoot,entry);const relative=path.relative(sdkRoot,target);
  assert.ok(relative && !relative.startsWith('..') && !path.isAbsolute(relative),'SDK ESM entry stays in installed package');
  return pathToFileURL(target).href;
}
assert.equal(sdkPackage.version,'2.3.1','Original reviewed SDK version');
const { Client, StreamableHTTPClientTransport }=await import(sdkEsmEntry('.'));
const { StdioClientTransport }=await import(sdkEsmEntry('./stdio'));
const binary=path.resolve(process.env.SPIDER_BINARY || path.join(root,'target','debug',`open-seo-spider${process.platform==='win32'?'.exe':''}`));
const binarySha=hash(await readFile(binary));
if(process.env.SPIDER_RELEASE_SHA){assert.match(process.env.SPIDER_RELEASE_SHA,/^[a-f0-9]{64}$/);assert.equal(binarySha,process.env.SPIDER_RELEASE_SHA,'Expected optimized release binary identity before spawn');}
const harnessSha=hash(await readFile(fileURLToPath(import.meta.url)));
const data=await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-mcp-'));
let revision=0,websiteRequests=0;
const fixture=http.createServer((request,response)=>{websiteRequests++;if(request.url==='/robots.txt'){response.end('User-agent: *\nAllow: /');return;}response.setHeader('content-type','text/html');response.end(`<html lang="de"><title>MCP ${request.url} revision ${revision}</title><body><h1>Fixture</h1></body></html>`);});
let server,client,api,fixtureOrigin,evidence;
let serverError,serverExit,serverClosed=false,serverOut='',serverErr='',livenessChecks=0;
let resolveEnded,resolveClosed;
const serverEnded=new Promise(resolve=>resolveEnded=resolve);
const serverClose=new Promise(resolve=>resolveClosed=resolve);
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function assertOwnedServerAlive(where){
  livenessChecks++;
  if(!server || serverError || serverExit || server.exitCode!==null || server.signalCode!==null || server.killed || !server.pid){
    throw new Error('Owned MCP app unavailable at '+where+': '+JSON.stringify({pid:server?.pid,error:serverError?.message,exit:serverExit,exitCode:server?.exitCode,signalCode:server?.signalCode,stderr:serverErr.slice(-8192)}));
  }
}
async function withOwnedServer(action,where){
  assertOwnedServerAlive('before '+where);
  try{
    const value=await Promise.race([Promise.resolve().then(action),serverEnded.then(()=>{assertOwnedServerAlive('during '+where);throw Error('Owned MCP app ended');})]);
    assertOwnedServerAlive('after '+where);return value;
  }catch(error){assertOwnedServerAlive('failed '+where);throw error;}
}
function guardOwnedClient(value){
  for(const method of ['connect','listTools','callTool']){
    const originalMethod=value[method].bind(value);
    value[method]=(...args)=>withOwnedServer(()=>originalMethod(...args),'SDK '+method);
  }
}
async function until(fn){for(let step=0;step<400;step++){assertOwnedServerAlive('acceptance wait');const result=await fn();if(result)return result;await delay(25);}throw new Error('MCP acceptance timeout');}
async function call(name,args={}){const result=await client.callTool({name,arguments:args});assert.equal(result.isError,false,JSON.stringify(result));return result.structuredContent;}
try{
  await new Promise((resolve,reject)=>{fixture.once('error',reject);fixture.listen(0,'127.0.0.1',resolve);});
  const fixtureAddress=fixture.address();assert.equal(fixtureAddress.address,'127.0.0.1');assert.ok(fixtureAddress.port>0);
  fixtureOrigin='http://127.0.0.1:'+fixtureAddress.port;
  server=spawn(binary,['serve','--listen','127.0.0.1:0','--data-dir',data],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
  server.once('error',error=>{serverError=error;resolveEnded({error:error.message});});
  server.once('exit',(code,signal)=>{serverExit={code,signal};resolveEnded(serverExit);});
  server.once('close',(code,signal)=>{serverClosed=true;resolveClosed({code,signal});});
  server.stdout.on('data',bytes=>{serverOut+=bytes;});server.stderr.on('data',bytes=>{serverErr+=bytes;});
  api=await until(()=>{
    const matches=[...serverOut.matchAll(/^Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)\r?\n/gm)];
    if(!matches.length)return false;
    assert.equal(matches.length,1,'Exactly one owned post-bind address');
    const value=new URL(matches[0][1]);assert.equal(value.hostname,'127.0.0.1');assert.equal(value.protocol,'http:');assert.ok(Number(value.port)>0 && Number(value.port)<=65535);
    assert.notEqual(value.origin,fixtureOrigin,'App and fixture own separate listeners');return value.origin;
  });
  await until(async()=>{
    try{return(await withOwnedServer(()=>fetch(api+'/healthz',{signal:AbortSignal.timeout(1000)}),'owned health HTTP')).ok;}
    catch(error){assertOwnedServerAlive('health retry');return false;}
  });
  const checks=[];
  for(const kind of ['stdio','streamable_http']){
    client=new Client({name:'open-seo-spider-acceptance',version:'0.1.0'});
    guardOwnedClient(client);
    const transport=kind==='stdio'?new StdioClientTransport({command:binary,args:['mcp','--server',api],cwd:root,stderr:'pipe'}):new StreamableHTTPClientTransport(new URL(api+'/mcp'));
    await client.connect(transport);
    const inventory=await client.listTools();assert.equal(inventory.tools.length,22);
    assert.ok(inventory.tools.some(tool=>tool.name==='spider_pause'&&tool.annotations.readOnlyHint===false));
    assert.ok(inventory.tools.some(tool=>tool.name==='spider_recrawl'&&tool.annotations.readOnlyHint===false&&tool.annotations.openWorldHint===true));
    const caps=await call('spider_capabilities');assert.equal(caps.crawl.url_limit,null);
    assert.ok(inventory.tools.some(tool=>tool.name==='spider_analyze_content'&&tool.annotations.readOnlyHint===true&&tool.annotations.openWorldHint===false));
    const suppliedText='one two three four five six seven eight nine ten eleven twelve.';
    const analysis=await call('spider_analyze_content',{input:{corpus_id:'mcp-owned-text',corpus_scope:'three supplied texts',corpus_complete:false,documents:[
      {id:'a',source:'manual_text',text:suppliedText,language:'en',complete:true},
      {id:'b',source:'manual_text',text:suppliedText,language:'en',complete:true},
      {id:'missing',source:'manual_text',text:null,language:null,complete:false},
    ]}});
    assert.equal(analysis.website_requests,0);assert.equal(analysis.pairs[0].near_duplicate,true);assert.equal(analysis.pairs[1].near_duplicate,null);assert.equal(analysis.corpus.missing_text_documents,1);
    const policy={scope:{roots:[{origin:`${fixtureOrigin}`,folders:['/']}]}};
    const preview=await call('spider_preview_policy',{input:{config:policy,inputs:[{occurrence_id:'own-mcp-seed',source_kind:'seed',raw_value:`${fixtureOrigin}/a`,base_url:null,source_url:null,source_location:null}]}});
    assert.equal(preview.website_requests,0);assert.equal(preview.decisions[0].fetch_candidate_url,`${fixtureOrigin}/a`);
    revision=0;
    const started=await call('spider_start_crawl',{config:{seed_urls:[`${fixtureOrigin}/a`,`${fixtureOrigin}/b`],crawl_links:false,discover_sitemaps:false,delay_ms:0,content_capture:{}}});
    const finished=await until(async()=>{const status=await call('spider_status',{id:started.id});return status.status==='completed'&&status;});assert.equal(finished.stats.fetched,2);
    const disabledLedger=await call('spider_policy',{id:started.id,limit:1});assert.equal(disabledLedger.enabled,false);assert.equal(disabledLedger.ledger_available,false);assert.deepEqual(disabledLedger.observations,[]);
    const pages=await call('spider_pages',{id:started.id,offset:1,limit:1});assert.equal(pages.pages.length,1);assert.equal(pages.total,2);
    const contentRows=await call('spider_content',{id:started.id,limit:1});assert.equal(contentRows.rows.length,1);assert.equal(contentRows.text_included,false);assert.equal(contentRows.has_more,true);assert.equal(contentRows.rows[0].status,'complete');
    assert.equal((await client.callTool({name:'spider_content',arguments:{id:started.id,source_kind:1}})).isError,true);
    const contentDocument=await call('spider_content_document',{id:started.id,url:contentRows.rows[0].requested_url});assert.equal(contentDocument.capture.result.text,'Fixture');assert.equal(contentDocument.capture.result.html_language,'de');
    const savedAnalysis=await call('spider_analyze_crawl_content',{id:started.id,input:{urls:[`${fixtureOrigin}/a`,`${fixtureOrigin}/b`],expected_generation:finished.generation}});assert.equal(savedAnalysis.website_requests,0);assert.equal(savedAnalysis.site_wide_coverage,false);assert.equal(savedAnalysis.documents.length,2);assert.equal(savedAnalysis.analysis.pairs[0].identical_supplied_text,true);
    assert((await call('spider_issues',{id:started.id})).issues.length>0);
    assert.equal((await call('spider_project',{id:started.id})).pages.length,2);
    assert.equal((await call('spider_analysis',{id:started.id})).graph.nodes.length,2);
    assert.equal((await call('spider_compare',{before:started.id,after:started.id})).changes.length,0);
    const beforeRead=websiteRequests;
    for(const name of ['spider_resources','spider_resource_transports']){
      assert.ok(inventory.tools.some(tool=>tool.name===name&&tool.annotations.readOnlyHint===true&&tool.annotations.openWorldHint===false));
      const rows=await call(name,{id:started.id,limit:1});assert.equal(rows.ledger_available,false);assert.equal(rows.website_requests,0);
    }
    const css=await call('spider_inspect_css',{input:{source:{css_text:'a{background:url("owned.png")}',base_url:`${fixtureOrigin}/css/main`,source_url:`${fixtureOrigin}/css/main`,source_revision:'mcp-supplied-css',inline:false}}});
    assert.equal(css.website_requests,0);assert.equal(css.dependencies[0].resolved_url,`${fixtureOrigin}/css/owned.png`);assert.equal(websiteRequests,beforeRead);
    assert.equal((await client.callTool({name:'spider_resources',arguments:{id:started.id,limit:'1'}})).isError,true);
    assert.equal((await client.callTool({name:'spider_resource_transports',arguments:{id:started.id,after_url:1}})).isError,true);
    await assert.rejects(client.callTool({name:'spider_resources',arguments:{id:started.id,after_url:'https://example.test/'}}),/Unexpected or missing tool argument/);
    await assert.rejects(client.callTool({name:'spider_inspect_css',arguments:{input:{},options:{}}}),/Unexpected or missing tool argument/);
    assert.equal(websiteRequests,beforeRead);
    assert((await call('spider_list_crawls')).crawls.length>0);
    const missing=await client.callTool({name:'spider_status',arguments:{id:'00000000-0000-0000-0000-000000000000'}});assert.equal(missing.isError,true);
    const latePause=await client.callTool({name:'spider_pause',arguments:{id:started.id}});assert.equal(latePause.isError,true);
    const parent=await call('spider_project',{id:started.id});revision=1;
    const recrawl=await call('spider_recrawl',{id:started.id,urls:[`${fixtureOrigin}/a`],max_attempts:0});assert.notEqual(recrawl.id,started.id);assert.equal(recrawl.parent_id,started.id);
    await until(async()=>{const status=await call('spider_status',{id:recrawl.id});if(status.status==='failed')throw Error(status.error);return status.status==='completed';});
    const child=await call('spider_project',{id:recrawl.id});assert.equal(child.pages.length,2);assert.deepEqual(child.pages.find(page=>page.url.endsWith('/b')),parent.pages.find(page=>page.url.endsWith('/b')));assert.match(child.pages.find(page=>page.url.endsWith('/a')).titles[0],/revision 1/);assert.deepEqual(await call('spider_project',{id:started.id}),parent);
    const changes=await call('spider_compare',{before:started.id,after:recrawl.id});assert.equal(changes.changes.length,1);assert.equal(changes.changes[0].url,`${fixtureOrigin}/a`);
    const policyJob=await call('spider_start_crawl',{config:{seed_urls:[`${fixtureOrigin}/a`,`${fixtureOrigin}/b`],crawl_links:false,discover_sitemaps:false,delay_ms:0,crawl_policy:policy}});
    await until(async()=>{const status=await call('spider_status',{id:policyJob.id});if(status.status==='failed')throw Error(status.error);return status.status==='completed';});
    const firstPolicyPage=await call('spider_policy',{id:policyJob.id,limit:1});assert.equal(firstPolicyPage.enabled,true);assert.equal(firstPolicyPage.ledger_available,true);assert.equal(firstPolicyPage.observations.length,1);assert.equal(firstPolicyPage.has_more,true);
    const secondPolicyPage=await call('spider_policy',{id:policyJob.id,after_sequence:firstPolicyPage.next_after_sequence,limit:1});assert.equal(secondPolicyPage.observations.length,1);assert.equal(secondPolicyPage.has_more,false);assert.notEqual(secondPolicyPage.observations[0].sequence,firstPolicyPage.observations[0].sequence);
    const resourceJob=await call('spider_start_crawl',{config:{seed_urls:[`${fixtureOrigin}/a?owned=1&next=%23`,`${fixtureOrigin}/b`],obey_robots:false,crawl_links:false,discover_sitemaps:false,delay_ms:0,resource_discovery:{}}});
    await until(async()=>{const status=await call('spider_status',{id:resourceJob.id});if(status.status==='failed')throw Error(status.error);return status.status==='completed';});
    const resourceBeforeRead=websiteRequests;
    const firstResource=await call('spider_resources',{id:resourceJob.id,limit:1});assert.equal(firstResource.observations.length,1);assert.equal(firstResource.has_more,true);
    const secondResource=await call('spider_resources',{id:resourceJob.id,limit:1,after_sequence:firstResource.next_after_sequence});assert.equal(secondResource.observations.length,1);assert.equal(secondResource.has_more,false);assert.notEqual(secondResource.observations[0].sequence,firstResource.observations[0].sequence);
    const firstTransport=await call('spider_resource_transports',{id:resourceJob.id,limit:1});assert.equal(firstTransport.transports.length,1);assert.match(firstTransport.next_after_url,/\?owned=1&next=%23$/);
    const nextTransport=await call('spider_resource_transports',{id:resourceJob.id,limit:1,after_url:firstTransport.next_after_url});assert.equal(nextTransport.transports.length,1);assert.equal(nextTransport.has_more,false);assert.equal(websiteRequests,resourceBeforeRead);
    checks.push(`${kind}: SDK initialize, 22 tools; bounded native resource occurrences and transport metadata with encoded URL cursor, zero-HTTP supplied CSS inspection and strict resource argument types; native saved-source inventory/document/cohort, supplied-text analysis, zero-HTTP policy preview, native policy cursor, disabled ledgers, crawl/read/analysis/compare, completed-pause conflict, derived selected recrawl preserving parent/unselected rows`);
    assertOwnedServerAlive('before transport close');
    await client.close();client=null;
    assertOwnedServerAlive('after transport close');
  }
  assertOwnedServerAlive('completed SDK acceptance');
  assert.equal(hash(await readFile(binary)),binarySha,'Binary identity unchanged after both transports');
  assertOwnedServerAlive('after final binary identity read');
  evidence={tested_at:new Date().toISOString(),sdk:'@modelcontextprotocol/client 2.3.1',platform:process.platform,result:'PASS',checks,
    binary_sha256:binarySha,binary_path:binary,source_harness_sha256:'f18675e22a217a6c4fabf48507991945c4150ef48cc4ef283fbd982d5ad05efb',derived_harness_sha256:harnessSha,
    ownership:{post_bind_stdout_required:true,app_pid:server.pid,app_url:api,fixture_url:fixtureOrigin,expected_release_sha256:process.env.SPIDER_RELEASE_SHA || null,liveness_checks:livenessChecks}};
}finally{
  try{if(client)await client.close();}
  finally{
    try{
      if(evidence)assertOwnedServerAlive('immediately before owned shutdown');
      if(server && !serverClosed){if(server.exitCode===null && server.signalCode===null && !serverError)server.kill();await serverClose;}
      if(evidence)evidence.ownership.owned_child_closed=serverClosed;
    }finally{
      if(fixture.listening)await new Promise((resolve,reject)=>fixture.close(error=>error?reject(error):resolve()));
      const cleanupPath=path.resolve(data);
      assert.equal(path.dirname(cleanupPath),path.resolve(os.tmpdir()),'cleanup must remain directly inside the temporary directory');
      assert.ok(path.basename(cleanupPath).startsWith('open-seo-spider-mcp-'),'cleanup must target the owned MCP fixture directory');
      await rm(cleanupPath,{recursive:true,force:true});
    }
  }
}
// Evidence is emitted only after SDK transports and the owned app/fixture have closed.
console.log(JSON.stringify(evidence,null,2));
if(process.env.SPIDER_EVIDENCE){await mkdir(path.dirname(process.env.SPIDER_EVIDENCE),{recursive:true});await writeFile(process.env.SPIDER_EVIDENCE,JSON.stringify(evidence,null,2)+'\n');}
