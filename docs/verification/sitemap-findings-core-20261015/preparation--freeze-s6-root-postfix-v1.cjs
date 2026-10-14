const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto');const sha=b=>createHash('sha256').update(b).digest('hex');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',repo=path.join(base,'sitemap-findings-core-integration-v1');
const old=path.join(base,'s6-root-session-core-v1'),out=path.join(base,'s6-root-session-postfix-v1');
assert(!fs.existsSync(out));const raw=fs.readFileSync(path.join(old,'manifest.json'));
assert.equal(sha(raw),'952b832dbec985155b8aaf7189f97fad6cdd7032688d68a1bfba492cf31ac0a0');
const prior=JSON.parse(raw);for(const f of prior.entries)assert.equal(sha(fs.readFileSync(path.join(old,f.payload))),f.sha256);
const paths=['crates/core/src/store/sitemap_findings_read.rs','crates/core/src/store/sitemap_findings_read_tests.rs','crates/core/src/sitemap_membership_analysis/findings_store_tests.rs'];
for(const f of prior.entries.filter(f=>!paths.includes(f.path)))assert.equal(sha(fs.readFileSync(path.join(repo,f.path))),f.sha256,'Preserved other Root input '+f.path);
const entries=paths.map(p=>{const previous=prior.entries.find(f=>f.path===p),b=fs.readFileSync(path.join(repo,p));assert(previous);assert(!b.includes(13));return {path:p,payload:p,before_sha256:previous.sha256,after_sha256:sha(b),bytes:b.length};});
fs.mkdirSync(out);for(const f of entries){const dest=path.join(out,f.payload);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(path.join(repo,f.path),dest);assert.equal(sha(fs.readFileSync(dest)),f.after_sha256);}
const receipt={schema_version:1,result:'ROOT_POSTFIX_SOURCE_ONLY_NOT_COMPILED_NOT_RUN',recorded_at:new Date().toISOString(),parent_manifest_sha256:sha(raw),entries,changes:['Count/stop a candidate before applying the seven finding predicates; bounded page/source projection and verification remain separate costs','Validate complete new saved-page typed qualification facts before user callback in available SavedPage domain','Two additional authored public Store/Crawler cases: valid large legacy task output budget and excluded invalid rehashed typed report fields'],new_authored_test_groups:2,executed_tests:0,compiler:'NOT_RUN_COORDINATED_COMPUTE_HOLD'};
const bytes=Buffer.from(JSON.stringify(receipt,null,2)+'\n');fs.writeFileSync(path.join(out,'manifest.json'),bytes,{flag:'wx'});console.log(JSON.stringify({path:path.join(out,'manifest.json'),sha256:sha(bytes),files:entries.length}));
