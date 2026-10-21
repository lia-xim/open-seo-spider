'use strict';
const fs=require('node:fs'),path=require('node:path'),Module=require('node:module'),assert=require('node:assert/strict');
const file=path.join(__dirname,'csv-baseline.cjs'),source=fs.readFileSync(file,'utf8');
const before="path.join(__dirname,'csv-baseline-proof.json')";assert.equal(source.split(before).length-1,1);
const moduleCopy=new Module(__filename,module);moduleCopy.filename=__filename;moduleCopy.paths=module.paths;
moduleCopy._compile(source.replace(before,"path.join(__dirname,'csv-baseline-empty-segment-proof.json')"),__filename);
