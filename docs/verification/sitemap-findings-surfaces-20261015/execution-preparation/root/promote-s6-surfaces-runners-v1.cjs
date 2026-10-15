const fs=require('node:fs'),path=require('node:path'),{createHash}=require('node:crypto'),{execFileSync}=require('node:child_process');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',dir=path.join(base,'s6-surfaces-runners-v1'),receipt=path.join(__dirname,'promote-s6-surfaces-runners-v1.json');
const sha=b=>createHash('sha256').update(b).digest('hex'),insist=(c,m)=>{if(!c)throw Error(m);};
insist(!fs.existsSync(receipt),'fresh receipt required');
const raw=fs.readFileSync(path.join(dir,'manifest.json'));insist(sha(raw)==='30a85f13c756792b43c284f6789e7207042c0719f64a077d5517580bb518a13e','runner manifest changed');const manifest=JSON.parse(raw);
const payload=[];
for(const entry of [...manifest.entries,...manifest.support_files]){const bytes=fs.readFileSync(path.join(dir,entry.path));insist(bytes.length===entry.bytes&&sha(bytes)===entry.sha256,'declared runner bytes changed');payload.push({entry,bytes});}
for(const entry of manifest.templates)insist(sha(fs.readFileSync(entry.path))===entry.sha256,'original driver changed');
const sourceRaw=fs.readFileSync(manifest.source_manifest);insist(sha(sourceRaw)==='69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751','source manifest changed');
for(const file of JSON.parse(sourceRaw).files)for(const root of [manifest.expected_paths.repo,manifest.expected_paths.source]){const bytes=fs.readFileSync(path.join(root,file.path));insist(bytes.length===file.bytes&&sha(bytes)===file.sha256,'source changed '+file.path);}
for(const entry of manifest.entries){insist(path.resolve(entry.intended_promotion)===path.resolve(__dirname,entry.path),'unexpected promotion target');insist(!fs.existsSync(entry.intended_promotion),'new runner target must be absent');}
// Every byte/source/before-image guard precedes the first write. No driver is evaluated.
for(const {entry,bytes} of payload.filter(p=>p.entry.intended_promotion)){fs.writeFileSync(entry.intended_promotion,bytes,{flag:'wx'});insist(sha(fs.readFileSync(entry.intended_promotion))===entry.sha256,'promotion mismatch');}
const checks=manifest.entries.map(entry=>({path:entry.intended_promotion,syntax:execFileSync(process.execPath,['--check',entry.intended_promotion],{encoding:'utf8'}).trim(),sha256:entry.sha256}));
const result={schema_version:1,result:'SOURCE_RUNNERS_PROMOTED_SYNTAX_ONLY',recorded_at:new Date().toISOString(),manifest_sha256:sha(raw),source_manifest_sha256:sha(sourceRaw),checks,drivers_imported_or_executed:false,compiler_tests_browser_build_runs:0,installed_runtime_modified:false,compute_hold:'ACTIVE_NOT_RELEASED'};
fs.writeFileSync(receipt,JSON.stringify(result,null,2)+'\n',{flag:'wx'});process.stdout.write(JSON.stringify(result)+'\n');
