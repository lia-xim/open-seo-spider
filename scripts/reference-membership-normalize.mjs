import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile, readdir } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { csv } from './reference-normalize.mjs';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const destination=resolve(root,'docs/parity/reference-membership/latest-run.json');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const compact=value=>value.toLocaleLowerCase().normalize('NFKC').replace(/[^\p{L}\p{N}]/gu,'');
const cp1252=new TextDecoder('windows-1252').decode(Uint8Array.from({length:256},(_,index)=>index));
const inverse=new Map([...cp1252].map((char,index)=>[char,index]));
function repairMojibake(value){if(!/[ÃÂ]/.test(value))return value;const bytes=[...value].map(char=>inverse.get(char));return bytes.some(byte=>byte===undefined)?value:new TextDecoder('utf8',{fatal:true}).decode(Uint8Array.from(bytes));}
export async function normalizeMembership(result){
 const snapshot=resolve(root,'../../work/reference-membership-profile',basename(result.output_directory));
 try {
  const before=await readFile(resolve(snapshot,'spider.config.before')),after=await readFile(resolve(snapshot,'spider.config.after'));
  assert.equal(hash(before),result.profile.before_sha256);assert.equal(hash(after),result.profile.after_sha256);
  const noncomments=bytes=>bytes.toString('latin1').split(/\r?\n/).filter(line=>!line.startsWith('#')&&!line.startsWith('!')).join('\n');
  result.profile.noncomment_content_unchanged=noncomments(before)===noncomments(after);
  result.profile.byte_difference_scope=result.profile.noncomment_content_unchanged?'COMMENT_LINES_ONLY':'INCLUDES_NONCOMMENT_CONTENT_REQUIRES_CONTEXT';
 } catch(error) {if(error.code!=='ENOENT')throw error;}
 const files=(await readdir(resolve(root,result.output_directory))).filter(name=>name.endsWith('.csv'));
 result.diagnostics=result.diagnostics.map(repairMojibake);
 const completed=result.diagnostics.flatMap(line=>{const matched=/Completed writing (.+?) in \d+ hrs/.exec(line);return matched?[matched[1]]:[];});
 assert.equal(completed.length,result.requested_exports.length,'Requested/completed export counts must match for ordinal binding.');
 assert.equal(files.length,result.requested_exports.length,'CSV/requested export counts must match.');
 result.bindings=result.requested_exports.map((argument,index)=>({argument,completion_label:completed[index],file:files.find(name=>compact(name.replace(/\.csv$/,''))===compact(completed[index]))??null,
  mapping_basis:'Ordered requested exports + completed-report observations; equal counts; unique normalized filename match. UTF-8 Java crawl logs; no bundled translation read.'}));
 assert.equal(result.bindings.filter(binding=>binding.file).length,result.bindings.length,'Every observed completion must bind exactly one generated filename.');
 assert.equal(new Set(result.bindings.map(binding=>binding.file)).size,result.bindings.length,'Bindings cannot reuse a file.');
 const addressPath=value=>{try{const parsed=new URL(value);return parsed.origin===result.fixture_origin?parsed.pathname+parsed.search+parsed.hash:value;}catch{return value;}};
 for(const item of result.membership){
  const binding=result.bindings.find(binding=>binding.argument===item.filter_argument);
  const contents=await readFile(resolve(root,result.output_directory,binding.file));
  const rows=csv(contents.toString('utf8'));const paths=[...new Set(rows.map(row=>addressPath(row.Adresse??row.Address??'')))].sort();
  const allBinding=result.bindings.find(binding=>binding.argument===item.filter_argument.split(':')[0]+':All');
  const allRows=csv(await readFile(resolve(root,result.output_directory,allBinding.file),'utf8'));const seen=new Set(allRows.map(row=>addressPath(row.Adresse??row.Address??'')));
  Object.assign(item,{file:binding.file,file_sha256:hash(contents),paths,row_count:rows.length,threshold_observation:null});
  const threshold=/(Über|Unter)\s+(\d+)\s+(Zeichen|Pixel)/u.exec(binding.completion_label);
  if(threshold)item.threshold_observation={direction:threshold[1]==='Über'?'over':'under',value:Number(threshold[2]),unit:threshold[3]==='Pixel'?'pixels':'characters',basis:'Observed rendered completion label; boundary operator is not inferred beyond label.'};
  item.case_checks=item.case_checks.map(check=>({...check,observed_in_all:seen.has(check.path),observed_member:paths.includes(check.path),result:!seen.has(check.path)?'URL_NOT_IN_TAB_COHORT':paths.includes(check.path)===check.expected_member?'EXPECTED_MEMBERSHIP_OBSERVED':'PROPOSAL_DIFFERS_FROM_REFERENCE'}));
  const positive=item.case_checks.some(check=>check.role==='proposed_trigger'&&check.result==='EXPECTED_MEMBERSHIP_OBSERVED');
  const control=item.case_checks.some(check=>check.role==='control'&&check.result==='EXPECTED_MEMBERSHIP_OBSERVED');
  item.proof_status=item.case_checks.every(check=>check.result==='EXPECTED_MEMBERSHIP_OBSERVED')&&positive&&control?'REFERENCE_POSITIVE_AND_CONTROL_OBSERVED':'PARTIAL_REFERENCE_OBSERVATION';
  if(/^(Page Titles|Meta Description):/.test(item.filter_argument)){
   item.refined_control={path:'/heading-control',selection_basis:'Distinct heading-control URL provides a clean metadata control; /baseline has query/redirect aliases creating real duplicate metadata and a title above the default60-character threshold. Original proposed control and its contradiction are retained.',observed_in_all:seen.has('/heading-control'),observed_member:paths.includes('/heading-control')};
   if(item.proof_status!=='REFERENCE_POSITIVE_AND_CONTROL_OBSERVED'&&positive&&seen.has('/heading-control')&&!paths.includes('/heading-control'))item.refined_pair_status='REFERENCE_TRIGGER_AND_REFINED_CONTROL_OBSERVED';
  }
 }
 result.status=result.exit_code===0?'REFERENCE_MEMBERSHIP_OBSERVED':result.status;
 result.counts={named_filters:result.membership.length,csv_files:files.length,positive_control_pairs:result.membership.filter(item=>item.proof_status==='REFERENCE_POSITIVE_AND_CONTROL_OBSERVED').length,
  refined_control_pairs:result.membership.filter(item=>item.refined_pair_status==='REFERENCE_TRIGGER_AND_REFINED_CONTROL_OBSERVED').length,
  partial_observations:result.membership.filter(item=>item.proof_status!=='REFERENCE_POSITIVE_AND_CONTROL_OBSERVED'&&item.refined_pair_status!=='REFERENCE_TRIGGER_AND_REFINED_CONTROL_OBSERVED').length};
 result.filter_paths=Object.fromEntries(result.membership.map(item=>[item.filter_argument,item.paths]));
 result.normalization={origin:'Replace observed localhost origin with exact pathname+query+fragment.',timestamps:'Excluded from membership comparisons.',filename_binding:'87 ordered request/completion records, unique filename identity required. No implicit translation guess.',empty_outputs:'Measured empty set; no automatic pass or missing implementation conclusion.',encoding:'Crawl logs and CSV are UTF-8. Earlier CP1252 log mojibake repaired losslessly before filename binding.'};
 await writeFile(destination,JSON.stringify(result,null,2)+'\n');
 const lines=['# Measured reference filter membership','',`Run ${result.observed_at}; original24.3, normal missing-licence CLI, unchanged profile parameters. Full product parity **NOT PROVEN**.`,
  '',`${result.counts.named_filters} named filter exports +9 All cohorts (${result.counts.csv_files} CSVs), ${result.requests.length} local requests. ${result.counts.positive_control_pairs} proposed trigger/control pairs confirmed; ${result.counts.refined_control_pairs} additional triggers have an explicitly refined clean control. ${result.counts.partial_observations} remain partial.`,
  '', 'Every exact CLI argument maps to a measured path set in `latest-run.json` (`filter_paths`). Empty sets are observations; they do not establish an unimplemented rule or successful acceptance.',
  '', '| Filter | Rows | Pair evidence | Remaining contradiction |','| --- | ---: | --- | --- |',
  ...result.membership.map(item=>`| ${item.filter_argument.replaceAll('|','\\|')} | ${item.row_count} | ${item.refined_pair_status??item.proof_status} | ${item.case_checks.filter(check=>check.result!=='EXPECTED_MEMBERSHIP_OBSERVED').map(check=>`${check.role}: ${check.path.length>70?check.path.slice(0,50)+'…':check.path} (${check.result})`).join('; ')} |`),
  '', 'The `All` cohort must contain a fixture before absence from a named filter can support a negative control. Non-followed nofollow targets and post-analysis-dependent filters remain explicit unknowns when their intended trigger is absent.',
  '', 'Initial/final aliases are preserved: redirect-one → redirect-two → baseline; baseline query variants are distinct observations with identical documents. Metadata duplicate controls therefore need distinct non-alias documents.',
  '', `Profile SHA before ${result.profile.before_sha256}, after ${result.profile.after_sha256}. Changed property keys: ${result.profile.changed_keys.map(change=>change.key).join(', ')||'none'}. Byte difference scope: ${result.profile.byte_difference_scope??'NOT_ASSESSED'}. Byte snapshots are task-local work files; no rollback or parameter write was performed.`,
  '', 'Sources: [official CLI](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/), [official tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/).',
  '', 'Run `node scripts/reference-membership.mjs` for a new controlled ordinary reference run; run `node scripts/reference-membership-normalize.mjs` to reparse the retained CSVs without executing the original.'];
 await writeFile(resolve(root,'docs/parity/reference-membership/membership.md'),lines.join('\n')+'\n');
 return result;
}
if(resolve(process.argv[1]??'')===fileURLToPath(import.meta.url)){const result=await normalizeMembership(JSON.parse(await readFile(destination,'utf8')));console.log(JSON.stringify({status:result.status,counts:result.counts,partial:result.membership.filter(item=>item.proof_status!=='REFERENCE_POSITIVE_AND_CONTROL_OBSERVED'&&!item.refined_pair_status).map(item=>({filter:item.filter_argument,paths:item.paths,checks:item.case_checks.filter(check=>check.result!=='EXPECTED_MEMBERSHIP_OBSERVED')}))}));}
