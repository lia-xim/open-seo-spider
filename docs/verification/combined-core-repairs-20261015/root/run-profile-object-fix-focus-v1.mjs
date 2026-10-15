import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',repo=path.join(base,'mcp-file-tools-v1'),out=path.join(base,'results/profile-object-fix-focus-v1');
const sha=b=>createHash('sha256').update(b).digest('hex');
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
for(const k of ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER'])assert(!process.env[k]);
const profile={CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_PROFILE_DEV_DEBUG:'0'};
for(const [k,v]of Object.entries(profile))assert(!process.env[k]||process.env[k]===v);
const parentBytes=fs.readFileSync(path.join(base,'combined-source-v1/manifest.json'));assert.equal(sha(parentBytes),'67aebeb959ac6cfb9f8352162becf3ff98ef90918c825a674ea2e92a5df353d0');const parent=JSON.parse(parentBytes);
const before=JSON.parse(fs.readFileSync(path.join(base,'results/core-three-repairs-focus-v1/source-inputs.json'))),previous=JSON.parse(fs.readFileSync(path.join(base,'results/core-three-repairs-focus-v1/proof.json')));assert.equal(previous.result,'PASS_THREE_REPAIRS_AND_TWO_NEW_TYPED_CONTROLS');assert.equal(previous.source_after_verified,true);
const proposalDir=path.join(base,'source-processing-object-profile-fix-v1'),proposalBytes=fs.readFileSync(path.join(proposalDir,'manifest.json'));assert.equal(sha(proposalBytes),'fffb84f1ffb412b8b5df3a815fbc5591b37d86c4a839d2ec39017a1dbd5f066b');const proposal=JSON.parse(proposalBytes);
const expected=new Map(proposal.files.map(f=>[f.path,f]));assert.equal(expected.size,2);
const changed=[],files=before.files.map(f=>{const b=fs.readFileSync(path.join(repo,f.path)),h=sha(b),p=expected.get(f.path);if(p){assert.equal(h,p.after_sha256);assert.equal(b.length,p.bytes);changed.push(f.path);}else{assert.equal(h,f.sha256);assert.equal(b.length,f.bytes);}return {path:f.path,sha256:h,bytes:b.length};});assert.equal(changed.length,2);assert.equal(files.length,parent.files.length);
const verify=()=>{for(const f of files){const b=fs.readFileSync(path.join(repo,f.path));assert.equal(sha(b),f.sha256,f.path);assert.equal(b.length,f.bytes);}};
assert(!fs.existsSync(out));fs.mkdirSync(out);const temp=path.join(out,'temp');fs.mkdirSync(temp);
fs.writeFileSync(path.join(out,'source-inputs.json'),JSON.stringify({files,profile,changed_from_previous_focused_input:changed,proposal_manifest_sha256:sha(proposalBytes)},null,2)+'\n',{flag:'wx'});
const coreNames=proposal.preserved.existing_test_names.map(n=>'sitemap_source_processing::tests::'+n);
const appNames=['create_dto_preserves_source_work_limits_without_widening_page_bodies','create_dto_missing_or_null_source_profile_keeps_historical_configuration','create_dto_rejects_invalid_or_unknown_source_profile_fields'].map(n=>'server::sitemap_processing_config_tests::'+n);
const groups=[{pkg:'open-seo-spider-core',filter:'sitemap_source_processing::tests::',names:coreNames},{pkg:'open-seo-spider',filter:'server::sitemap_processing_config_tests::',names:appNames}];
const env={...process.env,...profile,CARGO_INCREMENTAL:'0',CARGO_TARGET_DIR:path.join(base,'cargo-target'),CARGO_TERM_COLOR:'never',TEMP:temp,TMP:temp},runs=[],failures=[];
for(let i=0;i<groups.length;i++){
 const g=groups[i],args=['test','-p',g.pkg,...(g.pkg==='open-seo-spider-core'?['--lib']:['--bin','open-seo-spider']),'--locked',g.filter,'--','--test-threads=1'],logPath=path.join(out,`test-${i+1}.log`),fd=fs.openSync(logPath,'wx');let child,close,spawnError=null;const start=Date.now();
 try{verify();child=spawn('C:/Users/matth/.cargo/bin/cargo.exe',args,{cwd:repo,env,windowsHide:true,stdio:['ignore',fd,fd]});child.once('error',e=>{spawnError=e.message;});close=await new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));}finally{fs.closeSync(fd);}
 const bytes=fs.readFileSync(logPath),text=bytes.toString('utf8'),summaries=[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
 const passed=close.code===0&&close.signal===null&&!spawnError&&g.names.every(n=>text.includes('test '+n+' ... ok'))&&summaries.length===1&&summaries[0].passed===g.names.length&&summaries[0].failed===0&&summaries[0].ignored===0;
 const run={args,names:g.names,pid:child.pid,actual_close:close,spawn_error:spawnError,elapsed_ms:Date.now()-start,log_sha256:sha(bytes),summaries,passed};runs.push(run);console.log(JSON.stringify(run));if(!passed){failures.push(g.filter);console.error(text.slice(-5000));}
}
let sourceAfter=false;try{verify();sourceAfter=true;}catch(e){failures.push(e.message);}
const proof={kind:'actual_strict_object_profile_and_app_config_focus_v1',recorded_at:new Date().toISOString(),result:sourceAfter&&failures.length===0?'PASS_TEN_PROFILE_AND_THREE_APP_CONFIG_GROUPS':'FAIL_PROFILE_OBJECT_FOCUS',source_inputs:files.length,source_after_verified:sourceAfter,profile,runs,failures,full_workspace:'NOT_RUN_AFTER_REPAIRS',clippy:'NOT_RUN',four_renderer_gates:'NOT_RUN',runtime_installation:'UNCHANGED_AB1',manufacturer_parity:'NOT_PROVEN'};
fs.writeFileSync(path.join(out,'proof.json'),JSON.stringify(proof,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:proof.result,source_after:sourceAfter}));if(failures.length||!sourceAfter)process.exitCode=1;
