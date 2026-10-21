const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{spawn,execFileSync}=require('node:child_process'),{createHash}=require('node:crypto');
const root=path.resolve(__dirname,'..'),out=path.join(__dirname,'table-xlsx-build-gates-20261021-r1');assert(!fs.existsSync(out));fs.mkdirSync(out);
const hash=file=>createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const files=execFileSync('git',['ls-files','--cached','--others','--exclude-standard','-z'],{cwd:root,encoding:'utf8'}).split('\0').filter(Boolean).filter(p=>/^(?:crates\/|web\/|renderer\/|scripts\/)/.test(p)||['Cargo.toml','Cargo.lock','.gitattributes','.github/workflows/ci.yml'].includes(p)).sort();
const inputs=files.map(p=>({path:p,bytes:fs.statSync(path.join(root,p)).size,sha256:hash(path.join(root,p))}));
fs.writeFileSync(path.join(out,'inputs.json'),JSON.stringify({kind:'explicit_selected_build_and_test_inputs_not_transitive_toolchain_closure',files:inputs},null,2)+'\n');
const env={...process.env,CARGO_INCREMENTAL:'0',CARGO_PROFILE_DEV_DEBUG:'0',CARGO_PROFILE_TEST_DEBUG:'0',CARGO_NET_OFFLINE:'true',OPEN_SEO_BROWSER_PATH:path.resolve(root,'../browser-runtime/chrome-155.0.8059.80-36a30716-source12-v1/chrome.exe')};
const commands=[['format','cargo',['fmt','--all','--check']],['build','cargo',['build','--locked','-p','open-seo-spider']],['app-tests','cargo',['test','--locked','-p','open-seo-spider','--','--test-threads=2']],['clippy','cargo',['clippy','--workspace','--all-targets','--all-features','--locked','--','-D','warnings']]];
const results=[];let failure;
(async()=>{try{for(const[name,program,args]of commands){
 const start=Date.now(),stdout=fs.createWriteStream(path.join(out,name+'.stdout.log')),stderr=fs.createWriteStream(path.join(out,name+'.stderr.log'));
 const child=spawn(program,args,{cwd:root,env,windowsHide:true,stdio:['ignore','pipe','pipe']});child.stdout.pipe(stdout);child.stderr.pipe(stderr);let launchError;child.once('error',error=>launchError=String(error));
 const timer=setTimeout(()=>child.kill(),1200000);const status=await new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));clearTimeout(timer);
 await Promise.all([new Promise(resolve=>stdout.closed?resolve():stdout.once('close',resolve)),new Promise(resolve=>stderr.closed?resolve():stderr.once('close',resolve))]);
 const result={name,program,args,...status,launchError,duration_ms:Date.now()-start};results.push(result);fs.writeFileSync(path.join(out,'progress.json'),JSON.stringify(results,null,2)+'\n');console.log(JSON.stringify(result));assert.equal(status.code,0,name+' failed');
}}catch(error){failure=String(error.stack);}finally{
 const changed=inputs.filter(pin=>hash(path.join(root,pin.path))!==pin.sha256).map(pin=>pin.path);
 const binary=path.join(root,'target/debug/open-seo-spider.exe');
 const proof={kind:'table_xlsx_root_fresh_Rust_build_gates',created_at:new Date().toISOString(),results,failure,changed,complete:!failure&&changed.length===0&&results.length===commands.length,binary:fs.existsSync(binary)?{path:binary,bytes:fs.statSync(binary).size,sha256:hash(binary)}:null,environment:Object.fromEntries(['CARGO_INCREMENTAL','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_NET_OFFLINE','OPEN_SEO_BROWSER_PATH'].map(key=>[key,env[key]])),scope:'Fresh App tests only, ordinary Core suite is not executed. App native_enrichment_browser_fixture remains ignored. Core renderer opt-ins are separate.'};
 fs.writeFileSync(path.join(out,'proof.json'),JSON.stringify(proof,null,2)+'\n');console.log(JSON.stringify(proof));if(!proof.complete)process.exitCode=1;
}})();
