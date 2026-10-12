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
let revision=0;
const fixture=http.createServer((request,response)=>{if(request.url==='/robots.txt'){response.end('User-agent: *\nAllow: /');return;}response.setHeader('content-type','text/html');response.end(`<html lang="de"><title>MCP ${request.url} revision ${revision}</title><body><h1>Fixture</h1></body></html>`);});
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
    const inventory=await client.listTools();assert.equal(inventory.tools.length,13);
    assert.ok(inventory.tools.some(tool=>tool.name==='spider_pause'&&tool.annotations.readOnlyHint===false));
    assert.ok(inventory.tools.some(tool=>tool.name==='spider_recrawl'&&tool.annotations.readOnlyHint===false&&tool.annotations.openWorldHint===true));
    const caps=await call('spider_capabilities');assert.equal(caps.crawl.url_limit,null);
    revision=0;
    const started=await call('spider_start_crawl',{config:{seed_urls:['http://127.0.0.1:19089/a','http://127.0.0.1:19089/b'],crawl_links:false,discover_sitemaps:false,delay_ms:0}});
    const finished=await until(async()=>{const status=await call('spider_status',{id:started.id});return status.status==='completed'&&status;});assert.equal(finished.stats.fetched,2);
    const pages=await call('spider_pages',{id:started.id,offset:1,limit:1});assert.equal(pages.pages.length,1);assert.equal(pages.total,2);
    assert((await call('spider_issues',{id:started.id})).issues.length>0);
    assert.equal((await call('spider_project',{id:started.id})).pages.length,2);
    assert.equal((await call('spider_analysis',{id:started.id})).graph.nodes.length,2);
    assert.equal((await call('spider_compare',{before:started.id,after:started.id})).changes.length,0);
    assert((await call('spider_list_crawls')).crawls.length>0);
    const missing=await client.callTool({name:'spider_status',arguments:{id:'00000000-0000-0000-0000-000000000000'}});assert.equal(missing.isError,true);
    const latePause=await client.callTool({name:'spider_pause',arguments:{id:started.id}});assert.equal(latePause.isError,true);
    const parent=await call('spider_project',{id:started.id});revision=1;
    const recrawl=await call('spider_recrawl',{id:started.id,urls:['http://127.0.0.1:19089/a'],max_attempts:0});assert.notEqual(recrawl.id,started.id);assert.equal(recrawl.parent_id,started.id);
    await until(async()=>{const status=await call('spider_status',{id:recrawl.id});if(status.status==='failed')throw Error(status.error);return status.status==='completed';});
    const child=await call('spider_project',{id:recrawl.id});assert.equal(child.pages.length,2);assert.deepEqual(child.pages.find(page=>page.url.endsWith('/b')),parent.pages.find(page=>page.url.endsWith('/b')));assert.match(child.pages.find(page=>page.url.endsWith('/a')).titles[0],/revision 1/);assert.deepEqual(await call('spider_project',{id:started.id}),parent);
    const changes=await call('spider_compare',{before:started.id,after:recrawl.id});assert.equal(changes.changes.length,1);assert.equal(changes.changes[0].url,'http://127.0.0.1:19089/a');
    checks.push(`${kind}: SDK initialize, 13 tools, crawl/read/analysis/compare, completed-pause conflict, derived selected recrawl with fresh selected row and exact retained parent/unselected rows`);
    await client.close();client=null;
  }
  const evidence={tested_at:new Date().toISOString(),sdk:'@modelcontextprotocol/client 2.3.1',platform:process.platform,result:'PASS',checks};
  console.log(JSON.stringify(evidence,null,2));
  if(process.env.SPIDER_EVIDENCE){await mkdir(path.dirname(process.env.SPIDER_EVIDENCE),{recursive:true});await writeFile(process.env.SPIDER_EVIDENCE,JSON.stringify(evidence,null,2)+'\n');}
}finally{if(client)await client.close();if(server.exitCode===null){const stopped=new Promise(resolve=>server.once('exit',resolve));server.kill();await stopped;}await new Promise(resolve=>fixture.close(resolve));const cleanupPath=path.resolve(data);assert.equal(path.dirname(cleanupPath),path.resolve(os.tmpdir()),'cleanup must remain directly inside the temporary directory');assert.ok(path.basename(cleanupPath).startsWith('open-seo-spider-mcp-'),'cleanup must target the owned MCP fixture directory');await rm(cleanupPath,{recursive:true,force:true});}
