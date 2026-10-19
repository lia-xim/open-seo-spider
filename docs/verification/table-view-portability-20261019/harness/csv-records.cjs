'use strict';
const assert=require('node:assert/strict');

// Read exported RFC-style CSV with quoted commas, CRLF and escaped quotes.
module.exports=function csvRecords(bytes){
  const source=bytes.toString('utf8').replace(/^\uFEFF/,''),rows=[];
  let row=[],cell='',quoted=false;
  for(let i=0;i<source.length;i++){
    const c=source[i];
    if(c==='"'&&quoted&&source[i+1]==='"'){cell+='"';i++;}
    else if(c==='"')quoted=!quoted;
    else if(c===','&&!quoted){row.push(cell);cell='';}
    else if(c==='\r'&&!quoted){assert.equal(source[++i],'\n');row.push(cell);rows.push(row);row=[];cell='';}
    else cell+=c;
  }
  assert.equal(quoted,false);assert.equal(cell,'');assert.equal(row.length,0);
  const header=rows.shift();assert(header?.length);
  return rows.map(values=>{assert.equal(values.length,header.length);return Object.fromEntries(header.map((name,i)=>[name,values[i]]));});
};
