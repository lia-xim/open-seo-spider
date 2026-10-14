const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {execFileSync} = require('node:child_process');
const {createHash} = require('node:crypto');
const repo = 'E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-streaming-integration-v1';
const p = 'crates/app/src/sitemap_membership.rs';
const before = execFileSync('git', ['show', 'HEAD:' + p], {cwd:repo}).toString('utf8').replaceAll('\r\n','\n');
const afterBytes = fs.readFileSync(path.join(repo,p));
const after = afterBytes.toString('utf8').replaceAll('\r\n','\n');
// Ignore formatting whitespace only outside ordinary Rust string literals.
function tokens(source) {
  let output='', quoted=false, escaped=false;
  for (const c of source) {
    if (quoted) { output+=c; if (escaped) escaped=false; else if (c==='\\') escaped=true; else if (c==='"') quoted=false; }
    else if(c==='"') {quoted=true;output+=c;} else if(!/\s/.test(c))output+=c;
  }
  assert(!quoted); return output;
}
const oldRecord = before.slice(before.indexOf('fn csv_record('),before.indexOf('pub(crate) fn csv('));
const newRecord = after.slice(after.indexOf('fn csv_record('),after.indexOf('fn write_csv_fields('));
assert.equal(tokens(newRecord),tokens(oldRecord),'Exact CSV quoting/CRLF formatter must remain identical');
const oldHeader = before.slice(before.indexOf('    write(\n        [',before.indexOf('pub(crate) fn csv(')),before.indexOf('    for row in catalog',before.indexOf('pub(crate) fn csv(')));
const expectedHeader=oldHeader.replace('    write(\n','    write_csv_fields(\n        writer,\n').replace(/\)\?;\s*$/m,')');
const headerFunction=after.slice(after.indexOf('fn write_csv_header('),after.indexOf('fn write_csv_row('));
const actualHeader=headerFunction.slice(headerFunction.indexOf('{')+1,headerFunction.lastIndexOf('}'));
assert.equal(tokens(actualHeader),tokens(expectedHeader),'Exact original header and column order');
const rowStart=before.indexOf('        let (kind, source, id, ordinal, raw, member, alias, decision)',before.indexOf('pub(crate) fn csv('));
let expectedRow=before.slice(rowStart,before.indexOf('    Ok((out.bytes, snapshot))',rowStart));
expectedRow=expectedRow.slice(0,expectedRow.lastIndexOf('    }\n')).replace('        write(\n','        write_csv_fields(\n            writer,\n').replace(/\)\?;\s*$/m,')');
const rowFunction=after.slice(after.indexOf('fn write_csv_row('),after.indexOf('pub(crate) fn csv('));
const actualRow=rowFunction.slice(rowFunction.indexOf('{')+1,rowFunction.lastIndexOf('}'));
assert.equal(tokens(actualRow),tokens(expectedRow),'Exact original typed-row mapping, formula protection and JSON');
assert(after.includes('    write_csv_header(&mut out)?;'));
assert(after.includes('        write_csv_row(&mut out, row)?;'));
const receipt={schema_version:1,result:'PASS_SOURCE_TOKEN_EQUIVALENCE_ONLY',recorded_at:new Date().toISOString(),
  repo,path:p,baseline_commit:'380beb1ccacf73659d0ac8dd7fdb39c543b88883',after_sha256:createHash('sha256').update(afterBytes).digest('hex'),
  controls:['existing csv_record exact tokens/string bytes','original header exact tokens/string bytes','original per-row mapping exact tokens/string bytes','actual buffered export calls shared helpers'],
  product_execution:'NOT_RUN',compiler_tests_native:'NOT_RUN_COORDINATED_COMPUTE_HOLD'};
fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify(receipt));
