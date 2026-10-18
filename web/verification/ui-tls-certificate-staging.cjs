'use strict';
// AUTHORED_NOT_RUN: actual extracted staging functions under explicit DOM/File
// stubs. This is no browser, filesystem-picker, crypto or TLS acceptance proof.
const fs=require('node:fs'),crypto=require('node:crypto'),assert=require('node:assert/strict'),vm=require('node:vm');
const targets=[{"path":"web/app.js","bytes":436828,"sha256":"cdea5f64366ee0877f77ce71d67cb6195f0b7affcd889560d3b55af4706160f5"},{"path":"web/index.html","bytes":99978,"sha256":"3961c177f221d62ede690f7f62417fb0fc0d891fb7f5286265299d7758598844"}];assert.equal(process.argv.length,4,'Explicit current app.js and index.html paths required');
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const source=process.argv.slice(2).map((path,i)=>{const bytes=fs.readFileSync(path);assert.equal(bytes.length,targets[i].bytes);assert.equal(hash(bytes),targets[i].sha256);return bytes.toString('utf8').replaceAll('\r\n','\n');});
const [js,html]=source,regions=[];
function extract(name,async=false){const start=js.indexOf('  '+(async?'async ':'')+'function '+name+'(');assert(start>=0,name);const end=js.indexOf('\n  }',start)+4;assert(end>start);const text=js.slice(start,end);regions.push({name,sha256:hash(text),bytes:Buffer.byteLength(text)});return text;}
const functions=[extract('restoreTlsSettings'),extract('readTlsSettings'),extract('parseTlsSettings'),extract('stageTlsCertificateFiles',true)];
const revision='  let tlsFileRevision = 0;';assert.equal(js.split(revision).length-1,1);
regions.push({name:'tlsFileRevision',sha256:hash(revision),bytes:Buffer.byteLength(revision)});
const form={elements:{tls_enabled:{checked:true},tls_respect_hsts:{checked:false},tls_certificates_json:{value:'[]'}}};
const dialog={open:true},status={textContent:''},elements=new Map([['settings-form',form],['settings-dialog',dialog],['tls-file-status',status]]);
const $=id=>{assert(elements.has(id),id);return elements.get(id);};
const context=vm.createContext({$,TextEncoder,TextDecoder,Uint8Array,JSON,Array,Object,Error});
vm.runInContext(revision+'\n'+functions.join('\n')+'\nglobalThis.api={stageTlsCertificateFiles,parseTlsSettings,restoreTlsSettings,readTlsSettings};',context);
const api=context.api,done=[];
function reset(){dialog.open=true;form.elements.tls_certificates_json.value='[]';form.elements.tls_respect_hsts.checked=false;status.textContent='';}
// Deliberately a CERTIFICATE lexical fixture. X509/ClientBuilder acceptance is
// covered by the generated public CA in the Rust regression, not this DOM stub.
const pem='-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n';
const bytes=text=>Buffer.from(text,'utf8');
function file(name,raw,read=null){const b=Buffer.from(raw);return{name,size:b.length,arrayBuffer:read||(()=>Promise.resolve(Uint8Array.from(b).buffer))};}
async function stage(files){const input={files,value:'selected'};await api.stageTlsCertificateFiles({target:input});assert.equal(input.value,'');return JSON.parse(form.elements.tls_certificates_json.value);}
async function test(name,variants,run){await run();done.push({name,variants});}
async function main(){
 await test('UI-TLS-01 actual TLS controls and staging stay in the draft',1,async()=>{
  assert.match(html,/name="tls_enabled"/);assert.match(html,/name="tls_respect_hsts"/);assert.match(html,/name="tls_certificates_json"/);
  const input=html.match(/<input[^>]*id="tls-certificate-files"[^>]*>/);assert(input,'actual certificate picker');assert.match(input[0],/multiple/);assert.match(input[0],/\.crt/);assert.match(input[0],/\.pem/);assert.match(input[0],/\.cer/);assert.match(input[0],/\.der/);
  reset();const selected=await stage([file('public.pem',bytes(pem))]);assert.equal(selected[0].format,'pem');assert.match(status.textContent,/Erst Speichern/);
 });
 await test('UI-TLS-02 actual staging accepts exactly six ASCII prefix whitespace bytes and retains raw text',8,async()=>{
  for(const prefix of ['', '\n',' ','\t','\r','\v','\f',' \t\r\n\v\f']){reset();const raw=prefix+pem;const selected=await stage([file('public.pem',bytes(raw))]);assert.equal(selected[0].format,'pem');assert.equal(selected[0].name,'public.pem');assert.equal(selected[0].data,raw);}
 });
 await test('UI-TLS-03 prefixed private keys unknown objects and trailing preambles never replace a draft',3,async()=>{
  for(const raw of [' \n-----BEGIN PRIVATE KEY-----\nAA==\n-----END PRIVATE KEY-----','\t-----BEGIN PUBLIC KEY-----\nAA==\n-----END PUBLIC KEY-----','\n'+pem+'TRAILING']){reset();const prior='[{"prior":"untouched"}]';form.elements.tls_certificates_json.value=prior;await stage([file('reject.pem',bytes(raw))]);assert.equal(form.elements.tls_certificates_json.value,prior);assert.match(status.textContent,/PEM/);}
 });
 await test('UI-TLS-04 strict DER bytes stay hex and BOM Unicode or arbitrary preambles do not become PEM',5,async()=>{
  for(const raw of [Buffer.from([0x30,0x03,0x02,0x01,0]),bytes('\ufeff'+pem),bytes('\u00a0'+pem),bytes('arbitrary preamble\n'+pem),Buffer.from([0xff,0xfe,0xfd])]){reset();const selected=await stage([file('opaque.der',raw)]);assert.equal(selected[0].format,'der');assert.equal(selected[0].data,raw.toString('hex'));}
 });
 await test('UI-TLS-05 extension file count per-file and aggregate limits preserve the prior draft',4,async()=>{
  const bad=[[file('wrong.txt',bytes(pem))],[file('big.pem',Buffer.alloc(65537,1))],Array.from({length:9},(_,i)=>file('n'+i+'.der',Buffer.from([1]))),Array.from({length:5},(_,i)=>file('total'+i+'.der',Buffer.alloc(65536,1)))];
  for(const files of bad){reset();form.elements.tls_certificates_json.value='[{"prior":"untouched"}]';await stage(files);assert.equal(form.elements.tls_certificates_json.value,'[{"prior":"untouched"}]');assert.match(status.textContent,/Byte-Limit/);}
 });
 await test('UI-TLS-06 pending reads cannot overwrite an edited draft or a closed dialog',2,async()=>{
  for(const action of ['edit','close']){reset();let release;const pending=new Promise(resolve=>{release=resolve;});const input={files:[file('pending.pem',bytes(pem),()=>pending)],value:'selected'};const run=api.stageTlsCertificateFiles({target:input});if(action==='edit')form.elements.tls_certificates_json.value='[{"edited":true}]';else dialog.open=false;release(Uint8Array.from(bytes(pem)).buffer);await run;assert.equal(input.value,'');assert.equal(form.elements.tls_certificates_json.value,action==='edit'?'[{"edited":true}]':'[]');}
 });
 await test('UI-TLS-07 the latest picker selection owns the async draft',1,async()=>{
  reset();let release;const pending=new Promise(resolve=>{release=resolve;});const first={files:[file('old.pem',bytes(pem),()=>pending)],value:'old'};const old=api.stageTlsCertificateFiles({target:first});await stage([file('new.pem',bytes('\n'+pem))]);release(Uint8Array.from(bytes(pem)).buffer);await old;assert.equal(first.value,'');const current=JSON.parse(form.elements.tls_certificates_json.value);assert.equal(current.length,1);assert.equal(current[0].name,'new.pem');assert.equal(current[0].data,'\n'+pem);
 });
 await test('UI-TLS-08 inactive defaults omit TLS and active renderer combination refuses without mutation',2,async()=>{
  assert.equal(JSON.stringify(api.parseTlsSettings({tls_enabled:false,tls_certificates_json:'invalid'})),'{}');
  reset();const selected=await stage([file('public.pem',bytes(pem))]);const settings={tls_enabled:true,tls_respect_hsts:true,tls_certificates_json:JSON.stringify(selected),javascript_rendering:true};const before=JSON.stringify(settings);assert.throws(()=>api.parseTlsSettings(settings),/Renderer/);assert.equal(JSON.stringify(settings),before);
 });
 assert.equal(done.length,8);assert.equal(done.reduce((n,row)=>n+row.variants,0),26);
 console.log(JSON.stringify({result:'PASS_EXTRACTED_UI_VM',groups:8,variants:26,cases:done,targets,extracted_regions:regions,scope:'Exact staging/validation functions with explicit DOM and File stubs; public-certificate lexical fixture only. No actual browser/picker/save, crypto/X509, network/TLS or end-to-end product claim.'},null,2));
}
main().catch(error=>{console.error(error.stack||String(error));process.exitCode=1;});
