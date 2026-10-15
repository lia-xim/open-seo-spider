const fs=require('node:fs'), path=require('node:path'), {createHash}=require('node:crypto'), {execFileSync}=require('node:child_process');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013', task=path.dirname(__dirname);
const sha=b=>createHash('sha256').update(b).digest('hex'), must=(c,m)=>{if(!c)throw Error(m);};
function pinned(file,hash){const b=fs.readFileSync(file);must(sha(b)===hash,'Pin changed: '+file);return b;}
function packageAt(name,hash){const dir=path.join(base,name),raw=pinned(path.join(dir,'manifest.json'),hash),manifest=JSON.parse(raw);for(const f of [...manifest.entries,...manifest.support_files]){const b=pinned(path.join(dir,f.path),f.sha256);must(b.length===f.bytes,'Size changed: '+f.path);}return {dir,manifest};}
const original=packageAt('s6-native-acceptance-preparation-v1','18b306748cd301054d79a36cbc1f464691617bbda05ab3db3685644959306155');
const selected=packageAt('s6-native-acceptance-postfix-v1','cf40c019009836617b0ea24a6a237acc19818265c0bd08df864c63f597820770');
pinned(path.join(base,'s6-native-acceptance-postfix-review-v1/receipt.json'),'d7535479b7cda072b1863af839e6174cebfdeec1a98c1a6c2dcee3ecdffed61d');
for(const f of selected.manifest.preserved_adapter_dependencies)must(pinned(f.path,f.sha256).length===f.bytes,'Adapter changed');
const entry=selected.manifest.entries[0],target=path.join(task,'work/run-sitemap-findings-native-acceptance-v1.mjs');
must(path.resolve(entry.intended_promotion)===path.resolve(target),'Wrong promotion path');
must(entry.required_before==='ABSENT'&&!fs.existsSync(target),'Existing target must be preserved');
const publication=pinned(path.join(task,'work/sitemap-findings-surfaces-published-v1.json'),'b71188d145a8e720bd9d6c9aaf4720c03c20cced7f406839264e5432e85e84ff');
must(JSON.parse(publication).commit==='f43efc17d1393645a9eb8c8b8510743f3dfedb5c','Wrong source publication');
const output=path.join(task,'work/promote-sitemap-findings-native-v1.json');must(!fs.existsSync(output),'Receipt exists');
const bytes=pinned(path.join(selected.dir,entry.path),entry.sha256);fs.writeFileSync(target,bytes,{flag:'wx'});
execFileSync(process.execPath,['--check',target],{stdio:'pipe'});
must(sha(fs.readFileSync(target))===entry.sha256,'Promoted bytes differ');
const result={schema_version:1,result:'REVIEWED_NATIVE_GUARDIAN_PROMOTED_NOT_RUN',recorded_at:new Date().toISOString(),path:target,sha256:entry.sha256,bytes:bytes.length,source_manifest_sha256:'69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751',source_commit:'f43efc17d1393645a9eb8c8b8510743f3dfedb5c',base_manifest_sha256:'18b306748cd301054d79a36cbc1f464691617bbda05ab3db3685644959306155',selected_manifest_sha256:'cf40c019009836617b0ea24a6a237acc19818265c0bd08df864c63f597820770',independent_closure_sha256:'d7535479b7cda072b1863af839e6174cebfdeec1a98c1a6c2dcee3ecdffed61d',native_and_ui_adapters_verified:true,node_syntax_exit:0,driver_execution:0,operational_release_created:false,compute_hold:'ACTIVE',new_ui_e9_accepted:false,installed_runtime_modified:false};
fs.writeFileSync(output,JSON.stringify(result,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({result:result.result,sha256:result.sha256,receipt_sha256:sha(fs.readFileSync(output))}));
