// Ordinary reference CLI behavior on original independently authored localhost fixtures.
// No profile/settings/licensing changes are made by this runner.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startFixture, pages } from './reference-fixture.mjs';
import { csv } from './reference-normalize.mjs';
import { normalizeMembership } from './reference-membership-normalize.mjs';

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const output=resolve(root,'docs/parity/reference-membership');
const work=resolve(root,'../../work/reference-membership-profile');
const executable=process.env.OPEN_SEO_REFERENCE_CLI??'C:/Program Files (x86)/Screaming Frog SEO Spider/ScreamingFrogSEOSpiderCli.exe';
const profile=resolve(process.env.USERPROFILE??'','.ScreamingFrogSEOSpider/spider.config');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const inventory=JSON.parse(await readFile(resolve(root,'docs/parity/reference-cli-inventory.json'),'utf8'));
const specs=[];
const addSpec=(argument,positive,negative='/baseline')=>{assert.ok(inventory.export_tab_entries.some(entry=>entry.argument===argument&&entry.kind==='named_filter'),argument);specs.push({argument,positive_paths:[].concat(positive),negative_paths:[].concat(negative)});};
const addPage=(path,body='',head='',options={})=>pages.set(path,{status:200,headers:{},html:`<!doctype html><html lang="en"><head><meta charset="utf-8"><title>${options.title??'Controlled reference membership '+path}</title><meta name="description" content="${options.description??'A complete independent description for the controlled filter membership fixture '+path}">${head}</head><body>${options.heading===false?'':`<h1>${options.heading??'Unique primary heading '+path}</h1>`}<p>Independent controlled main content for measuring concrete reference filter membership. No external requests or copied assets.</p>${body}</body></html>`});
addPage('/heading-missing','','',{heading:false}); addPage('/heading-empty','<h1></h1>','',{heading:false});
addPage('/heading-multiple','<h1>Second heading</h1><h2>Secondary one</h2><h2>Secondary two</h2>');
addPage('/heading-long','<h2>'+('W'.repeat(120))+'</h2>','',{heading:'W'.repeat(120)});
addPage('/heading-alt','<h1><img src="/image.png" alt="Alternate primary heading"></h1>','',{heading:false});
addPage('/heading-order','<h2>Secondary before primary</h2><h1>Primary after secondary</h1>','',{heading:false});
addPage('/heading-duplicate-a','<h2>Duplicated secondary heading</h2>','',{heading:'Duplicated primary heading'});
addPage('/heading-duplicate-b','<h2>Duplicated secondary heading</h2>','',{heading:'Duplicated primary heading'});
addPage('/heading-control','<h2>Unique secondary heading</h2>');
addPage('/same-title-h1','','',{title:'Shared title and primary heading',heading:'Shared title and primary heading'});
addPage('/description-long','','',{description:'W'.repeat(200)}); addPage('/description-short','','',{description:'Tiny'});
addPage('/canonical-unlinked','', '<link rel="canonical" href="/only-canonical-target">'); addPage('/only-canonical-target');
addPage('/canonical-invalid','','<link rel="canonical" href="/baseline" hreflang="en">');
addPage('/canonical-fragment','','<link rel="canonical" href="/baseline#target-fragment">');
addPage('/canonical-body','<link rel="canonical" href="/baseline">');
addPage('/directive-body','<meta name="robots" content="noindex">');
addPage('/directive-index','','<meta name="robots" content="index, follow">');
addPage('/directive-nofollow','','<meta name="robots" content="index, nofollow">');
addPage('/directive-refresh','','<meta http-equiv="refresh" content="10;url=/baseline">');
addPage('/links-uncrawlable','<span href="/baseline">Not an anchor</span><div onclick="location.href=\'/baseline\'">Click navigation</div>');
addPage('/links-nondescriptive','<a href="/baseline">Click here</a>');
addPage('/links-nofollow-only','<a href="/nofollow-only-target" rel="nofollow">Target by nofollow only</a>'); addPage('/nofollow-only-target');
addPage('/links-nonindex-source','<a href="/nonindex-only-target">Target from noindex source only</a>','<meta name="robots" content="noindex">'); addPage('/nonindex-only-target');
for(let level=0;level<6;level++)addPage('/depth-'+level,level<5?`<a href="/depth-${level+1}">Next depth level</a>`:'');
addPage('/caf%C3%A9');addPage('/space%20case');addPage('/double//slash');addPage('/repeat/repeat/repeat');
const longPath='/'+('long'.repeat(550));addPage(longPath);
const noRoot=new Set(['/orphan-only','/only-canonical-target','/nofollow-only-target','/nonindex-only-target',...Array.from({length:5},(_,i)=>'/depth-'+(i+1))]);
const index=pages.get('/');
index.html=index.html.replace(/<body>[\s\S]*<\/body>/, '<body><h1>Fixture index</h1>'+[...pages.keys()].filter(path=>path!=='/'&&!noRoot.has(path)).map(path=>`<a href="${path}">${path}</a>`).join('\n')+'<a href="/redirect-one">Redirect chain</a><a href="/missing">Missing</a><a href="/server-error">Server error</a><a href="/baseline?utm_source=fixture&amp;utm_campaign=controlled">Tracking URL</a><a href="/redirect-loop-one">Redirect loop</a></body>');
assert.ok(pages.size+10<100,'Fixture must stay below100 distinct paths');
// The independently authored cases bind intended triggers and controls before observing the reference.
for(const [filter,pos,neg]of[
 ['Missing',['/missing-title','/empty-title','/blank-title']],['Duplicate',['/duplicate-a','/duplicate-b']],['Over X Characters','/long-title'],['Below X Characters','/short-title'],['Over X Pixels','/long-title'],['Below X Pixels','/short-title'],['Same as H1','/same-title-h1'],['Multiple','/multiple-title'],['Outside <head>','/body-metadata']])addSpec('Page Titles:'+filter,pos,neg);
for(const [filter,pos,neg]of[
 ['Missing',['/missing-description','/empty-description']],['Duplicate',['/duplicate-a','/duplicate-b']],['Over X Characters','/description-long'],['Below X Characters','/description-short'],['Over X Pixels','/description-long'],['Below X Pixels','/description-short'],['Multiple','/multiple-description'],['Outside <head>','/body-metadata']])addSpec('Meta Description:'+filter,pos,neg);
for(const tab of['H1','H2'])for(const [filter,pos,neg]of[
 ['Missing',tab==='H1'?['/heading-missing','/heading-empty']:'/baseline','/heading-control'],['Duplicate',['/heading-duplicate-a','/heading-duplicate-b'],'/heading-control'],['Over X Characters','/heading-long','/heading-control'],['Multiple','/heading-multiple','/heading-control'],['Non-Sequential','/heading-order','/heading-control'],...(tab==='H1'?[['Alt Text in H1','/heading-alt','/heading-control']]:[])])addSpec(tab+':'+filter,pos,neg);
for(const [filter,pos,neg]of[
 ['Contains Canonical','/canonical-self','/baseline'],['Self Referencing','/canonical-self','/canonical-other'],['Canonicalised','/canonical-other','/canonical-self'],['Missing','/baseline','/canonical-self'],['Multiple','/canonical-conflict','/canonical-self'],['Non-Indexable Canonical',['/canonical-404','/canonical-conflict'],'/canonical-self'],['Multiple Conflicting','/canonical-conflict','/canonical-self'],['Canonical Is Relative','/canonical-self','/baseline'],['Unlinked','/canonical-unlinked','/canonical-other'],['Invalid Attribute In Annotation','/canonical-invalid','/canonical-self'],['Contains Fragment URL','/canonical-fragment','/canonical-self'],['Outside <head>','/canonical-body','/canonical-self']])addSpec('Canonicals:'+filter,pos,neg);
for(const [filter,pos,neg]of[
 ['Index','/directive-index','/noindex'],['Noindex',['/noindex','/header-noindex'],'/baseline'],['Follow','/directive-index','/directive-nofollow'],['Nofollow',['/directive-nofollow','/header-noindex'],'/directive-index'],['Refresh','/directive-refresh','/baseline'],['Outside <head>','/directive-body','/noindex']])addSpec('Directives:'+filter,pos,neg);
for(const [filter,pos]of[
 ['Non ASCII Characters','/caf%C3%A9'],['Underscores','/Upper_Case'],['Uppercase','/Upper_Case'],['Parameters','/baseline?a=1&b=2'],['Over X Characters',longPath],['Multiple Slashes','/double//slash'],['Repetitive Path','/repeat/repeat/repeat'],['Contains Space','/space%20case'],['Broken Bookmark','/links'],['GA Tracking Parameters','/baseline?utm_source=fixture&utm_campaign=controlled']])addSpec('URL:'+filter,pos);
for(const [filter,pos,neg]of[
 ['Pages Without Internal Outlinks','/baseline','/links'],['Internal Nofollow Outlinks','/links','/baseline'],['Internal Outlinks With No Anchor Text','/links','/links-nondescriptive'],['Non-Descriptive Anchor Text In Internal Outlinks','/links-nondescriptive','/links'],['Follow & Nofollow Internal Inlinks To Page','/baseline','/heading-control'],['Internal Nofollow Inlinks Only','/nofollow-only-target','/baseline'],['Pages With High Crawl Depth','/depth-5','/baseline'],['Non-Indexable Page Inlinks Only','/nonindex-only-target','/baseline'],['Pages With Uncrawlable Internal Outlinks','/links-uncrawlable','/baseline'],['Outlinks To Localhost','/links','/baseline']])addSpec('Links:'+filter,pos,neg);
for(const [filter,pos,neg]of[
 ['Blocked by Robots.txt','/blocked','/baseline'],['Success (2xx)','/baseline','/missing'],['Redirection (3xx)','/redirect-one','/baseline'],['Client Error (4xx)','/missing','/baseline'],['Server Error (5xx)','/server-error','/baseline'],['Internal Blocked by Robots.txt','/blocked','/baseline'],['Internal Success (2xx)','/baseline','/missing'],['Internal Redirection (3xx)','/redirect-one','/baseline'],['Internal Client Error (4xx)','/missing','/baseline'],['Internal Server Error (5xx)','/server-error','/baseline'],['Internal Redirect Chain','/redirect-one','/baseline'],['Internal Redirect Loop','/redirect-loop-one','/baseline']])addSpec('Response Codes:'+filter,pos,neg);
assert.equal(new Set(specs.map(spec=>spec.argument)).size,specs.length);
const fixture=await startFixture();
// Two new redirect handlers use server event ordering while keeping the shared baseline fixture source unchanged.
const normal=fixture.server.listeners('request')[0];fixture.server.removeListener('request',normal);
fixture.server.on('request',(request,response)=>{
 if(['/redirect-loop-one','/redirect-loop-two'].includes(new URL(request.url,fixture.origin).pathname)){
  fixture.requests.push({path:request.url,method:request.method,user_agent:request.headers['user-agent']??null});
  response.statusCode=301;response.setHeader('Location',request.url==='/redirect-loop-one'?'/redirect-loop-two':'/redirect-loop-one');return response.end();
 }
 return normal(request,response);
});
const timestamp=new Date().toISOString().replace(/[:.]/g,'-');const runOutput=resolve(output,timestamp),snapshot=resolve(work,timestamp);
const profileKeys=bytes=>new Map(bytes.toString('latin1').split(/\r?\n/).filter(line=>line&&!line.startsWith('#')&&line.includes('=')).map(line=>{const at=line.indexOf('=');return[line.slice(0,at),line.slice(at+1)];}));
const before=await readFile(profile);await mkdir(snapshot,{recursive:true});await writeFile(resolve(snapshot,'spider.config.before'),before);
await mkdir(runOutput,{recursive:true});
const requested=[...new Set(specs.map(spec=>spec.argument.split(':')[0]))].flatMap(tab=>[tab+':All',...specs.filter(spec=>spec.argument.startsWith(tab+':')).map(spec=>spec.argument)]);
const args=['--headless','--crawl',fixture.origin+'/','--output-folder',runOutput,'--export-format','csv','--export-tabs',requested.join(',')];
try{
 const run=await new Promise((resolveRun,reject)=>{const child=spawn(executable,args,{windowsHide:true,stdio:['ignore','pipe','pipe']});const stdout=[],stderr=[];let timedOut=false;const timer=setTimeout(()=>{timedOut=true;child.kill();},120000);child.stdout.on('data',data=>stdout.push(data));child.stderr.on('data',data=>stderr.push(data));child.on('error',error=>{clearTimeout(timer);reject(error);});child.on('close',code=>{clearTimeout(timer);resolveRun({code,timedOut,text:new TextDecoder('utf8').decode(Buffer.concat([...stdout,...stderr]))});});});
 const after=await readFile(profile);await writeFile(resolve(snapshot,'spider.config.after'),after);
 const beforeKeys=profileKeys(before),afterKeys=profileKeys(after);
 const keyChanges=[...new Set([...beforeKeys.keys(),...afterKeys.keys()])].filter(key=>beforeKeys.get(key)!==afterKeys.get(key)).map(key=>({key,before_value_sha256:beforeKeys.has(key)?hash(beforeKeys.get(key)):null,after_value_sha256:afterKeys.has(key)?hash(afterKeys.get(key)):null,
  classification:/^(analytics\.last_sent|updatechecker\.)/.test(key)?'ORDINARY_ANALYTICS_OR_UPDATE_STATE':'UNMATCHED_PROFILE_KEY_CHANGE_REQUIRES_CONTEXT_NO_ROLLBACK'}));
 const files=(await readdir(runOutput)).filter(name=>name.endsWith('.csv'));
 const completed=[...run.text.matchAll(/Completed writing (.+?) in \d+ hrs/g)].map(match=>match[1]);
 const compact=value=>value.toLocaleLowerCase().normalize('NFKC').replace(/[^\p{L}\p{N}]/gu,'');
 const mappingUsable=completed.length===requested.length&&files.length===requested.length;
 const bindings=requested.map((argument,index)=>({argument,completion_label:completed[index]??null,file:mappingUsable?files.find(name=>compact(name.replace(/\.csv$/,''))===compact(completed[index]))??null:null,mapping_basis:'Ordered requested exports + ordered completed-report observations, with matching normalized generated filename. No translation bundle read.'}));
 const addressPath=value=>{try{const parsed=new URL(value);return parsed.origin===fixture.origin?parsed.pathname+parsed.search+parsed.hash:value;}catch{return value;}};
 const membership=[];
 for(const spec of specs){const binding=bindings.find(item=>item.argument===spec.argument);const rows=binding.file?csv(await readFile(resolve(runOutput,binding.file),'utf8')):null;const paths=rows?[...new Set(rows.map(row=>addressPath(row.Adresse??row.Address??'')))].sort():null;
  const allBinding=bindings.find(item=>item.argument===spec.argument.split(':')[0]+':All');const allRows=allBinding.file?csv(await readFile(resolve(runOutput,allBinding.file),'utf8')):[];const seen=new Set(allRows.map(row=>addressPath(row.Adresse??row.Address??'')));
  const caseChecks=[...spec.positive_paths.map(path=>({path,role:'proposed_trigger',expected_member:true})),...spec.negative_paths.map(path=>({path,role:'control',expected_member:false}))].map(check=>({...check,observed_in_all:seen.has(check.path),observed_member:paths?paths.includes(check.path):null,
   result:!paths?'EXPORT_UNAVAILABLE':!seen.has(check.path)?'URL_NOT_IN_TAB_COHORT':paths.includes(check.path)===check.expected_member?'EXPECTED_MEMBERSHIP_OBSERVED':'PROPOSAL_DIFFERS_FROM_REFERENCE'}));
  membership.push({filter_argument:spec.argument,file:binding.file,file_sha256:binding.file?hash(await readFile(resolve(runOutput,binding.file))):null,paths,row_count:rows?.length??null,case_checks:caseChecks,
   proof_status:caseChecks.every(check=>check.result==='EXPECTED_MEMBERSHIP_OBSERVED')&&spec.positive_paths.length&&spec.negative_paths.length?'REFERENCE_POSITIVE_AND_CONTROL_OBSERVED':'PARTIAL_REFERENCE_OBSERVATION',
   independent_product_comparison:'NOT_PROVEN',empty_filter_policy:'An empty export alone is not an acceptance pass or proof a rule is unavailable.'});}
 const htmlCases=[...pages].map(([path,value])=>({path,response:value,html_sha256:hash(value.html)}));
 const manifest={schema_version:1,authored_by:'Open SEO Spider independent extended membership fixture',html_cases:htmlCases,transport_cases:['/robots.txt','/sitemap.xml','/image.png','/redirect-one','/redirect-two','/redirect-loop-one','/redirect-loop-two','/missing','/server-error'],filter_case_contracts:specs,maximum_distinct_paths:pages.size+10};manifest.fixture_sha256=hash(JSON.stringify({html_cases:htmlCases,transport_cases:manifest.transport_cases,filter_case_contracts:specs}));
 const result={schema_version:1,observed_at:new Date().toISOString(),reference_version:'24.3',fixture_origin:fixture.origin,fixture_sha256:manifest.fixture_sha256,fixture_manifest:'docs/parity/reference-membership/fixture-cases.json',requested_exports:requested,
  exit_code:run.code,timed_out:run.timedOut,status:run.code===0&&mappingUsable&&bindings.every(binding=>binding.file)?'REFERENCE_MEMBERSHIP_OBSERVED':/licen[cs]e.*required|lizenz.*erforderlich/i.test(run.text)?'LICENSE_BLOCKED':'REFERENCE_MEMBERSHIP_NOT_PROVEN',
  output_directory:'docs/parity/reference-membership/'+timestamp,profile:{before_sha256:hash(before),after_sha256:hash(after),unchanged:hash(before)===hash(after),changed_keys:keyChanges,unmatched_keys:keyChanges.filter(change=>change.classification.startsWith('UNMATCHED')).map(change=>change.key),local_byte_snapshots_saved_in_work:true,settings_modified_by_runner:false,rollback_performed:false},
  diagnostics:run.text.split(/\r?\n/).filter(line=>/Licence Status:|Completed writing|Application Exited|Exception|licen[cs]e.*required|lizenz.*erforderlich/i.test(line)),bindings,membership,requests:fixture.requests,
  counts:{named_filters:specs.length,csv_files:files.length,positive_control_pairs:membership.filter(item=>item.proof_status==='REFERENCE_POSITIVE_AND_CONTROL_OBSERVED').length,partial_observations:membership.filter(item=>item.proof_status!=='REFERENCE_POSITIVE_AND_CONTROL_OBSERVED').length},
  initial_final_aliases:[{initial:'/redirect-one',intermediate:'/redirect-two',final:'/baseline'},{initial:'/redirect-loop-one',next:'/redirect-loop-two',cycle:true}],
  source_urls:['https://www.screamingfrog.co.uk/seo-spider/user-guide/general/','https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/'],
  limitations:['Filter cohorts and predicates are measured only on these cases under unchanged existing profile. Default thresholds/post-analysis eligibility are not inferred from empty outputs.','Membership filename binding is conditional on equal export/completion counts and matching filenames; log and request order are retained for review.','Missing licence does not block this ordinary local run when successful; advanced entitlement remains separate.','Profile bytes are preserved only in task work. Values/credentials and vendor configuration text are not copied into the repository.','This run does not compare independent product filter predicates. Full product parity NOT_PROVEN.']};
 await writeFile(resolve(output,'fixture-cases.json'),JSON.stringify(manifest,null,2)+'\n');await writeFile(resolve(output,'latest-run.json'),JSON.stringify(result,null,2)+'\n');
 if(result.exit_code===0&&mappingUsable)await normalizeMembership(result);
 console.log(JSON.stringify({status:result.status,counts:result.counts,requests:fixture.requests.length,profile:result.profile,unconfirmed:membership.filter(item=>item.proof_status!=='REFERENCE_POSITIVE_AND_CONTROL_OBSERVED').map(item=>({filter:item.filter_argument,checks:item.case_checks.filter(check=>check.result!=='EXPECTED_MEMBERSHIP_OBSERVED')}))}));
}finally{await new Promise(resolveClose=>fixture.server.close(resolveClose));}
