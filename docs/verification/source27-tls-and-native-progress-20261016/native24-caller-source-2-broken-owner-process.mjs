import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const CAP=4*1024*1024,MCP_CAP=256*1024,UUID=/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;
const digest=b=>createHash('sha256').update(b).digest('hex'),decode=b=>JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(b));
// Definitions only. This port accepts job IDs; only Root can resolve actual owned stores.
export function createBrokenSourceProcessCallbacks({R,runtime,origin,candidate,output,checkpoint,resolveStore,knownJob}){
 assert.equal(typeof resolveStore,'function');assert.equal(typeof knownJob,'function');
 const cliRecords=[],MCPRecords=[],pending=new Map();let owner=null,sequence=0,tail=Buffer.alloc(0),initialized=false,closing=false;
 const reference=p=>{runtime.ownedExisting(R.A,p);const st=fs.lstatSync(p);assert(st.isFile()&&!st.isSymbolicLink()&&st.nlink===1&&st.size<=CAP);const b=fs.readFileSync(p);assert.equal(b.length,st.size);return {path:p,sha256:digest(b),bytes:b.length};};
 const rejectPending=error=>{for(const p of pending.values()){clearTimeout(p.timer);p.reject(error);}pending.clear();};
 async function eofExpected(h,expected){
  assert([0,1].includes(expected));assert(!h.close,'Parent EOF must precede one-shot child exit');h.stdin.eof_sent=true;h.events.push({event:'stdin_eof',at:Date.now()});h.cp.stdin.end();await runtime.bound(h.closed,30000,'Actual broken-source CLI close '+h.label);R.natural(h,expected);assert.deepEqual(h.stdin,{eof_sent:true,finished:true,closed:true,error:null});const order=['stdin_eof','exit','close'].map(e=>h.events.findIndex(v=>v.event===e));assert(order[0]>=0&&order[1]>order[0]&&order[2]>order[1]);
 }
 async function resourceCLI({id,request,outputJSON,outputCSV,expectedCode}){
  checkpoint('ROOT_BEFORE_BROKEN_SOURCE_CLI');assert(!closing);assert(UUID.test(id)&&knownJob(id));assert([0,1].includes(expectedCode));assert(cliRecords.length<8);
  assert(request&&path.isAbsolute(request.path)&&Number.isSafeInteger(request.bytes)&&request.bytes>0&&request.bytes<=4096);const qref=reference(request.path);assert.deepEqual(qref,request);const q=decode(fs.readFileSync(request.path));assert(q&&typeof q==='object'&&!Array.isArray(q));const keys=Object.keys(q).sort();assert.deepEqual(keys,['after_sequence','expected_snapshot_token','filter','limit',...(Object.hasOwn(q,'reporting_mode')?['reporting_mode']:[])].sort());assert.equal(q.filter,'all');assert.equal(q.limit,2);assert.equal(q.after_sequence,null);assert([undefined,null,'raw','reported'].includes(q.reporting_mode));if(expectedCode===0)assert.equal(q.expected_snapshot_token,null);else{assert.equal(q.reporting_mode,'reported');assert.match(q.expected_snapshot_token,/^[a-f0-9]{64}$/);}
  const destinations=[outputJSON,outputCSV];assert.equal(new Set(destinations.map(p=>path.resolve(p).toLowerCase())).size,2);for(const [i,p]of destinations.entries()){assert.equal(path.dirname(path.resolve(p)),path.resolve(output));assert.equal(path.extname(p),i?'.csv':'.json');runtime.newOwnedPath(R.A,p);}assert.equal(outputJSON.slice(0,-5),outputCSV.slice(0,-4));
  const capability=resolveStore(id,'before_cli');assert.equal(capability.id,id);assert(knownJob(id));assert(path.isAbsolute(capability.path));assert(capability.evidence&&capability.physical_content_immutability_claimed===false);
  const args=['resource-broken-sources',capability.path,'--request',request.path,'--output',outputJSON,'--csv',outputCSV],record={label:'bs-cli-'+(cliRecords.length+1),args,expected_code:expectedCode,store_capability:capability,request,pid:null,settled:false};cliRecords.push(record);
  const h=R.launch(record.label,candidate.path,args);h.brokenSourceExpectedCode=expectedCode;record.pid=h.pid;await eofExpected(h,expectedCode);
  const after=resolveStore(id,'after_cli');assert.equal(after.path,capability.path);for(const k of ['dev','ino','birthtimeMs'])assert.equal(after.identity[k],capability.identity[k]);record.store_after=after;
  const stdoutPath=h.logs.find(l=>l.name==='stdout').path,stderrPath=h.logs.find(l=>l.name==='stderr').path,outputs=destinations.map(p=>{const o={path:p,exists:fs.existsSync(p)};if(o.exists){const file=reference(p);assert(file.bytes>0);o.raw=fs.readFileSync(p);o.file=file;}return o;}),result={exit:h.exit,close:h.close,stdout:fs.readFileSync(stdoutPath),stderr:fs.readFileSync(stderrPath),stdout_ref:reference(stdoutPath),stderr_ref:reference(stderrPath),outputs};
  assert.equal(result.stdout.length,0);if(expectedCode===0){assert.equal(result.stderr.length,0);assert(outputs.every(o=>o.exists));}else{const text=new TextDecoder('utf-8',{fatal:true}).decode(result.stderr);assert.match(text,/Stale resource/);assert(!/no such file|permission denied|access is denied|not found|cannot open/i.test(text));assert(outputs.every(o=>!o.exists));}
  Object.assign(record,{settled:true,exit:h.exit,close:h.close,stdout:result.stdout_ref,stderr:result.stderr_ref,outputs:outputs.map(o=>({path:o.path,exists:o.exists,file:o.file??null})),actual_stdin:h.stdin,events:h.events});return result;
 }
 async function rpc(method,params){
  assert(owner&&!owner.close&&!closing);const id=++sequence,request={jsonrpc:'2.0',id,method,params},bytes=Buffer.from(JSON.stringify(request)+'\n');assert(bytes.length<=8192);const request_ref=R.save('bs-mcp-'+id+'-request.json',bytes),record={request,request_ref,settled:false};MCPRecords.push(record);
  const got=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>{pending.delete(id);owner.errors.push('bs-MCP-reply-deadline:'+id);reject(Error('15s broken-source MCP reply deadline'));},15000);pending.set(id,{resolve,reject,timer});owner.cp.stdin.write(bytes,error=>{if(error){owner.errors.push('bs-MCP-stdin:'+error);const p=pending.get(id);if(p){clearTimeout(p.timer);pending.delete(id);p.reject(error);}}});});
  assert.equal(got.reply.jsonrpc,'2.0');assert.equal(got.reply.id,id);assert(!Object.hasOwn(got.reply,'error'));assert(got.reply.result&&typeof got.reply.result==='object');const raw_ref=R.save('bs-mcp-'+id+'-reply.json',got.raw);Object.assign(record,{settled:true,reply:got.reply,raw_ref});return {request,raw:got.raw,raw_ref,reply:got.reply};
 }
 async function start(){
  if(initialized)return;assert(!owner&&!closing);checkpoint('ROOT_BEFORE_BROKEN_SOURCE_MCP');owner=R.launch('mcp',candidate.path,['mcp','--server',origin]);
  owner.onStdout=chunk=>{try{tail=Buffer.concat([tail,chunk]);assert(tail.length<=MCP_CAP,'256KiB actual MCP frame bound');let at;while((at=tail.indexOf(10))!==-1){const raw=Buffer.from(tail.subarray(0,at+1));tail=tail.subarray(at+1);assert(raw.length>1&&raw.length<=MCP_CAP);const reply=decode(raw);assert.equal(reply.jsonrpc,'2.0');assert(Number.isSafeInteger(reply.id));const p=pending.get(reply.id);assert(p,'Unsolicited or duplicate MCP reply');clearTimeout(p.timer);pending.delete(reply.id);p.resolve({raw,reply});}}catch(error){owner.errors.push('bs-MCP-wire:'+error);rejectPending(error);}};
  owner.cp.once('close',()=>{if(pending.size)rejectPending(Error('MCP closed with pending actual replies'));});
  const init=await rpc('initialize',{protocolVersion:'2024-11-05',capabilities:{},clientInfo:{name:'Open SEO Spider broken-source native cases',version:'1'}});assert(init.reply.result);initialized=true;
 }
 async function mcp({method,name,args,expectedError}){
  assert.equal(typeof expectedError,'boolean');await start();if(method==='tools/list'){assert(name===undefined&&args===undefined&&!expectedError);return rpc('tools/list',{});}
  assert.equal(method,undefined);assert.equal(name,'spider_broken_sources');assert(args&&typeof args==='object'&&!Array.isArray(args));assert.deepEqual(Object.keys(args).sort(),['action','id','request','scope']);assert(UUID.test(args.id)&&knownJob(args.id));assert(['page_links','resources'].includes(args.scope));assert(['preview','csv'].includes(args.action));assert(args.request&&typeof args.request==='object'&&!Array.isArray(args.request));assert(Buffer.byteLength(JSON.stringify(args))<=4096);
  const result=await rpc('tools/call',{name,arguments:args});assert.equal(result.reply.result.isError,expectedError);return result;
 }
 async function close(){assert(!closing);closing=true;if(!owner)return;assert.equal(pending.size,0);await R.eof(owner);assert.equal(tail.length,0);assert.equal(cliRecords.length,8);assert(cliRecords.every(r=>r.settled));assert(MCPRecords.every(r=>r.settled));}
 return {resourceCLI,mcp,close,cliRecords,MCPRecords,pending,get owner(){return owner;}};
}