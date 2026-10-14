const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const base='E:/CodexWork/open-seo-spider-01a12ac3-20261013',root=path.join(base,'sitemap-csv-streaming-integration-v1');
const sha=b=>createHash('sha256').update(b).digest('hex'),expected=new Map(),inputs=[];
for(const [dir,pin] of [
 ['s4-csv-root-session-v1','115531f52ad1df716de592f68e317c1e3897470615a9ddc36c2833805358481a'],
 ['s4-csv-streaming-app-v1','3dac7530629186a60d9da074f12a765167dc5e470110d4c63469f6d9a90ed99e'],
 ['s4-csv-control-core-v1','aaceebb8167634bf0c748e92340948ed8b84ca3c1540e874b8c951f04782bebb'],
 ['s4-csv-streaming-tests-v1','83707b519f05b0b7e964ac25c4fdfacfbf998791409b7149218b0ae45fc891f0'],
 ['s4-csv-reviewed-postfix-v1','5f97d60596b1ba05549b81a124ada1f2680341696f4aaf569343a0bfbb744932'],
 ['s4-csv-compat-wrappers-v1','c1da9605e574e302edeaece94e7f220b73c7df9df76fdbc12502b0f7e4035024'],
]){const raw=fs.readFileSync(path.join(base,dir,'manifest.json'));assert.equal(sha(raw),pin);const m=JSON.parse(raw);for(const e of m.entries??m.payloads??m.files){const b=fs.readFileSync(path.join(base,dir,e.payload??e.path));assert.equal(sha(b),e.sha256??e.after_sha256);expected.set(e.path,sha(b));}inputs.push({path:dir+'/manifest.json',sha256:pin});}
const hookRaw=fs.readFileSync(path.join(__dirname,'promote-sitemap-csv-wire-tests-v1.json')),hook=JSON.parse(hookRaw);assert.equal(hook.parent.before_sha256,expected.get(hook.parent.path));expected.set(hook.parent.path,hook.parent.sha256);assert.equal(expected.size,26);
const edits=[...expected].sort(([a],[b])=>a.localeCompare(b)).map(([p,pin])=>{const file=path.join(root,p),before=fs.readFileSync(file);assert.equal(sha(before),pin,p);const text=before.toString('utf8');assert(!/\r(?!\n)/.test(text),'No bare CR '+p);const after=Buffer.from(text.replaceAll('\r\n','\n'));return {path:p,before_sha256:pin,sha256:sha(after),before_bytes:before.length,bytes:after.length,crlf_replaced:(text.match(/\r\n/g)??[]).length,raw:after};});
const receiptPath=__filename.replace(/\.cjs$/,'.json');assert(!fs.existsSync(receiptPath));
for(const e of edits){if(e.crlf_replaced)fs.writeFileSync(path.join(root,e.path),e.raw);assert.equal(sha(fs.readFileSync(path.join(root,e.path))),e.sha256);}
const receipt={created_at:new Date().toISOString(),scope:'Only 26 exact already-owned reviewed product paths; CRLF to LF to satisfy repository *.rs eol=lf and exact staged/frozen compiler bytes. No other bytes, original immutable handoffs preserved.',inputs,hook_receipt_sha256:sha(hookRaw),entries:edits.map(({raw,...e})=>e),tests_executed:0,compiler:'NOT_RUN'};
fs.writeFileSync(receiptPath,JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({owned:edits.length,normalized:edits.filter(e=>e.crlf_replaced).map(e=>({path:e.path,lines:e.crlf_replaced})),compiler_tests:'NOT_RUN'}));
