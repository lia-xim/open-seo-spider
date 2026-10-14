const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{execFileSync}=require('node:child_process'),{createHash}=require('node:crypto');
const root='E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-csv-streaming-integration-v1',relative='crates/core/src/store-sitemap-source-tests.rs';
const old=execFileSync('git',['show','HEAD:'+relative],{cwd:root}),file=path.join(root,relative),actual=fs.readFileSync(file);assert(actual.equals(old),'Original parent prefix must be exact');
const next=Buffer.concat([old,Buffer.from('\n#[path = "store/sitemap_stored_source_tests.rs"]\nmod stored_source_session_tests;\n')]);fs.writeFileSync(file,next);
const sha=b=>createHash('sha256').update(b).digest('hex');fs.writeFileSync(__filename.replace(/\.cjs$/,'.json'),JSON.stringify({path:relative,before:sha(old),after:sha(next),original_prefix_preserved:true,tests_authored:2,tests_executed:0},null,2)+'\n',{flag:'wx'});
