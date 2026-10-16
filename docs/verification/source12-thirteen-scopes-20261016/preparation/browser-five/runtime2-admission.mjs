// Narrow admission shared only by these five browser-bearing Source12 native scopes.
import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import{createHash}from'node:crypto';import{fileURLToPath,pathToFileURL}from'node:url';
const HERE=path.dirname(fileURLToPath(import.meta.url)),SELF=fileURLToPath(import.meta.url),sha=b=>createHash('sha256').update(b).digest('hex'),norm=p=>path.resolve(p).replaceAll('\\','/').toLowerCase();
const selfSHA=sha(fs.readFileSync(SELF));
function read(r,max=8*1024*1024){assert(r&&path.isAbsolute(r.path));assert.match(r.sha256,/^[a-f0-9]{64}$/);assert(Number.isSafeInteger(r.bytes)&&r.bytes>=0&&r.bytes<=max);const s=fs.lstatSync(r.path);assert(s.isFile()&&!s.isSymbolicLink()&&s.size===r.bytes);const b=fs.readFileSync(r.path);assert.equal(b.length,r.bytes);assert.equal(sha(b),r.sha256,r.path);return b;}
const pinRef={"path":"E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source12-browser-five-runtime2-preparation-v4\\RUNTIME2-PINS.json","sha256":"0f154c1510f1480c549011e1eaf5cddb52b6f683191c99e57667cc8488e334d1","bytes":2266},P=JSON.parse(read(pinRef));
function same(a,b){assert(a&&b);assert.equal(norm(a.path),norm(b.path));assert.equal(a.sha256,b.sha256);assert.equal(a.bytes,b.bytes);read(a);read(b);}
function pass(r,kind){const v=JSON.parse(read(r));assert.equal(v.kind,kind);assert.equal(v.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(v.findings,[]);return v;}
// Only the exact Node binary ref already admitted by the current runtime manifest uses this binary path.
// Ordinary proof/JSON refs retain read()'s unchanged8MiB cap. No full executable buffer is allocated.
function verifyNodeIdentity(expected,declared){
 assert(expected&&path.isAbsolute(expected.path));assert.match(expected.sha256,/^[a-f0-9]{64}$/);
 assert(Number.isSafeInteger(expected.bytes)&&expected.bytes>0&&expected.bytes<=128*1024*1024,'Specific Node binary cap');
 assert.equal(norm(process.execPath),norm(expected.path));
 if(declared){assert.equal(norm(declared.path),norm(expected.path));assert.equal(declared.sha256,expected.sha256);assert.equal(declared.bytes,expected.bytes);}
 const before=fs.lstatSync(expected.path,{bigint:true});assert(before.isFile()&&!before.isSymbolicLink());assert.equal(before.size,BigInt(expected.bytes));
 const fd=fs.openSync(expected.path,'r'),keys=['dev','ino','size','mtimeNs','ctimeNs','birthtimeNs'],hash=createHash('sha256'),buffer=Buffer.alloc(1024*1024);let total=0;
 try{const opened=fs.fstatSync(fd,{bigint:true});for(const k of keys)assert.equal(opened[k],before[k]);
  for(;;){const count=fs.readSync(fd,buffer,0,Math.min(buffer.length,expected.bytes-total+1),null);if(count===0)break;total+=count;assert(total<=expected.bytes);hash.update(buffer.subarray(0,count));}
  assert.equal(total,expected.bytes);const ended=fs.fstatSync(fd,{bigint:true});for(const k of keys)assert.equal(ended[k],before[k]);
 }finally{fs.closeSync(fd);}
 const after=fs.lstatSync(expected.path,{bigint:true});assert(after.isFile()&&!after.isSymbolicLink());for(const k of keys)assert.equal(after[k],before[k]);assert.equal(hash.digest('hex'),expected.sha256);
}
export async function admitBrowserRuntime(R){
 for(const name of P.required_runtime_fields)read(R[name]);
 same(R.runtime_bridge_preparation,P.bridge_preparation);same(R.runtime_bridge_review,P.bridge_review);same(R.current_renderer_compatibility_preparation,P.bridge_preparation);same(R.current_renderer_compatibility_review,P.renderer_source_review);
 const m=JSON.parse(read(P.bridge_preparation));assert.equal(m.kind,'source12_historical_current_runtime_bridge_preparation');same(m.binder,P.binder);for(const f of m.payloads)read({...f,path:path.join(path.dirname(P.bridge_preparation.path),f.path)},2*1024*1024);
 const br=pass(P.binder_review,'source12_historical_current_runtime_binding_independent_review');same(br.proposal_manifest,P.bridge_preparation);same(br.binder,P.binder);
 const pr=pass(P.bridge_review,'source12_historical_current_runtime_bridge_independent_review');same(pr.proposal_manifest,P.bridge_preparation);same(pr.binder,P.binder);
 if(R.binder){same({path:R.binder.path,sha256:R.binder.sha256,bytes:R.binder.bytes},P.binder);if(R.binder.independent_review)same(R.binder.independent_review,P.binder_review);}
 assert.equal(R.bindings_sha256,P.binder.sha256);for(const[k,v]of Object.entries(P.core))if(k!=='source_commit'||R[k]!==undefined)assert.equal(R[k],v,k);
 assert.match(R.node_sha256,/^[a-f0-9]{64}$/);assert(path.isAbsolute(R.playwright_root));read(R.chrome);
 const imported=await import(pathToFileURL(P.binder.path));read(P.binder);const binding=imported.bindCombined(R,R.source_manifest_sha256);assert.equal(binding.runtime_witness?.all_admissions_true,true);
 const runtime=JSON.parse(read(R.current_runtime_manifest)),witness=binding.runtime_witness;
 function aliases(){assert.equal(norm(process.execPath),norm(runtime.node.path));assert.equal(R.node_sha256,runtime.node.sha256);verifyNodeIdentity({path:runtime.node.path,sha256:runtime.node.sha256,bytes:runtime.node.bytes},R.node);same(R.chrome,runtime.chrome.executable);same(witness.chrome,R.chrome);if(R.chrome_executable!==undefined)assert.equal(norm(R.chrome_executable),norm(R.chrome.path));if(R.chrome_sha256!==undefined)assert.equal(R.chrome_sha256,R.chrome.sha256);const pw=runtime.renderer.packages.find(p=>p.name==='playwright-core');assert(pw);assert.equal(norm(R.playwright_root),norm(pw.root));if(R.playwright_package_sha256!==undefined)assert.equal(R.playwright_package_sha256,pw.package_json.sha256);if(R.playwright_package){assert.equal(R.playwright_package.sha256,pw.package_json.sha256);assert.equal(R.playwright_package.bytes,pw.package_json.bytes);}if(R.renderer_runtime_root!==undefined)assert.equal(norm(R.renderer_runtime_root),norm(runtime.renderer.runtime_root));}
 function verify(){assert.equal(sha(fs.readFileSync(SELF)),selfSHA);read(pinRef);for(const name of P.required_runtime_fields)read(R[name]);read(P.binder);read(P.binder_review);read(P.bridge_review);aliases();binding.verify();}
 aliases();verify();return{binding,witness,runtime,verify,binder:P.binder,binder_review:P.binder_review};
}
