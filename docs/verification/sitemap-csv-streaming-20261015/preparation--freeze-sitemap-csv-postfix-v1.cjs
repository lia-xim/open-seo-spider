const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',root=path.join(base,'sitemap-csv-streaming-integration-v1'),out=path.join(base,'s4-csv-reviewed-postfix-v1');
const sha=b=>createHash('sha256').update(b).digest('hex');
const inputs=[
 ['s4-csv-root-session-v1/manifest.json','115531f52ad1df716de592f68e317c1e3897470615a9ddc36c2833805358481a'],
 ['s4-csv-streaming-app-v1/manifest.json','3dac7530629186a60d9da074f12a765167dc5e470110d4c63469f6d9a90ed99e'],
 ['s4-csv-root-session-review-v1/receipt.json','d82396cf8c4d2bc5b80b4c2ab2bc16d145158b6d3733ca453a66ec3d7dbbd7c7'],
 ['s4-csv-app-independent-review-v1/receipt.json','199aea85c56fd8d81b77fbaaff34580c59ba33a9a5743144cd7706a0d5260a52'],
 ['s4-csv-control-independent-review-v1/receipt.json','1259eeb385f566f7f3819caf7bcfa99124894a214b9e8558d6a7dd1fbe534cc2'],
];
const verified=inputs.map(([p,pin])=>{const b=fs.readFileSync(path.join(base,p));assert.equal(sha(b),pin,p);return {path:p,sha256:pin};});
const rm=JSON.parse(fs.readFileSync(path.join(base,inputs[0][0]))),am=JSON.parse(fs.readFileSync(path.join(base,inputs[1][0])));
const targets=[
 ['crates/core/src/store/sitemap_read_context.rs','s4-csv-root-session-v1',rm.entries.find(e=>e.path==='crates/core/src/store/sitemap_read_context.rs')],
 ['crates/app/src/sitemap_membership.rs','s4-csv-streaming-app-v1',am.payloads.find(e=>e.path==='crates/app/src/sitemap_membership.rs')],
 ['crates/app/src/server/sitemap_csv_stream.rs','s4-csv-streaming-app-v1',am.payloads.find(e=>e.path==='crates/app/src/server/sitemap_csv_stream.rs')],
];
assert(!fs.existsSync(out),'fresh immutable destination');
const entries=targets.map(([p,dir,e])=>{assert(e);const old=fs.readFileSync(path.join(base,dir,e.payload??p));assert.equal(sha(old),e.sha256??e.after_sha256,p);const raw=fs.readFileSync(path.join(root,p));assert.notEqual(sha(raw),sha(old),p);return {path:p,payload:'product/'+p,before_payload:dir+'/'+(e.payload??p),before_sha256:sha(old),sha256:sha(raw),bytes:raw.length,raw};});
const membership=entries[1].raw.toString().replaceAll('\r\n','\n'),beforeMembership=fs.readFileSync(path.join(base,entries[1].before_payload),'utf8').replaceAll('\r\n','\n');
assert.equal(membership,beforeMembership.replace('#[cfg(test)]\nstruct BoundedBytes','struct BoundedBytes').replace('#[cfg(test)]\nimpl Write for BoundedBytes','impl Write for BoundedBytes'),'membership change only the two mistaken cfg attributes');
fs.mkdirSync(out);for(const e of entries){const dest=path.join(out,e.payload);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.writeFileSync(dest,e.raw,{flag:'wx'});assert.equal(sha(fs.readFileSync(dest)),e.sha256);}
const manifest={schema_version:1,name:'s4-csv-reviewed-postfix-v1',immutable:true,recorded_at:new Date().toISOString(),baseline_commit:'d105fb8a0c527ffd99164e44ec644bd0e49f61a5',inputs:verified,entries:entries.map(({raw,...e})=>e),corrections:[
 {file:entries[0].path,issue:'Static saved-config borrow conflict',change:'Clone saved-config SHA into owned header; original remains borrowed by prepared analysis until actual drop.'},
 {file:entries[0].path,issue:'Cooperative saved-config work checkpoints',change:'Actual shared control before saved-config metadata and after parsing/policy; chunked control.sha256 over actual unchanged saved bytes.'},
 {file:entries[1].path,issue:'Production JSON helper incorrectly gated as test-only',change:'Remove only two cfg(test) attributes on actual BoundedBytes type and Write implementation.'},
 {file:entries[2].path,issue:'Active queued drain falsely counted as idle after producer join',change:'Body retains shared progress sender and refreshes last activity on actual emitted prefix; fixed queue/chunk bounds and absolute stream deadline remain.'},
 {file:entries[2].path,issue:'Test-observation DTO/method unused in production',change:'Gate only ExportStats and snapshot with cfg(test); actual counters/worker behavior remain.'},
],checks:{combined_cargo_fmt_exit:0,git_diff_check_exit:0,source_only:true,compiler:'NOT_RUN',tests:'NOT_RUN',sqlite:'NOT_RUN',http:'NOT_RUN',browser:'NOT_RUN',native:'NOT_RUN'},runtime_correctness:'NOT_PROVEN'};
const raw=Buffer.from(JSON.stringify(manifest,null,2)+'\n');fs.writeFileSync(path.join(out,'manifest.json'),raw,{flag:'wx'});console.log(JSON.stringify({out,manifest_sha256:sha(raw),entries:manifest.entries}));
