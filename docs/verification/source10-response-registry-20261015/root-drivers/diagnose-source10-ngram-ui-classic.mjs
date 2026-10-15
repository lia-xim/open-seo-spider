import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {createRequire} from 'node:module';
import {createHash,randomBytes} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013', O=E+'/results/source10-ngram-ui-diagnostic-v2', lock=E+'/results/combined-native-outer-v4/active.lock';
const H=b=>createHash('sha256').update(b).digest('hex'),R=JSON.parse(fs.readFileSync('work/v10-browser-native-release.json'));
assert.equal(H(fs.readFileSync('work/v10-browser-native-release.json')),'d185f40cc5fed97a0b01a77b68b6fde1abd6f897012fa1f8cc8a7e16ec7e5beb');
assert(!fs.existsSync(O)&&!fs.existsSync(lock));
const binderPath=E+'/native-combined-rebind-v10-v1/combined-bindings.mjs';
assert.equal(H(fs.readFileSync(binderPath)),R.bindings_sha256);
const binding=(await import(pathToFileURL(binderPath))).bindCombined(R,R.source_manifest_sha256);binding.verify();
const lockRaw=Buffer.from(JSON.stringify({gate:'source10-ngram-ui-diagnostic-v2',owner_pid:process.pid,token:randomBytes(32).toString('hex'),source_manifest_sha256:R.source_manifest_sha256})+'\n');fs.writeFileSync(lock,lockRaw,{flag:'wx'});
let app,browser,closed,exit;const proof={kind:'source10_ngram_ui_diagnostic',recorded_at:new Date().toISOString(),result:'RUNNING',source_manifest_sha256:R.source_manifest_sha256,candidate_binary_sha256:R.candidate_binary_sha256,qualification:'Compiled actual Source10; synthetic failed-browser data copy; settings-only diagnostic, not native acceptance or installation.'};
try{
 fs.mkdirSync(O);fs.cpSync(E+'/results/resource-directory-workflow-native-v10-r2/data',O+'/data',{recursive:true,errorOnExist:true,force:false});
 app=spawn(binding.binary,['serve','--listen','127.0.0.1:0','--data-dir',O+'/data','--shutdown-on-stdin-eof'],{windowsHide:true,stdio:['pipe','pipe','pipe']});closed=once(app,'close').then(([code,signal])=>exit={code,signal});let stdout='';
 app.stdout.on('data',b=>{stdout+=b;fs.appendFileSync(O+'/app.stdout.log',b)});app.stderr.on('data',b=>fs.appendFileSync(O+'/app.stderr.log',b));
 let api;for(let i=0;i<200;i++){api=/Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)/.exec(stdout)?.[1];if(api)break;assert(!exit);await new Promise(r=>setTimeout(r,50))}assert(api);
 const require=createRequire('C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/outputs/open-seo-spider/renderer/node_modules/playwright-core/package.json'),pw=require('playwright-core');
 browser=await pw.chromium.launch({executablePath:R.chrome_executable,headless:true,ignoreDefaultArgs:['--hide-scrollbars'],args:['--disable-background-networking','--no-first-run']});

 const {browserWorkflow}=await import(pathToFileURL(path.resolve('work/ngram-ui-diagnostic-helper.mjs')));proof.diagnostic_helper_sha256=H(fs.readFileSync('work/ngram-ui-diagnostic-helper.mjs'));const list=await(await fetch(api+'/api/crawls')).json();let job;for(const row of list.crawls){if(row.status!=='completed')continue;const project=await(await fetch(api+'/api/crawls/'+row.id+'/project.json')).json();if(project.pages.length===206){job=row;break}}assert(job);proof.selected_job=job.id;
 const base={source_kind:'raw_http_html',options:{max_n:6},view:'occurrences',limit:200,n:6,exact_tokens:['a','b','c','d','e','f'],expected_generation:job.generation};const post=async q=>{const r=await fetch(api+'/api/crawls/'+job.id+'/ngrams/preview',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(q)});assert.equal(r.status,200);return await r.json()};const first=await post(base);assert.equal(first.total_rows,205);assert.equal(first.rows.length,200);const tail=await post({...base,after:200,expected_snapshot_token:first.snapshot.snapshot_token});assert.equal(tail.rows.length,5);const occurrence={first,rows:[...first.rows,...tail.rows]},phases=[];
 proof.ngram_ui=await browserWorkflow({browser,api,job,output:O,occurrence,unchanged:label=>phases.push(label)});proof.layouts=globalThis.__ngramDiagnosticLayouts;assert.equal(proof.layouts.length,10);assert(proof.layouts.every(x=>x.doc>0&&x.doc<=x.width+1&&x.body>0&&x.body<=x.width+1&&x.bad.length===0));assert(proof.layouts.every(x=>x.doc<=x.client+1&&x.body<=x.client+1),'Actual horizontal overflow against content viewport');assert(proof.layouts.some(x=>x.width>x.client),'Classic vertical scrollbar expected');proof.phases=phases;proof.result='PASS_ACTUAL_CLASSIC_SCROLLBAR_SOURCE10_NGRAM_UI_1440_AND_320_DIAGNOSTIC';proof.qualification='Classic visible scrollbar configuration, including doc/body against document.clientWidth. Compiled actual Source10 on copied genuine206-page synthetic saved crawl. Full Ngram UI pagination100+100+5,205-row CSV,5-row JSON,unknown DOM,all actual control bounds at1440/320. Separate exported helper copy changes only width oracle and adds layout metrics. No active original fixture/cold/registry/whole native or installation attestation.';binding.verify();

}catch(e){proof.result='FAIL_DIAGNOSTIC';proof.failure={message:e.message,stack:e.stack};process.exitCode=1}
finally{if(browser){await browser.close();proof.browser_close_awaited=true}if(app){app.stdin.end();await Promise.race([closed,new Promise((_,reject)=>setTimeout(()=>reject(Error('Owned diagnostic App close')),15000))]);proof.app_actual_close=exit}binding.verify();assert(fs.readFileSync(lock).equals(lockRaw));fs.unlinkSync(lock);fs.writeFileSync(O+'/proof.json',JSON.stringify(proof,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:proof.result,proof_sha256:H(fs.readFileSync(O+'/proof.json')),layouts:proof.layouts,app_exit:exit,failure:proof.failure}));}
