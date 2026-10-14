const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const sha=b=>createHash('sha256').update(b).digest('hex');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
const repo=path.join(base,'sitemap-findings-core-integration-v1');
const out=path.join(base,'s6-root-session-core-v1');
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:repo}).toString().trim(),'c8f539be979ac5436dcc962c5ce7677aab9f3b0d');
assert(!fs.existsSync(out),'Immutable new handoff only');
const priorRaw=fs.readFileSync(path.join(base,'sitemap-csv-streaming-source-v1/manifest.json'));
assert.equal(sha(priorRaw),'a31936510381cca40901eb547f15090a77c833f617f84f51f67b057fe7a5714e');
const prior=JSON.parse(priorRaw);
const paths=[
 'crates/core/src/lib.rs','crates/core/src/store.rs','crates/core/src/store/sitemap_read_context.rs',
 'crates/core/src/sitemap_findings.rs','crates/core/src/store/sitemap_findings_read.rs',
 'crates/core/src/store/sitemap_findings_read_tests.rs','crates/core/src/sitemap_membership_analysis_tests.rs',
 'crates/core/src/sitemap_membership_analysis/findings_store_tests.rs',
];
const entries=paths.map(p=>{const b=fs.readFileSync(path.join(repo,p));
 assert(!b.includes(13),'Repository LF source '+p);
 const before=prior.files.find(f=>f.path===p)?.sha256??null;
 if(before)assert.equal(sha(fs.readFileSync(path.join(base,'sitemap-csv-streaming-source-v1',p))),before);
 return {path:p,payload:p,before_sha256:before,sha256:sha(b),bytes:b.length};
});
fs.mkdirSync(out);
for(const f of entries){const dest=path.join(out,f.payload);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(path.join(repo,f.path),dest);assert.equal(sha(fs.readFileSync(dest)),f.sha256);}
const modelRaw=fs.readFileSync(path.join(base,'s6-findings-model-v1/manifest.json'));
assert.equal(sha(modelRaw),'d7d62bd04bb4fb5fcd9befec5623b93b8c4bdc6b90f0cdf697546ae02307079c');
const model=fs.readFileSync(path.join(base,'s6-findings-model-v1/model.rs'));
assert.equal(sha(model),'723200ced8fafb9652a89f6a8f19fa416dffbee3a62ddd780093c5380e36d1ef');
assert.equal(sha(fs.readFileSync(path.join(repo,'crates/core/src/sitemap_findings.rs'))),sha(model));
const namesIn=b=>[...b.toString().matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)/g)].map(x=>x[1]);
const tests=entries.filter(f=>f.path.endsWith('_tests.rs')).map(f=>{
 const previous=prior.files.some(p=>p.path===f.path)?namesIn(fs.readFileSync(path.join(base,'sitemap-csv-streaming-source-v1',f.path))):[];
 const current=namesIn(fs.readFileSync(path.join(repo,f.path)));
 for(const n of previous)assert(current.includes(n),'Preserved previous actual test '+n);
 return {path:f.path,names:current.filter(n=>!previous.includes(n))};
}).filter(g=>g.names.length);
const receipt={schema_version:1,result:'ROOT_SESSION_SOURCE_ONLY_REVIEW_INPUT',recorded_at:new Date().toISOString(),baseline_head:'c8f539be979ac5436dcc962c5ce7677aab9f3b0d',previous_manifest_sha256:sha(priorRaw),model_manifest_sha256:sha(modelRaw),entries,authored_tests:tests,executed_tests:0,compiler:'NOT_RUN_COORDINATED_COMPUTE_HOLD',installed_runtime_modified:false};
const raw=Buffer.from(JSON.stringify(receipt,null,2)+'\n');fs.writeFileSync(path.join(out,'manifest.json'),raw,{flag:'wx'});
console.log(JSON.stringify({path:path.join(out,'manifest.json'),sha256:sha(raw),files:entries.length,authored_tests:tests.reduce((n,g)=>n+g.names.length,0)}));
