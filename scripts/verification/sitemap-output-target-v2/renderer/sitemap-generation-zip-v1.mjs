// Independent bounded reader for this generator's flat, stored ZIP and XML output.
// This is acceptance code, not a general ZIP library or an XSD validator.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const decoder=new TextDecoder('utf-8',{fatal:true});
const table=Array.from({length:256},(_,n)=>{for(let k=0;k<8;k++)n=(n&1)?0xedb88320^(n>>>1):n>>>1;return n>>>0;});
export function crc32(bytes){let crc=0xffffffff;for(const byte of bytes)crc=table[(crc^byte)&255]^(crc>>>8);return (crc^0xffffffff)>>>0;}
export function storedZip(bytes,max=8388608){
  assert(Buffer.isBuffer(bytes)&&bytes.length<=max&&bytes.length>=22);
  const end=bytes.length-22;assert.equal(bytes.readUInt32LE(end),0x06054b50);assert.equal(bytes.readUInt16LE(end+20),0,'No ZIP comments');
  assert.equal(bytes.readUInt32LE(end+4),0,'Single ZIP disk');const count=bytes.readUInt16LE(end+10);assert(count>0&&count<=100,'Fixture ZIP entry bound');assert.equal(bytes.readUInt16LE(end+8),count);
  const directory=bytes.readUInt32LE(end+16),length=bytes.readUInt32LE(end+12);assert.equal(directory+length,end);
  const files=new Map();let cursor=directory,localCursor=0,total=0;
  for(let index=0;index<count;index++){
    assert(cursor+46<=end);assert.equal(bytes.readUInt32LE(cursor),0x02014b50);
    const flags=bytes.readUInt16LE(cursor+8),method=bytes.readUInt16LE(cursor+10),crc=bytes.readUInt32LE(cursor+16),compressed=bytes.readUInt32LE(cursor+20),size=bytes.readUInt32LE(cursor+24),nameLength=bytes.readUInt16LE(cursor+28),extraLength=bytes.readUInt16LE(cursor+30),commentLength=bytes.readUInt16LE(cursor+32),offset=bytes.readUInt32LE(cursor+42);
    assert.equal(method,0,'Stored generator entries');assert.equal(flags&~0x800,0,'UTF-8 only, no encryption/descriptors');assert.equal(compressed,size);assert.equal(bytes.readUInt16LE(cursor+34),0);assert.equal(offset,localCursor);
    assert(cursor+46+nameLength+extraLength+commentLength<=end);const name=decoder.decode(bytes.subarray(cursor+46,cursor+46+nameLength));assert(/^(?:manifest\.json|sitemap-index\.xml|sitemap-[0-9]{5}\.xml)$/.test(name));assert(!files.has(name));
    assert(offset+30<=directory);assert.equal(bytes.readUInt32LE(offset),0x04034b50);assert.equal(bytes.readUInt16LE(offset+6),flags);assert.equal(bytes.readUInt16LE(offset+8),method);assert.equal(bytes.readUInt32LE(offset+14),crc);assert.equal(bytes.readUInt32LE(offset+18),compressed);assert.equal(bytes.readUInt32LE(offset+22),size);
    const localName=bytes.readUInt16LE(offset+26),localExtra=bytes.readUInt16LE(offset+28),start=offset+30+localName+localExtra;assert.equal(localName,nameLength);assert.equal(decoder.decode(bytes.subarray(offset+30,offset+30+localName)),name);assert(start+size<=directory);total+=size;assert(total<=max);
    const body=bytes.subarray(start,start+size);assert.equal(crc32(body),crc,'Independent CRC '+name);files.set(name,body);localCursor=start+size;cursor+=46+nameLength+extraLength+commentLength;
  }
  assert.equal(cursor,end);assert.equal(localCursor,directory);return files;
}
function decode(value){return value.replace(/&([^;]*);/g,(_,entity)=>{const fixed={amp:'&',lt:'<',gt:'>',quot:'"',apos:"'"};if(Object.hasOwn(fixed,entity))return fixed[entity];assert(/^#(?:[0-9]+|x[0-9a-fA-F]+)$/.test(entity),'Unknown XML entity');const n=entity[1]==='x'?parseInt(entity.slice(2),16):Number(entity.slice(1));assert(n===9||n===10||n===13||n>=32&&n<=0xd7ff||n>=0xe000&&n<=0xfffd||n>=0x10000&&n<=0x10ffff);return String.fromCodePoint(n);});}
function plain(value){assert(!value.includes(']]>'),'Forbidden XML text terminator');assert(!/&(?!(?:amp|lt|gt|quot|apos|#[0-9]+|#x[0-9a-fA-F]+);)/.test(value),'Unescaped XML ampersand');assert(!/[\u0000-\u0008\u000B\u000C\u000E-\u001F\uFFFE\uFFFF]/.test(value));return decode(value);}
export function xml(bytes){
  let text=decoder.decode(bytes);assert(text.startsWith('<?xml version="1.0" encoding="UTF-8"?>\n'));text=text.slice(text.indexOf('?>')+2);assert(!/<!|<\?/.test(text),'No DTD, CDATA, comments or further PI in generated XML');
  const stack=[],roots=[];let cursor=0,nodes=0;
  while(cursor<text.length){
    if(text[cursor]!=='<'){let end=text.indexOf('<',cursor);if(end<0)end=text.length;const value=plain(text.slice(cursor,end));if(stack.length)stack.at(-1).text+=value;else assert.equal(value.trim(),'');cursor=end;continue;}
    const closing=/^<\/([A-Za-z_][\w:.-]*)\s*>/.exec(text.slice(cursor));if(closing){const node=stack.pop();assert(node&&node.qname===closing[1]);cursor+=closing[0].length;continue;}
    const open=/^<([A-Za-z_][\w:.-]*)/.exec(text.slice(cursor));assert(open,'Invalid XML opening');cursor+=open[0].length;const attrs={},namespace={...(stack.at(-1)?.namespace||{})};let self=false;
    for(;;){const rest=text.slice(cursor),close=/^\s*(\/?)>/.exec(rest);if(close){self=close[1]==='/';cursor+=close[0].length;break;}const attribute=/^\s+([A-Za-z_][\w:.-]*)\s*=\s*(["'])([^<]*?)\2/.exec(rest);assert(attribute,'Invalid XML attribute');assert(!Object.hasOwn(attrs,attribute[1]));attrs[attribute[1]]=plain(attribute[3]);cursor+=attribute[0].length;}
    for(const [key,value] of Object.entries(attrs)){if(key==='xmlns')namespace['']=value;else if(key.startsWith('xmlns:'))namespace[key.slice(6)]=value;}
    const parts=open[1].split(':');assert(parts.length<=2);const prefix=parts.length===2?parts[0]:'';assert(prefix===''||namespace[prefix]);const node={qname:open[1],name:parts.at(-1),ns:namespace[prefix]||'',namespace,attrs,text:'',children:[]};
    if(stack.length)stack.at(-1).children.push(node);else roots.push(node);assert(++nodes<=200000&&stack.length<16);if(!self)stack.push(node);
  }
  assert.equal(stack.length,0);assert.equal(roots.length,1);return roots[0];
}
const NS='http://www.sitemaps.org/schemas/sitemap/0.9',IMG='http://www.google.com/schemas/sitemap-image/1.1',XHTML='http://www.w3.org/1999/xhtml';
const children=(node,name,ns=NS)=>node.children.filter(n=>n.name===name&&n.ns===ns);
const one=(node,name,ns=NS)=>{const values=children(node,name,ns);assert.equal(values.length,1,name);return values[0];};
export function verifyBundle(bytes,{base,eligible,images,lastmods,ambiguous}={}){
  const files=storedZip(bytes),manifest=JSON.parse(decoder.decode(files.get('manifest.json')));assert.equal(manifest.engine,'open-seo-spider/segmented-sitemap-generation-v1');assert.equal(manifest.options.public_base_url,base);assert.equal(manifest.negative_membership_claim,false);
  assert.deepEqual([...files.keys()].sort(),['manifest.json',...manifest.files.map(f=>f.name)].sort());const rows=[];let segments=0;
  for(const descriptor of manifest.files){const body=files.get(descriptor.name);assert.equal(body.length,descriptor.uncompressed_bytes);assert.equal(sha(body),descriptor.sha256);const root=xml(body);assert.equal(root.ns,NS);
    if(descriptor.kind==='urlset'){segments++;assert.equal(root.name,'urlset');assert(body.length<=manifest.options.max_urlset_uncompressed_bytes);const urls=children(root,'url');assert.equal(urls.length,descriptor.url_count);assert(urls.length<=manifest.options.max_urls_per_file);
      for(const node of urls){const loc=one(node,'loc').text;assert([...loc].length<2048);const membership=manifest.memberships.find(m=>m.url===loc);assert(membership&&membership.segment===descriptor.name);rows.push(loc);assert.deepEqual(children(node,'image',IMG).map(n=>one(n,'loc',IMG).text),membership.images);assert.deepEqual(children(node,'link',XHTML).map(n=>({language:n.attrs.hreflang,url:n.attrs.href})),membership.hreflang.map(h=>({language:h.language,url:h.url})));assert(children(node,'link',XHTML).every(n=>n.attrs.rel==='alternate'));assert.deepEqual(children(node,'lastmod').map(n=>n.text),membership.lastmod?[membership.lastmod.value]:[]);assert.deepEqual(children(node,'priority').map(n=>n.text),manifest.options.priority===null?[]:[manifest.options.priority]);assert.deepEqual(children(node,'changefreq').map(n=>n.text),manifest.options.changefreq===null?[]:[manifest.options.changefreq]);}
    }else {assert.equal(descriptor.kind,'sitemapindex');assert.equal(root.name,'sitemapindex');assert.equal(children(root,'sitemap').length,descriptor.url_count);assert(body.length<=manifest.options.max_index_uncompressed_bytes);assert.deepEqual(children(root,'sitemap').map(n=>one(n,'loc').text),manifest.files.filter(f=>f.kind==='urlset').map(f=>f.public_url));}
  }
  assert(segments>0);assert.equal(new Set(rows).size,rows.length);assert.deepEqual(rows.slice().sort(),manifest.memberships.map(m=>m.url).sort());if(eligible)assert.deepEqual(rows.slice().sort(),eligible.slice().sort());
  if(images)for(const [url,values] of Object.entries(images))assert.deepEqual(manifest.memberships.find(m=>m.url===url).images,values);
  if(lastmods)for(const [url,value] of Object.entries(lastmods))assert.equal(manifest.memberships.find(m=>m.url===url).lastmod?.value??null,value);
  if(ambiguous)for(const url of ambiguous)assert(manifest.diagnostics.some(d=>d.code==='captured_lastmod_ambiguous'&&d.value===url));
  return {manifest,sha256:sha(bytes),bytes:bytes.length,files:[...files].map(([name,body])=>({name,bytes:body.length,sha256:sha(body)}))};
}
