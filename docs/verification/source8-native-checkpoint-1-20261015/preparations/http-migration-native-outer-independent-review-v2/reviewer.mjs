// Bounded independent retarget review. No target imports or runtime calls.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import{createHash}from'node:crypto';import{execFileSync}from'node:child_process';import{fileURLToPath}from'node:url';
const B='E:/CodexWork/open-seo-spider-01a12ac3-20261013',P=path.join(B,'http-migration-native-outer-preparation-v2'),O=path.join(B,'http-migration-native-outer-independent-review-v2');
const sha=b=>createHash('sha256').update(b).digest('hex'),read=p=>fs.readFileSync(p),json=p=>JSON.parse(read(p).toString('utf8')),pins=[];
function pin(r,h){const b=read(r.path);assert.equal(sha(b),h??r.sha256,r.path);if(r.bytes!==undefined)assert.equal(b.length,r.bytes);const p={path:r.path,sha256:sha(b),bytes:b.length};pins.push(p);return p;}
const proposal=pin({path:path.join(P,'manifest.json')},'68db357848827d1529911020d22e38c626b05bf6b76cdd2cd1a5abd084921e7c'),m=json(proposal.path);
for(const k of ['previous_preparation','previous_findings','parent_manifest','parent_independent_review','migration_manifest','migration_harness','migration_review','browser_helper','binder_manifest','binder','binder_review','outer','preflight','derivation','checks','release_contract','handoff','author_source'])pin(m[k]);
assert.equal(m.kind,'http_migration_native_outer_preparation_v2');assert.equal(m.status,'AUTHORED_NOT_RUN');assert(Object.values(m.future_actual_inputs).every(v=>v===null));
const prior=json(m.previous_preparation.path);assert.deepEqual(json(m.previous_findings.path).findings.map(f=>f.id),['MIGRATION-OUTER-01']);
for(const key of ['migration_manifest','migration_harness','migration_review','browser_helper'])assert.deepEqual(m[key],prior[key]);
const d=json(m.derivation.path);assert.equal(d.files.length,2);let operations=0;
for(const f of d.files){pin(f.before);pin(f.after);let cur=read(f.before.path);
 for(const op of f.operations){assert.equal(sha(cur),op.before_sha256);const a=Buffer.from(op.before_base64,'base64'),b=Buffer.from(op.after_base64,'base64');assert.equal(cur.indexOf(a),op.offset);assert.equal(cur.indexOf(a,op.offset+1),-1);cur=Buffer.concat([cur.subarray(0,op.offset),b,cur.subarray(op.offset+a.length)]);assert.equal(sha(cur),op.after_sha256);operations++;}
 assert(cur.equals(read(f.after.path)));
 for(const op of [...f.operations].reverse()){const a=Buffer.from(op.after_base64,'base64'),b=Buffer.from(op.before_base64,'base64');assert(cur.subarray(op.offset,op.offset+a.length).equals(a));cur=Buffer.concat([cur.subarray(0,op.offset),b,cur.subarray(op.offset+a.length)]);assert.equal(sha(cur),op.before_sha256);}
 assert(cur.equals(read(f.before.path)));
}
assert.equal(operations,9);
const critical=[];
const oldOuter=read(prior.outer.path).toString('utf8'),outer=read(m.outer.path).toString('utf8');
for(const[start,end]of[["  const lockFd = fs.openSync(lockPath, 'wx');",'  const failures = [], streams = {}, signals = [];'],['  const failures = [], streams = {}, signals = [];','  try {\n    readPinned(issuedPath,ISSUED_SHA);'],['  const complete = actualClose','  const proof = {']]){const cut=t=>{const i=t.indexOf(start),j=t.indexOf(end,i+start.length);assert(i>=0&&j>i);return Buffer.from(t.slice(i,j));},a=cut(oldOuter),b=cut(outer);assert(a.equals(b));critical.push({start,end,bytes:a.length,sha256:sha(a),byte_equal:true});}
assert.deepEqual(critical.map(r=>r.bytes),[290,3623,707]);
const bm=json(m.binder_manifest.path),br=json(m.binder_review.path);assert.equal(br.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(br.findings,[]);assert.equal(br.proposal_manifest.sha256,m.binder_manifest.sha256);assert.equal(br.binder.sha256,m.binder.sha256);
const freezer=bm.files.find(f=>f.path==='freeze-combined-source-v8.mjs');assert(freezer);const freezerRef=pin({path:path.join(path.dirname(m.binder_manifest.path),freezer.path),sha256:freezer.sha256,bytes:freezer.bytes});
const binder=read(m.binder.path).toString('utf8'),expected=/assert\.equal\(manifest\.freezer_sha256,'([a-f0-9]{64})'\)/.exec(binder)?.[1];assert.equal(expected,freezerRef.sha256);assert.equal(expected,'b86e687e8b12050fdf772ac996e95a9e7103a6b17f068083a716861901d9945a');
const oldBinder=read(prior.binder.path).toString('utf8');assert.equal(binder,oldBinder.replace("manifest.freezer_sha256,'2124d86290957b2a7cc1b1bdeb3b2c1e6427b4b91e2c623aa4bdadcee73fdce8'",`manifest.freezer_sha256,'${expected}'`));
const preflight=read(m.preflight.path).toString('utf8');
for(const r of [m.binder,m.binder_manifest,m.binder_review,m.migration_manifest,m.migration_harness,m.migration_review,m.browser_helper])assert(preflight.includes(r.sha256),r.sha256);
assert(preflight.includes("assert.equal(preparation.kind,'http_migration_native_outer_preparation_v2')"));assert(!preflight.includes('combined-v8-runner-preparation-v3'));assert(!preflight.includes('2eff7bcd89be0f43808b733bfb46fb52d377bc2f2dbf6ff0ad9455d2d619ca3d'));
const mapping=d.embedded_sha256_audit??d.embedded_hash_audit??d.literal_sha_audit??d.embedded_pins;let audit=[];
// The author inventory has a distinct field name; find its typed rows structurally.
for(const v of Object.values(d))if(Array.isArray(v)&&v.length&&v.every(r=>r&&typeof r==='object'&&r.actual_artifact&&r.value))audit=v;
assert.equal(audit.length,10);let actualOccurrences=0;
for(const f of d.files){const text=read(f.after.path).toString('utf8'),rows=audit.filter(r=>r.file===f.path),actual=[...text.matchAll(/[a-f0-9]{64}/g)].map(r=>r[0]);assert.deepEqual([...actual].sort(),rows.map(r=>r.value).sort());actualOccurrences+=actual.length;for(const row of rows){pin(row.actual_artifact);assert.equal(row.value,row.actual_artifact.sha256);}}
assert.equal(actualOccurrences,10);
const syntax=[];for(const p of [m.outer.path,m.preflight.path]){execFileSync(process.execPath,['--check',p],{stdio:'pipe'});syntax.push({path:p,exit:0,command:'node --check'});}
assert(!fs.existsSync(O));fs.mkdirSync(O);function put(n,v){const p=path.join(O,n),b=Buffer.from(JSON.stringify(v,null,2)+'\n');fs.writeFileSync(p,b,{flag:'wx'});return{path:p,sha256:sha(b),bytes:b.length};}
const checks=put('checks.json',{kind:'http_migration_native_outer_v2_independent_source_checks',proposal,pins,full_inverse_operations:operations,critical_regions:critical,embedded_sha256_occurrences_checked:actualOccurrences,current_binder_freezer_dependency_actual:true,unchanged_eight_inner_groups_helper_and_harness:true,unchanged_root_hex_token_and_four_child_cli_pairs:true,syntax,target_imports:0,target_calls:0,compiler_fixture_native_sql_http_tcp_browser_root_git_writes:0});
const receipt=put('receipt.json',{schema_version:1,kind:'http_migration_native_outer_independent_review',result:'NO_FINDINGS_BOUNDED_SOURCE_REVIEW',findings:[],proposal_manifest:proposal,outer:m.outer,preflight:m.preflight,previous_findings:m.previous_findings,binder:m.binder,binder_review:m.binder_review,migration_manifest:m.migration_manifest,migration_review:m.migration_review,source_checks:checks,closed_findings:[{id:'MIGRATION-OUTER-01',status:'SOURCE_FIX_AND_CURRENT_DEPENDENCY_VERIFIED',runtime:'NOT_RUN'}],review_scope:{full_inverse_operations:operations,critical_region_bytes:critical.map(r=>r.bytes),embedded_sha256_occurrences:actualOccurrences,source_pins:pins.length},qualification:'Narrow independent follow-up to the complete V1 admission and outer review. Current repaired BinderV4, actual freezer and direct V4 review are bound. Every other byte outside nine metadata retargets is unchanged, including all process/lock ownership and four-pair child CLI; current migration harness/helper remain exact. Actual Source8 source/full-suite/build/candidate/release and all eight native acceptance groups are future mandatory inputs. No import, runtime, compiler or grant was performed.'});
fs.writeFileSync(path.join(O,'reviewer.mjs'),read(fileURLToPath(import.meta.url)),{flag:'wx'});console.log(JSON.stringify(receipt));
