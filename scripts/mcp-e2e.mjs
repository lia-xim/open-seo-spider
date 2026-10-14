import { Client, StreamableHTTPClientTransport } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { spawn } from 'node:child_process';
import http from 'node:http';
import { mkdtemp, rm, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||path.join(root,'target','debug',`open-seo-spider${process.platform==='win32'?'.exe':''}`);
const data=await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-mcp-'));
const api='http://127.0.0.1:18788';
let revision=0,websiteRequests=0;
const fixture=http.createServer((request,response)=>{websiteRequests++;if(request.url==='/robots.txt'){response.end('User-agent: *\nAllow: /');return;}response.setHeader('content-type','text/html');response.end(`<html lang="de"><title>MCP ${request.url} revision ${revision}</title><body><h1>Fixture</h1></body></html>`);});
await new Promise(resolve=>fixture.listen(19089,'127.0.0.1',resolve));
const server=spawn(binary,['serve','--listen','127.0.0.1:18788','--data-dir',data],{cwd:root,windowsHide:true,stdio:'ignore'});
let client;
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(fn){for(let step=0;step<400;step++){const result=await fn();if(result)return result;await delay(25);}throw new Error('MCP acceptance timeout');}
async function call(name,args={}){const result=await client.callTool({name,arguments:args});assert.equal(result.isError,false,JSON.stringify(result));return result.structuredContent;}
try{
  await until(async()=>{try{return(await fetch(api+'/healthz')).ok;}catch{return false;}});
  const checks=[];
  for(const kind of ['stdio','streamable_http']){
    client=new Client({name:'open-seo-spider-acceptance',version:'0.1.0'});
    const transport=kind==='stdio'?new StdioClientTransport({command:binary,args:['mcp','--server',api],cwd:root,stderr:'pipe'}):new StreamableHTTPClientTransport(new URL(api+'/mcp'));
    await client.connect(transport);
    const inventory=await client.listTools();assert.equal(inventory.tools.length,25);
    assert.ok(inventory.tools.some(tool=>tool.name==='spider_compare_saved'&&tool.annotations.readOnlyHint===true&&tool.annotations.openWorldHint===false));
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
    const policy={scope:{roots:[{origin:'http://127.0.0.1:19089',folders:['/']}]}};
    const preview=await call('spider_preview_policy',{input:{config:policy,inputs:[{occurrence_id:'own-mcp-seed',source_kind:'seed',raw_value:'http://127.0.0.1:19089/a',base_url:null,source_url:null,source_location:null}]}});
    assert.equal(preview.website_requests,0);assert.equal(preview.decisions[0].fetch_candidate_url,'http://127.0.0.1:19089/a');
    revision=0;
    const started=await call('spider_start_crawl',{config:{seed_urls:['http://127.0.0.1:19089/a','http://127.0.0.1:19089/b'],crawl_links:false,discover_sitemaps:false,delay_ms:0,content_capture:{}}});
    const finished=await until(async()=>{const status=await call('spider_status',{id:started.id});return status.status==='completed'&&status;});assert.equal(finished.stats.fetched,2);
    const disabledLedger=await call('spider_policy',{id:started.id,limit:1});assert.equal(disabledLedger.enabled,false);assert.equal(disabledLedger.ledger_available,false);assert.deepEqual(disabledLedger.observations,[]);
    const pages=await call('spider_pages',{id:started.id,offset:1,limit:1});assert.equal(pages.pages.length,1);assert.equal(pages.total,2);
    const contentRows=await call('spider_content',{id:started.id,limit:1});assert.equal(contentRows.rows.length,1);assert.equal(contentRows.text_included,false);assert.equal(contentRows.has_more,true);assert.equal(contentRows.rows[0].status,'complete');
    assert.equal((await client.callTool({name:'spider_content',arguments:{id:started.id,source_kind:1}})).isError,true);
    const contentDocument=await call('spider_content_document',{id:started.id,url:contentRows.rows[0].requested_url});assert.equal(contentDocument.capture.result.text,'Fixture');assert.equal(contentDocument.capture.result.html_language,'de');
    const savedAnalysis=await call('spider_analyze_crawl_content',{id:started.id,input:{urls:['http://127.0.0.1:19089/a','http://127.0.0.1:19089/b'],expected_generation:finished.generation}});assert.equal(savedAnalysis.website_requests,0);assert.equal(savedAnalysis.site_wide_coverage,false);assert.equal(savedAnalysis.documents.length,2);assert.equal(savedAnalysis.analysis.pairs[0].identical_supplied_text,true);
    assert((await call('spider_issues',{id:started.id})).issues.length>0);
    assert.equal((await call('spider_project',{id:started.id})).pages.length,2);
    assert.equal((await call('spider_analysis',{id:started.id})).graph.nodes.length,2);
    assert.equal((await call('spider_compare',{before:started.id,after:started.id})).changes.length,0);
    const beforeSavedComparison=websiteRequests;
    const savedComparison=await call('spider_compare_saved',{before:started.id,after:started.id});
    assert.equal(savedComparison.website_requests,0);
    assert.equal(savedComparison.comparison.engine,'open-seo-spider/saved-comparison-v1');
    assert.equal(savedComparison.comparison.counts.changed,0);
    assert.equal(savedComparison.comparison.counts.unchanged,2);
    assert.equal(websiteRequests,beforeSavedComparison);
    const beforeRead=websiteRequests;
    for(const name of ['spider_resources','spider_resource_transports']){
      assert.ok(inventory.tools.some(tool=>tool.name===name&&tool.annotations.readOnlyHint===true&&tool.annotations.openWorldHint===false));
      const rows=await call(name,{id:started.id,limit:1});assert.equal(rows.ledger_available,false);assert.equal(rows.website_requests,0);
    }
    const css=await call('spider_inspect_css',{input:{source:{css_text:'a{background:url("owned.png")}',base_url:'http://127.0.0.1:19089/css/main',source_url:'http://127.0.0.1:19089/css/main',source_revision:'mcp-supplied-css',inline:false}}});
    assert.equal(css.website_requests,0);assert.equal(css.dependencies[0].resolved_url,'http://127.0.0.1:19089/css/owned.png');assert.equal(websiteRequests,beforeRead);
    assert.equal((await client.callTool({name:'spider_resources',arguments:{id:started.id,limit:'1'}})).isError,true);
    assert.equal((await client.callTool({name:'spider_resource_transports',arguments:{id:started.id,after_url:1}})).isError,true);
    await assert.rejects(client.callTool({name:'spider_resources',arguments:{id:started.id,after_url:'https://example.test/'}}),/Unexpected or missing tool argument/);
    await assert.rejects(client.callTool({name:'spider_inspect_css',arguments:{input:{},options:{}}}),/Unexpected or missing tool argument/);
    assert.equal(websiteRequests,beforeRead);
    assert((await call('spider_list_crawls')).crawls.length>0);
    const missing=await client.callTool({name:'spider_status',arguments:{id:'00000000-0000-0000-0000-000000000000'}});assert.equal(missing.isError,true);
    const latePause=await client.callTool({name:'spider_pause',arguments:{id:started.id}});assert.equal(latePause.isError,true);
    const parent=await call('spider_project',{id:started.id});revision=1;
    const recrawl=await call('spider_recrawl',{id:started.id,urls:['http://127.0.0.1:19089/a'],max_attempts:0});assert.notEqual(recrawl.id,started.id);assert.equal(recrawl.parent_id,started.id);
    await until(async()=>{const status=await call('spider_status',{id:recrawl.id});if(status.status==='failed')throw Error(status.error);return status.status==='completed';});
    const child=await call('spider_project',{id:recrawl.id});assert.equal(child.pages.length,2);assert.deepEqual(child.pages.find(page=>page.url.endsWith('/b')),parent.pages.find(page=>page.url.endsWith('/b')));assert.match(child.pages.find(page=>page.url.endsWith('/a')).titles[0],/revision 1/);assert.deepEqual(await call('spider_project',{id:started.id}),parent);
    const changes=await call('spider_compare',{before:started.id,after:recrawl.id});assert.equal(changes.changes.length,1);assert.equal(changes.changes[0].url,'http://127.0.0.1:19089/a');
    const policyJob=await call('spider_start_crawl',{config:{seed_urls:['http://127.0.0.1:19089/a','http://127.0.0.1:19089/b'],crawl_links:false,discover_sitemaps:false,delay_ms:0,crawl_policy:policy}});
    await until(async()=>{const status=await call('spider_status',{id:policyJob.id});if(status.status==='failed')throw Error(status.error);return status.status==='completed';});
    const firstPolicyPage=await call('spider_policy',{id:policyJob.id,limit:1});assert.equal(firstPolicyPage.enabled,true);assert.equal(firstPolicyPage.ledger_available,true);assert.equal(firstPolicyPage.observations.length,1);assert.equal(firstPolicyPage.has_more,true);
    const secondPolicyPage=await call('spider_policy',{id:policyJob.id,after_sequence:firstPolicyPage.next_after_sequence,limit:1});assert.equal(secondPolicyPage.observations.length,1);assert.equal(secondPolicyPage.has_more,false);assert.notEqual(secondPolicyPage.observations[0].sequence,firstPolicyPage.observations[0].sequence);
    const resourceJob=await call('spider_start_crawl',{config:{seed_urls:['http://127.0.0.1:19089/a?owned=1&next=%23','http://127.0.0.1:19089/b'],obey_robots:false,crawl_links:false,discover_sitemaps:false,delay_ms:0,resource_discovery:{}}});
    await until(async()=>{const status=await call('spider_status',{id:resourceJob.id});if(status.status==='failed')throw Error(status.error);return status.status==='completed';});
    const resourceBeforeRead=websiteRequests;
    const firstResource=await call('spider_resources',{id:resourceJob.id,limit:1});assert.equal(firstResource.observations.length,1);assert.equal(firstResource.has_more,true);
    const secondResource=await call('spider_resources',{id:resourceJob.id,limit:1,after_sequence:firstResource.next_after_sequence});assert.equal(secondResource.observations.length,1);assert.equal(secondResource.has_more,false);assert.notEqual(secondResource.observations[0].sequence,firstResource.observations[0].sequence);
    const firstTransport=await call('spider_resource_transports',{id:resourceJob.id,limit:1});assert.equal(firstTransport.transports.length,1);assert.match(firstTransport.next_after_url,/\?owned=1&next=%23$/);
    const nextTransport=await call('spider_resource_transports',{id:resourceJob.id,limit:1,after_url:firstTransport.next_after_url});assert.equal(nextTransport.transports.length,1);assert.equal(nextTransport.has_more,false);assert.equal(websiteRequests,resourceBeforeRead);
    checks.push(`${kind}: SDK initialize, 25 tools; bounded native resource occurrences and transport metadata with encoded URL cursor, zero-HTTP supplied CSS inspection and strict resource argument types; native saved-source inventory/document/cohort, supplied-text analysis, zero-HTTP policy preview, native policy cursor, disabled ledgers, crawl/read/analysis/compare, additional saved comparison with zero website requests, completed-pause conflict, derived selected recrawl preserving parent/unselected rows`);
    await client.close();client=null;
  }
  const evidence={tested_at:new Date().toISOString(),sdk:'@modelcontextprotocol/client 2.3.1',platform:process.platform,result:'PASS',checks};
  console.log(JSON.stringify(evidence,null,2));
  if(process.env.SPIDER_EVIDENCE){await mkdir(path.dirname(process.env.SPIDER_EVIDENCE),{recursive:true});await writeFile(process.env.SPIDER_EVIDENCE,JSON.stringify(evidence,null,2)+'\n');}
}finally{if(client)await client.close();if(server.exitCode===null){const stopped=new Promise(resolve=>server.once('exit',resolve));server.kill();await stopped;}await new Promise(resolve=>fixture.close(resolve));const cleanupPath=path.resolve(data);assert.equal(path.dirname(cleanupPath),path.resolve(os.tmpdir()),'cleanup must remain directly inside the temporary directory');assert.ok(path.basename(cleanupPath).startsWith('open-seo-spider-mcp-'),'cleanup must target the owned MCP fixture directory');await rm(cleanupPath,{recursive:true,force:true});}
