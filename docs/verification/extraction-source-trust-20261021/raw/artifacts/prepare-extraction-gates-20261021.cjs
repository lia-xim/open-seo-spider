const fs=require('fs'),crypto=require('crypto'),assert=require('assert/strict');
const app=fs.readFileSync('web/app.js'),sha=crypto.createHash('sha256').update(app).digest('hex');
let s=fs.readFileSync('scripts/check-ui-fixtures.mjs','utf8');
const previous="bytes:527607,sha256:'aef8cd313a93fa83bcabd25bff04859b6c587b2d50e6e56a7e8ab34de8507b9f'";
assert(s.includes(previous));s=s.replace(previous,`bytes:${app.length},sha256:'${sha}'`);fs.writeFileSync('scripts/check-ui-fixtures.mjs',s);
let pure=fs.readFileSync('artifacts/run-xlsx-source-followup-pure-20261021-r2.cjs','utf8').replaceAll('xlsx-source-followup-pure-20261021-r2','extraction-source-pure-20261021-r1');
pure=pure.replace('console.log(JSON.stringify(proof));','console.log(JSON.stringify({complete:proof.complete,passed:proof.passed,failure:proof.failure,changed:proof.changed}));');
fs.writeFileSync('artifacts/run-extraction-source-pure-20261021-r1.cjs',pure);
console.log({bytes:app.length,sha256:sha});
