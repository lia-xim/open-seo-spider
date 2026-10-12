// Real local API/CLI acceptance for supplied text; source labels cannot trigger fetches.
import http from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,writeFile,mkdir,rm} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||path.join(root,'target','debug',`open-seo-spider${process.platform==='win32'?'.exe':''}`);
const data=await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-content-'));
const api='http://127.0.0.1:18794';let websiteRequests=0;
const website=http.createServer((_,response)=>{websiteRequests++;response.end('Unexpected content fetch');});
await new Promise(resolve=>website.listen(19094,'127.0.0.1',resolve));
const text='One clear sentence makes this sample easy to read. Another short sentence contains familiar words. We compare this same supplied text twice and keep every missing observation separate.';
const input={corpus_id:'owned-content-fixture',corpus_scope:'supplied text with one missing sample',corpus_complete:false,documents:[
  {id:'a',source:'http://127.0.0.1:19094/never-fetch',text,language:'en',complete:true},
  {id:'b',source:'manual_text',text,language:'en',complete:true},
  {id:'missing',source:'manual_text',text:null,language:null,complete:false},
]};
const server=spawn(binary,['serve','--listen','127.0.0.1:18794','--data-dir',path.join(data,'saved')],{cwd:root,windowsHide:true,stdio:'ignore'});
async function execute(args){return new Promise((resolve,reject)=>{const child=spawn(binary,args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';child.stdout.on('data',chunk=>stdout+=chunk);child.stderr.on('data',chunk=>stderr+=chunk);child.on('error',reject);child.on('exit',code=>resolve({code,stdout,stderr}));});}
async function analyze(value){const response=await fetch(api+'/api/content/analyze',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(value)});const text=await response.text();let result;try{result=JSON.parse(text);}catch{result={error:text};}return {response,value:result};}
try{
  for(let step=0;step<100;step++){try{if((await fetch(api+'/healthz')).ok)break;}catch{}if(step===99)throw Error('Content server startup timeout');await new Promise(resolve=>setTimeout(resolve,50));}
  const checks=[];
  const analyzed=await analyze(input);assert.equal(analyzed.response.status,200);const result=analyzed.value;
  assert.equal(result.website_requests,0);assert.equal(result.corpus.missing_text_documents,1);assert.equal(result.corpus.declared_complete,false);assert.equal(result.pairs.length,3);assert.equal(result.pairs[0].observed_jaccard,1);assert.equal(result.pairs[0].near_duplicate,true);assert.equal(result.pairs[1].near_duplicate,null);assert.equal(result.pairs[1].observed_jaccard,null);
  checks.push('API complete identical pair scored1; missing pair remains unknown; explicit incomplete corpus and zero website requests');
  const configured=structuredClone(input);configured.config={stopwords:['one'],similarity:{shingle_size:2,threshold:0.75}};const changed=(await analyze(configured)).value;assert.notEqual(changed.config_sha256,result.config_sha256);assert.notEqual(changed.input_sha256,result.input_sha256);
  const incomplete=structuredClone(input);incomplete.documents[1].complete=false;incomplete.documents[1].truncation_reason='owned test excerpt';const truncated=(await analyze(incomplete)).value;assert.equal(truncated.pairs[0].observed_jaccard,1);assert.equal(truncated.pairs[0].near_duplicate,null);assert.equal(truncated.pairs[0].sample_complete,false);
  checks.push('Configuration changes hashes; excerpt similarity stays observed while near-duplicate classification stays unknown');
  for(const invalid of [{...input,documents:[]},{...input,unknown:true},{...input,config:{limits:{max_documents:129}}},{...input,config:{similarity:{threshold:2}}},{...input,config:{stopwords:['not a single token']}}]){const rejected=await analyze(invalid);assert.ok([400,422].includes(rejected.response.status),JSON.stringify(rejected.value));}
  checks.push('Empty corpus, unknown fields, excessive limits, invalid threshold and invalid stopword rejected');
  const requestPath=path.join(data,'input.json'),outputPath=path.join(data,'result.json');await writeFile(requestPath,JSON.stringify(input));
  const cli=await execute(['analyze-content',requestPath,'--output',outputPath]);assert.equal(cli.code,0,cli.stderr);assert.deepEqual(JSON.parse(await readFile(outputPath,'utf8')),result);
  const original=await readFile(requestPath);const refused=await execute(['analyze-content',requestPath,'--output',requestPath]);assert.notEqual(refused.code,0);assert.deepEqual(await readFile(requestPath),original);
  const existing=await execute(['analyze-content',requestPath,'--output',outputPath]);assert.notEqual(existing.code,0);assert.deepEqual(JSON.parse(await readFile(outputPath,'utf8')),result);
  assert.equal(websiteRequests,0);checks.push('CLI matches typed API exactly, input/existing output preserved, URL-shaped source label never fetched');
  const evidence={tested_at:new Date().toISOString(),platform:process.platform,result:'PASS',checks,website_requests:websiteRequests,engine:result.engine,token_profile:result.token_profile,similarity_profile:result.similarity_profile,full_reference_parity:'NOT_PROVEN'};console.log(JSON.stringify(evidence,null,2));
  if(process.env.SPIDER_EVIDENCE){await mkdir(path.dirname(process.env.SPIDER_EVIDENCE),{recursive:true});await writeFile(process.env.SPIDER_EVIDENCE,JSON.stringify(evidence,null,2)+'\n');}
}finally{if(server.exitCode===null){const stopped=new Promise(resolve=>server.once('exit',resolve));server.kill();await stopped;}await new Promise(resolve=>website.close(resolve));assert.equal(path.dirname(path.resolve(data)),path.resolve(os.tmpdir()));assert.ok(path.basename(data).startsWith('open-seo-spider-content-'));await rm(data,{recursive:true,force:true});}
