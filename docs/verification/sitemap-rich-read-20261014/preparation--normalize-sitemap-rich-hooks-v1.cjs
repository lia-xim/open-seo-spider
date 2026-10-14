const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const root='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-rich-read-integration-v1';
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const previous=JSON.parse(fs.readFileSync(__filename.replace('normalize-sitemap-rich-hooks-v1.cjs','promote-sitemap-rich-app-v1.json')));
const manifest=JSON.parse(fs.readFileSync('E:/CodexWork/open-seo-spider-01a12ac3-20261013/s4-app-rich-read-v1/manifest.json'));
const entries=[];
for(const hook of manifest.root_test_hooks){
 const file=path.join(root,hook.path),bytes=fs.readFileSync(file),before=previous.entries.find(e=>e.path===hook.path);
 assert.equal(sha(bytes),before.after_sha256);
 const tail=Buffer.from('\r\n'+hook.append_text.replaceAll('\n','\r\n')+'\r\n');
 assert(bytes.subarray(bytes.length-tail.length).equals(tail));
 const prefix=bytes.subarray(0,bytes.length-tail.length);assert.equal(sha(prefix),hook.before_sha256);
 const next=Buffer.concat([prefix,Buffer.from('\n'+hook.append_text+'\n')]);
 entries.push({file,path:hook.path,before:sha(bytes),after:sha(next),prefix:sha(prefix),bytes:next});
}
for(const e of entries){fs.writeFileSync(e.file,e.bytes);assert.equal(sha(fs.readFileSync(e.file)),e.after);}
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify({created:new Date().toISOString(),purpose:'Only new fixture hook suffix LF; original fixture prefixes exact',entries:entries.map(({path,before,after,prefix})=>({path,before,after,prefix}))},null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({normalized:entries.length,old_prefixes_preserved:true}));
