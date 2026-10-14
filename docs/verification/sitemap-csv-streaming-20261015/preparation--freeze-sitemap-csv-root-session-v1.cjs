const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{execFileSync}=require('node:child_process'),{createHash}=require('node:crypto');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',root=path.join(base,'sitemap-csv-streaming-integration-v1'),out=path.join(base,'s4-csv-root-session-v1');
const git=args=>execFileSync('git',args,{cwd:root}),sha=b=>createHash('sha256').update(b).digest('hex');
assert.equal(git(['rev-parse','HEAD']).toString().trim(),'d105fb8a0c527ffd99164e44ec644bd0e49f61a5');assert(!fs.existsSync(out));
const paths=['crates/core/src/lib.rs','crates/core/src/store.rs','crates/core/src/store-sitemap-source-tests.rs',
 'crates/core/src/store/sitemap_read_window.rs','crates/core/src/store/sitemap_read_context.rs',
 'crates/core/src/store/sitemap_stored_rows.rs','crates/core/src/store/sitemap_stored_rows_tests.rs',
 'crates/core/src/store/sitemap_stored_source_tests.rs'];
const tracked=new Set(git(['ls-tree','-r','--name-only','HEAD']).toString().split('\n'));
const entries=paths.map(p=>{const bytes=fs.readFileSync(path.join(root,p));return {path:p,payload:'product/'+p,before_sha256:tracked.has(p)?sha(git(['show','HEAD:'+p])):'ABSENT',sha256:sha(bytes),bytes:bytes.length,raw:bytes};});
const names=p=>[...fs.readFileSync(path.join(root,p),'utf8').matchAll(/#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+(\w+)/g)].map(x=>x[1]);
const groups=paths.filter(p=>p.endsWith('_tests.rs')).map(p=>({path:p,names:names(p)}));assert.equal(groups.reduce((n,g)=>n+g.names.length,0),9);
fs.mkdirSync(out);for(const f of entries){const dest=path.join(out,f.payload);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.writeFileSync(dest,f.raw,{flag:'wx'});assert.equal(sha(fs.readFileSync(dest)),f.sha256);}
const manifest={schema_version:1,name:'s4-csv-root-session-v1',immutable:true,recorded_at:new Date().toISOString(),baseline:'d105fb8a0c527ffd99164e44ec644bd0e49f61a5',entries:entries.map(({raw,...rest})=>rest),new_authored_tests:9,test_groups:groups,format:'rustfmt --edition2024 skip_children=true parser only',compiler_tests_sqlite_native:'NOT_RUN_COMPUTE_HOLD',scope:'Shared actual read preparation, public opaque fixed-filter repeated allstored session, existing rich window state preserved. ActualRawNone unavailable priority preserved for allstored reader; strict window Expected preserved. Controlled private readers and App transport are separate coordinated handoffs.',product_root_owned_only:true};
const raw=Buffer.from(JSON.stringify(manifest,null,2)+'\n');fs.writeFileSync(path.join(out,'manifest.json'),raw,{flag:'wx'});console.log(JSON.stringify({out,manifest_sha256:sha(raw),entries:entries.length,new_authored_tests:9,executed:0}));
