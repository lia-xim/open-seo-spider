// Independent archive reader: ZIP framing and CRC use Node builtins, XML uses
// the owned Chromium DOMParser. No product generator/parser is imported.
import assert from 'node:assert/strict';
import {crc32} from 'node:zlib';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const NS='http://www.sitemaps.org/schemas/sitemap/0.9';
export function readStoredZip(raw){
 assert(Buffer.isBuffer(raw)&&raw.length>=22&&raw.length<=8*1024*1024);
 const end=raw.length-22;assert.equal(raw.readUInt32LE(end),0x06054b50,'Complete EOCD with no hidden tail required');
 assert.equal(raw.readUInt16LE(end+4),0);assert.equal(raw.readUInt16LE(end+6),0);assert.equal(raw.readUInt16LE(end+20),0);
 const count=raw.readUInt16LE(end+10);assert(count>=3&&count<=16);assert.equal(raw.readUInt16LE(end+8),count);
 const centralSize=raw.readUInt32LE(end+12),central=raw.readUInt32LE(end+16);assert.equal(central+centralSize,end);
 let at=central;const files=new Map(),ranges=[];
 for(let n=0;n<count;n++){
  assert(at+46<=end);assert.equal(raw.readUInt32LE(at),0x02014b50);const flags=raw.readUInt16LE(at+8),method=raw.readUInt16LE(at+10),crc=raw.readUInt32LE(at+16),compressed=raw.readUInt32LE(at+20),size=raw.readUInt32LE(at+24),len=raw.readUInt16LE(at+28),extra=raw.readUInt16LE(at+30),comment=raw.readUInt16LE(at+32),local=raw.readUInt32LE(at+42);
  assert.equal(flags&~0x808,0,'No encrypted or unknown flags');assert.equal(method,0,'This small fixture must use independently readable ZIP Stored');assert.equal(compressed,size);assert.equal(raw.readUInt16LE(at+34),0);assert(size<=8*1024*1024);assert(at+46+len+extra+comment<=end);
  const name=new TextDecoder('utf-8',{fatal:true}).decode(raw.subarray(at+46,at+46+len));assert(/^(?:manifest\.json|sitemap-index\.xml|sitemap-\d{5}\.xml)$/.test(name));assert(!files.has(name),'Unique flat archive names required');
  assert(local+30<=central);assert.equal(raw.readUInt32LE(local),0x04034b50);assert.equal(raw.readUInt16LE(local+6),flags);assert.equal(raw.readUInt16LE(local+8),0);const localLen=raw.readUInt16LE(local+26),localExtra=raw.readUInt16LE(local+28);assert.equal(localLen,len);assert(raw.subarray(local+30,local+30+len).equals(Buffer.from(name)));
  const start=local+30+localLen+localExtra,finish=start+size;assert(finish<=central);const bytes=raw.subarray(start,finish);assert.equal(crc32(bytes),crc,'Independent full entry CRC');let next=finish;
  if(flags&8){if(raw.readUInt32LE(next)===0x08074b50)next+=4;assert(next+12<=central);assert.equal(raw.readUInt32LE(next),crc);assert.equal(raw.readUInt32LE(next+4),size);assert.equal(raw.readUInt32LE(next+8),size);next+=12;}else{assert.equal(raw.readUInt32LE(local+14),crc);assert.equal(raw.readUInt32LE(local+18),size);assert.equal(raw.readUInt32LE(local+22),size);}
  ranges.push([local,next]);files.set(name,Buffer.from(bytes));at+=46+len+extra+comment;
 }
 assert.equal(at,end);ranges.sort((a,b)=>a[0]-b[0]);assert.equal(ranges[0][0],0);for(let n=1;n<ranges.length;n++)assert.equal(ranges[n][0],ranges[n-1][1]);assert.equal(ranges.at(-1)[1],central,'Every ZIP byte is accounted for');return files;
}
export async function inspectBundle(browser,raw,{report,expectedUrls,options,segments}){
 const files=readStoredZip(raw);assert(files.has('manifest.json')&&files.has('sitemap-index.xml'));
 const manifest=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(files.get('manifest.json')));assert.equal(manifest.engine,'open-seo-spider/segmented-sitemap-generation-v1');assert.equal(manifest.schema_version,1);assert.equal(manifest.negative_membership_claim,false);assert.equal(manifest.selected_recrawl,false);assert(manifest.source_scope.includes('no current freshness/full-site completeness inference'));assert.equal(manifest.input_pages,6);assert.equal(manifest.excluded_pages,3);assert.deepEqual(manifest.options,options);assert.match(manifest.report_sha256,/^[a-f0-9]{64}$/);
 assert.equal(manifest.memberships.length,3);assert.deepEqual(manifest.memberships.map(m=>m.url).sort(),[...expectedUrls].sort());assert.equal(new Set(manifest.memberships.map(m=>m.url)).size,3);
 const names=[...files.keys()].filter(n=>/^sitemap-\d/.test(n)).sort();if(segments!==undefined)assert.equal(names.length,segments);assert.equal(files.size,names.length+2);assert.equal(manifest.files.length,names.length+1);assert.deepEqual(manifest.files.map(f=>f.name).sort(),[...names,'sitemap-index.xml'].sort());
 const domContext=await browser.newContext();const page=await domContext.newPage();let documents;
 try{documents=await page.evaluate(({documents,NS})=>documents.map(({name,text})=>{
  if(/<!DOCTYPE/i.test(text))throw Error('Unexpected XML doctype');const doc=new DOMParser().parseFromString(text,'application/xml');if(doc.getElementsByTagName('parsererror').length)throw Error('XML parse error');if(doc.documentElement.namespaceURI!==NS)throw Error('Wrong sitemap namespace');
  const direct=(e,local,ns=NS)=>[...e.children].filter(c=>c.localName===local&&c.namespaceURI===ns);const one=(e,local,ns=NS)=>{const v=direct(e,local,ns);if(v.length!==1)throw Error('Nonunique '+local);return v[0].textContent;};
  if(doc.documentElement.localName==='sitemapindex')return{name,kind:'index',locs:direct(doc.documentElement,'sitemap').map(e=>one(e,'loc'))};
  if(doc.documentElement.localName!=='urlset')throw Error('Unknown root');return{name,kind:'urlset',rows:direct(doc.documentElement,'url').map(e=>({url:one(e,'loc'),lastmod:direct(e,'lastmod').map(v=>v.textContent),images:direct(e,'image','http://www.google.com/schemas/sitemap-image/1.1').map(v=>one(v,'loc','http://www.google.com/schemas/sitemap-image/1.1')),hreflang:direct(e,'link','http://www.w3.org/1999/xhtml').map(v=>({language:v.getAttribute('hreflang'),url:v.getAttribute('href'),rel:v.getAttribute('rel')}))}))};
 }),{documents:[...files].filter(([n])=>n.endsWith('.xml')).map(([name,b])=>({name,text:new TextDecoder('utf-8',{fatal:true}).decode(b)})),NS});}finally{await domContext.close();}
 const index=documents.find(d=>d.name==='sitemap-index.xml');assert.equal(index.kind,'index');assert.deepEqual(index.locs,[...names].map(n=>new URL(n,options.public_base_url).href));assert.equal(new Set(index.locs).size,names.length);
 const all=[];
 for(const descriptor of manifest.files){const b=files.get(descriptor.name);assert(b);assert.equal(b.length,descriptor.uncompressed_bytes);assert.equal(sha(b),descriptor.sha256);assert.equal(descriptor.public_url,new URL(descriptor.name,options.public_base_url).href);const d=documents.find(v=>v.name===descriptor.name);assert(d);assert.equal(descriptor.kind,d.kind==='index'?'sitemapindex':'urlset');if(d.kind==='index'){assert.equal(descriptor.url_count,names.length);continue;}assert.equal(descriptor.url_count,d.rows.length);assert(d.rows.length>0&&d.rows.length<=options.max_urls_per_file);assert(b.length<=options.max_urlset_uncompressed_bytes);
  for(const row of d.rows){all.push(row.url);const member=manifest.memberships.find(m=>m.url===row.url);assert(member);assert.equal(member.segment,descriptor.name);assert.equal(report.pages[member.page_ordinal].url,row.url);assert.deepEqual(row.images,member.images);assert.equal(row.images.length,1);assert.equal(row.images[0],new URL('/image%20name.png?x=1&y=2',options.public_base_url).href);assert.deepEqual(row.hreflang.map(({language,url})=>({language,url})),member.hreflang.map(({language,url})=>({language,url})));assert.equal(row.hreflang.length,3);assert(row.hreflang.every(v=>v.rel==='alternate'));assert(row.hreflang.some(v=>v.url===row.url));assert(member.hreflang.every(v=>v.source_ordinals.length>0));assert.deepEqual(row.lastmod,['2024-02-29']);assert.equal(member.lastmod.value,'2024-02-29');assert.equal(member.lastmod.source.source,'captured_sitemap');assert.equal(member.lastmod.source.original_value,'2024-02-29');assert(member.lastmod.source.observation_id&&member.lastmod.source.entry_id);assert(Number.isSafeInteger(member.lastmod.source.field_ordinal));}
 }
 for(const member of manifest.memberships){
  const page=report.pages[member.page_ordinal];assert(page.links.some(l=>l.kind==='image'&&l.url===member.images[0]));const pairs=rows=>rows.map(r=>r.language+'\t'+r.url).sort();assert.deepEqual(pairs(member.hreflang),pairs(page.hreflang));
  const source=member.lastmod.source,ledger=report.sitemap_catalog.rows;assert(Array.isArray(ledger));const entry=ledger.filter(r=>r.kind==='entry').map(r=>r.row).find(r=>r.entry_id===source.entry_id&&r.observation_id===source.observation_id);assert(entry,'Lastmod provenance must identify an actual saved entry');const field=entry.observed.lastmod.find(f=>f.ordinal===source.field_ordinal);assert(field);assert.equal(field.decoded_value,source.original_value);assert(ledger.filter(r=>r.kind==='location').map(r=>r.row).some(r=>r.observation_id===entry.observation_id&&r.observed.entry_ordinal===entry.observed.ordinal&&r.observed.normalized_fetch_url===member.url&&r.observed.role==='page_location'));assert(ledger.some(r=>r.kind==='document'&&r.row.observation_id===entry.observation_id));
 }
 assert.deepEqual(all.sort(),[...expectedUrls].sort());assert.equal(new Set(all).size,3);assert(files.get('sitemap-index.xml').length<=options.max_index_uncompressed_bytes);assert(raw.length<=options.max_total_output_bytes);
 return{bundle_sha256:sha(raw),bundle_bytes:raw.length,canonical_report_sha256:manifest.report_sha256,manifest,files:[...files].map(([name,b])=>({name,bytes:b.length,sha256:sha(b)})),actual_independent_xml_documents:documents.length};
}
