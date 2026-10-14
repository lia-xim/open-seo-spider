const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',root=path.join(base,'sitemap-rich-read-integration-v1'),dir=path.join(base,'s4-app-rich-read-v1');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const bytes=fs.readFileSync(path.join(dir,'manifest.json'));
assert.equal(sha(bytes),'b02737c28cc72bbe081f1030a03a74dce1a7e129b8fa89b7dc40f5aa90e1d3f8');
const manifest=JSON.parse(bytes),entries=[];
for(const entry of manifest.payloads){
 const input=path.join(dir,entry.payload),output=path.join(root,entry.path),payload=fs.readFileSync(input);
 assert.equal(sha(payload),entry.after_sha256);assert.equal(payload.length,entry.bytes);
 assert.equal(fs.existsSync(output)?sha(fs.readFileSync(output)):'ABSENT',entry.before_sha256,entry.path);
 entries.push({...entry,input,output,payload});
}
for(const hook of manifest.root_test_hooks){
 const output=path.join(root,hook.path),before=fs.readFileSync(output);
 assert.equal(sha(before),hook.before_sha256,hook.path);
 const payload=Buffer.concat([before,Buffer.from('\r\n'+hook.append_text.replaceAll('\n','\r\n')+'\r\n')]);
 assert(payload.subarray(0,before.length).equals(before));
 entries.push({path:hook.path,output,payload,before_sha256:sha(before),after_sha256:sha(payload),bytes:payload.length});
}
for(const entry of entries){fs.mkdirSync(path.dirname(entry.output),{recursive:true});fs.writeFileSync(entry.output,entry.payload);assert.equal(sha(fs.readFileSync(entry.output)),entry.after_sha256);}
const receipt={created:new Date().toISOString(),manifest_sha256:sha(bytes),entries:entries.map(({path,before_sha256,after_sha256,bytes})=>({path,before_sha256,after_sha256,bytes})),original_fixture_prefixes_preserved:true,compiler_tests:'NOT_RUN_COMPUTE_HOLD'};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({promoted:entries.length,fixture_hooks:2,compiler_tests:receipt.compiler_tests}));
