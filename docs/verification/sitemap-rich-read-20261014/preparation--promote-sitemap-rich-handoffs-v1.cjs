const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const root=path.join(base,'sitemap-rich-read-integration-v1');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const specs=[
 ['s4-analysis-read-core-v1','c702fb80a5abf5ad3e1f1b7afd8d513ee07baf07020b3d79501ab0e5da31e35c'],
 ['s4-coverage-read-core-v1','5d1c6b6a35d3864417a4711a099f84ef9820d682c766266e33388efc4e4fcece']
];
const all=[];
for(const [dir,pin] of specs){
 const dirpath=path.join(base,dir),bytes=fs.readFileSync(path.join(dirpath,'manifest.json'));
 if(hash(bytes)!==pin)throw Error('Manifest drift '+dir);
 const manifest=JSON.parse(bytes);
 for(const entry of manifest.entries??manifest.payloads){
  if(path.isAbsolute(entry.path)||entry.path.split(/[\\/]/).includes('..'))throw Error('Unsafe path');
  const input=path.join(dirpath,entry.payload),output=path.join(root,entry.path),payload=fs.readFileSync(input);
  const expected=entry.sha256??entry.after_sha256;
  if(hash(payload)!==expected||payload.length!==entry.bytes)throw Error('Payload drift '+entry.path);
  const before=fs.existsSync(output)?hash(fs.readFileSync(output)):'ABSENT';
  if(before!==entry.before_sha256)throw Error('Root drift '+entry.path+' '+before);
  all.push({input,output,path:entry.path,before,after:expected,payload});
 }
}
if(new Set(all.map(x=>x.output)).size!==all.length)throw Error('Duplicate output');
for(const entry of all){fs.mkdirSync(path.dirname(entry.output),{recursive:true});fs.copyFileSync(entry.input,entry.output);if(hash(fs.readFileSync(entry.output))!==entry.after)throw Error('Copy drift');}
const receipt={created:new Date().toISOString(),root,manifests:specs,entries:all.map(({path,before,after})=>({path,before,after})),compiler_tests:'NOT_RUN_COMPUTE_HOLD'};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n');
console.log(JSON.stringify({promoted:all.length,root,compiler_tests:receipt.compiler_tests}));
