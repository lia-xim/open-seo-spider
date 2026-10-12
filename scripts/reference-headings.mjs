// Four ordinary reference exports on newly authored local heading fixtures.
// No config/paid/auth/license flags and no vendor profile writes by this runner.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {resolve,dirname,basename} from 'node:path';
import {fileURLToPath} from 'node:url';
import {pages,startFixture} from './reference-fixture.mjs';
import {csv} from './reference-normalize.mjs';

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const output=resolve(root,'docs/parity/reference-headings');
const executable=process.env.OPEN_SEO_REFERENCE_CLI??'C:/Program Files (x86)/Screaming Frog SEO Spider/ScreamingFrogSEOSpiderCli.exe';
const profile=resolve(process.env.USERPROFILE??'','.ScreamingFrogSEOSpider/spider.config');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const cases=[
  {path:'/h1-h2',levels:[1,2],body:'<h1>Primary</h1><h2>Section</h2>',purpose:'ordinary sequential control'},
  {path:'/h2-h1',levels:[2,1],body:'<h2>Early section</h2><h1>Primary</h1>',purpose:'H1 occurs after H2'},
  {path:'/h1-h3-h2',levels:[1,3,2],body:'<h1>Primary</h1><h3>Skipped level</h3><h2>Late section</h2>',purpose:'first H2 follows H3 rather than H1'},
  {path:'/h1-h2-h3-h2',levels:[1,2,3,2],body:'<h1>Primary</h1><h2>First section</h2><h3>Detail</h3><h2>Second section</h2>',purpose:'first H2 sequential; later H2 follows H3'},
  {path:'/h3-h2-no-h1',levels:[3,2],body:'<h3>Detail first</h3><h2>Section</h2>',purpose:'H2 after H3 without any H1'},
  {path:'/repeated-h2',levels:[1,2,2],body:'<h1>Primary</h1><h2>Section one</h2><h2>Section two</h2>',purpose:'same-level sections are a sequential control'},
  {path:'/h2-only',levels:[2],body:'<h2>Only section</h2>',purpose:'first heading is H2 and no H1 exists'},
  {path:'/h3-h1-h2',levels:[3,1,2],body:'<h3>Early detail</h3><h1>Primary</h1><h2>Section</h2>',purpose:'H1 non-first but first H2 follows an H1'},
  {path:'/empty-h1-before-h2',levels:[1,2],body:'<h1></h1><h2>Section</h2>',purpose:'empty H1 tag retains tag-order evidence'},
  {path:'/empty-h2-before-h1',levels:[2,1,2],body:'<h2></h2><h1>Primary</h1><h2>Section</h2>',purpose:'empty early H2 tag vs extracted text eligibility'},
  {path:'/hidden-h2-before-h1',levels:[2,1,2],body:'<h2 hidden>Hidden section</h2><h1>Primary</h1><h2>Section</h2>',purpose:'hidden attribute is a raw HTML observation'},
  {path:'/css-hidden-h2-before-h1',levels:[2,1,2],body:'<h2 style="display:none">CSS hidden section</h2><h1>Primary</h1><h2>Section</h2>',purpose:'default HTTP capture does not prove computed visibility'},
  {path:'/hidden-h1-before-h2',levels:[1,2],body:'<h1 hidden>Hidden primary</h1><h2>Section</h2>',purpose:'hidden first H1 is a tag-order control'},
  {path:'/image-alt-h1-before-h2',levels:[1,2],body:'<h1><img src="/image.png" alt="Image-only primary"></h1><h2>Section</h2>',purpose:'H1 has only image alt, no ordinary text node'},
  {path:'/image-alt-h2-before-h1',levels:[2,1,2],body:'<h2><img src="/image.png" alt="Image-only early section"></h2><h1>Primary</h1><h2>Section</h2>',purpose:'H2 image-alt-only occurs before H1'},
  {path:'/image-alt-only-h1-h2',levels:[1,2],body:'<h1><img src="/image.png" alt="Image-only primary"></h1><h2><img src="/image.png" alt="Image-only section"></h2>',purpose:'both H1 and H2 have image alt only'},
  {path:'/empty-h2-after-h1',levels:[1,2],body:'<h1>Primary</h1><h2></h2>',purpose:'empty H2 after first H1'},
  {path:'/no-headings',levels:[],body:'<p>No heading tags.</p>',purpose:'no matching-level heading exists'},
  {path:'/h3-only',levels:[3],body:'<h3>Only detail</h3>',purpose:'no H1/H2 tags exist'},
  {path:'/aria-heading-before-h1',levels:[1,2],body:'<div role="heading" aria-level="2">ARIA early heading</div><h1>Primary</h1><h2>Section</h2>',purpose:'ARIA heading is independent from literal heading-tag order'},
  {path:'/h1-h3-h1-h2',levels:[1,3,1,2],body:'<h1>First primary</h1><h3>Detail</h3><h1>Second primary</h1><h2>Section</h2>',purpose:'later H1 after another heading vs first-H1 predicate'},
  {path:'/h1-h1-h2',levels:[1,1,2],body:'<h1>First primary</h1><h1>Second primary</h1><h2>Section</h2>',purpose:'distinguish any intervening heading from only higher-numbered headings'},
  {path:'/h1-h2-h1-h2',levels:[1,2,1,2],body:'<h1>First primary</h1><h2>First section</h2><h1>Second primary</h1><h2>Second section</h2>',purpose:'later H1 does not change first H2 order control'},
];
pages.clear();
const html=(path,body)=>`<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Owned heading order ${path}</title><meta name="description" content="Independently authored local heading-order test ${path}"></head><body>${body}</body></html>`;
for(const item of cases)pages.set(item.path,{status:200,headers:{},html:html(item.path,item.body)});
const home={path:'/',levels:[1,2],body:'<h1>Owned heading fixture</h1><h2>Cases</h2>'+cases.map(item=>`<a href="${item.path}">${item.path}</a>`).join('')};
pages.set('/',{status:200,headers:{},html:html('/',home.body)});
const fixture=await startFixture();
const stamp=new Date().toISOString().replace(/[:.]/g,'-');
const runOutput=resolve(output,stamp),snapshot=resolve(root,'../../work/reference-headings-profile',stamp);
const requested=['H1:All','H1:Non-Sequential','H2:All','H2:Non-Sequential'];
const inventory=JSON.parse(await readFile(resolve(root,'docs/parity/reference-cli-inventory.json'),'utf8'));
assert.ok(requested.every(argument=>inventory.export_tab_entries.some(entry=>entry.argument===argument)));
const manifest={schema_version:1,author:'Open SEO Spider independent local fixtures',html_cases:[home,...cases].map(item=>({...item,response:pages.get(item.path),html_sha256:sha(pages.get(item.path).html)})),transport_cases:['/robots.txt','/image.png'],maximum_distinct_paths:28};
manifest.fixture_sha256=sha(JSON.stringify(manifest.html_cases));
assert.ok(manifest.html_cases.length+manifest.transport_cases.length<=30);
const properties=bytes=>new Map(bytes.toString('latin1').split(/\r?\n/).filter(line=>line&&!line.startsWith('#')&&!line.startsWith('!')&&line.includes('=')).map(line=>{const at=line.indexOf('=');return [line.slice(0,at),line.slice(at+1)];}));
const noncomments=bytes=>bytes.toString('latin1').split(/\r?\n/).filter(line=>!line.startsWith('#')&&!line.startsWith('!')).join('\n');
try{
  await mkdir(runOutput,{recursive:true});await mkdir(snapshot,{recursive:true});
  const before=await readFile(profile);await writeFile(resolve(snapshot,'spider.config.before'),before);
  const args=['--headless','--crawl',fixture.origin+'/','--output-folder',runOutput,'--export-format','csv','--export-tabs',requested.join(',')];
  const run=await new Promise((done,reject)=>{const child=spawn(executable,args,{windowsHide:true,stdio:['ignore','pipe','pipe']});const chunks=[];let timedOut=false;const timer=setTimeout(()=>{timedOut=true;child.kill();},120000);child.stdout.on('data',chunk=>chunks.push(chunk));child.stderr.on('data',chunk=>chunks.push(chunk));child.on('error',error=>{clearTimeout(timer);reject(error);});child.on('close',exit_code=>{clearTimeout(timer);done({exit_code,timedOut,text:new TextDecoder('utf8').decode(Buffer.concat(chunks))});});});
  const after=await readFile(profile);await writeFile(resolve(snapshot,'spider.config.after'),after);
  const beforeKeys=properties(before),afterKeys=properties(after);
  const changed_keys=[...new Set([...beforeKeys.keys(),...afterKeys.keys()])].filter(key=>beforeKeys.get(key)!==afterKeys.get(key)).map(key=>({key,before_value_sha256:beforeKeys.has(key)?sha(beforeKeys.get(key)):null,after_value_sha256:afterKeys.has(key)?sha(afterKeys.get(key)):null,classification:/^(analytics\.last_sent|updatechecker\.)/.test(key)?'ORDINARY_ANALYTICS_OR_UPDATE_STATE':'UNMATCHED_PROFILE_KEY_CHANGE_NO_ROLLBACK'}));
  const files=(await readdir(runOutput)).filter(name=>name.endsWith('.csv'));
  const completion=[...run.text.matchAll(/Completed writing (.+?) in \d+ hrs/g)].map(match=>match[1]);
  const compact=value=>value.toLocaleLowerCase().normalize('NFKC').replace(/[^\p{L}\p{N}]/gu,'');
  const bindings=requested.map((argument,index)=>({argument,completion_label:completion[index]??null,file:files.length===requested.length&&completion.length===requested.length?files.find(name=>compact(name.replace(/\.csv$/,''))===compact(completion[index]))??null:null}));
  const binding_ok=bindings.every(item=>item.file)&&new Set(bindings.map(item=>item.file)).size===requested.length;
  const path=value=>{try{const url=new URL(value);return url.origin===fixture.origin?url.pathname+url.search+url.hash:value;}catch{return value;}};
  const observations={};
  for(const binding of bindings)if(binding.file){const bytes=await readFile(resolve(runOutput,binding.file));const rows=csv(bytes.toString('utf8'));observations[binding.argument]={file:binding.file,sha256:sha(bytes),row_count:rows.length,paths:rows.map(row=>path(row.Adresse??row.Address??'')).sort(),rows:rows.map(row=>({...row,path:path(row.Adresse??row.Address??'')}))};}
  // A source-bound independent predicate prediction, not a native Rust test.
  const corePath=resolve(root,'crates/core/src/dom_facts.rs');const coreSource=await readFile(corePath);
  const predicted=levels=>{const firstH1=levels.indexOf(1),firstH2=levels.indexOf(2);return {h1_nonsequential:levels.includes(1)&&levels[0]!==1,h2_nonsequential:firstH1>=0&&firstH2>firstH1&&levels.slice(firstH1+1,firstH2).some(level=>level>2)};};
  const membership=[home,...cases].map(item=>{const core=predicted(item.levels);return {path:item.path,purpose:item.purpose,levels:item.levels,h1:{observed_in_all:observations['H1:All']?.paths.includes(item.path)??null,reference_member:observations['H1:Non-Sequential']?.paths.includes(item.path)??null,independent_source_prediction:core.h1_nonsequential},h2:{observed_in_all:observations['H2:All']?.paths.includes(item.path)??null,reference_member:observations['H2:Non-Sequential']?.paths.includes(item.path)??null,independent_source_prediction:core.h2_nonsequential}};});
  const discrepancies=membership.flatMap(item=>['h1','h2'].filter(level=>item[level].observed_in_all&&item[level].reference_member!==item[level].independent_source_prediction).map(level=>({path:item.path,level,...item[level]})));
  const result={schema_version:1,observed_at:new Date().toISOString(),status:run.exit_code===0&&binding_ok?'REFERENCE_HEADING_MEMBERSHIP_OBSERVED':/licen[cs]e.*required|lizenz.*erforderlich/i.test(run.text)?'LICENSE_BLOCKED':'REFERENCE_HEADING_MEMBERSHIP_NOT_PROVEN',full_vendor_parity:'NOT_PROVEN',reference_version:'24.3',fixture_origin:fixture.origin,fixture_sha256:manifest.fixture_sha256,requested_exports:requested,arguments:args.map(argument=>argument===runOutput?'[owned output directory]':argument),exit_code:run.exit_code,timed_out:run.timedOut,output_directory:'docs/parity/reference-headings/'+stamp,bindings,observations,membership,independent:{kind:'SOURCE_PREDICATE_PREDICTION_NOT_RUNTIME',source:'crates/core/src/dom_facts.rs',source_sha256:sha(coreSource),function:'audit_dom_metadata',discrepancies},profile:{before_sha256:sha(before),after_sha256:sha(after),noncomment_content_unchanged:noncomments(before)===noncomments(after),changed_keys,unmatched_keys:changed_keys.filter(item=>item.classification.startsWith('UNMATCHED')).map(item=>item.key),local_byte_snapshots_saved_in_work:true,settings_modified_by_runner:false,rollback_performed:false},requests:fixture.requests,distinct_requested_paths:[...new Set(fixture.requests.map(item=>item.path))].sort(),diagnostics:run.text.split(/\r?\n/).filter(line=>/Licence Status:|Completed writing|Application Exited|Exception|licen[cs]e.*required|lizenz.*erforderlich/i.test(line)),sources:['https://www.screamingfrog.co.uk/seo-spider/issues/h1/h1-non-sequential/','https://www.screamingfrog.co.uk/seo-spider/issues/h2/h2-non-sequential/'],limitations:['Only these literal/raw heading cases and unchanged existing normal HTTP profile are measured. No paid JavaScript/computed visibility or broad accessibility equivalence is inferred.','All cohort membership is required before filter absence counts as a negative observation. Empty exports do not automatically pass a rule.','Independent predicates are source-bound predictions until the same fixture is executed by a new native build.','Original CLI profile timestamp/analytics changes are ordinary observed side effects; no unknown profile keys are rolled back.','No original executable, implementation, translations, fonts or profile values are distributed.']};
  assert.ok(result.distinct_requested_paths.length<=30,'Fixture must remain below30 distinct local paths');
  await writeFile(resolve(runOutput,'fixture-cases.json'),JSON.stringify(manifest,null,2)+'\n');await writeFile(resolve(runOutput,'run.json'),JSON.stringify(result,null,2)+'\n');
  await writeFile(resolve(output,'fixture-cases.json'),JSON.stringify(manifest,null,2)+'\n');await writeFile(resolve(output,'latest-run.json'),JSON.stringify(result,null,2)+'\n');
  const lines=['# Bounded original heading-order membership','',`Observed ${result.observed_at}; original24.3 ordinary CLI, exit ${result.exit_code}, ${result.distinct_requested_paths.length} local paths. Full vendor parity **NOT_PROVEN**.`, '', '| Case | Tag order | H1 All | H1 non-sequential | H2 All | H2 non-sequential | Source prediction mismatch |','| --- | --- | --- | --- | --- | --- | --- |',...membership.map(item=>`| ${item.path} | ${item.levels.join(' → ')||'no headings'} | ${item.h1.observed_in_all} | ${item.h1.reference_member} | ${item.h2.observed_in_all} | ${item.h2.reference_member} | ${discrepancies.filter(mismatch=>mismatch.path===item.path).map(mismatch=>mismatch.level).join(', ')||'none'} |`),'','The independent column is a snapshot of the current newly authored `audit_dom_metadata` predicate, not an executed Rust parity result. Raw owned HTML is in fixture-cases.json; exact export paths, hashes and rows are in latest-run.json. Hidden attributes, inline CSS and image alt are raw facts; rendered visibility is not inferred.','',`Profile bytes: before ${result.profile.before_sha256}, after ${result.profile.after_sha256}. Noncomment content unchanged: ${result.profile.noncomment_content_unchanged}. Changed property keys: ${changed_keys.map(item=>item.key).join(', ')||'none'}. Private byte snapshots remain only in task work; no profile writes or rollback by the runner.`,'','Sources: [H1 non-sequential](https://www.screamingfrog.co.uk/seo-spider/issues/h1/h1-non-sequential/), [H2 non-sequential](https://www.screamingfrog.co.uk/seo-spider/issues/h2/h2-non-sequential/).','', 'Rerun with `node scripts/reference-headings.mjs`; this creates a new ordinary reference observation without changing existing settings.'];
  await writeFile(resolve(output,'membership.md'),lines.join('\n')+'\n');
  console.log(JSON.stringify({status:result.status,local_paths:result.distinct_requested_paths.length,exports:bindings,filter_paths:{h1:observations['H1:Non-Sequential']?.paths,h2:observations['H2:Non-Sequential']?.paths},discrepancies,profile:result.profile}));
}finally{await new Promise(done=>fixture.server.close(done));}
