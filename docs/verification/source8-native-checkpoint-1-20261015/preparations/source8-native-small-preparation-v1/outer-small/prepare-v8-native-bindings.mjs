import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {pathToFileURL,fileURLToPath} from 'node:url';
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',task='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https',sha=b=>createHash('sha256').update(b).digest('hex');
assert(!process.env.NODE_OPTIONS&&!process.env.NODE_PATH&&process.execArgv.length===0);assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');
assert.equal(path.resolve(fileURLToPath(import.meta.url)),path.join(task,'work/prepare-v8-native-bindings.mjs'),'Promote exact reviewed writer');
assert.equal(process.argv.length,12,'Use --manifest-sha ROOT64 --suite-proof-sha ROOT64 --build-proof-sha ROOT64 --body-helper-review ABS --body-helper-review-sha ROOT64');assert.equal(process.argv[2],'--manifest-sha');assert.equal(process.argv[4],'--suite-proof-sha');assert.equal(process.argv[6],'--build-proof-sha');assert.equal(process.argv[8],'--body-helper-review');assert.equal(process.argv[10],'--body-helper-review-sha');assert(path.isAbsolute(process.argv[9]));assert.match(process.argv[11],/^[a-f0-9]{64}$/);
const [sourceSha,suiteSha,buildSha]=[process.argv[3],process.argv[5],process.argv[7]];assert.equal(sourceSha,'58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720');for(const s of [sourceSha,suiteSha,buildSha])assert.match(s,/^[a-f0-9]{64}$/);const bodyHelperReview={path:process.argv[9],sha256:process.argv[11]};const brs=fs.lstatSync(bodyHelperReview.path);assert(brs.isFile()&&!brs.isSymbolicLink()&&brs.size<=1024*1024);const brr=fs.readFileSync(bodyHelperReview.path);assert.equal(sha(brr),bodyHelperReview.sha256);const bhr=JSON.parse(brr);assert.equal(bhr.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(bhr.findings,[]);assert.equal(bhr.proposal_manifest.sha256,'d3dc9c5fb5f362a03c4d3fbf9ac433991903dd3fb9a728fc680ae93b22bbedd9');assert.equal(bhr.helper.sha256,'539c67c2af6cc34022327a6c201df41fbda600b35684d9b0455a8051efc00981');
const binderSha='c58f1984c6b6593d384915157afdcdae206acd1bb8af72279a9ee82702e1b7e7',buildDriverSha='e48e2cadc361a100eecbbed5465bf4436d28f7fa19068682550932f320e1c547';
const suitePath=path.join(base,'results/combined-suite-v8/proof.json'),suiteRaw=fs.readFileSync(suitePath);assert.equal(sha(suiteRaw),suiteSha);
const buildPath=path.join(base,'results/combined-optimized-build-v8/proof.json'),buildRaw=fs.readFileSync(buildPath);assert.equal(sha(buildRaw),buildSha);const build=JSON.parse(buildRaw);assert.equal(build.result,'PASS_COMBINED_OPTIMIZED_BUILD_V8');assert.equal(build.source_manifest_sha256,sourceSha);assert(build.source_final_verified&&build.suite_final_verified);assert(build.binary?.path&&build.binary.sha256);assert.equal(build.builder_sha256,buildDriverSha);assert.equal(build.required_suite.proof_sha256,suiteSha);
const binderPath=path.join(base,'combined-v8-runner-preparation-v4/combined-bindings-v8.mjs');assert.equal(sha(fs.readFileSync(binderPath)),binderSha);const {bindCombined}=await import(pathToFileURL(binderPath).href);assert.equal(sha(fs.readFileSync(binderPath)),binderSha);const core={source_manifest_sha256:sourceSha,suite_proof_sha256:suiteSha,build_proof_sha256:buildSha,candidate_binary_sha256:build.binary.sha256,build_driver_sha256:buildDriverSha},binding=bindCombined(core,sourceSha);binding.verify();assert.equal(binding.manifest.kind,'combined_source_v8');assert.equal(binding.manifest.source_inputs,335);
const preparations=[
  {
    "gate": "native",
    "driver": "C:\\Users\\matth\\Documents\\Codex\\2026-10-11\\https-github-com-morluto-rea-https\\work\\run-combined-findings-acceptance-v8.mjs",
    "sha256": "8075ea60abfcb15117254da6b62d485060e007fec5860d4e5a94cc07f95bdec5",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\adapters\\manifest.json",
    "manifest_sha256": "65f6022ce6a8edc5d68773b21ab3257b08bfa945f1909a0a594c8a2d48f9d13d"
  },
  {
    "gate": "ui-mock",
    "driver": "C:\\Users\\matth\\Documents\\Codex\\2026-10-11\\https-github-com-morluto-rea-https\\work\\run-combined-findings-ui-mock-acceptance-v8.mjs",
    "sha256": "0c6942c893d375cac6c9a356e05741ac2dd2d38d7bb043d18a605d4be3e0e3ab",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\adapters\\manifest.json",
    "manifest_sha256": "65f6022ce6a8edc5d68773b21ab3257b08bfa945f1909a0a594c8a2d48f9d13d"
  },
  {
    "gate": "browser-native",
    "driver": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\browser\\run-sitemap-findings-browser-combined-v8.mjs",
    "sha256": "254e443f5b69af4a811913b2840681cd7645bd013981d966b025ef1247b58269",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\browser\\manifest.json",
    "manifest_sha256": "a8329c77ad797396cc628a8c1e19cda6d9906ee8fba1caae002419c2f7f70974"
  },
  {
    "gate": "physical-findings",
    "driver": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\physical\\native-finding-physical-combined-v8.mjs",
    "sha256": "e1225460134ab5d84098ab5c3e775c368aff2935e5013d928cfff2e0f29d4139",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\physical\\manifest.json",
    "manifest_sha256": "3f0f670dc0cda65b8cf68b0174fa7efdf21740a842ebf6a3a46637cb3aaae044"
  },
  {
    "gate": "file-sdk",
    "driver": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\file-sdk\\mcp-file-tools-sdk.mjs",
    "sha256": "963db114159fb5c9263f675ea89ed2ec6aa0fc5b936de1a9ee2068e6834f278d",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\file-sdk\\manifest.json",
    "manifest_sha256": "c796a9af1cb4285c6f69e55325f0a92a935c100d22120d075648355f41f310e8"
  },
  {
    "gate": "root-junction",
    "driver": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\root-junction\\root-junction-cli.mjs",
    "sha256": "d68cc16fcdb9e94508f34ab83de551e4a4769bbc36dfc6dab798b4b9b22f2c01",
    "path": "E:\\CodexWork\\open-seo-spider-01a12ac3-20261013\\source8-native-small-preparation-v1\\root-junction\\manifest.json",
    "manifest_sha256": "7dc9271382a7ae4dc584f099669742b0b15178d0ad112f64bddfa910483b9f85"
  }
];
for(const p of preparations){assert.equal(sha(fs.readFileSync(p.driver)),p.sha256);const raw=fs.readFileSync(p.path);assert.equal(sha(raw),p.manifest_sha256);const m=JSON.parse(raw),rows=m.files??m.entries??m.payloads;assert(Array.isArray(rows)&&rows.length>0);for(const f of rows){assert(typeof f.path==='string'&&!path.isAbsolute(f.path)&&!f.path.split(/[\\/]/).includes('..'));const data=fs.readFileSync(path.join(path.dirname(p.path),f.path));assert.equal(data.length,f.bytes);assert.equal(sha(data),f.sha256);}}
const chrome='C:/Program Files/Google/Chrome/Application/chrome.exe',chromeSha=sha(fs.readFileSync(chrome)),worker=JSON.parse(fs.readFileSync(path.join(base,'results/combined-suite-v8/owned-worker.json')));assert.equal(chromeSha,worker.chrome_sha256,'Use the actually suite-tested Chrome identity');
const now=new Date(),notBefore=new Date(now.getTime()-1000).toISOString(),expires=new Date(now.getTime()+2*3600000).toISOString();const pending=[];const add=(name,value)=>{const file=path.join(task,'work',name);assert(!fs.existsSync(file));pending.push({file,value});};
const common={status:'RELEASED',build_mode:'optimized',...core,bindings_sha256:binderSha,not_before:notBefore,expires_at:expires};
add('v8-native-release.json',{...common,kind:'root_released_combined_findings_acceptance_v8',gate:'native',driver_sha256:'8075ea60abfcb15117254da6b62d485060e007fec5860d4e5a94cc07f95bdec5',output:path.join(base,'results/combined-findings-native-optimized-acceptance-v8')});
add('v8-ui-mock-release.json',{...common,kind:'root_released_combined_findings_ui_mock_acceptance_v8',gate:'ui-mock',driver_sha256:'0c6942c893d375cac6c9a356e05741ac2dd2d38d7bb043d18a605d4be3e0e3ab',output:path.join(base,'results/combined-findings-ui-mock-optimized-acceptance-v8'),chrome_executable:chrome,chrome_sha256:chromeSha});
add('v8-browser-native-release.json',{...common,kind:'root_released_combined_findings_browser_native_v8',scope:'native_combined_findings_browser_v8',driver_sha256:'254e443f5b69af4a811913b2840681cd7645bd013981d966b025ef1247b58269',original_body_helper_review:bodyHelperReview,preparation_manifest_sha256:'a8329c77ad797396cc628a8c1e19cda6d9906ee8fba1caae002419c2f7f70974',chrome_executable:chrome,chrome_sha256:chromeSha,output:path.join(base,'results/combined-findings-browser-native-optimized-v8')});
add('v8-physical-release.json',{...common,kind:'root_released_combined_finding_physical_v8',scope:'native_combined_finding_csv_json_physical_v8',harness_sha256:'e1225460134ab5d84098ab5c3e775c368aff2935e5013d928cfff2e0f29d4139',suite_proof_sha256:sha(binding.suiteRaw),preparation_manifest_sha256:'3f0f670dc0cda65b8cf68b0174fa7efdf21740a842ebf6a3a46637cb3aaae044',output:path.join(base,'source8-native-small-preparation-v1/physical/runs/optimized-v8-acceptance')});
const sdkBinding={binary:binding.binary,binarySha256:binding.binarySHA,sourceRoot:binding.manifest.source_worktree,sourceManifest:{path:path.join(base,'combined-source-v8/manifest.json'),sha256:sourceSha},sourceManifestSha256:sourceSha,sourceFiles:binding.manifest.files.map(f=>({path:f.path,sha256:f.sha256,bytes:f.bytes})),sdkInstallRoot:path.join(task,'outputs/open-seo-spider/scripts'),resultParent:path.join(base,'results'),preparationManifestSha256:'c796a9af1cb4285c6f69e55325f0a92a935c100d22120d075648355f41f310e8',buildProof:{path:buildPath,sha256:sha(buildRaw)}};
add('v8-mcp-file-sdk-binding.json',sdkBinding);add('v8-mcp-root-junction-binding.json',{...sdkBinding,preparationManifestSha256:'7dc9271382a7ae4dc584f099669742b0b15178d0ad112f64bddfa910483b9f85'});
const rows=[];for(const {file,value}of pending){const bytes=Buffer.from(JSON.stringify(value,null,2)+'\n');fs.writeFileSync(file,bytes,{flag:'wx'});rows.push({path:file,sha256:sha(bytes),bytes:bytes.length});}binding.verify();
const receipt={original_body_helper_review:bodyHelperReview,kind:'root_concrete_v8_native_bindings_from_actual_optimized_build',recorded_at:new Date().toISOString(),...core,bindings_sha256:binderSha,compiled_source_commit:binding.manifest.source_commit,writer:{path:fileURLToPath(import.meta.url),sha256:sha(fs.readFileSync(fileURLToPath(import.meta.url)))},binary:{path:binding.binary,sha256:binding.binarySHA,bytes:build.binary.bytes},expires_at:expires,rows,native_sdk_browser_physical_junction:'NOT_RUN',installed_runtime:'UNCHANGED_NO_INSTALL',manufacturer_parity:'NOT_PROVEN'};const receiptPath=path.join(task,'work/v8-native-bindings-issued.json');assert(!fs.existsSync(receiptPath));fs.writeFileSync(receiptPath,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({source:sourceSha,binary:receipt.binary,bindings:rows.map(r=>({path:r.path,sha256:r.sha256})),native_execution:'NOT_RUN'}));
