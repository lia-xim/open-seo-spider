import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {createRequire} from 'node:module';
import {createHash,randomBytes} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013', O=E+'/results/source10-known-length-registry-diagnostic-v1', lock=E+'/results/combined-native-outer-v4/active.lock';
const H=b=>createHash('sha256').update(b).digest('hex'),R=JSON.parse(fs.readFileSync('work/v10-browser-native-release.json'));
assert.equal(H(fs.readFileSync('work/v10-browser-native-release.json')),'d185f40cc5fed97a0b01a77b68b6fde1abd6f897012fa1f8cc8a7e16ec7e5beb');
assert(!fs.existsSync(O)&&!fs.existsSync(lock));
const binderPath=E+'/native-combined-rebind-v10-v1/combined-bindings.mjs';
assert.equal(H(fs.readFileSync(binderPath)),R.bindings_sha256);
const binding=(await import(pathToFileURL(binderPath))).bindCombined(R,R.source_manifest_sha256);binding.verify();
const lockRaw=Buffer.from(JSON.stringify({gate:'source10-known-length-registry-diagnostic-v1',owner_pid:process.pid,token:randomBytes(32).toString('hex'),source_manifest_sha256:R.source_manifest_sha256})+'\n');fs.writeFileSync(lock,lockRaw,{flag:'wx'});
let app,browser,closed,exit;const proof={kind:'source10_known_length_registry_diagnostic',recorded_at:new Date().toISOString(),result:'RUNNING',source_manifest_sha256:R.source_manifest_sha256,candidate_binary_sha256:R.candidate_binary_sha256,qualification:'Compiled actual Source10; synthetic failed-browser data copy; settings-only diagnostic, not native acceptance or installation.'};
try{
 fs.mkdirSync(O);fs.cpSync(E+'/results/resource-directory-workflow-native-v10-r3/data',O+'/data',{recursive:true,errorOnExist:true,force:false});
 app=spawn(binding.binary,['serve','--listen','127.0.0.1:0','--data-dir',O+'/data','--shutdown-on-stdin-eof'],{windowsHide:true,stdio:['pipe','pipe','pipe']});closed=once(app,'close').then(([code,signal])=>exit={code,signal});let stdout='';
 app.stdout.on('data',b=>{stdout+=b;fs.appendFileSync(O+'/app.stdout.log',b)});app.stderr.on('data',b=>fs.appendFileSync(O+'/app.stderr.log',b));
 let api;for(let i=0;i<200;i++){api=/Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)/.exec(stdout)?.[1];if(api)break;assert(!exit);await new Promise(r=>setTimeout(r,50))}assert(api);

 const list=await(await fetch(api+'/api/crawls')).json();let job;for(const row of list.crawls){if(row.status!=='completed')continue;const project=await(await fetch(api+'/api/crawls/'+row.id+'/project.json')).json();if(project.pages.length===206){job=row;break}}assert(job);proof.selected_job=job.id;
 const request={source_kind:'raw_http_html',options:{max_n:6},view:'occurrences',limit:200,n:6,exact_tokens:['a','b','c','d','e','f'],expected_generation:job.generation},previewResponse=await fetch(api+'/api/crawls/'+job.id+'/ngrams/preview',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(request)});assert.equal(previewResponse.status,200);const preview=await previewResponse.json();assert.equal(preview.total_rows,205);const response=await fetch(api+'/api/crawls/'+job.id+'/ngrams/csv',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({...request,expected_snapshot_token:preview.snapshot.snapshot_token})});assert.equal(response.status,200);const bytes=Buffer.from(await response.arrayBuffer()),length=Number(response.headers.get('content-length')),hash=response.headers.get('x-content-sha256');assert(length>65536);assert.equal(bytes.length,length);assert.equal(H(bytes),hash);assert.equal(bytes.toString().split(/\r?\n/).filter(Boolean).length,206);proof.client={status:200,content_length:length,actual_bytes:bytes.length,expected_sha256:hash,actual_sha256:H(bytes),rows:205};proof.result='REPRODUCED_FULL_EXACT_LENGTH_CSV_BEFORE_REGISTRY_SHUTDOWN';proof.qualification='One real HTTP Content-Length CSV over actual compiled Source10, copied206-page synthetic saved crawl. Full advertised bytes/SHA205rows consumed before own stdin EOF. Shutdown classification recorded separately. No complete native or installation attestation.';binding.verify();

}catch(e){proof.result='FAIL_DIAGNOSTIC';proof.failure={message:e.message,stack:e.stack};process.exitCode=1}
finally{if(browser){await browser.close();proof.browser_close_awaited=true}if(app){app.stdin.end();await Promise.race([closed,new Promise((_,reject)=>setTimeout(()=>reject(Error('Owned diagnostic App close')),15000))]);proof.app_actual_close=exit};const shutdowns=fs.readFileSync(O+'/app.stderr.log','utf8').trim().split(/\r?\n/).filter(Boolean).map(x=>{try{return JSON.parse(x)}catch{return null}}).filter(x=>x?.kind==='owned_application_shutdown_v1');assert.equal(shutdowns.length,1);proof.shutdown=shutdowns[0];assert.equal(proof.shutdown.response_streams.admitted,1);assert.equal(proof.shutdown.response_streams.joined,1);assert.equal(proof.shutdown.response_streams.complete,0);assert.equal(proof.shutdown.response_streams.aborted,1);assert.equal(proof.shutdown.response_streams.failed,0);assert.equal(proof.shutdown.response_streams.retained,0);proof.result='REPRODUCED_COMPLETE205_ROW_EXACT_LENGTH_CSV_CLASSIFIED_ABORTED';binding.verify();assert(fs.readFileSync(lock).equals(lockRaw));fs.unlinkSync(lock);fs.writeFileSync(O+'/proof.json',JSON.stringify(proof,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:proof.result,proof_sha256:H(fs.readFileSync(O+'/proof.json')),layouts:proof.layouts,app_exit:exit,failure:proof.failure}));}
