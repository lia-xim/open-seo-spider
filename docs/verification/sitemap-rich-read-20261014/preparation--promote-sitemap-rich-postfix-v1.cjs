const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',root=path.join(base,'sitemap-rich-read-integration-v1'),source=path.join(base,'s4-analysis-report-key-postfix-v1');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const raw=fs.readFileSync(path.join(source,'manifest.json'));assert.equal(sha(raw),'8b9d6db457f74f71b8380b05bae8a98d9f6ce6067582a5b022171ebee8fde854');
const manifest=JSON.parse(raw),entries=[];
for(const entry of manifest.entries){
 const input=path.join(source,entry.payload),output=path.join(root,entry.path),payload=fs.readFileSync(input);
 assert.equal(sha(payload),entry.sha256);assert.equal(payload.length,entry.bytes);assert.equal(sha(fs.readFileSync(output)),entry.before_sha256,entry.path);
 entries.push({...entry,input,output});
}
for(const entry of entries){fs.copyFileSync(entry.input,entry.output);assert.equal(sha(fs.readFileSync(entry.output)),entry.sha256);}
const receipt={created:new Date().toISOString(),manifest_sha256:sha(raw),entries:entries.map(({path,before_sha256,sha256,bytes})=>({path,before_sha256,sha256,bytes})),new_tests:0,compiler_tests:'NOT_RUN_COMPUTE_HOLD'};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({promoted:entries.length,new_test_names:0,compiler_tests:receipt.compiler_tests}));
