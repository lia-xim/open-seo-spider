import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const E='E:/CodexWork/open-seo-spider-01a12ac3-20261013',R=E+'/mcp-file-tools-v1',D=E+'/combined-source-v11-draft-v1';
const H=b=>createHash('sha256').update(b).digest('hex');
const ref=p=>{const b=fs.readFileSync(p);return {path:p,sha256:H(b),bytes:b.length};};
const read=(p,h)=>{const b=fs.readFileSync(p);if(h)assert.equal(H(b),h,p);return JSON.parse(b);};
const git=(args,raw=false)=>execFileSync('C:/Program Files/Git/cmd/git.exe',args,{cwd:R,encoding:raw?undefined:'utf8',windowsHide:true,maxBuffer:16*1024*1024});
const [fmtReviewPath,fmtReviewSHA,directedSHA]=process.argv.slice(2);assert.equal(process.argv.length,5);assert(path.isAbsolute(fmtReviewPath));for(const s of[fmtReviewSHA,directedSHA])assert.match(s,/^[a-f0-9]{64}$/);
assert(!fs.existsSync(E+'/results/combined-native-outer-v4/active.lock'));assert(!fs.existsSync(D+'/private-publication.json'));
const parent=read(E+'/combined-source-v10/manifest.json','12b238c43bd504b71e4948a8e11d4622f34589a35290ca8ef923b833464206c7');
const pre=read(D+'/preformat.json','bc644f498902eb1c274738a1b0cc92db0552726dbfdb4c2502686fc736af7767');
const fmt=read(D+'/actual-format.json','cc78d21e25a716ee7074c894e35257776f89dc60351f4d42944552b82d03ab08');
const cl=read(D+'/actual-preliminary-clippy.json','6e5aff5e6fe01c6fdb25bacbcd1a48fd908820132cf4d80eb0b359f7a584c26c');
const review=read(fmtReviewPath,fmtReviewSHA);assert.equal(review.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(review.findings,[]);assert(JSON.stringify(review).includes(ref(D+'/preformat.json').sha256));assert(JSON.stringify(review).includes(ref(D+'/actual-format.json').sha256));
const compositionReviewPath=E+'/source11-composition-independent-review-v1/receipt.json',compositionReview=read(compositionReviewPath,'54775a6d6b8d29cb87b76ce1ae0af417148ac72fe15e0b84b57275082fe6243b');assert.equal(compositionReview.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(compositionReview.findings,[]);assert(JSON.stringify(compositionReview).includes(ref(D+'/preformat.json').sha256));
for(const r of[pre.response_review,pre.css_review]){const row=read(r.path,r.sha256);assert.equal(row.result,'NO_FINDINGS_BOUNDED_SOURCE_REVIEW');assert.deepEqual(row.findings,[]);}
const directedPath=E+'/results/source11-response-stream-tests-v1/proof.json',directed=read(directedPath,directedSHA);assert.equal(directed.result,'PASS_20_RESPONSE_STREAM_TEST_NAMES_ONLY');assert.equal(directed.new_names_verified,4);assert.equal(directed.old_names_verified,16);assert(directed.source_final_verified);assert.deepEqual(directed.failures,[]);assert.equal(directed.preliminary_clippy_sha256,ref(D+'/actual-preliminary-clippy.json').sha256);
assert.equal(cl.result,'PASS_PRELIMINARY_CLIPPY_ONLY');assert(cl.actual_close_event&&cl.source_final_verified);assert.deepEqual(cl.exit,{code:0,signal:null});assert.equal(cl.files.length,369);assert.equal(pre.owned_paths,5);assert.equal(fmt.changed_files.length,4);
const owned=pre.owned_source_paths,ownedSet=new Set(owned),base=new Map(parent.files.map(f=>[f.path,f]));assert.equal(owned.length,5);assert.equal(ownedSet.size,5);
assert.equal(git(['status','--porcelain']).trim(),'');assert.equal(git(['diff','--cached','--name-only']).trim(),'');const before=git(['rev-parse','HEAD']).trim();assert.equal(before,'4dcbf82fd1590d60272c6b8b27d7866ae3955ec4');
const privateRepo=JSON.parse(execFileSync('C:/Program Files/GitHub CLI/gh.exe',['repo','view','lia-xim/open-seo-spider','--json','isPrivate'],{encoding:'utf8',windowsHide:true}));assert.equal(privateRepo.isPrivate,true);
for(const f of parent.files){const b=fs.readFileSync(R+'/'+f.path);assert.equal(H(b),f.sha256,f.path);assert.equal(b.length,f.bytes);}
for(const f of cl.files){const b=fs.readFileSync(D+'/source/'+f.path);assert.equal(H(b),f.sha256,f.path);assert.equal(b.length,f.bytes);if(!ownedSet.has(f.path))assert.equal(f.sha256,base.get(f.path).sha256);else{assert.notEqual(f.sha256,base.get(f.path).sha256);const attrs=git(['check-attr','text','eol','--',f.path]);if(f.path.endsWith('.rs')){assert(attrs.includes(': text: set\n'));assert(attrs.includes(': eol: lf\n'));assert(!b.includes(13),'Reviewed Rust input already has exact LF bytes');}else assert(attrs.includes(': text: unset\n'));}}
// All guards above are read-only; only the exact five reviewed product paths follow.
for(const f of cl.files.filter(f=>ownedSet.has(f.path)))fs.writeFileSync(R+'/'+f.path,fs.readFileSync(D+'/source/'+f.path));
assert.deepEqual(git(['diff','--name-only','-z']).split('\0').filter(Boolean).sort(),[...owned].sort());
git(['add','--',...owned]);assert.deepEqual(git(['diff','--cached','--name-only','-z']).split('\0').filter(Boolean).sort(),[...owned].sort());git(['diff','--cached','--check']);git(['commit','-m','Join exact-length export workers before final HTTP frame and wrap settings hints']);
const commit=git(['rev-parse','HEAD']).trim();assert.notEqual(commit,before);
for(let i=0;i<cl.files.length;i+=24){const group=cl.files.slice(i,i+24),rows=git(['ls-tree','-rz',commit,'--',...group.map(f=>f.path)],true).toString('utf8').split('\0').filter(Boolean),oids=new Map(rows.map(r=>{const[a,p]=r.split('\t');return[p,a.split(' ')[2]];}));for(const f of group){const b=fs.readFileSync(R+'/'+f.path),blob=git(['cat-file','blob',oids.get(f.path)],true);assert.equal(H(b),f.sha256,f.path);assert.equal(b.length,f.bytes);assert.equal(H(blob),f.sha256,f.path);assert.equal(blob.length,f.bytes);}}
git(['push','origin','HEAD']);assert.equal(git(['ls-remote','origin','refs/heads/codex/mcp-file-tools-20261015']).trim().split(/\s+/)[0],commit);
const publication={kind:'actual_private_source11_code_publication',recorded_at:new Date().toISOString(),source_commit:commit,parent_code_commit:parent.source_commit,previous_documentation_commit:before,repository:'lia-xim/open-seo-spider',branch:'codex/mcp-file-tools-20261015',is_private:true,all369_root_source_hashes_verified:true,all369_commit_blob_hashes_verified:true,remote_exact:true,code_and_owned_raw_byte_attributes:true,owned_committed_paths:[...owned].sort(),additional_tracked_files:[],source_inputs:369,new_cargo_names:4,modified_inputs:5,unchanged_parent_inputs:364,proposal:pre.response_proposal,independent_review:pre.response_review,composition:ref(D+'/preformat.json'),composition_review:ref(compositionReviewPath),actual_format:ref(D+'/actual-format.json'),formatter_review:ref(fmtReviewPath),preliminary_clippy:ref(D+'/actual-preliminary-clippy.json'),directed_tests:ref(directedPath),source11_suite:'NOT_RUN',source11_build:'NOT_RUN',source11_native:'NOT_RUN',installed:'UNCHANGED_SOURCE5_R3'};
fs.writeFileSync(D+'/private-publication.json',JSON.stringify(publication,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({result:'PUBLISHED_EXACT_FIVE_REVIEWED_PATHS',commit,publication:ref(D+'/private-publication.json'),installed:publication.installed}));
