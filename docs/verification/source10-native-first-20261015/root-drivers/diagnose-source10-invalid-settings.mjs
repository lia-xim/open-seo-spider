import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {createRequire} from 'node:module';
import {createHash,randomBytes} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013', O=E+'/results/source10-invalid-settings-diagnostic-v1', lock=E+'/results/combined-native-outer-v4/active.lock';
const H=b=>createHash('sha256').update(b).digest('hex'),R=JSON.parse(fs.readFileSync('work/v10-browser-native-release.json'));
assert.equal(H(fs.readFileSync('work/v10-browser-native-release.json')),'d185f40cc5fed97a0b01a77b68b6fde1abd6f897012fa1f8cc8a7e16ec7e5beb');
assert(!fs.existsSync(O)&&!fs.existsSync(lock));
const binderPath=E+'/native-combined-rebind-v10-v1/combined-bindings.mjs';
assert.equal(H(fs.readFileSync(binderPath)),R.bindings_sha256);
const binding=(await import(pathToFileURL(binderPath))).bindCombined(R,R.source_manifest_sha256);binding.verify();
const lockRaw=Buffer.from(JSON.stringify({gate:'source10-invalid-settings-diagnostic-v1',owner_pid:process.pid,token:randomBytes(32).toString('hex'),source_manifest_sha256:R.source_manifest_sha256})+'\n');fs.writeFileSync(lock,lockRaw,{flag:'wx'});
let app,browser,closed,exit;const proof={kind:'source10_invalid_settings_diagnostic',recorded_at:new Date().toISOString(),result:'RUNNING',source_manifest_sha256:R.source_manifest_sha256,candidate_binary_sha256:R.candidate_binary_sha256,qualification:'Compiled actual Source10; synthetic failed-browser data copy; settings-only diagnostic, not native acceptance or installation.'};
try{
 fs.mkdirSync(O);fs.cpSync(E+'/results/combined-findings-browser-native-optimized-v10/data',O+'/data',{recursive:true,errorOnExist:true,force:false});
 app=spawn(binding.binary,['serve','--listen','127.0.0.1:0','--data-dir',O+'/data','--shutdown-on-stdin-eof'],{windowsHide:true,stdio:['pipe','pipe','pipe']});closed=once(app,'close').then(([code,signal])=>exit={code,signal});let stdout='';
 app.stdout.on('data',b=>{stdout+=b;fs.appendFileSync(O+'/app.stdout.log',b)});app.stderr.on('data',b=>fs.appendFileSync(O+'/app.stderr.log',b));
 let api;for(let i=0;i<200;i++){api=/Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)/.exec(stdout)?.[1];if(api)break;assert(!exit);await new Promise(r=>setTimeout(r,50))}assert(api);
 const require=createRequire('C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https/outputs/open-seo-spider/renderer/node_modules/playwright-core/package.json'),pw=require('playwright-core');
 browser=await pw.chromium.launch({executablePath:R.chrome_executable,headless:true,args:['--disable-background-networking','--no-first-run']});
 const context=await browser.newContext({viewport:{width:1440,height:1000}}),page=await context.newPage(),external=[],posts=[],errors=[];
 page.on('pageerror',e=>errors.push(String(e)));await page.route('**/*',r=>{const u=new URL(r.request().url());if(u.origin!==api){external.push(r.request().url());return r.abort()}if(r.request().method()==='POST')posts.push(r.request().url());return r.continue()});
 await page.goto(api+'/',{waitUntil:'networkidle'});await page.locator('#connection-state.connected').waitFor();await page.locator('#settings-open').click();for(const id of['advanced-config','sitemap-source-archive-settings']){const item=page.locator('#'+id);if(await item.getAttribute('open')===null)await item.locator(':scope > summary').click()}
 const input=n=>page.locator('#settings-form [name="'+n+'"]');await input('sitemap_source_archive_enabled').check();await input('sitemap_source_archive_post_http').uncheck();await input('sitemap_source_archive_decoded_xml').uncheck();await page.locator('#settings-form .dialog-footer button[type="submit"]').click();await page.locator('#settings-error').waitFor();
 proof.layouts=[];for(const width of[1440,320]){await page.setViewportSize({width,height:1000});await page.locator('#settings-error').scrollIntoViewIfNeeded();const layout=await page.locator('#settings-dialog').evaluate(root=>{const box=e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e);return {tag:e.tagName,id:e.id,cls:e.className,text:(e.textContent||'').slice(0,180),client:e.clientWidth,scroll:e.scrollWidth,x:r.x,width:r.width,right:r.right,display:s.display,whiteSpace:s.whiteSpace,minWidth:s.minWidth,overflowWrap:s.overflowWrap,wordBreak:s.wordBreak,gridTemplateColumns:s.gridTemplateColumns}};return {viewport:innerWidth,document_scroll:document.documentElement.scrollWidth,root:box(root),overflowing:[...root.querySelectorAll('*')].filter(e=>e.clientWidth>0&&e.scrollWidth>e.clientWidth+1).map(box)}});proof.layouts.push(layout);await page.screenshot({path:O+'/invalid-'+width+'.png',fullPage:true})}
 assert(proof.layouts[1].root.scroll>proof.layouts[1].root.client+1,'Expected real Source10 overflow');assert.deepEqual(external,[]);assert.deepEqual(posts,[]);assert.deepEqual(errors,[]);proof.result='REPRODUCED_INVALID_SETTINGS_320_OVERFLOW';await context.close();binding.verify();
}catch(e){proof.result='FAIL_DIAGNOSTIC';proof.failure={message:e.message,stack:e.stack};process.exitCode=1}
finally{if(browser){await browser.close();proof.browser_close_awaited=true}if(app){app.stdin.end();await Promise.race([closed,new Promise((_,reject)=>setTimeout(()=>reject(Error('Owned diagnostic App close')),15000))]);proof.app_actual_close=exit}binding.verify();assert(fs.readFileSync(lock).equals(lockRaw));fs.unlinkSync(lock);fs.writeFileSync(O+'/proof.json',JSON.stringify(proof,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:proof.result,proof_sha256:H(fs.readFileSync(O+'/proof.json')),layouts:proof.layouts,app_exit:exit,failure:proof.failure}));}
