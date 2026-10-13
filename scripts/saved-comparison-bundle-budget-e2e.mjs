// Actual native CLI ceiling: complete 64 MiB envelope fits; +1 refuses both files.
// Synthetic typed reports exercise serialization limits, not crawl authenticity.
import assert from 'node:assert/strict';
import http from 'node:http';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createReadStream} from 'node:fs';
import {mkdir, mkdtemp, readFile, writeFile, stat, rm} from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const binary = process.env.SPIDER_BINARY;
assert(binary && path.isAbsolute(binary), 'Explicit absolute SPIDER_BINARY required');
const ceiling = 64 * 1024 * 1024;
const binarySha = createHash('sha256').update(await readFile(binary)).digest('hex');
const harnessSha = createHash('sha256').update(await readFile(fileURLToPath(import.meta.url))).digest('hex');
const binaryChecks = [], checks = [];
let websiteAttempts = 0;
const witness = http.createServer((req,res) => {websiteAttempts++; res.writeHead(410); res.end('owned offline witness');});
await new Promise((resolve,reject) => {witness.once('error',reject); witness.listen(0,'127.0.0.1',resolve);});
const origin = `http://127.0.0.1:${witness.address().port}`;
const temp = await mkdtemp(path.join(os.tmpdir(),'open-seo-spider-native-cli-budget-'));
const beforePath = path.join(temp,'before.json'), afterPath = path.join(temp,'after.json');
const alive = new Set();
async function binaryCheck(label) {
  const sha = createHash('sha256').update(await readFile(binary)).digest('hex');
  assert.equal(sha,binarySha,label); binaryChecks.push({label,sha256:sha});
}
async function cli(label,args) {
  await binaryCheck(label);
  const child = spawn(binary,['compare-saved',beforePath,afterPath,...args],{windowsHide:true,stdio:['ignore','pipe','pipe']});
  alive.add(child);
  let stdout='', stderr='';
  child.stdout.on('data',chunk => {stdout+=chunk; assert(stdout.length < 2*1024*1024,'Unexpected unbounded stdout');});
  child.stderr.on('data',chunk => {stderr+=chunk; assert(stderr.length < 2*1024*1024,'Unexpected unbounded stderr');});
  const result = await new Promise((resolve,reject) => {
    const timer=setTimeout(() => {child.kill(); reject(Error('Native budget control timed out: '+label));},120000);
    child.once('error',error => {clearTimeout(timer); reject(error);});
    child.once('close',(code,signal) => {clearTimeout(timer); alive.delete(child); resolve({code,signal,stdout,stderr});});
  });
  return result;
}
async function fileSha(file) {const hash=createHash('sha256'); for await (const chunk of createReadStream(file)) hash.update(chunk); return hash.digest('hex');}
async function missing(file) {try {await stat(file); return false;} catch(error) {if(error.code==='ENOENT') return true; throw error;}}
const report = () => ({config:{seed_urls:[origin+'/']},pages:[],issues:[],summary:{started_at:'',finished_at:'',elapsed_ms:0,completed:0,discovered:0,pending:0,html_pages:0,indexable_pages:0,errors:0,warnings:0,cancelled:false,limited:false,status_counts:{}},warnings:[''],operation:null,policy_ledger:null,resource_ledger:null});
let proof;
try {
  const before=report(), after=report();
  await writeFile(beforePath,JSON.stringify(before)); await writeFile(afterPath,JSON.stringify(after));
  const baselinePath=path.join(temp,'small.json');
  const baseline=await cli('calibrate-small-complete-envelope',['--include-original-reports','--output',baselinePath]);
  assert.equal(baseline.code,0,baseline.stderr);
  const baselineBytes=await readFile(baselinePath), small=JSON.parse(baselineBytes);
  const beforeBytes=small.original_reports.before.typed_report_json_bytes;
  const afterBytes=small.original_reports.after.typed_report_json_bytes;
  assert.deepEqual(small.original_reports.before.report.config.seed_urls,[origin+'/']);
  assert.deepEqual(small.original_reports.after.report.config.seed_urls,[origin+'/']);
  assert.equal(small.website_requests,0); assert.equal(small.comparison.website_requests,0);
  checks.push({id:'native-small-envelope-calibration',bytes:baselineBytes.length,before_report_bytes:beforeBytes,after_report_bytes:afterBytes});
  const digits=x=>String(x).length;
  const beforePadding=Math.floor((ceiling-baselineBytes.length)/2);
  let afterPadding=ceiling-baselineBytes.length-beforePadding;
  const predicted=()=>baselineBytes.length+beforePadding+afterPadding+2*(digits(beforeBytes+beforePadding)-digits(beforeBytes))+2*(digits(afterBytes+afterPadding)-digits(afterBytes));
  for(let i=0;i<8 && predicted()!==ceiling;i++) afterPadding+=ceiling-predicted();
  assert.equal(predicted(),ceiling,'Could not derive exact serialization boundary');
  before.warnings=['x'.repeat(beforePadding)]; after.warnings=['y'.repeat(afterPadding)];
  await writeFile(beforePath,JSON.stringify(before)); await writeFile(afterPath,JSON.stringify(after));
  const fitPath=path.join(temp,'exact-fit.json'), fitCsv=path.join(temp,'exact-fit.csv');
  const fit=await cli('exact-64MiB-envelope',['--include-original-reports','--output',fitPath,'--csv',fitCsv]);
  assert.equal(fit.code,0,fit.stderr); assert.equal((await stat(fitPath)).size,ceiling);
  const fitSha=await fileSha(fitPath), csvSha=await fileSha(fitCsv);
  checks.push({id:'actual-fixed-ceiling-exact-fit',bytes:ceiling,sha256:fitSha,csv_sha256:csvSha,before_padding:beforePadding,after_padding:afterPadding});
  after.warnings[0]+='z'; await writeFile(afterPath,JSON.stringify(after));
  const rejectedJson=path.join(temp,'plus-one.json'), rejectedCsv=path.join(temp,'plus-one.csv');
  const rejected=await cli('64MiB-plus-one-envelope',['--include-original-reports','--output',rejectedJson,'--csv',rejectedCsv]);
  assert.equal(rejected.code,2); assert.match(rejected.stdout+rejected.stderr,/original_report_bundle_byte_budget/);
  assert(await missing(rejectedJson)); assert(await missing(rejectedCsv));
  assert.equal(await fileSha(fitPath),fitSha); assert.equal(await fileSha(fitCsv),csvSha);
  checks.push({id:'actual-fixed-ceiling-plus-one-refuses-both-reservations',would_be_bytes:ceiling+1,error_code:'original_report_bundle_byte_budget',existing_fit_bytes_unchanged:true});
  const defaultPath=path.join(temp,'projection.json');
  const defaultResult=await cli('default-projection-from-same-large-inputs',['--output',defaultPath]);
  assert.equal(defaultResult.code,0,defaultResult.stderr);
  const projection=JSON.parse(await readFile(defaultPath,'utf8'));
  assert(!Object.hasOwn(projection,'original_reports')); assert.equal(projection.website_requests,0);
  checks.push({id:'default-remains-explicit-projection',bytes:(await stat(defaultPath)).size});
  assert.equal(alive.size,0); await binaryCheck('final-all-native-children-closed');
  assert.equal(createHash('sha256').update(await readFile(fileURLToPath(import.meta.url))).digest('hex'),harnessSha,'Harness changed during native controls');
  await new Promise((resolve,reject)=>witness.close(error=>error?reject(error):resolve()));
  assert.equal(websiteAttempts,0);
  proof={tested_at:new Date().toISOString(),result:'PASS',scope:'Actual native CLI fixed-ceiling control on synthetic typed reports; no crawl authenticity or API/MCP fixed-ceiling claim',binary_sha256:binarySha,harness_sha256:harnessSha,ceiling_bytes:ceiling,checks,binary_checks:binaryChecks,final_active_children:alive.size,website_attempts_after_children_and_witness_drained:websiteAttempts};
} finally {
  for(const child of alive) {child.kill(); await new Promise(resolve=>child.once('close',resolve));}
  if(witness.listening) await new Promise(resolve=>witness.close(resolve));
  const resolved=path.resolve(temp); assert(resolved.startsWith(path.resolve(os.tmpdir())+path.sep) && path.basename(resolved).startsWith('open-seo-spider-native-cli-budget-'),'Temp cleanup outside owned fixture path');
  await rm(resolved,{recursive:true,force:true});
}
assert(proof,'No accepted final proof');
const proofPath=process.env.SPIDER_BUNDLE_BUDGET_PROOF || path.join(here,'../docs/verification/saved-comparison-original-bundle-cli-budget.json');
await mkdir(path.dirname(proofPath),{recursive:true}); await writeFile(proofPath,JSON.stringify(proof,null,2)+'\n');
console.log(JSON.stringify({result:'PASS',native_groups:checks.length,binary_sha256:binarySha,proof:proofPath}));
