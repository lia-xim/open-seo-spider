'use strict';
const BEFORE='b1111111-1111-4111-8111-111111111111', AFTER='b2222222-2222-4222-8222-222222222222', ACTIVE='b3333333-3333-4333-8333-333333333333';
const markup='<img src=x onerror="window.__comparisonInjected=1"> & <script>window.__comparisonInjected=2</script>', formula='\uFEFF \t=HYPERLINK("https://inert.invalid","probe")';
const fact=(state,value,profile='raw_http_html-v1',extra={})=>({state,value,profile,reasons:state==='partial'?['text_byte_limit']:state==='unknown'?['no_complete_source_inventory']:[],provenance:{source:'saved_fixture',generation:3},...extra});
const field=(name,before,after,outcome='unchanged',complete=true)=>({field:name,before,after,outcome,availability_changed:before.state!==after.state,comparable_complete_values:complete,reason:complete?null:'partial_or_incompatible_sources'});
const reference=(index,side)=>({ordinal:index,original_requested_url:`https://${side}.fixture/${index}`,document_url:`https://${side}.fixture/${index}`,fetched_at:'2026-10-13T08:00:00Z',rendered_at:null,basis:'fixture_saved_native_observation',committed_selected_generation:null,parent_snapshot_sha256:null});
const snapshot=(side)=>({typed_report_sha256:(side==='before'?'1':'2').repeat(64),typed_report_sha_profile:'typed-crawl-report-serde-json-v1',typed_report_json_bytes:4096,caller_binding:{job_id:side==='before'?BEFORE:AFTER,generation:3,origin:'app_declared_imported_or_detached_report',basis:'{"declaration":"app_declared","sql_attested":false}',active_checkpoint:false},caller_binding_attested_by_core:false,coverage:{saved_page_count:54,pending:1,limited:side==='before',cancelled:false,mixed_age_selected_recrawl:side==='after',operation_engine:null,operation_coverage:null,website_completeness_proven:false}});
function envelope() {
  const pages=Array.from({length:55},(_,index)=>{
    const kind=index===0?'changed':index===1?'added':index===2?'removed':'unchanged';
    const fields=index===0?[
      field('title',fact('observed','Alter Titel'),fact('observed',markup),'changed'),
      field('captured_text',fact('partial','gleicher gespeicherter Präfix'),fact('partial','gleicher gespeicherter Präfix'),'unknown',false),
      field('hash',fact('observed','same','raw_http_html-v1'),fact('observed','same','rendered_dom_html-v1'),'unknown',false),
      field('empty_vs_unknown',fact('observed_empty',[]),fact('unknown',null,null),'unknown',false),
      field('json_ld',fact('observed',[{name:'old'}],'json-structural-v1'),fact('observed',[{name:markup}],'json-structural-v1'),'changed'),
      ...Array.from({length:56},(_,i)=>field('extra_'+i,fact('observed',i),fact('observed',i)))
    ]:[field('status',fact(index===1?'missing':'observed',index===1?null:200),fact(index===2?'missing':'observed',index===2?null:200),['added','removed'].includes(kind)?'unknown':'unchanged',!['added','removed'].includes(kind))];
    const unknown=fields.filter(item=>item.outcome==='unknown').map(item=>item.field);
    return {comparison_key:`https://comparison.fixture/${index}`,kind,before:kind==='added'?null:reference(index,'before'),after:kind==='removed'?null:reference(index,'after'),fields,equality_proven_for_compared_fields:kind==='unchanged'&&index!==3,unknown_fields:index===3?['not_captured']:unknown};
  });
  return {comparison:{schema_version:1,engine:'open-seo-spider/saved-comparison-v1',identity_profile:'exact-recorded-requested-url-v1',mapping_profile:'origin-segment-path-prefix-join-key-only-v1',structured_data_profile:'json-structural-equality-block-and-array-order-v1',config:{engine:'open-seo-spider/saved-comparison-v1',mappings:[],include_selected_text:false,limits:{max_pages_per_snapshot:100000}},config_sha256:'3'.repeat(64),before_snapshot:snapshot('before'),after_snapshot:snapshot('after'),counts:{added:1,removed:1,changed:1,unchanged:52,pages_with_unknown_fields:4},pages,ignored_runtime_fields:['fetched_at'],website_requests:0,vendor_parity:'NOT_PROVEN',limitations:['Fixture values; no native comparison or persistence executed.','Raw JSON-LD source positions are not stored.']},website_requests:0};
}
const jobs=[{id:AFTER,status:'completed',seed_url:'https://after.fixture/',started_at:'2026-10-13T09:00:00Z',generation:3,recrawl_available:true,stats:{}},{id:BEFORE,status:'limited',seed_url:'https://before.fixture/',started_at:'2026-10-12T09:00:00Z',generation:2,recrawl_available:true,stats:{}},{id:ACTIVE,status:'pausing',seed_url:'https://active.fixture/',started_at:'2026-10-13T10:00:00Z',generation:7,recrawl_available:false,stats:{}}];
const failure=(code='comparison_identity_collision')=>({error:{code,message:markup,collisions:[{side:'before',comparison_key:'https://production.fixture/private',occurrences:[{ordinal:0,original_requested_url:'https://staging.fixture/private'},{ordinal:1,original_requested_url:markup}]}]},context:{side:'before',job_id:BEFORE,status:'paused',observed_generation:4,expected_generation:2},website_requests:0});
module.exports={BEFORE,AFTER,ACTIVE,markup,formula,envelope,jobs,failure};
