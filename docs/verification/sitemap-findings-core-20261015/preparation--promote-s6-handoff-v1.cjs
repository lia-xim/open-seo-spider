const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto');
const sha=b=>createHash('sha256').update(b).digest('hex');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const repo=path.join(base,'sitemap-findings-core-integration-v1');
const [dir,pin,file='manifest.json']=process.argv.slice(2);
assert(/^s6-[a-z0-9-]+-v[0-9]+$/.test(dir??''));assert(/^[a-f0-9]{64}$/.test(pin??''));assert(['manifest.json','receipt.json'].includes(file));
const raw=fs.readFileSync(path.join(base,dir,file));assert.equal(sha(raw),pin);
const m=JSON.parse(raw),entries=m.entries??m.payloads;assert(Array.isArray(entries)&&entries.length>0);
const model=fs.readFileSync(path.join(repo,'crates/core/src/sitemap_findings.rs'));
assert.equal(sha(model),'723200ced8fafb9652a89f6a8f19fa416dffbee3a62ddd780093c5380e36d1ef');
const changes=entries.map(f=>{
 assert(/^crates\/core\/src\/[a-zA-Z0-9_./-]+\.rs$/.test(f.path));assert(!f.path.split('/').includes('..'));
 const payload=f.payload??f.path;assert(!path.isAbsolute(payload));assert(!payload.split(/[\\/]/).includes('..'));
 const b=fs.readFileSync(path.join(base,dir,payload));assert.equal(sha(b),f.after_sha256??f.sha256,f.path+' payload');assert.equal(b.length,f.bytes);
 const target=path.join(repo,f.path),before=f.before_sha256;
 if(before===null||f.require_target_absent)assert(!fs.existsSync(target),'New target required '+f.path);
 else{assert(/^[a-f0-9]{64}$/.test(before));assert.equal(sha(fs.readFileSync(target)),before,f.path+' actual before');}
 return {path:f.path,target,bytes:b,before_sha256:before??null,after_sha256:sha(b)};
});
assert.equal(new Set(changes.map(f=>f.path)).size,changes.length);
// Every payload and every actual target has passed before the first mutation.
for(const f of changes){fs.mkdirSync(path.dirname(f.target),{recursive:true});fs.writeFileSync(f.target,f.bytes);assert.equal(sha(fs.readFileSync(f.target)),f.after_sha256);}
const receipt={schema_version:1,result:'EXACT_HASH_GUARDED_SOURCE_PROMOTION_NO_EXECUTION',recorded_at:new Date().toISOString(),handoff:dir+'/'+file,handoff_sha256:pin,entries:changes.map(({path,before_sha256,after_sha256,bytes})=>({path,before_sha256,after_sha256,bytes:bytes.length})),executed_tests:0,compiler:'NOT_RUN_COORDINATED_COMPUTE_HOLD'};
const dest=path.join(__dirname,'promote-'+dir+'.json');fs.writeFileSync(dest,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({receipt:dest,files:changes.length}));
