import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';

export const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
export const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
export const SOURCE=path.join(BASE,'sitemap-findings-surfaces-source-v1');
export const SOURCE_SHA='69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751';
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
  assert.equal(release.kind,'root_released_sitemap_findings_browser_native_v1');assert.equal(release.build_mode,mode);assert.equal(release.source_manifest_sha256,SOURCE_SHA);
  assert(Number.isFinite(Date.parse(release.expires_at))&&Date.parse(release.expires_at)>now,'Expired execution release');
  assert.equal(sha(bytes(driverPath)),release.driver_sha256);
  const preparationRaw=hash(path.join(preparationRoot,'manifest.json'),release.preparation_manifest_sha256,1024*1024),preparation=JSON.parse(preparationRaw);
  for(const file of preparation.files){assert(!path.isAbsolute(file.path)&&!file.path.split(/[\\/]/).includes('..'));const data=hash(path.join(preparationRoot,file.path),file.sha256,2*1024*1024);assert.equal(data.length,file.bytes);}
  const sourceRaw=hash(path.join(SOURCE,'manifest.json'),SOURCE_SHA,2*1024*1024),manifest=JSON.parse(sourceRaw);
  assert.equal(manifest.files.length,297);assert.equal(manifest.required_test_names,388);assert.equal(manifest.minimum_expected_workspace_tests,1170);assert.equal(manifest.new_authored_test_functions,47);
  for(const file of manifest.files){assert(!path.isAbsolute(file.path)&&!file.path.split(/[\\/]/).includes('..'));const data=hash(path.join(SOURCE,file.path),file.sha256,64*1024*1024);assert.equal(data.length,file.bytes);}
  const suitePath=path.join(BASE,'results/sitemap-findings-surfaces-suite-v1/proof.json'),suiteRaw=bytes(suitePath,4*1024*1024),suite=JSON.parse(suiteRaw);
  assert.equal(suite.result,'PASS_LOCAL_CURRENT_WORKSPACE_REAL_RENDERER_CLIPPY_FMT');assert.equal(suite.source_manifest_sha256,SOURCE_SHA);assert.equal(suite.files,297);assert.equal(suite.required_test_names,388);assert(suite.passed>=1170&&suite.ignored===0);assert.equal(suite.owned_worker_closed,true);assert.deepEqual(suite.failures,[]);
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
  const buildDir=path.join(BASE,`results/sitemap-findings-surfaces-${mode}-build-v2`),buildPath=path.join(buildDir,'proof.json'),buildRaw=hash(buildPath,release.build_proof_sha256,4*1024*1024),build=JSON.parse(buildRaw);
  assert.equal(build.result,`PASS_LOCAL_CURRENT_${mode.toUpperCase()}_BUILD`);assert.equal(build.mode,mode);assert.equal(build.source_manifest_sha256,SOURCE_SHA);assert.equal(build.input_files,297);assert.deepEqual(build.compiler_profile,mode==='optimized'?{lto:'thin',codegen_units:1,strip:true,declared_in:'Cargo.toml'}:{debug:0,incremental:false});assert.equal(build.source_and_checkout_identity_verified,true);assert.deepEqual(build.failures,[]);
  assert.equal(path.resolve(build.required_workspace.path),path.resolve(suitePath));assert.equal(build.required_workspace.sha256,sha(suiteRaw));assert.equal(build.required_workspace.passed,suite.passed);assert.equal(build.required_workspace.ignored,0);assert.equal(build.required_workspace.required_test_names,388);assert.equal(build.required_workspace.owned_renderer_gate_count,4);assert.equal(build.required_workspace.owned_renderer_gates_verified,true);
  const buildRuns=[['cargo-version',['--version','--verbose']],['rustc-version',['--version','--verbose']],['build',['build','--locked','-p','open-seo-spider',...(mode==='optimized'?['--release']:[])]]];
  assert.deepEqual(build.runs.map(x=>x.name),buildRuns.map(x=>x[0]));for(const [index,[name,args]] of buildRuns.entries())actualRun(build.runs[index],args,path.join(buildDir,name+'.log'));
  const binary=path.join(buildDir,'open-seo-spider.exe');assert.equal(path.resolve(build.binary.path),path.resolve(binary));assert.equal(build.binary.sha256,release.candidate_binary_sha256);const exe=hash(binary,build.binary.sha256,256*1024*1024);assert.equal(exe.length,build.binary.bytes);assert.equal(exe[0],0x4d);assert.equal(exe[1],0x5a);
  assert(path.isAbsolute(release.chrome_executable));const chrome=fs.realpathSync(release.chrome_executable);hash(chrome,release.chrome_sha256,512*1024*1024);
  const runtime=path.join(TASK,'outputs/open-seo-spider'),playwrightRoot=path.join(runtime,'renderer/node_modules/playwright-core');const packageRaw=hash(path.join(playwrightRoot,'package.json'),'1d6b6f80ffb0c52865af0f9fe01abf2140df019e49ae6ac134605e462d583f3a',128*1024);hash(path.join(playwrightRoot,'index.js'),'a58fb2cec4293e7dcf73c58a179898a4f3986666451a19da4e731fa3af63265b',128*1024);assert.equal(JSON.parse(packageRaw).version,'1.64.0');
  const output=path.join(BASE,`results/sitemap-findings-browser-native-${mode}-v1`);assert.equal(path.resolve(release.output),path.resolve(output));assert(!fs.existsSync(output),'Output must be absent, never overwritten');
  return {mode,output,binary,binarySha:build.binary.sha256,chrome,chromeSha:release.chrome_sha256,playwrightRoot,sourceRoot:SOURCE,sourceManifest:manifest,sourceSha:SOURCE_SHA,suiteSha:sha(suiteRaw),buildSha:sha(buildRaw),releaseSha:sha(bytes(releasePath,65536)),preparationSha:sha(preparationRaw)};
}