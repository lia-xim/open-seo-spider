const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',root=path.join(base,'sitemap-csv-streaming-integration-v1'),dir=path.join(base,'s4-csv-control-core-v1');
const sha=b=>createHash('sha256').update(b).digest('hex'),raw=fs.readFileSync(path.join(dir,'manifest.json'));
assert.equal(sha(raw),'aaceebb8167634bf0c748e92340948ed8b84ca3c1540e874b8c951f04782bebb');
const manifest=JSON.parse(raw),entries=[];
for(const entry of manifest.files){
 const output=path.join(root,entry.path),bytes=fs.readFileSync(path.join(dir,entry.payload));
 assert.equal(fs.existsSync(output)?sha(fs.readFileSync(output)):null,entry.before_sha256,entry.path);
 assert.equal(sha(bytes),entry.after_sha256);assert.equal(bytes.length,entry.bytes);
 entries.push({...entry,bytes_value:bytes,output});
}
for(const e of entries){fs.mkdirSync(path.dirname(e.output),{recursive:true});fs.writeFileSync(e.output,e.bytes_value);assert.equal(sha(fs.readFileSync(e.output)),e.after_sha256);}
const receipt={created:new Date().toISOString(),manifest_sha256:sha(raw),entries:entries.map(({bytes_value,output,...e})=>e),new_authored_tests:10,tests_executed:0,compiler_tests:'NOT_RUN_COMPUTE_HOLD'};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({promoted:entries.length,new_tests:10,tests_executed:0}));
