const fs=require('node:fs'),path=require('node:path'),{createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const task=path.dirname(__dirname),base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',dir=path.join(base,'s4-source-ui-suite-preparation-v1');
const sha=b=>createHash('sha256').update(b).digest('hex'),must=(c,m)=>{if(!c)throw Error(m);};
const [reviewName,reviewSHA]=process.argv.slice(2);must(process.argv.length===4&&/^[a-z0-9-]+$/.test(reviewName??'')&&/^[a-f0-9]{64}$/.test(reviewSHA??''),'Actual independent source review required');
const pinned=(file,pin)=>{const b=fs.readFileSync(file);must(sha(b)===pin,'Pin changed: '+file);return b;};
const raw=pinned(path.join(dir,'manifest.json'),'7f956d0ade7ed0d70ced3ec92766df5d390c0da9abe674e0665a37d49dd6c2aa'),manifest=JSON.parse(raw);
pinned(path.join(base,reviewName,'receipt.json'),reviewSHA);
must(manifest.ui_source_manifest_sha256==='e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5'&&manifest.new_ui_rust_tests===0&&manifest.entries.length===2,'UI rebind contract changed');
pinned(path.join(task,'work/sitemap-source-processing-ui-published-v1.json'),'4316704b11e47d7f49bb541e9b1db9b1c31ac9c53d5b68dd2c47ab339b4bc24b');
for(const f of [...manifest.entries,...manifest.support_files])must(pinned(path.join(dir,f.path),f.sha256).length===f.bytes,'Size changed: '+f.path);
const planned=manifest.entries.map(f=>{const target=path.join(task,'work',f.path);must(path.resolve(target)===path.resolve(f.intended_task_promotion),'Wrong UI runner target');must(f.required_before==='ABSENT'&&!fs.existsSync(target),'Existing UI runner must be preserved');return {...f,target};});
const out=path.join(task,'work/promote-sitemap-source-ui-runners-v1.json');must(!fs.existsSync(out),'Fresh receipt required');
for(const f of planned){fs.writeFileSync(f.target,pinned(path.join(dir,f.path),f.sha256),{flag:'wx'});execFileSync(process.execPath,['--check',f.target],{stdio:'pipe'});must(sha(fs.readFileSync(f.target))===f.sha256,'Promoted UI runner differs');}
const result={schema_version:1,result:'REVIEWED_UI_FULL_SUITE_BUILD_RUNNERS_PROMOTED_NOT_RUN',recorded_at:new Date().toISOString(),source_manifest_sha256:manifest.ui_source_manifest_sha256,source_publication_sha256:manifest.ui_publication_receipt_sha256,selected_manifest_sha256:sha(raw),independent_review_path:path.join(base,reviewName,'receipt.json'),independent_review_sha256:reviewSHA,selected:planned.map(f=>({path:f.target,sha256:f.sha256,bytes:f.bytes})),syntax_exit_codes:[0,0],driver_runs:0,compiler_or_native_runs:0,native_acceptance_transferred:false,operational_release_created:false,compute_hold:'ACTIVE',installed_runtime_modified:false};
fs.writeFileSync(out,JSON.stringify(result,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:result.result,receipt_sha256:sha(fs.readFileSync(out)),selected:result.selected}));
