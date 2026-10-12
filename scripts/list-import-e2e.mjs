// Owned byte-only imports -> actual HTTP/API and CLI -> preserved source rows.
import assert from 'node:assert/strict';
import http from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve,join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const binary=process.env.SPIDER_BINARY||join(root,'target','debug',process.platform==='win32'?'open-seo-spider.exe':'open-seo-spider');
const temporary=await mkdtemp(join(tmpdir(),'open-seo-spider-lists-'));
const app='http://127.0.0.1:18785';
const delay=ms=>new Promise(done=>setTimeout(done,ms));
const requests=new Map();const checks=[];let child,log='';
const fixture=http.createServer((req,res)=>{
 requests.set(req.url,(requests.get(req.url)||0)+1);
 if(req.url==='/robots.txt')return res.end('User-agent: *\nDisallow: /blocked\n');
 if(req.url==='/nested.xml'){res.setHeader('content-type','application/xml');return res.end(`<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>${origin}/sitemap-page</loc></url></urlset>`);}
 res.setHeader('content-type','text/html');res.end(`<title>Owned ${req.url}</title><h1>${req.url}</h1><a href="/must-not-follow">No list-mode discovery</a>`);
});
await new Promise(done=>fixture.listen(0,'127.0.0.1',done));
const origin=`http://127.0.0.1:${fixture.address().port}`;
const call=async(route,body)=>{
 const response=await fetch(app+route,body===undefined?{}:{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});
 assert.equal(response.status,200,`${route}: ${await response.clone().text()}`);return response.json();
};
async function until(fn,label){for(let i=0;i<600;i++){const value=await fn();if(value)return value;await delay(50);}throw Error(`Timed out: ${label}; ${log.slice(-2000)}`);}
async function launch(){child=spawn(binary,['serve','--listen','127.0.0.1:18785','--data-dir',join(temporary,'data')],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});child.stdout.on('data',x=>{log+=x});child.stderr.on('data',x=>{log+=x});child.on('error',e=>{log+=e.message});await until(async()=>{try{return(await call('/healthz')).ok}catch{return false}},'API ready');}
async function stop(){if(!child||child.exitCode!==null)return;const ended=new Promise(done=>child.once('exit',done));child.kill();await ended;child=null;}
async function run(args,expected=0){const process=spawn(binary,args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});let output='';process.stdout.on('data',x=>{output+=x});process.stderr.on('data',x=>{output+=x});const code=await new Promise((done,reject)=>{process.once('exit',done);process.once('error',reject)});assert.equal(code,expected,`CLI ${args[0]}: ${output.slice(-1200)}`);return output;}
const terminal=id=>until(async()=>{const state=await call(`/api/crawls/${id}`);return ['completed','failed','cancelled'].includes(state.status)&&state},'crawl terminal');
const importBytes=(bytes,format)=>call('/api/lists/import',{content_base64:Buffer.from(bytes).toString('base64'),options:{format}});
try{
 await launch();
 const text=`${origin}/a#first\r\n\r\nnot a URL\n${origin}/a#second\n${origin}/b?b=2&a=1\n`;
 const imported=await importBytes(text,{kind:'txt'});
 assert.equal(imported.website_requests,0);assert.equal(imported.rows.length,5);assert.deepEqual(imported.rows.map(r=>r.source_index),[1,2,3,4,5]);assert.equal(imported.rows[0].fragment,'first');assert.equal(imported.rows[3].fragment,'second');assert.ok(imported.rows[1].error);assert.equal(requests.size,0);
 checks.push('TXT preserves original empty, invalid and duplicate fragment rows; import makes zero HTTP requests');
 const csv=await importBytes(`Label;URL\r\nfirst;${origin}/a\r\nsecond;${origin}/a\r\nbad;=FORMULA()\r\n`,{kind:'csv',delimiter:'semicolon',column:{kind:'header',name:'URL',row:1},start_row:2});
 assert.deepEqual(csv.rows.map(r=>r.source_row),[2,3,4]);assert.equal(csv.rows[2].raw_value,'=FORMULA()');assert.ok(csv.rows[2].error);assert.equal(requests.size,0);
 checks.push('CSV explicit exact header/delimiter selection retains duplicate and invalid source rows');
 const index=await importBytes(`<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>${origin}/nested.xml</loc></sitemap></sitemapindex>`,{kind:'xml'});
 assert.equal(index.rows[0].candidate_kind,'sitemap_document');assert.equal(requests.size,0);
 checks.push('XML sitemap index retains document candidate identity and fetches nothing during import');
 const xlsx=await readFile(join(root,'crates/core/tests/fixtures/list-import/ordered-formulas.xlsx'));
 const inspection=await call('/api/lists/inspect',{content_base64:xlsx.toString('base64')});
 assert.equal(inspection.website_requests,0);assert.deepEqual(inspection.sheets.map(s=>s.name),['URLs','Other','Hidden']);assert.equal(inspection.sheets[2].visibility,'Hidden');
 const format={kind:'xlsx',sheet:{kind:'name',value:'URLs'},column:{kind:'header',name:'URL',row:1},start_row:2,formula_policy:'reject'};
 const workbook=await importBytes(xlsx,format);const formula=workbook.rows.find(row=>row.cell==='B6');assert.ok(formula.error);assert.equal(formula.normalized_url,null);assert.equal(workbook.rows.find(row=>row.cell==='B4').cell_type,'missing');
 const cached=await importBytes(xlsx,{...format,formula_policy:'cached_string_only'});assert.equal(cached.rows.find(row=>row.cell==='B6').normalized_url,'https://example.test/cached');assert.equal(requests.size,0);
 checks.push('XLSX sheet inspection/selection, missing selected cells and formula reject/explicit cached string preserve provenance with zero HTTP');
 for(const body of [{content_base64:'%%%invalid',options:{format:{kind:'txt'}}},{content_base64:Buffer.from(text).toString('base64'),options:{format:{kind:'txt'},limits:{max_input_bytes:1}}}])assert.equal((await fetch(app+'/api/lists/import',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)})).status,400);
 const forged=structuredClone(imported);forged.rows[0].normalized_url=origin+'/forged-target';
 assert.equal((await fetch(app+'/api/crawls',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({input_imports:[forged],mode:'list'})})).status,400);assert.equal(requests.size,0);
 checks.push('Invalid base64/bounds and transported normalized-target forgery rejected before any network work');
 const started=await call('/api/crawls',{input_imports:[imported],mode:'list',discover_sitemaps:false,delay_ms:0});assert.equal((await terminal(started.id)).status,'completed');
 const report=await call(`/api/crawls/${started.id}/project.json`);assert.equal(report.pages.length,2);assert.equal(requests.get('/a'),1);assert.equal(requests.get('/b?b=2&a=1'),1);assert.equal(requests.get('/must-not-follow'),undefined);assert.deepEqual(report.config.input_imports[0],imported);
 const exported=await(await fetch(app+`/api/crawls/${started.id}/inputs.csv`)).text();assert.match(exported,/declared_client_input/);assert.match(exported,/invalid_input/);assert.match(exported,/first/);assert.match(exported,/second/);
 checks.push('Actual API list crawl fetches two distinct identities once, preserves five declared occurrences and exports per-input observations');
 const snapshot=await readFile(join(temporary,'data',`${started.id}.json`));await stop();await launch();assert.ok(snapshot.equals(await readFile(join(temporary,'data',`${started.id}.json`))));assert.deepEqual((await call(`/api/crawls/${started.id}/project.json`)).config.input_imports[0],imported);
 checks.push('Saved original input order/invalids/duplicates persist byte-identically across server restart');
 const sitemap=await call('/api/crawls',{input_imports:[index],mode:'list',discover_sitemaps:false,delay_ms:0});assert.equal((await terminal(sitemap.id)).status,'completed');const sitemapReport=await call(`/api/crawls/${sitemap.id}/project.json`);assert.equal(sitemapReport.config.seed_urls.length,0);assert.equal(requests.get('/nested.xml'),1);assert.ok(sitemapReport.pages.some(p=>p.url.endsWith('/sitemap-page')));assert.ok(!sitemapReport.pages.some(p=>p.url.endsWith('/nested.xml')));
 checks.push('Sitemap document inputs use recursive sitemap discovery after explicit crawl start and are never manufactured page records');
 const txtPath=join(temporary,'urls.txt'),out=join(temporary,'import.json'),configPath=join(temporary,'config.json'),cliReportPath=join(temporary,'crawl.json'),inputCsvPath=join(temporary,'inputs.csv');await writeFile(txtPath,text);
 const beforeCli=new Map(requests);await run(['import-list',txtPath,'--output',out]);assert.deepEqual(JSON.parse(await readFile(out,'utf8')),imported);assert.deepEqual(requests,beforeCli);
 await run(['import-list',join(root,'crates/core/tests/fixtures/list-import/ordered-formulas.xlsx'),'--inspect','--output',out]);assert.deepEqual(JSON.parse(await readFile(out,'utf8')).sheets,inspection.sheets);assert.deepEqual(requests,beforeCli);
 checks.push('Offline CLI TXT import and XLSX inspection have native/API equivalent outputs with zero crawl requests');
 await writeFile(configPath,JSON.stringify({concurrency:3,delay_ms:0,discover_sitemaps:false,custom_extractions:[{name:'heading',selector:'h1',attribute:null}],validate_fragments:true}));
 await run(['crawl','--config',configPath,'--list-file',txtPath,'--output',cliReportPath,'--input-csv',inputCsvPath]);const cliReport=JSON.parse(await readFile(cliReportPath,'utf8'));assert.equal(cliReport.config.concurrency,3);assert.equal(cliReport.config.crawl_links,false);assert.equal(cliReport.config.validate_fragments,true);assert.equal(cliReport.config.custom_extractions[0].name,'heading');assert.equal(cliReport.pages.length,2);assert.equal(cliReport.config.input_imports[0].rows.length,5);assert.equal(requests.get('/a'),2);assert.equal(requests.get('/must-not-follow'),undefined);assert.match(await readFile(inputCsvPath,'utf8'),/invalid_input/);
 checks.push('CLI config values survive unset flags; list-file crawl retains five source rows and deduplicates network work');
 const beforeErrors=new Map(requests),missingOutput=join(temporary,'must-not-exist.json');
 await run(['import-list',join(root,'crates/core/tests/fixtures/list-import/ordered-formulas.xlsx'),'--output',missingOutput],1);
 await writeFile(configPath,'{broken-json');
 await run(['crawl','--config',configPath,'--output',missingOutput],1);
 await run(['crawl','ftp://example.test/unsupported','--output',missingOutput],1);
 await run(['crawl','--definitely-unknown-option'],2);
 await assert.rejects(readFile(missingOutput),{code:'ENOENT'});assert.deepEqual(requests,beforeErrors);
 assert.ok(Array.isArray(cliReport.issues));assert.ok(cliReport.issues.length>0,'Authored pages without descriptions must produce analyzed issues');
 checks.push('CLI analysis/export succeeds with exit 0; invalid workbook selection/config/scheme exit 1 and unknown option exits 2 without requests or output');
 const proof={tested_at:new Date().toISOString(),result:'PASS',platform:process.platform,checks,offline_website_requests:0,formats:['txt','csv','xml','xlsx'],limitations:'Owned byte/HTTP/API/CLI fixtures only; proprietary file compatibility and complete vendor list workflow parity remain NOT_PROVEN.'};
 if(process.env.SPIDER_EVIDENCE){await mkdir(dirname(process.env.SPIDER_EVIDENCE),{recursive:true});await writeFile(process.env.SPIDER_EVIDENCE,JSON.stringify(proof,null,2)+'\n');}
 console.log(JSON.stringify(proof,null,2));
}finally{await stop();await new Promise(done=>fixture.close(done));assert.ok(temporary.startsWith(join(tmpdir(),'open-seo-spider-lists-')));await rm(temporary,{recursive:true,force:true});}
