// Ownership and final-witness controls; importing oracles launches no listeners.
// These controls reject incomplete witnesses and endpoints outside the owned bind profile.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {matrix, parsePostBindLine, finalCaseResult} from './sitemap-transport-e2e.mjs';

test('matrix preserves exact sixteen known case IDs, modes and target expectations',()=>{
  assert.deepEqual(matrix().map(row=>[row.id,row.policy,row.resources,row.format,row.extension,row.target]),[
    ['plain-00',false,false,'plain',false,1],['plain-01',false,true,'plain',false,1],
    ['plain-10',true,false,'plain',false,1],['plain-11',true,true,'plain',false,1],
    ['file-gzip-00',false,false,'file-gzip',false,1],['file-gzip-01',false,true,'file-gzip',false,1],
    ['file-gzip-10',true,false,'file-gzip',false,1],['file-gzip-11',true,true,'file-gzip',false,1],
    ['misleading-gz-0',false,false,'misleading-gz',false,1],['misleading-gz-1',true,false,'misleading-gz',false,1],
    ['http-gzip-0',false,false,'http-gzip',false,1],['http-gzip-1',true,false,'http-gzip',false,1],
    ['corrupt-gzip-0',false,false,'corrupt-gzip',false,0],['corrupt-gzip-1',true,false,'corrupt-gzip',false,0],
    ['extension-0',false,false,'plain',true,1],['extension-1',true,false,'plain',true,1],
  ]);
});
test('owned complete IPv4 bind stdout accepts Unix and Windows line endings',()=>{
  assert.equal(parsePostBindLine('Open SEO Spider: http://127.0.0.1:51234'),'http://127.0.0.1:51234');
  assert.equal(parsePostBindLine('Open SEO Spider: http://127.0.0.1:51234\r'),'http://127.0.0.1:51234');
});
test('user App and renderer ports are refused before API construction',()=>{
  for(const port of [8787,8790])assert.throws(()=>parsePostBindLine('Open SEO Spider: http://127.0.0.1:'+port));
});
test('port zero, overflow and noncanonical decimal are refused',()=>{
  for(const port of ['0','65536','051234','-1'])assert.throws(()=>parsePostBindLine('Open SEO Spider: http://127.0.0.1:'+port));
});
test('external host, wildcard host and URL suffixes are refused',()=>{
  for(const url of ['http://localhost:51234','http://0.0.0.0:51234','http://192.0.2.1:51234',
    'http://127.0.0.1:51234/','http://127.0.0.1:51234/api/crawls','http://127.0.0.1:51234?q=x'])
    assert.throws(()=>parsePostBindLine('Open SEO Spider: '+url));
});
test('stderr-like prefix, partial line and extra output cannot supply an endpoint',()=>{
  for(const line of ['log Open SEO Spider: http://127.0.0.1:51234','Open SEO Spider: http://127.0.0.1:',
    'Open SEO Spider: http://127.0.0.1:51234 more','Open SEO Spider: http://127.0.0.1:51234\nanything'])
    assert.throws(()=>parsePostBindLine(line));
});

function observed(target=0){return {definition:{id:'witness',policy:false,resources:false,format:'corrupt-gzip',extension:false,target},
  seed:'/seed',map:'/map',target:'/target',foreign:'/foreign',target_pages:target,foreign_pages:0,
  job_id:'fixture',generation:1,terminal_status:'completed',report_sha256:'report',config_sha256:'config',warning_count:1,warnings:['failure retained']};}
const baseHits=[{path:'/seed'},{path:'/map'}];
test('absence before final drain is never a negative HTTP proof',()=>{
  const result=finalCaseResult(observed(),baseHits,false);
  assert.equal(result.result,'NOT_PROVEN');assert.equal(result.http_counts_phase,'incomplete_witness');
});
test('clean drained corrupt-gzip absence passes',()=>{
  assert.equal(finalCaseResult(observed(),baseHits,true).result,'EXPECTED_BEHAVIOR_PASS');
});
test('late target request invalidates previously absent target after final drain',()=>{
  assert.equal(finalCaseResult(observed(),[...baseHits,{path:'/target'}],true).result,'EXPECTED_BEHAVIOR_FAILURE');
});
test('late foreign extension request invalidates final negative witness',()=>{
  assert.equal(finalCaseResult(observed(),[...baseHits,{path:'/foreign'}],true).result,'EXPECTED_BEHAVIOR_FAILURE');
});
test('positive target requires one actual request and one terminal page',()=>{
  assert.equal(finalCaseResult(observed(1),[...baseHits,{path:'/target'}],true).result,'EXPECTED_BEHAVIOR_PASS');
  assert.equal(finalCaseResult(observed(1),baseHits,true).result,'EXPECTED_BEHAVIOR_FAILURE');
});
test('duplicate target requests and failed independent seed controls fail',()=>{
  assert.equal(finalCaseResult(observed(1),[...baseHits,{path:'/target'},{path:'/target'}],true).result,'EXPECTED_BEHAVIOR_FAILURE');
  assert.equal(finalCaseResult(observed(),[{path:'/map'}],true).result,'EXPECTED_BEHAVIOR_FAILURE');
});
