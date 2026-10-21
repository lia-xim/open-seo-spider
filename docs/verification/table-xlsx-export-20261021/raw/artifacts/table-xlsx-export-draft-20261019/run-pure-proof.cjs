'use strict';
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),cp=require('node:child_process'),assert=require('node:assert/strict');
const label=process.argv[2]||'r1';assert(/^r[1-9][0-9]*$/.test(label));
const base=__dirname,node='C:/Users/matth/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node.exe';
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const pin=name=>{const b=fs.readFileSync(path.join(base,name));return{path:name,bytes:b.length,sha256:sha(b)};};
const inputs=['table-xlsx.cjs','table-xlsx.test.cjs','inspect_xlsx.py','run-pure-proof.cjs'].map(pin),started=new Date().toISOString(),start=Date.now();
const result=cp.spawnSync(node,['--test',path.join(base,'table-xlsx.test.cjs')],{encoding:null,timeout:60000,maxBuffer:4*1024*1024});
for(const input of inputs)assert.deepEqual(pin(input.path),input);
for(const[tail,b]of[['stdout.log',result.stdout],['stderr.log',result.stderr]])fs.writeFileSync(path.join(base,label+'.'+tail),b,{flag:'wx'});
const log=result.stdout.toString(),counts={};for(const key of['tests','pass','fail','cancelled','skipped','todo']){const m=log.match(new RegExp('(?:^|\\n)(?:#|ℹ) '+key+' (\\d+)(?:\\r?\\n|$)','u'));counts[key]=m?Number(m[1]):null;}
const files=fs.existsSync(path.join(base,'actual-files'))?fs.readdirSync(path.join(base,'actual-files')).sort().map(name=>pin('actual-files/'+name)):[];
const receipt={kind:'ignored_XLSX_format_core_pure_proof',result:result.status===0?'PASS_PURE_FORMAT_CONTROLS':'FAIL',started_at:started,duration_ms:Date.now()-start,node,command:['--test','table-xlsx.test.cjs'],code:result.status,signal:result.signal,counts,inputs,stdout:{path:label+'.stdout.log',bytes:result.stdout.length,sha256:sha(result.stdout)},stderr:{path:label+'.stderr.log',bytes:result.stderr.length,sha256:sha(result.stderr)},actual_files:files,scope:'Pure projection/format tests with actual ZIP/XML and installed openpyxl reads. No App/DOM/native/browser/Cargo/Git, no Excel application or production acceptance.'};
fs.writeFileSync(path.join(base,label+'.proof.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({result:receipt.result,code:result.status,counts,receipt:label+'.proof.json'},null,2));
if(result.status!==0){console.log(log);process.exitCode=1;}
