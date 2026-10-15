import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';

export const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
export const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
export const SOURCE=path.join(BASE,'sitemap-source-processing-ui-source-v1');
export const SOURCE_SHA='e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';
export const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
export function bytes(file,limit=32*1024*1024){const stat=fs.lstatSync(file);assert(stat.isFile()&&!stat.isSymbolicLink());assert(stat.size<=limit,'Oversized proof/input '+file);return fs.readFileSync(file);}
function hash(file,expected,limit){assert.match(expected,/^[a-f0-9]{64}$/);const data=bytes(file,limit);assert.equal(sha(data),expected,file+' SHA');return data;}
function json(file,limit){return JSON.parse(bytes(file,limit));}
function actualRun(run,args,log){assert.deepEqual(run.args,args);assert.equal(run.actual_close_event,true);assert.equal(run.spawn_error,null);assert.deepEqual(run.exit,{code:0,signal:null});return hash(log,run.log_sha256,128*1024*1024);}
export function preflight({mode,releasePath,driverPath,preparationRoot}){
  // No mkdir, subprocess, dynamic runtime import or listener is permitted above this gate.
  assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed','Compute HOLD must be explicitly released');
  assert(['debug','optimized'].includes(mode));assert(path.isAbsolute(releasePath));const [nodeMajor,nodeMinor]=process.versions.node.split('.').map(Number);assert(nodeMajor>22||nodeMajor===22&&nodeMinor>=16,'node:sqlite readonly-capable Node >=22.16 required');
  const release=json(releasePath,65536),now=Date.now();
  assert.equal(release.kind,'root_released_sitemap_findings_browser_ui_native_v1');assert.equal(release.build_mode,mode);assert.equal(release.source_manifest_sha256,SOURCE_SHA);
  assert(Number.isFinite(Date.parse(release.expires_at))&&Date.parse(release.expires_at)>now,'Expired execution release');
  assert.equal(sha(bytes(driverPath)),release.driver_sha256);
  const preparationRaw=hash(path.join(preparationRoot,'manifest.json'),release.preparation_manifest_sha256,1024*1024),preparation=JSON.parse(preparationRaw);
  for(const file of preparation.files){assert(!path.isAbsolute(file.path)&&!file.path.split(/[\\/]/).includes('..'));const data=hash(path.join(preparationRoot,file.path),file.sha256,2*1024*1024);assert.equal(data.length,file.bytes);}
  const sourceRaw=hash(path.join(SOURCE,'manifest.json'),SOURCE_SHA,2*1024*1024),manifest=JSON.parse(sourceRaw);
  assert.equal(manifest.files.length,297);assert.equal(manifest.required_test_names,388);assert.equal(manifest.minimum_expected_workspace_tests,1170);assert.equal(manifest.new_authored_test_functions,0);assert.equal(manifest.approved_product_bindings.length,2);
  for(const file of manifest.files){assert(!path.isAbsolute(file.path)&&!file.path.split(/[\\/]/).includes('..'));const data=hash(path.join(SOURCE,file.path),file.sha256,64*1024*1024);assert.equal(data.length,file.bytes);}
  // Exact lineage logic copied from selected UI Builder1c72, not old694 admission.
  const root=BASE,task=TASK,repo=path.join(BASE,'sitemap-source-processing-ui-v1'),source=SOURCE;
function verifyUiLineage() {
  const baseSha = '69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751';
  const uiSha = 'e9bbc54573247e028dcb47937f80eaa1c1918240a810d8e796874464bf5cc7f5';
  const baseCommit = 'f43efc17d1393645a9eb8c8b8510743f3dfedb5c';
  const uiCommit = 'f4ed4b9bb7c760755ec7e448aac660cfb6063ade';
  const basePublicationSha = 'b71188d145a8e720bd9d6c9aaf4720c03c20cced7f406839264e5432e85e84ff';
  const uiPublicationSha = '4316704b11e47d7f49bb541e9b1db9b1c31ac9c53d5b68dd2c47ab339b4bc24b';
  const baseDirectory = path.join(root, 'sitemap-findings-surfaces-source-v1');
  const baseRaw = fs.readFileSync(path.join(baseDirectory, 'manifest.json'));
  assert.equal(sha(baseRaw), baseSha, 'Genuine base694 manifest required');
  assert.equal(sha(fs.readFileSync(path.join(source, 'manifest.json'))), uiSha, 'Current UI frozen manifest');
  const base = JSON.parse(baseRaw);
  const basePublicationRaw = fs.readFileSync(path.join(task, 'work/sitemap-findings-surfaces-published-v1.json'));
  const uiPublicationRaw = fs.readFileSync(path.join(task, 'work/sitemap-source-processing-ui-published-v1.json'));
  assert.equal(sha(basePublicationRaw), basePublicationSha, 'Fixed base publication receipt');
  assert.equal(sha(uiPublicationRaw), uiPublicationSha, 'Fixed UI publication receipt');
  const basePublication = JSON.parse(basePublicationRaw), uiPublication = JSON.parse(uiPublicationRaw);
  assert.equal(basePublication.commit, baseCommit); assert.equal(basePublication.source_manifest_sha256, baseSha); assert.equal(basePublication.repository.isPrivate, true);
  assert.equal(uiPublication.commit, uiCommit); assert.equal(uiPublication.source_manifest_sha256, uiSha); assert.equal(uiPublication.repository.isPrivate, true);
  assert.equal(uiPublication.base_source_commit, baseCommit); assert.equal(uiPublication.base_source_manifest_sha256, baseSha);
  assert.equal(uiPublication.source_inputs, 297); assert.equal(uiPublication.unchanged_base_inputs, 295); assert.equal(uiPublication.changed_ui_inputs, 2);
  assert.equal(uiPublication.new_authored_rust_tests, 0); assert.equal(uiPublication.inherited_source_authored_test_expectations, 376);
  assert.equal(uiPublication.required_test_names, 388); assert.equal(uiPublication.minimum_expected_workspace_tests, 1170);
  assert.equal(uiPublication.source_preparation_proof_sha256, 'b71c0b855945f41958a68bf168f7a2a00fd51cb15d17b5c7157045fcefe2f458');
  assert.equal(base.files.length, 297); assert.equal(base.new_authored_test_functions, 47); assert.equal(base.approved_product_bindings.length, 38);
  assert.equal(base.required_test_names, 388); assert.equal(base.minimum_expected_workspace_tests, 1170);
  assert.equal(manifest.base_source_commit, baseCommit); assert.equal(manifest.base_source_manifest_sha256, baseSha);
  assert.equal(manifest.base_inputs, 297); assert.equal(manifest.source_inputs, 297); assert.equal(manifest.unchanged_inputs, 295);
  assert.equal(manifest.new_authored_test_functions, 0); assert.equal(manifest.inherited_source_authored_test_expectations, 376);
  assert.equal(path.resolve(manifest.source_worktree), path.resolve(repo));
  assert.deepEqual(manifest.required_tests, base.required_tests, 'All inherited grouped names/order remain exact');
  assert.equal(manifest.review.sha256, '3e67b838a3f90a0381b274b85d9fd51cc41fcc435b71f15377b5e53d40fbbb5f');
  assert.equal(sha(fs.readFileSync(path.join(root, 's4-source-processing-ui-review-v1/receipt.json'))), manifest.review.sha256);
  const expectedUI = [
    {path: 'web/app.js', before_sha256: '84df9b724802d86d69262d98dcee10e22d465c7a6583fc034b438df452bb0bb9', after_sha256: 'b1ec57080fec11529e89b21ce07471ea3ddc2141d4f64ae79ffc236c9b5af3f0', bytes: 379587},
    {path: 'web/index.html', before_sha256: 'f5f494781f91029f1b64ace3c0f1ddcba742014690e0d4ee0dbbef53ebf79cff', after_sha256: 'ed8b3d0a624d02b428a3a7430845b417eca73a34bd3ecd6b0d5899d46bd15e77', bytes: 82970},
  ];
  assert.deepEqual(manifest.approved_product_bindings, expectedUI, 'Only the exact two reviewed UI payloads');
  const baseFiles = new Map(base.files.map(file => [file.path, file]));
  const uiFiles = new Map(manifest.files.map(file => [file.path, file]));
  assert.equal(baseFiles.size, 297); assert.equal(uiFiles.size, 297);
  assert.deepEqual([...baseFiles.keys()].sort(), [...uiFiles.keys()].sort(), 'Identical297 input path set');
  let unchanged = 0, changed = 0;
  const verifyFile = (directory, file) => {
    const current = fs.readFileSync(path.join(directory, file.path));
    assert.equal(current.length, file.bytes, file.path + ' exact bytes');
    assert.equal(sha(current), file.sha256, file.path + ' exact SHA');
  };
  for (const [filePath, previous] of baseFiles) {
    const current = uiFiles.get(filePath), binding = expectedUI.find(item => item.path === filePath);
    if (binding) {
      assert.equal(previous.sha256, binding.before_sha256); assert.equal(current.sha256, binding.after_sha256); assert.equal(current.bytes, binding.bytes);
      assert.notEqual(current.sha256, previous.sha256); changed++;
    } else { assert.deepEqual(current, previous, filePath + ' inherited input unchanged'); unchanged++; }
    verifyFile(baseDirectory, previous); verifyFile(source, current); verifyFile(repo, current);
  }
  assert.equal(unchanged, 295); assert.equal(changed, 2);
  return {base_source_manifest_sha256: baseSha, base_source_commit: baseCommit, base_publication_receipt_sha256: basePublicationSha, ui_source_manifest_sha256: uiSha, ui_source_commit: uiCommit, ui_publication_receipt_sha256: uiPublicationSha, base_inputs: 297, unchanged_inputs: unchanged, changed_ui_inputs: changed, inherited_base_new_rust_tests: 47, inherited_base_approved_bindings: 38, new_ui_rust_tests: 0, approved_ui_bindings: 2, inherited_source_authored_expectations: 376, inherited_required_names: 388, minimum_expected_workspace_tests: 1170};
}
  const sourceUILineage=verifyUiLineage();
  hash(path.join(TASK,'work/run-sitemap-source-ui-suite-v1.mjs'),'b4c0ee42c03b6ade2f34f603bd501862568ed07960579d99cec4c14c1a07d43e',512*1024);
  hash(path.join(TASK,'work/build-sitemap-source-ui-v2.mjs'),'1c72e8b67ef818665b081d91689d91c737a9c95f69f9fde954b72a8323f0bded',512*1024);
  const publicationPath=path.join(TASK,'work/sitemap-source-processing-ui-published-v1.json'),publicationSha='4316704b11e47d7f49bb541e9b1db9b1c31ac9c53d5b68dd2c47ab339b4bc24b';
  const publication=JSON.parse(hash(publicationPath,publicationSha,1024*1024));
  assert.equal(publication.result,'PUSHED_PRIVATE_UI_SOURCE_ONLY_GOAL_ACTIVE');assert.equal(publication.new_rust_tests_executed,0);assert.equal(publication.browser_dom_payload_large_source,'NOT_RUN');assert.equal(publication.compiler_tests_clippy_native_installation,'NOT_RUN_COORDINATED_COMPUTE_HOLD');
  const metadataPublicationPath=path.join(TASK,'work/sitemap-native-preparations-published-v1.json'),metadataPublicationSha='ab9c37420bee6861f1317632dd8fbe90cd16dcf69f662d80cd00d76566c0b795';
  const metadata=JSON.parse(hash(metadataPublicationPath,metadataPublicationSha,1024*1024));
  assert.equal(metadata.result,'PUSHED_PRIVATE_SOURCE_PREPARATIONS_ONLY_GOAL_ACTIVE');assert.equal(metadata.commit,'70f2f45f92a896a12d81b3a46558c2b50f0e8db7');assert.equal(metadata.repository.isPrivate,true);
  assert.equal(metadata.current_ui_source_commit,'f4ed4b9bb7c760755ec7e448aac660cfb6063ade');assert.equal(metadata.current_ui_source_manifest_sha256,SOURCE_SHA);assert.equal(metadata.source_inputs,297);assert.equal(metadata.unchanged_base_inputs,295);assert.equal(metadata.new_ui_bindings,2);assert.equal(metadata.new_ui_rust_tests,0);assert.equal(metadata.native_acceptance_transferred_to_changed_ui,false);assert.equal(metadata.native_runs,0);assert.equal(metadata.compiler_clippy_tests_native_browser_build,'NOT_RUN_COORDINATED_COMPUTE_HOLD');
  const selected=metadata.selected.separate_ui_suite_builder;
  assert.equal(path.resolve(selected.suite_path),path.resolve(TASK,'work/run-sitemap-source-ui-suite-v1.mjs'));assert.equal(selected.suite_sha256,'b4c0ee42c03b6ade2f34f603bd501862568ed07960579d99cec4c14c1a07d43e');
  assert.equal(path.resolve(selected.builder_path),path.resolve(TASK,'work/build-sitemap-source-ui-v2.mjs'));assert.equal(selected.builder_sha256,'1c72e8b67ef818665b081d91689d91c737a9c95f69f9fde954b72a8323f0bded');assert.equal(selected.source_manifest_sha256,SOURCE_SHA);assert.equal(selected.execution,0);assert.equal(selected.native_acceptance_transferred,false);
  assert.equal(metadata.selected.browser_native.sha256,'581ec6ad181d22b48772f034931c37cf87d91bd7deb7bbb8a24090995532a5dc');assert.equal(metadata.selected.browser_native.selected_manifest_sha256,'eddbf80f1e8f95dfe51e562e1ba94aa9d1389930003009c73e808a8d8f303792');assert.equal(metadata.selected.browser_native.execution,0);
  const publicationLineage=Object.freeze({source_ui_lineage:sourceUILineage,ui_publication_path:publicationPath,ui_publication_sha256:publicationSha,ui_checkpoint_result:publication.result,ui_checkpoint_execution:publication.compiler_tests_clippy_native_installation,metadata_publication_path:metadataPublicationPath,metadata_publication_sha256:metadataPublicationSha,metadata_commit:metadata.commit,old_native_acceptance_transferred:false});
  const suitePath=path.join(BASE,'results/sitemap-source-ui-suite-v1/proof.json'),suiteRaw=bytes(suitePath,4*1024*1024),suite=JSON.parse(suiteRaw);
  assert.deepEqual(suite.source_lineage,sourceUILineage);assert.equal(suite.source_authored_delta,0);assert.equal(suite.source_authored_total,376);assert.equal(suite.inherited_source_authored_delta,47);assert.equal(suite.inherited_approved_product_bindings,38);assert.equal(suite.source_ui_bindings,2);assert.equal(suite.result,'PASS_LOCAL_CURRENT_WORKSPACE_REAL_RENDERER_CLIPPY_FMT');assert.equal(suite.source_manifest_sha256,SOURCE_SHA);assert.equal(suite.files,297);assert.equal(suite.required_test_names,388);assert(suite.passed>=1170&&suite.ignored===0);assert.equal(suite.owned_worker_closed,true);assert.deepEqual(suite.failures,[]);
  const renderer=['live_browser_worker_executes_javascript_for_core_crawl','live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness','live_browser_worker_pause_drains_render_and_retains_rendered_discoveries','live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion'];
  assert.deepEqual(suite.owned_renderer_gates,renderer);assert.equal(suite.owned_renderer_gate_count,4);assert.equal(suite.owned_renderer_gates_verified,true);
  const commands=[['format',['fmt','--all','--check']],['workspace-live',['test','--workspace','--locked','--','--include-ignored','--test-threads=1']],['clippy',['clippy','--workspace','--all-targets','--locked','--','-D','warnings']]];
  assert.deepEqual(suite.runs.map(x=>x.name),commands.map(x=>x[0]));
  for(const [index,[name,args]] of commands.entries()){
    const log=actualRun(suite.runs[index],args,path.join(path.dirname(suitePath),name+'.log')).toString('utf8');
    const summaries=[...log.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)].map(x=>({state:x[1],passed:+x[2],failed:+x[3],ignored:+x[4]}));assert.deepEqual(suite.runs[index].summaries,summaries);
    for(const row of summaries){assert.equal(row.state,'ok');assert.equal(row.failed,0);assert.equal(row.ignored,0);assert(Number.isSafeInteger(row.passed));}
    if(name==='workspace-live'){assert(summaries.length);assert.equal(summaries.reduce((n,x)=>n+x.passed,0),suite.passed);for(const group of manifest.required_tests)for(const test of group.names)assert(log.includes('::'+test+' ... ok'),test);for(const test of renderer)assert(log.includes('test '+test+' ... ok'),test);}
  }
  const buildDir=path.join(BASE,`results/sitemap-source-ui-${mode}-build-v2`),buildPath=path.join(buildDir,'proof.json'),buildRaw=hash(buildPath,release.build_proof_sha256,4*1024*1024),build=JSON.parse(buildRaw);
  assert.deepEqual(build.source_lineage,sourceUILineage);assert.equal(build.result,`PASS_LOCAL_CURRENT_${mode.toUpperCase()}_BUILD`);assert.equal(build.mode,mode);assert.equal(build.source_manifest_sha256,SOURCE_SHA);assert.equal(build.input_files,297);assert.deepEqual(build.compiler_profile,mode==='optimized'?{lto:'thin',codegen_units:1,strip:true,declared_in:'Cargo.toml'}:{debug:0,incremental:false});assert.equal(build.source_and_checkout_identity_verified,true);assert.deepEqual(build.failures,[]);
  assert.equal(path.resolve(build.required_workspace.path),path.resolve(suitePath));assert.equal(build.required_workspace.sha256,sha(suiteRaw));assert.equal(build.required_workspace.passed,suite.passed);assert.equal(build.required_workspace.ignored,0);assert.equal(build.required_workspace.required_test_names,388);assert.equal(build.required_workspace.owned_renderer_gate_count,4);assert.equal(build.required_workspace.owned_renderer_gates_verified,true);
  const buildRuns=[['cargo-version',['--version','--verbose']],['rustc-version',['--version','--verbose']],['build',['build','--locked','-p','open-seo-spider',...(mode==='optimized'?['--release']:[])]]];
  assert.deepEqual(build.runs.map(x=>x.name),buildRuns.map(x=>x[0]));for(const [index,[name,args]] of buildRuns.entries())actualRun(build.runs[index],args,path.join(buildDir,name+'.log'));
  const binary=path.join(buildDir,'open-seo-spider.exe');assert.equal(path.resolve(build.binary.path),path.resolve(binary));assert.equal(build.binary.sha256,release.candidate_binary_sha256);const exe=hash(binary,build.binary.sha256,256*1024*1024);assert.equal(exe.length,build.binary.bytes);assert.equal(exe[0],0x4d);assert.equal(exe[1],0x5a);
  assert(path.isAbsolute(release.chrome_executable));const chrome=fs.realpathSync(release.chrome_executable);hash(chrome,release.chrome_sha256,512*1024*1024);
  const runtime=path.join(TASK,'outputs/open-seo-spider'),playwrightRoot=path.join(runtime,'renderer/node_modules/playwright-core');const packageRaw=hash(path.join(playwrightRoot,'package.json'),'1d6b6f80ffb0c52865af0f9fe01abf2140df019e49ae6ac134605e462d583f3a',128*1024);hash(path.join(playwrightRoot,'index.js'),'a58fb2cec4293e7dcf73c58a179898a4f3986666451a19da4e731fa3af63265b',128*1024);assert.equal(JSON.parse(packageRaw).version,'1.64.0');
  const output=path.join(BASE,`results/sitemap-findings-browser-ui-native-${mode}-v1`);assert.equal(path.resolve(release.output),path.resolve(output));assert(!fs.existsSync(output),'Output must be absent, never overwritten');
  return {publicationPath,publicationSha,publicationLineage,metadataPublicationPath,metadataPublicationSha,sourceUILineage,verifyUiLineage,compilerRoot:repo,mode,output,binary,binarySha:build.binary.sha256,chrome,chromeSha:release.chrome_sha256,playwrightRoot,sourceRoot:SOURCE,sourceManifest:manifest,sourceSha:SOURCE_SHA,suiteSha:sha(suiteRaw),buildSha:sha(buildRaw),releaseSha:sha(bytes(releasePath,65536)),preparationSha:sha(preparationRaw)};
}