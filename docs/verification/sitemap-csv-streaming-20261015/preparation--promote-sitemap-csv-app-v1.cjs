const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',root=path.join(base,'sitemap-csv-streaming-integration-v1'),source=path.join(base,'s4-csv-streaming-app-v1');
const sha=b=>createHash('sha256').update(b).digest('hex'),raw=fs.readFileSync(path.join(source,'manifest.json'));
assert.equal(sha(raw),'3dac7530629186a60d9da074f12a765167dc5e470110d4c63469f6d9a90ed99e');
const manifest=JSON.parse(raw),files=[];
for(const e of manifest.payloads){
 const before=fs.existsSync(path.join(root,e.path))?sha(fs.readFileSync(path.join(root,e.path))):null;
 assert.equal(before,e.before_sha256,e.path);
 const bytes=fs.readFileSync(path.join(source,e.path));assert.equal(sha(bytes),e.after_sha256,e.path);
 files.push({...e,bytes});
}
for(const f of files){const dest=path.join(root,f.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.writeFileSync(dest,f.bytes);assert.equal(sha(fs.readFileSync(dest)),f.after_sha256);}
const receipt={created:new Date().toISOString(),manifest_sha256:sha(raw),files:files.map(({bytes,...e})=>({...e,bytes:bytes.length})),compiler_tests:'NOT_RUN_COMPUTE_HOLD'};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({promoted:files.length,tests_executed:0}));
