const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',root=path.join(base,'sitemap-csv-streaming-integration-v1'),source=path.join(base,'s4-csv-streaming-tests-v1');
const sha=b=>createHash('sha256').update(b).digest('hex'),raw=fs.readFileSync(path.join(source,'manifest.json'));assert.equal(sha(raw),'83707b519f05b0b7e964ac25c4fdfacfbf998791409b7149218b0ae45fc891f0');
const m=JSON.parse(raw);assert.equal(m.payloads.length,2);assert.equal(m.authored_named_test_groups,13);
for(const e of m.artifacts??[]){const b=fs.readFileSync(path.join(source,e.path));assert.equal(sha(b),e.sha256);assert.equal(b.length,e.bytes);}
for(const e of m.dependencies){assert.equal(sha(fs.readFileSync(path.join(base,e.path))),e.sha256);}
const entries=m.payloads.map(e=>{assert.equal(e.before_sha256,null);assert(!fs.existsSync(path.join(root,e.path)),'Refuse overwrite '+e.path);const b=fs.readFileSync(path.join(source,e.path));assert.equal(sha(b),e.after_sha256);assert.equal(b.length,e.bytes);return {...e,raw:b};});
const parentPath=path.join(root,m.parent_hook.path),parent=fs.readFileSync(parentPath);assert.equal(sha(parent),m.parent_hook.before_sha256);
const hook='\n'+m.parent_hook.append_lines.join('\n')+'\n',next=Buffer.concat([parent,Buffer.from(hook)]);assert(next.subarray(0,parent.length).equals(parent));
for(const e of entries){const dest=path.join(root,e.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.writeFileSync(dest,e.raw,{flag:'wx'});assert.equal(sha(fs.readFileSync(dest)),e.after_sha256);}
fs.writeFileSync(parentPath,next);assert.equal(sha(fs.readFileSync(parentPath)),sha(next));
const receipt={created_at:new Date().toISOString(),manifest_sha256:sha(raw),entries:entries.map(({raw,...e})=>e),parent:{path:m.parent_hook.path,before_sha256:sha(parent),sha256:sha(next),preserved_prefix_bytes:parent.length,appended_lines:m.parent_hook.append_lines},new_authored_test_groups:13,new_executed:0,compiler_tests:'NOT_RUN_COORDINATED_COMPUTE_HOLD'};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({files:2,hook:receipt.parent,new_authored:13,executed:0}));
