// Independent checks against actual native report/catalog inputs and fixed fixture expectations.
import assert from 'node:assert/strict';
const engine='open-seo-spider/sitemap-target-inlink-analysis-v1';
export function assertTargetPage(value,job,{available=true}={}){
  const analysis=value.membership_analysis;
  if(!value.catalog_available){assert.equal(analysis,null);assert.equal(value.next_page_cursor,null);return null;}
  assert(analysis && analysis.context && analysis.context.engine===engine);
  const context=analysis.context;
  assert.equal(context.available,available);assert.equal(context.website_wide_orphan_inference,false);
  assert.equal(context.source_scope,value.coverage.derived.scope);
  assert.equal(context.generation,value.snapshot.crawl_generation);
  if(!available){assert.equal(analysis.location_findings,null);assert.equal(analysis.saved_pages,null);assert.equal(analysis.saved_pages_have_more,false);assert.equal(value.next_page_cursor,null);return analysis;}
  assert.equal(context.reason,'validated_terminal_saved_report');
  assert.equal(context.checkpoint_revision,job.checkpoint.revision);
  assert.equal(context.report_sha256,job.checkpoint.report_sha256);
  assert.equal(context.outcome,job.checkpoint.outcome);
  assert.equal(context.saved_page_count,job.report.pages.length);
  assert.equal(context.pending_page_count,job.report.summary.pending);
  assert(Number.isSafeInteger(context.report_bytes)&&context.report_bytes>0&&context.report_bytes<=268435456);
  assert(Array.isArray(analysis.location_findings)&&Array.isArray(analysis.saved_pages));
  const visible=value.rows.filter(row=>row.kind==='location'&&row.row.observed.role==='page_location');
  assert.deepEqual(analysis.location_findings.map(row=>row.sequence),visible.map(row=>row.row.sequence));
  for(const finding of analysis.location_findings)assertVisibleLocationFinding(finding,visible,job);
  for(const finding of analysis.saved_pages)assertSavedFinding(finding,job,value.coverage.derived);
  assert.equal(analysis.last_saved_page_ordinal,analysis.saved_pages.at(-1)?.observation.report_page_ordinal??null);
  assert.equal(analysis.last_saved_page_url,analysis.saved_pages.at(-1)?.observation.requested_url??null);
  assert.equal(typeof value.next_page_cursor==='string',analysis.saved_pages_have_more);
  return analysis;
}
function assertObservation(value,raw,job){
  assert(raw,'Actual saved requested identity must exist; no redirect/alias substitution');
  assert.equal(value.requested_url,raw.url);assert.equal(value.report_page_ordinal,job.report.pages.findIndex(row=>row.url===raw.url));
  for(const key of ['final_url','status','final_status','indexable','indexability_reasons','blocked_by_robots','error','truncated','renderer_error','rendered','rendered_url','rendered_status','fetched_at'])assert.deepEqual(value[key],raw[key],'Retain actual native '+key);
  assert.equal(value.redirect_chain_present,raw.redirect_chain.length>0);
  const expected=raw.rendered_url?new URL(raw.rendered_url):null;if(expected)expected.hash='';
  assert.equal(value.rendered_fetch_url,expected?.href??null);
  if(job.report.operation){const target=job.report.operation.targets.find(row=>row.requested_url===raw.url);assert.equal(value.age_scope,target?.committed_generation===job.generation?'selected_current_generation':target?.committed_generation!=null?'selected_prior_generation':'inherited_parent_observation');}
  else assert.equal(value.age_scope,'captured_checkpoint_history');
}
function anchors(job,target){return job.report.pages.flatMap((page,pageOrdinal)=>page.links.map((link,linkOrdinal)=>({page,pageOrdinal,link,linkOrdinal}))).filter(value=>value.link.kind==='anchor'&&value.link.url===target);}
function assertInlinks(value,job,target){
  const all=anchors(job,target),nonself=all.filter(row=>row.page.url!==target);
  assert.equal(value.anchor_occurrences,all.length);assert.equal(value.nonself_anchor_occurrences,nonself.length);
  assert.equal(value.internal_nonself_anchor_occurrences,nonself.filter(row=>row.link.internal).length);
  assert.equal(value.distinct_nonself_requested_sources,new Set(nonself.map(row=>row.page.url)).size);
  const ordered=all.sort((left,right)=>left.page.url<right.page.url?-1:left.page.url>right.page.url?1:left.pageOrdinal-right.pageOrdinal||left.linkOrdinal-right.linkOrdinal).slice(0,5);
  assert.equal(value.samples_have_more,all.length>5);
  assert.deepEqual(value.samples,ordered.map(row=>({source_requested_url:row.page.url,target_requested_url:target,source_page_ordinal:row.pageOrdinal,link_ordinal:row.linkOrdinal,text:row.link.text,rel:row.link.rel,internal:row.link.internal,nofollow:row.link.nofollow})));
}
export function assertVisibleLocationFinding(value,rows,job){
  const tagged=rows.find(row=>row.row.sequence===value.sequence),raw=tagged?.row;assert(raw);
  assert.equal(value.observation_id,raw.observation_id);assert.equal(value.occurrence_id,raw.occurrence_id);assert.equal(value.entry_ordinal,raw.observed.entry_ordinal);
  const original=raw.observed.validation_error===null&&raw.observed.loc_ordinal!==null?raw.observed.normalized_fetch_url:null;
  assert.equal(value.original_membership_url,original);assert.equal(value.admitted_fetch_url,raw.admission.admitted_fetch_url);assert.equal(value.admission_decision,raw.admission.decision);
  const page=job.report.pages.find(row=>row.url===original);
  if(value.original_observation)assertObservation(value.original_observation,page,job);else assert.equal(page,undefined);
  if(value.admitted_alias_observation){assert.notEqual(value.admitted_fetch_url,original);assertObservation(value.admitted_alias_observation,job.report.pages.find(row=>row.url===value.admitted_fetch_url),job);}
  if(original)assertInlinks(value.observed_original_inlinks,job,original);else {assert.equal(value.observed_original_inlinks,null);assert.equal(value.original_state,'invalid_or_missing_original_location');}
}
export function assertSavedFinding(value,job,coverage){
  const raw=job.report.pages.find(row=>row.url===value.observation.requested_url);assertObservation(value.observation,raw,job);
  const locations=job.report.sitemap_catalog.rows.filter(row=>row.kind==='location'&&row.row.observed.role==='page_location').map(row=>row.row);
  const originals=locations.filter(row=>row.observed.loc_ordinal!==null&&row.observed.validation_error===null&&row.observed.normalized_fetch_url===raw.url);
  const aliases=locations.filter(row=>row.admission.admitted_fetch_url===raw.url&&row.observed.normalized_fetch_url!==raw.url);
  const documents=new Map(job.report.sitemap_catalog.rows.filter(row=>row.kind==='document').map(row=>[row.row.observation_id,row.row]));
  const sources=originals.map(row=>documents.get(row.observation_id));assert(sources.every(Boolean));
  const distinct=[...new Set(sources.map(row=>row.task_url))].sort();
  assert.equal(value.original_membership_occurrences,originals.length);assert.equal(value.admitted_alias_occurrences,aliases.length);
  assert.equal(value.original_source_document_count,distinct.length);assert.equal(value.original_source_observation_count,new Set(sources.map(row=>row.observation_id)).size);
  assert.equal(value.source_document_samples_have_more,distinct.length>5);
  assert.deepEqual(value.original_source_document_samples.map(row=>row.task_url),distinct.slice(0,5));
  for(const sample of value.original_source_document_samples){const source=documents.get(sample.observation_id);assert(source);assert.equal(sample.task_url,source.task_url);assert.equal(sample.source_final_url,source.transport?.final_url??null);}
  const membership=originals.length?'listed_original':coverage.current_valid_original_location_negative_permitted?'not_listed_current_declared_sources':coverage.historical_valid_original_location_negative_permitted?'not_listed_historical_declared_sources':'unknown';
  assert.equal(value.membership,membership);assertInlinks(value.observed_inlinks,job,raw.url);
  assert.equal(value.listed_without_observed_nonself_inlinks,originals.length>0&&value.observed_inlinks.nonself_anchor_occurrences===0);
}
