// Execute the independent Rust crawler on retained, owned heading fixtures.
// Retained ordinary reference exports are hash-checked; the original is not run.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {pages,startFixture} from './reference-fixture.mjs';
import {csv} from './reference-normalize.mjs';

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const app=process.env.SPIDER_APP_URL||'http://127.0.0.1:8787';
const binary=resolve(process.env.SPIDER_BINARY||resolve(root,'target/debug',process.platform==='win32'?'open-seo-spider.exe':'open-seo-spider'));
const sha=value=>createHash('sha256').update(value).digest('hex');
const reference=JSON.parse(await readFile(resolve(root,'docs/parity/reference-headings/latest-run.json'),'utf8'));
const fixtureManifest=JSON.parse(await readFile(resolve(root,'docs/parity/reference-headings/fixture-cases.json'),'utf8'));
assert.equal(reference.status,'REFERENCE_HEADING_MEMBERSHIP_OBSERVED');
assert.equal(sha(JSON.stringify(fixtureManifest.html_cases)),reference.fixture_sha256);
assert.equal(fixtureManifest.fixture_sha256,reference.fixture_sha256);
const pathOf=value=>new URL(value).pathname;
const referenceSets={},files=[];
for(const name of ['H1:All','H1:Non-Sequential','H2:All','H2:Non-Sequential']){
  const observation=reference.observations[name];assert.ok(observation);
  const bytes=await readFile(resolve(root,reference.output_directory,observation.file));
  assert.equal(sha(bytes),observation.sha256);
  referenceSets[name]=new Set(csv(bytes.toString('utf8')).map(row=>pathOf(row.Adresse??row.Address)));
  files.push({argument:name,file:observation.file,sha256:sha(bytes),row_count:referenceSets[name].size});
}
pages.clear();
for(const item of fixtureManifest.html_cases){
  assert.equal(sha(item.response.html),item.html_sha256);
  pages.set(item.path,item.response);
}
const fixture=await startFixture();
const request=async(route,options={})=>{
  const response=await fetch(app+route,options);assert.equal(response.status,200,`${route}: ${await response.clone().text()}`);return response.json();
};
try{
  const job=await request('/api/crawls',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({seed_url:fixture.origin+'/',delay_ms:0,concurrency:4,discover_sitemaps:false})});
  const start=Date.now();let state;
  while(Date.now()-start<30000){state=await request(`/api/crawls/${job.id}`);if(['completed','cancelled','failed'].includes(state.status))break;await new Promise(done=>setTimeout(done,50));}
  assert.equal(state?.status,'completed');
  const report=await request(`/api/crawls/${job.id}/project.json`);
  const captured=new Map(report.pages.map(page=>[pathOf(page.url),page]));
  const observations=fixtureManifest.html_cases.flatMap(item=>{
    const page=captured.get(item.path);assert.ok(page,`Missing native fixture ${item.path}`);
    assert.ok(page.dom_metadata?.complete,`Incomplete DOM ${item.path}`);
    assert.equal(page.final_status,200);
    return ['h1','h2'].map(level=>{
      assert.ok(referenceSets[level.toUpperCase()+':All'].has(item.path),'Reference All cohort is required before absence means negative membership');
      const expected=referenceSets[level.toUpperCase()+':Non-Sequential'].has(item.path);
      const native=report.issues.some(issue=>issue.url===page.url&&issue.code===level+'_nonsequential');
      return {path:item.path,level,expected,native,status:expected===native?'MATCH':'DIFFERENT',observed_levels:page.dom_metadata.headings.map(heading=>heading.tag_level)};
    });
  });
  const counts=observations.reduce((result,item)=>(result[item.status]++,result),{MATCH:0,DIFFERENT:0});
  const proof={schema_version:1,tested_at:new Date().toISOString(),result:counts.DIFFERENT?'DIFFERENCES_RETAINED':'PASS_BOUNDED_NATIVE_HEADING_MEMBERSHIP',full_vendor_parity:'NOT_PROVEN',app_url:app,binary_sha256:sha(await readFile(binary)),native_job_id:job.id,fixture_sha256:reference.fixture_sha256,reference_run_at:reference.observed_at,files,html_cases:fixtureManifest.html_cases.length,counts,observations,requests:fixture.requests,limitations:['Only literal heading order in this bounded ordinary Windows HTTP fixture is compared.','This does not establish computed visibility, accessibility outline, parser equivalence on arbitrary malformed HTML or whole-feature parity.','The more general heading_level_skipped finding is an independent rule and is excluded from this membership comparison.']};
  await writeFile(resolve(root,'docs/parity/native-heading-comparison.json'),JSON.stringify(proof,null,2)+'\n');
  console.log(JSON.stringify({result:proof.result,html_cases:proof.html_cases,counts}));
  assert.equal(counts.DIFFERENT,0);
}finally{await new Promise(done=>fixture.server.close(done));}
