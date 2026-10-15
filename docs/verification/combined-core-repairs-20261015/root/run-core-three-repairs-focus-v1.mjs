import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
const root='E:/CodexWork/open-seo-spider-01a12ac3-20261013',repo=path.join(root,'mcp-file-tools-v1'),out=path.join(root,'results/core-three-repairs-focus-v1');
const sha=b=>createHash('sha256').update(b).digest('hex');
assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
for(const name of ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER'])assert(!process.env[name]);
const profile={CARGO_PROFILE_TEST_OPT_LEVEL:'1',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS:'true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS:'true',CARGO_PROFILE_TEST_DEBUG:'0'};
for(const [key,value]of Object.entries(profile))assert(!process.env[key]||process.env[key]===value,'Contrary inherited test profile '+key);
const raw=fs.readFileSync(path.join(root,'combined-source-v1/manifest.json'));assert.equal(sha(raw),'67aebeb959ac6cfb9f8352162becf3ff98ef90918c825a674ea2e92a5df353d0');const parent=JSON.parse(raw);
const expected={
 'crates/core/src/sitemap/count_tests.rs':'decc7bd19a236535ab63d42cf00a1be32f5be444d165a9a663152618d4d82b84',
 'crates/core/src/sitemap_source_measurement.rs':'b70ced5d81c5a2708529e014ca61a24dea2458011277aa227530808db2bd2dca',
 'crates/core/src/sitemap_source_measurement_tests.rs':'5ae47c3571ad3e8a64f86c0a991a8ffd377440ac76d9d893004e6e9f3ed8cdac',
 'crates/core/src/sitemap_source_store_tests.rs':'074abf3a397e3a5b75d45c4f5cce83b0ee6b592b94dc2fe19b9a433ab7ef7b6b',
};
const changed=[],files=parent.files.map(old=>{const bytes=fs.readFileSync(path.join(repo,old.path)),hash=sha(bytes);if(hash!==old.sha256){assert.equal(hash,expected[old.path],old.path);changed.push(old.path);}return {path:old.path,bytes:bytes.length,sha256:hash};});assert.deepEqual(changed.sort(),Object.keys(expected).sort());
const now=new Date();for(const p of changed)fs.utimesSync(path.join(repo,p),now,now);
const verify=()=>{for(const f of files){const b=fs.readFileSync(path.join(repo,f.path));assert.equal(b.length,f.bytes);assert.equal(sha(b),f.sha256);}};
const groups=[
 {filter:'crawler::sitemap_capture_tests::actual_oversized_redirect_context_retains_known_transport_with_unknown_decode_and_count',names:['crawler::sitemap_capture_tests::actual_oversized_redirect_context_retains_known_transport_with_unknown_decode_and_count']},
 {filter:'sitemap::count_tests::count_same_strict_reader_stops_preserve_only_actual_completed_prefix_counts',names:['sitemap::count_tests::count_same_strict_reader_stops_preserve_only_actual_completed_prefix_counts']},
 {filter:'sitemap_source_store::tests::actual_record_and_total_exact_bytes_plus_one_omit_durably_without_replacement',names:['sitemap_source_store::tests::actual_record_and_total_exact_bytes_plus_one_omit_durably_without_replacement']},
 {filter:'context_unavailable_final_url_',names:['sitemap_source_measurement::tests::context_unavailable_final_url_preserves_known_transport_and_unknown_assessments','sitemap_source_measurement::tests::context_unavailable_final_url_rejects_unqualified_mismatches_and_invented_facts']},
];
assert(!fs.existsSync(out));fs.mkdirSync(out);const temp=path.join(out,'temp');fs.mkdirSync(temp);
fs.writeFileSync(path.join(out,'source-inputs.json'),JSON.stringify({files,changed,profile,groups},null,2)+'\n',{flag:'wx'});
const env={...process.env,...profile,CARGO_TARGET_DIR:path.join(root,'cargo-target'),CARGO_INCREMENTAL:'0',CARGO_PROFILE_DEV_DEBUG:'0',CARGO_TERM_COLOR:'never',TEMP:temp,TMP:temp};const runs=[],failures=[];
for(let i=0;i<groups.length;i++){
 const group=groups[i],args=['test','-p','open-seo-spider-core','--lib','--locked',group.filter,'--',...(group.names.length===1?['--exact']:[]),'--test-threads=1'];
 const logPath=path.join(out,`test-${i+1}.log`),fd=fs.openSync(logPath,'wx'),start=Date.now();let child,closed,spawnError=null;
 try{verify();child=spawn('C:/Users/matth/.cargo/bin/cargo.exe',args,{cwd:repo,env,windowsHide:true,stdio:['ignore',fd,fd]});child.once('error',e=>{spawnError=e.message;});closed=await new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));}catch(error){failures.push(error.message);}finally{fs.closeSync(fd);}
 const bytes=fs.readFileSync(logPath),text=bytes.toString('utf8'),summaries=[...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(m=>({state:m[1],passed:+m[2],failed:+m[3],ignored:+m[4]}));
 const pass=closed?.code===0&&closed?.signal===null&&!spawnError&&group.names.every(n=>text.includes('test '+n+' ... ok'))&&summaries.length===1&&summaries[0].passed===group.names.length&&summaries[0].failed===0&&summaries[0].ignored===0;
 const run={names:group.names,args,pid:child?.pid,actual_cargo_close:closed,spawn_error:spawnError,elapsed_ms:Date.now()-start,log_sha256:sha(bytes),summaries,passed:pass};runs.push(run);console.log(JSON.stringify(run));if(!pass){failures.push(group.filter);console.error(text.slice(-5000));}
}
let sourceAfter=false;try{verify();sourceAfter=true;}catch(error){failures.push(error.message);}
const proof={kind:'actual_core_three_repairs_five_focused_tests_v1',recorded_at:new Date().toISOString(),result:sourceAfter&&failures.length===0?'PASS_THREE_REPAIRS_AND_TWO_NEW_TYPED_CONTROLS':'FAIL_FOCUSED_CORE_REPAIRS',source_inputs:files.length,changed_inputs:changed.length,source_after_verified:sourceAfter,profile,runs,failures,known_profile_array_failure:'NOT_FIXED_NOT_RUN_BY_THIS_GATE',full_workspace:'NOT_RUN_AFTER_REPAIRS',clippy:'NOT_RUN',four_real_renderer_gates:'NOT_RUN',installed_runtime:'UNCHANGED_AB1',manufacturer_parity:'NOT_PROVEN'};
fs.writeFileSync(path.join(out,'proof.json'),JSON.stringify(proof,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:proof.result,source_after:sourceAfter,actual_test_functions:5}));if(failures.length||!sourceAfter)process.exitCode=1;
