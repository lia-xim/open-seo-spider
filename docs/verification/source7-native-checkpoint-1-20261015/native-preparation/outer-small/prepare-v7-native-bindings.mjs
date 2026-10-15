import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {pathToFileURL,fileURLToPath} from 'node:url';
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',task='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https',sha=b=>createHash('sha256').update(b).digest('hex');
assert(!process.env.NODE_OPTIONS&&!process.env.NODE_PATH&&process.execArgv.length===0);assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
assert.equal(path.resolve(fileURLToPath(import.meta.url)),path.join(task,'work/prepare-v7-native-bindings.mjs'),'Promote exact reviewed writer');
assert.equal(process.argv.length,12,'Use --manifest-sha ROOT64 --suite-proof-sha ROOT64 --build-proof-sha ROOT64 --body-helper-review ABS --body-helper-review-sha ROOT64');assert.equal(process.argv[2],'--manifest-sha');assert.equal(process.argv[4],'--suite-proof-sha');assert.equal(process.argv[6],'--build-proof-sha');assert.equal(process.argv[8],'--body-helper-review');assert.equal(process.argv[10],'--body-helper-review-sha');assert(path.isAbsolute(process.argv[9]));assert.match(process.argv[11],/^[a-f0-9]{64}$/);
const [sourceSha,suiteSha,buildSha]=[process.argv[3],process.argv[5],process.argv[7]];assert.equal(sourceSha,'fd587ba252595d971e7557e5cca7f5ab90219ecda200a3f07fcd5c10c13b6397');for(const s of [sourceSha,suiteSha,buildSha])assert.match(s,/^[a-f0-9]{64}$/);const bodyHelperReview={path:process.argv[9],sha256:process.argv[11]};const brs=fs.lstatSync(bodyHelperReview.path);assert(brs.isFile()&&!brs.isSymbolicLink()&&brs.size<=1024*1024);const brr=fs.readFileSync(bodyHelperReview.path);assert.equal(sha(brr),bodyHelperReview.sha256);const bhr=JSON.parse(brr);assert.equal(bhr.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(bhr.findings,[]);assert.equal(bhr.proposal_manifest.sha256,'d3dc9c5fb5f362a03c4d3fbf9ac433991903dd3fb9a728fc680ae93b22bbedd9');assert.equal(bhr.helper.sha256,'539c67c2af6cc34022327a6c201df41fbda600b35684d9b0455a8051efc00981');
const binderSha='c2bf39d4e6c177374a1f9331f1bcb06b04993bb1000777dff80319532b5e9978',buildDriverSha='1704796d260e46bb7f9bedd28e2f951a0d4bda0c1d70bef655cf4cf58042dee6';
const suitePath=path.join(base,'results/combined-suite-v7/proof.json'),suiteRaw=fs.readFileSync(suitePath);assert.equal(sha(suiteRaw),suiteSha);
const buildPath=path.join(base,'results/combined-optimized-build-v7/proof.json'),buildRaw=fs.readFileSync(buildPath);assert.equal(sha(buildRaw),buildSha);const build=JSON.parse(buildRaw);assert.equal(build.result,'PASS_COMBINED_OPTIMIZED_BUILD_V7');assert.equal(build.source_manifest_sha256,sourceSha);assert(build.source_final_verified&&build.suite_final_verified);assert(build.binary?.path&&build.binary.sha256);assert.equal(build.builder_sha256,buildDriverSha);assert.equal(build.required_suite.proof_sha256,suiteSha);
const binderPath=path.join(base,'combined-v7-runner-preparation-v2/combined-bindings-v7.mjs');assert.equal(sha(fs.readFileSync(binderPath)),binderSha);const {bindCombined}=await import(pathToFileURL(binderPath).href);assert.equal(sha(fs.readFileSync(binderPath)),binderSha);const core={source_manifest_sha256:sourceSha,suite_proof_sha256:suiteSha,build_proof_sha256:buildSha,candidate_binary_sha256:build.binary.sha256,build_driver_sha256:buildDriverSha},binding=bindCombined(core,sourceSha);binding.verify();assert.equal(binding.manifest.kind,'combined_source_v7');assert.equal(binding.manifest.source_inputs,326);
const preparations=[
  {
    "gate": "native",
    "driver": "C:\\Users\\matth\\Documents\\Codex\\2026-10-11\\https-github-com-morluto-rea-https\\work\\run-combined-findings-acceptance-v7.mjs",
    "sha256": "1bc4bc60513119cde70803bfd00720e64ef1cc9b6da8dce07fcb0a8c5afabaf8",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/adapters\\manifest.json",
    "manifest_sha256": "b593002399fc8439fd6764011691e9c8579a71838195f811d666d861f2bdffe3"
  },
  {
    "gate": "ui-mock",
    "driver": "C:\\Users\\matth\\Documents\\Codex\\2026-10-11\\https-github-com-morluto-rea-https\\work\\run-combined-findings-acceptance-v7.mjs",
    "sha256": "1bc4bc60513119cde70803bfd00720e64ef1cc9b6da8dce07fcb0a8c5afabaf8",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/adapters\\manifest.json",
    "manifest_sha256": "b593002399fc8439fd6764011691e9c8579a71838195f811d666d861f2bdffe3"
  },
  {
    "gate": "browser-native",
    "driver": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/browser\\run-sitemap-findings-browser-combined-v7.mjs",
    "sha256": "e94f24449c2a1b705501bab1b80fcf8b57088888d4f266f316face8d44c397db",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/browser\\manifest.json",
    "manifest_sha256": "0f17a452745e9fae45a8d350a13c13249fce93595406f375c108f25497657f16"
  },
  {
    "gate": "physical-findings",
    "driver": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/physical\\native-finding-physical-combined-v7.mjs",
    "sha256": "ff10d9a7b6b5258faa486ac73cc38de5b89236300658de3f39ab69b2801a6ac3",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/physical\\manifest.json",
    "manifest_sha256": "9bed04bbb1f8b2aea4c3d7cdde9b93d3510fdfa34b673a97ae2e639b0e3efac8"
  },
  {
    "gate": "file-sdk",
    "driver": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/file-sdk\\mcp-file-tools-sdk.mjs",
    "sha256": "e5dee53db3862540cfee4f25367a047630138511a2a8a00b8044ec55822d373b",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/file-sdk\\manifest.json",
    "manifest_sha256": "69174c5635c833ab25df1590e19016cbabd7cf8b0940569b3b9156a664678d22"
  },
  {
    "gate": "root-junction",
    "driver": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/root-junction\\root-junction-cli.mjs",
    "sha256": "1ad30857ffb8fbd524558952617e4905c62fbb0398c8da0cc6269ce7282b532d",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source7-native-preparation-v2/root-junction\\manifest.json",
    "manifest_sha256": "0e2047bec610fb3dec8c5769cbac8cc7506703d377b4769a2bda26ebf530ec79"
  }
];
for(const p of preparations){assert.equal(sha(fs.readFileSync(p.driver)),p.sha256);const raw=fs.readFileSync(p.path);assert.equal(sha(raw),p.manifest_sha256);const m=JSON.parse(raw),rows=m.files??m.entries??m.payloads;assert(Array.isArray(rows)&&rows.length>0);for(const f of rows){assert(typeof f.path==='string'&&!path.isAbsolute(f.path)&&!f.path.split(/[\\/]/).includes('..'));const data=fs.readFileSync(path.join(path.dirname(p.path),f.path));assert.equal(data.length,f.bytes);assert.equal(sha(data),f.sha256);}}
const chrome='C:/Program Files/Google/Chrome/Application/chrome.exe',chromeSha=sha(fs.readFileSync(chrome)),worker=JSON.parse(fs.readFileSync(path.join(base,'results/combined-suite-v7/owned-worker.json')));assert.equal(chromeSha,worker.chrome_sha256,'Use the actually suite-tested Chrome identity');
const now=new Date(),notBefore=new Date(now.getTime()-1000).toISOString(),expires=new Date(now.getTime()+2*3600000).toISOString();const pending=[];const add=(name,value)=>{const file=path.join(task,'work',name);assert(!fs.existsSync(file));pending.push({file,value});};
const common={status:'RELEASED',build_mode:'optimized',...core,bindings_sha256:binderSha,not_before:notBefore,expires_at:expires};
for(const gate of ['native','ui-mock'])add('v7-'+gate+'-release.json',{...common,kind:'root_released_combined_findings_acceptance_v7',gate,driver_sha256:'1bc4bc60513119cde70803bfd00720e64ef1cc9b6da8dce07fcb0a8c5afabaf8',output:path.join(base,'results/combined-findings-'+gate+'-optimized-acceptance-v7'),...(gate==='ui-mock'?{chrome_executable:chrome,chrome_sha256:chromeSha}:{})});
add('v7-browser-native-release.json',{...common,kind:'root_released_combined_findings_browser_native_v7',scope:'native_combined_findings_browser_v7',driver_sha256:'e94f24449c2a1b705501bab1b80fcf8b57088888d4f266f316face8d44c397db',original_body_helper_review:bodyHelperReview,preparation_manifest_sha256:'0f17a452745e9fae45a8d350a13c13249fce93595406f375c108f25497657f16',chrome_executable:chrome,chrome_sha256:chromeSha,output:path.join(base,'results/combined-findings-browser-native-optimized-v7')});
add('v7-physical-release.json',{...common,kind:'root_released_combined_finding_physical_v7',scope:'native_combined_finding_csv_json_physical_v7',harness_sha256:'ff10d9a7b6b5258faa486ac73cc38de5b89236300658de3f39ab69b2801a6ac3',suite_proof_sha256:sha(binding.suiteRaw),preparation_manifest_sha256:'9bed04bbb1f8b2aea4c3d7cdde9b93d3510fdfa34b673a97ae2e639b0e3efac8',output:path.join(base,'source7-native-preparation-v2/physical/runs/optimized-v7-acceptance')});
const sdkBinding={binary:binding.binary,binarySha256:binding.binarySHA,sourceRoot:binding.manifest.source_worktree,sourceManifest:{path:path.join(base,'combined-source-v7/manifest.json'),sha256:sourceSha},sourceManifestSha256:sourceSha,sourceFiles:binding.manifest.files.map(f=>({path:f.path,sha256:f.sha256,bytes:f.bytes})),sdkInstallRoot:path.join(task,'outputs/open-seo-spider/scripts'),resultParent:path.join(base,'results'),preparationManifestSha256:'69174c5635c833ab25df1590e19016cbabd7cf8b0940569b3b9156a664678d22',buildProof:{path:buildPath,sha256:sha(buildRaw)}};
add('v7-mcp-file-sdk-binding.json',sdkBinding);add('v7-mcp-root-junction-binding.json',{...sdkBinding,preparationManifestSha256:'0e2047bec610fb3dec8c5769cbac8cc7506703d377b4769a2bda26ebf530ec79'});
const rows=[];for(const {file,value}of pending){const bytes=Buffer.from(JSON.stringify(value,null,2)+'\n');fs.writeFileSync(file,bytes,{flag:'wx'});rows.push({path:file,sha256:sha(bytes),bytes:bytes.length});}binding.verify();
const receipt={original_body_helper_review:bodyHelperReview,kind:'root_concrete_v7_native_bindings_from_actual_optimized_build',recorded_at:new Date().toISOString(),...core,bindings_sha256:binderSha,compiled_source_commit:binding.manifest.source_commit,writer:{path:fileURLToPath(import.meta.url),sha256:sha(fs.readFileSync(fileURLToPath(import.meta.url)))},binary:{path:binding.binary,sha256:binding.binarySHA,bytes:build.binary.bytes},expires_at:expires,rows,native_sdk_browser_physical_junction:'NOT_RUN',installed_runtime:'UNCHANGED_NO_INSTALL',manufacturer_parity:'NOT_PROVEN'};const receiptPath=path.join(task,'work/v7-native-bindings-issued.json');assert(!fs.existsSync(receiptPath));fs.writeFileSync(receiptPath,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({source:sourceSha,binary:receipt.binary,bindings:rows.map(r=>({path:r.path,sha256:r.sha256})),native_execution:'NOT_RUN'}));
