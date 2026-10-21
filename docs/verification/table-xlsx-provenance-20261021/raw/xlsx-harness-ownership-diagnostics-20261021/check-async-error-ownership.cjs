'use strict';
// Declared PURE FAILURE controls. No native process, browser, transport mock,
// success event, App state or product writer is involved in this preparation.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),{spawnSync}=require('node:child_process'),{createHash}=require('node:crypto');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const provenance=fs.readFileSync(path.join(__dirname,'ui-table-xlsx-provenance-live.cjs'),'utf8'),availability=fs.readFileSync(path.join(__dirname,'ui-table-xlsx-availability-live.cjs'),'utf8');
function extract(source){const start=source.indexOf('async function actualDownload('),end=source.indexOf('\n}',start)+2;assert(start>0&&end>start);return source.slice(start,end);}
const helper=extract(provenance);assert.equal(extract(availability),helper,'The exact same helper owns both download gates');
function failureControl(kind){
  const unsafe=kind==='original-unowned',earlyClick=kind==='click-first',disabled=kind==='disabled';
  const script=[
    "'use strict'; const assert=require('node:assert/strict');",
    "const log=value=>process.stdout.write(value+'\\n'); let arms=0,clicks=0;",
    "const rejectAfter=(delay,message)=>new Promise((_,reject)=>setTimeout(()=>{log(message.toUpperCase());reject(Error(message));},delay));",
    'const event=()=>{arms++;return rejectAfter('+(earlyClick?25:10)+",'event-timeout');};",
    'const control={isVisible:async()=>true,isEnabled:async()=>'+(!disabled)+',click:()=>{clicks++;return rejectAfter('+(earlyClick?10:25)+",'click-timeout');}};",
    "const page={locator:()=>control,waitForEvent:()=>event()};",
    'const actualDownload=eval('+JSON.stringify('('+helper+')')+');',
    '(async()=>{try{log("START");'+(unsafe?'const pending=event();await control.click();await pending;':'await actualDownload("#export-csv","declared negative only");')+'throw Error("Unexpected success");}finally{log("OUTER_FINALLY");}})().catch(error=>{log("OWNED_FAILURE:"+error.message);log("ARMS:"+arms+" CLICKS:"+clicks);process.exitCode=2;});'
  ].join('\n');
  const run=spawnSync(process.execPath,['--unhandled-rejections=strict','-e',script],{encoding:'utf8',timeout:5000,windowsHide:true});assert(!run.error);assert.equal(run.signal,null);
  if(unsafe){assert.equal(run.status,1);assert(!run.stdout.includes('OUTER_FINALLY'));assert(!run.stdout.includes('OWNED_FAILURE'));assert(run.stderr.includes('event-timeout'));}
  else{assert.equal(run.status,2,'The negative event/click/assertion remains a failed operation, never a false PASS');assert(run.stdout.includes('OUTER_FINALLY'));assert(run.stdout.includes('OWNED_FAILURE:'));assert(!run.stderr.includes('triggerUncaughtException'));if(disabled)assert(run.stdout.includes('ARMS:0 CLICKS:0'));else assert(run.stdout.includes('ARMS:1 CLICKS:1'));}
  return{kind,status:run.status,signal:run.signal,stdout:run.stdout,stderr:run.stderr};
}
const controls=['original-unowned','event-first','click-first','disabled'].map(failureControl);
const proof={result:'PASS_PURE_ERROR_OWNERSHIP_NEGATIVE_CONTROLS',node:process.version,helper_sha256:sha(Buffer.from(helper)),controls,scope:'Exact ignored candidate helper evaluated only with declared rejecting promises/disabled control. Original unowned promise reproduces abrupt exit before outer finally; corrected helper propagates every failure through finally with nonzero operation exit. No success event/HTTP/native/browser proof claimed.'};
fs.writeFileSync(path.join(__dirname,'async-error-ownership-proof.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify({result:proof.result,node:proof.node,helper_sha256:proof.helper_sha256,controls:controls.map(({kind,status,signal})=>({kind,status,signal}))},null,2));
