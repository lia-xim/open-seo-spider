import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {createRequire} from 'node:module';
import {randomBytes} from 'node:crypto';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const base=path.join(E,'results/combined-findings-browser-native-optimized-v8');
const out=path.join(E,'results/source9-composed-header-browser-preview-v1');
const lock=path.join(E,'results/combined-native-outer-v4/active.lock');
assert(!fs.existsSync(out));assert(!fs.existsSync(lock));
const token=randomBytes(32).toString('hex');
const lockRaw=Buffer.from(JSON.stringify({gate:'source8-overflow-diagnostic',owner_pid:process.pid,token})+'\n');
fs.writeFileSync(lock,lockRaw,{flag:'wx'});
let app,browser,actualClose,appExit;
const proof={kind:'source8_findings_overflow_diagnostic',result:'RUNNING',observations:[],qualification:'Static HTML/CSS override from composed Source9 draft against an owned synthetic Source8 API copy. Browser header preview only, not compiled-product native acceptance or installation.'};
try{
 fs.mkdirSync(out);fs.cpSync(path.join(base,'data'),path.join(out,'data'),{recursive:true,errorOnExist:true,force:false});
 const b=JSON.parse(fs.readFileSync(path.join(E,'results/combined-optimized-build-v8/proof.json')));
 app=spawn(b.binary.path,['serve','--listen','127.0.0.1:0','--data-dir',path.join(out,'data'),'--shutdown-on-stdin-eof'],{stdio:['pipe','pipe','pipe'],windowsHide:true});
 actualClose=once(app,'close').then(([code,signal])=>{appExit={code,signal};return appExit});let stdout='';app.stdout.on('data',b=>{stdout+=b;fs.appendFileSync(path.join(out,'app.stdout.log'),b)});app.stderr.on('data',b=>fs.appendFileSync(path.join(out,'app.stderr.log'),b));
 let api;for(let i=0;i<200;i++){const m=stdout.match(/Open SEO Spider: (http:\/\/127\.0\.0\.1:\d+)/);if(m){api=m[1];break}if(appExit)throw Error('App closed '+JSON.stringify(appExit));await new Promise(r=>setTimeout(r,50))}assert(api,'App ready');
 const require=createRequire(path.join(process.cwd(),'outputs/open-seo-spider/renderer/node_modules/playwright-core/package.json'));
 const pw=require('playwright-core');browser=await pw.chromium.launch({executablePath:'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true,args:['--disable-background-networking','--no-first-run']});
 const page=await browser.newPage({viewport:{width:1440,height:900}});await page.route('**/*',r=>{const u=new URL(r.request().url());if(u.origin!==api)return r.abort();if(r.request().method()==='GET'&&['/','/styles.css'].includes(u.pathname)){const f=u.pathname==='/'?'index.html':'styles.css';return r.fulfill({status:200,contentType:f.endsWith('html')?'text/html; charset=utf-8':'text/css; charset=utf-8',body:fs.readFileSync('E:/CodexWork/open-seo-spider-01a12ac3-20261013/combined-source-v9-draft-v3/source/web/'+f)});}return r.continue();});
 await page.goto(api+'/',{waitUntil:'networkidle'});const job=JSON.parse(fs.readFileSync(path.join(base,'completed-job.json')));await page.selectOption('#crawl-history',job.id);await page.locator('[data-section="sitemap-findings"]').click();await page.waitForFunction(()=>document.querySelector('#sitemap-findings-view').getAttribute('aria-busy')==='false');
 await page.selectOption('#sitemap-findings-filter','urls-in-sitemap');await page.selectOption('#sitemap-findings-limit','1');await page.selectOption('#sitemap-findings-candidates','5000');await page.locator('#sitemap-findings-form button[type="submit"]').click();await page.waitForFunction(()=>document.querySelector('#sitemap-findings-view').getAttribute('aria-busy')==='false');
 for(const width of [320,560,561,600,768,850,1440]){await page.setViewportSize({width,height:900});await page.locator('#sitemap-findings-export').scrollIntoViewIfNeeded();const d=await page.evaluate(()=>({width:innerWidth,client:document.documentElement.clientWidth,document:document.documentElement.scrollWidth,body:document.body.scrollWidth,elements:[...document.querySelectorAll('body *')].filter(el=>{const r=el.getBoundingClientRect(),s=getComputedStyle(el);return r.width>0&&r.height>0&&s.visibility!=='hidden'&&(r.right>innerWidth+1||r.left< -1)}).map(el=>{const r=el.getBoundingClientRect(),s=getComputedStyle(el);return{tag:el.tagName,id:el.id,class:el.className,left:r.left,right:r.right,width:r.width,client:el.clientWidth,scroll:el.scrollWidth,overflow:s.overflow,minWidth:s.minWidth,text:el.textContent.slice(0,180)}})}));assert(d.document<=width+1&&d.body<=width+1,'Composed header must fit viewport '+width);proof.observations.push(d);await page.screenshot({path:path.join(out,'diagnostic-'+width+'.png'),fullPage:true});}
 proof.result='DIAGNOSTIC_COMPLETE';
}catch(e){proof.result='DIAGNOSTIC_FAILED';proof.error={message:e.message,stack:e.stack};process.exitCode=1}
finally{if(browser)await browser.close();if(app){app.stdin.end();await Promise.race([actualClose,new Promise((_,reject)=>setTimeout(()=>reject(Error('Owned diagnostic App close pending')),15000))]);proof.app_actual_close=appExit}if(fs.readFileSync(lock).equals(lockRaw))fs.unlinkSync(lock);else throw Error('Foreign lock changed');fs.writeFileSync(path.join(out,'proof.json'),JSON.stringify(proof,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:proof.result,out,observations:proof.observations.map(({width,document,body})=>({width,document,body}))}));}
