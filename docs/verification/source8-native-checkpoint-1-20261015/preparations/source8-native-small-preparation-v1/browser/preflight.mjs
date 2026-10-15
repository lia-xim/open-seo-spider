import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';

export const BASE='E:/CodexWork/open-seo-spider-01a12ac3-20261013';
export const TASK='C:/Users/matth/Documents/Codex/2026-10-11/https-github-com-morluto-rea-https';
export const SOURCE=path.join(BASE,'combined-source-v8');
export const BINDINGS_SHA='c58f1984c6b6593d384915157afdcdae206acd1bb8af72279a9ee82702e1b7e7';
export const sha=data=>createHash('sha256').update(data).digest('hex');
export function bytes(file,limit=32*1024*1024){const stat=fs.lstatSync(file);assert(stat.isFile()&&!stat.isSymbolicLink());assert(stat.size<=limit,'Oversized proof/input '+file);const data=fs.readFileSync(file);assert.equal(data.length,stat.size);return data;}
function hash(file,expected,limit){assert.match(expected,/^[a-f0-9]{64}$/);const data=bytes(file,limit);assert.equal(sha(data),expected,file+' SHA');return data;}

export async function preflight({mode,releasePath,driverPath,preparationRoot,expectedManifestSha}){
  // Release, caller, own inputs and exact Binder bytes are checked before its
  // dynamic import; no output/SQLite/runtime dependency/process/listener here.
  assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed','Compute HOLD must be explicitly released');
  assert(!process.env.NODE_OPTIONS&&!process.env.NODE_PATH,'No inherited Node preload or dependency redirect');
  assert.equal(mode,'optimized');assert.match(expectedManifestSha,/^[a-f0-9]{64}$/);assert.equal(expectedManifestSha,'58ab1ff000b9ec5ffa005eeab573a0cde3f4af4bb7c9c132b4d85251ea4de720');
  assert(path.isAbsolute(releasePath));const [major,minor]=process.versions.node.split('.').map(Number);assert(major>22||major===22&&minor>=16,'node:sqlite readonly-capable Node >=22.16 required');
  const own=path.join(BASE,'source8-native-small-preparation-v1/browser');assert.equal(path.resolve(preparationRoot),own);assert.equal(path.resolve(driverPath),path.join(own,'run-sitemap-findings-browser-combined-v8.mjs'));
  const releaseRaw=bytes(releasePath,65536),release=JSON.parse(releaseRaw),now=Date.now();
  assert.equal(release.status,'RELEASED');assert.equal(release.kind,'root_released_combined_findings_browser_native_v8');assert.equal(release.scope,'native_combined_findings_browser_v8');assert.equal(release.build_mode,mode);assert.equal(release.source_manifest_sha256,expectedManifestSha);
  const notBefore=Date.parse(release.not_before),expires=Date.parse(release.expires_at);assert(Number.isFinite(notBefore)&&Number.isFinite(expires)&&notBefore<=now&&now<expires&&expires-notBefore<=4*3600000,'Active bounded Root release required');
  const driverSha=sha(bytes(driverPath,512*1024));assert.equal(driverSha,release.driver_sha256);
  const preparationPath=path.join(own,'manifest.json'),preparationRaw=hash(preparationPath,release.preparation_manifest_sha256,1024*1024),preparation=JSON.parse(preparationRaw),seen=new Set();
  assert.equal(preparation.kind,'browser_combined_rebind_v8');assert(Array.isArray(preparation.files));
  const required=['preflight.mjs','run-sitemap-findings-browser-combined-v8.mjs','browser-flow.mjs','profile-form-flow.mjs','combined-bindings.mjs','browser-membership-flow.mjs','browser-original-body-responsive.mjs'];assert.equal(preparation.files.length,required.length);
  for(const file of preparation.files){assert(required.includes(file.path)&&!seen.has(file.path));seen.add(file.path);const data=hash(path.join(own,file.path),file.sha256,2*1024*1024);assert.equal(data.length,file.bytes);}
  assert.equal(seen.size,required.length);
  const bodyReview=release.original_body_helper_review;assert(bodyReview&&path.isAbsolute(bodyReview.path));const bodyReviewRaw=hash(bodyReview.path,bodyReview.sha256,1024*1024),bodyReviewed=JSON.parse(bodyReviewRaw);assert.equal(bodyReviewed.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(bodyReviewed.findings,[]);assert.equal(bodyReviewed.proposal_manifest.sha256,'d3dc9c5fb5f362a03c4d3fbf9ac433991903dd3fb9a728fc680ae93b22bbedd9');assert.equal(bodyReviewed.helper.sha256,'539c67c2af6cc34022327a6c201df41fbda600b35684d9b0455a8051efc00981');
  const binderPath=path.join(own,'combined-bindings.mjs');assert.equal(release.bindings_sha256,BINDINGS_SHA);hash(binderPath,BINDINGS_SHA,512*1024);
  const {bindCombined}=await import(pathToFileURL(binderPath).href);hash(binderPath,BINDINGS_SHA,512*1024);
  const binding=bindCombined(release,expectedManifestSha);
  assert.equal(path.resolve(binding.manifest.source_worktree),path.join(BASE,'mcp-file-tools-v1'));
  assert(path.isAbsolute(release.chrome_executable));const chrome=fs.realpathSync(release.chrome_executable);hash(chrome,release.chrome_sha256,512*1024*1024);
  const runtime=path.join(TASK,'outputs/open-seo-spider'),playwrightRoot=path.join(runtime,'renderer/node_modules/playwright-core');const packageRaw=hash(path.join(playwrightRoot,'package.json'),'1d6b6f80ffb0c52865af0f9fe01abf2140df019e49ae6ac134605e462d583f3a',128*1024);hash(path.join(playwrightRoot,'index.js'),'a58fb2cec4293e7dcf73c58a179898a4f3986666451a19da4e731fa3af63265b',128*1024);assert.equal(JSON.parse(packageRaw).version,'1.64.0');
  const output=path.join(BASE,'results/combined-findings-browser-native-optimized-v8');assert.equal(path.resolve(release.output),output);assert(!fs.existsSync(output),'Output must be absent, never overwritten');
  function verify(){hash(bodyReview.path,bodyReview.sha256,1024*1024);assert.equal(process.env.OPEN_SEO_COORDINATED_EXECUTION_WINDOW,'confirmed');assert(Date.now()>=notBefore&&Date.now()<expires,'Root release expired');assert.equal(sha(bytes(releasePath,65536)),sha(releaseRaw),'Root release changed/revoked');hash(preparationPath,sha(preparationRaw),1024*1024);hash(driverPath,driverSha,512*1024);for(const file of preparation.files){const data=hash(path.join(own,file.path),file.sha256,2*1024*1024);assert.equal(data.length,file.bytes);}binding.verify();hash(chrome,release.chrome_sha256,512*1024*1024);}
  verify();
  return {publicationLineage:binding.publicationLineage,sourceLineage:binding.sourceLineage,verify,compilerRoot:binding.manifest.source_worktree,mode,output,binary:binding.binary,binarySha:binding.binarySHA,chrome,chromeSha:release.chrome_sha256,playwrightRoot,sourceRoot:SOURCE,sourceManifest:binding.manifest,sourceSha:expectedManifestSha,suiteSha:sha(binding.suiteRaw),buildSha:sha(binding.buildRaw),releaseSha:sha(releaseRaw),preparationSha:sha(preparationRaw),bindingsSha:BINDINGS_SHA};
}
