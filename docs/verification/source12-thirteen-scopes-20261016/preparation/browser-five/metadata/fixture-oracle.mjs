// Literal fixture authority. Never imports product parsing/tokenization/aggregation.
import assert from 'node:assert/strict';import{createHash}from'node:crypto';
export const sha=b=>createHash('sha256').update(b).digest('hex');
export const CHANNELS=['title','meta_description','h1','h2','h3','h4','h5','h6'];
const blank=()=>Object.fromEntries(CHANNELS.map(c=>[c,[]]));
export const LONG_WORD='longword'.repeat(70),INERT_MARKUP='<img src=x onerror="window.NGRAM_INJECTED=1">';
const multi={...blank(),title:['Alpha beta','Alpha beta',''],meta_description:['desc one','desc one',''],h1:['red blue','red blue',''],h2:['left','right'],h3:['Café 東京',''],h4:[''],h5:[],h6:[INERT_MARKUP,LONG_WORD]};
const unicode={...blank(),title:['Alpha Äpfel 東京 αβ'],meta_description:['quote " café'],h1:['Äpfel Äpfel'],h2:['eins und zwei'],h3:['東京 αβ']};
export const FIXTURES=[
 {path:'/multi',raw:multi,body:'Genuine body text excluded because no optional capture.'},
 {path:'/empty',raw:blank(),body:'Known complete HTML with no retained channel elements.'},
 {path:'/unicode',raw:unicode,body:'Unicode sources remain separate.'},
 {path:'/long',raw:{...blank(),title:[Array.from({length:205},(_,i)=>'metaword'+String(i).padStart(3,'0')).join(' ')]},body:'Finite real metadata pagination.'},
 {path:'/notfound',status:404,raw:{...blank(),title:['must never count']},body:'An actual HTTP404 is unknown, never an observed empty200.'},
 {path:'/budget',raw:{...blank(),title:['Budget alpha'],meta_description:['Budget description'],h1:['Budget heading']},body:'Body text exceeds one byte and must be explicitly incomplete.'},
 {path:'/dom-a',raw:{...blank(),title:['Raw alpha'],meta_description:['raw description'],h1:['raw heading']},dom:{...blank(),title:['Rendered alpha','Rendered alpha'],meta_description:['rendered description','rendered description'],h1:['dom red blue'],h2:['dom left','dom right'],h3:[''],h6:[LONG_WORD]},body:'Real dynamic document A.'},
 {path:'/dom-b',raw:{...blank(),title:['Native beta'],h3:['native café']},dom:{...blank(),title:['DOM 東京'],h3:['DOM café 東京','DOM café 東京'],h4:[''],h5:['dynamic five']},body:'Real dynamic document B.'},
];
export const RAW_PATHS=FIXTURES.slice(0,5).map(p=>p.path),DOM_PATHS=['/dom-a','/dom-b'];
const html=s=>s.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
export function documentFor(spec){
 const titles=spec.raw.title.map(s=>'<title>'+html(s)+'</title>').join(''),meta=spec.raw.meta_description.map(s=>'<meta name="description" content="'+html(s)+'">').join('');
 const headings=CHANNELS.filter(c=>/^h\d$/.test(c)).flatMap(c=>spec.raw[c].map(s=>'<'+c+'>'+html(s)+'</'+c+'>')).join('');
 // Real browser script changes actual DOM. No render-protocol/PageRecord stub.
 const script=spec.dom?'<script>const wanted='+JSON.stringify(spec.dom).replaceAll('<','\\u003c')+';document.querySelectorAll("title,meta[name=description],h1,h2,h3,h4,h5,h6").forEach(e=>e.remove());for(const value of wanted.title){const e=document.createElement("title");e.textContent=value;document.head.append(e);}for(const value of wanted.meta_description){const e=document.createElement("meta");e.name="description";e.content=value;document.head.append(e);}for(const c of ["h1","h2","h3","h4","h5","h6"])for(const value of wanted[c]){const e=document.createElement(c);e.textContent=value;document.body.append(e);}window.NGRAM_REAL_DOM_READY=true;</script>':'';
 return Buffer.from('<!doctype html><html><head><meta charset="utf-8"><link rel="icon" href="data:image/x-icon;base64,AA">'+titles+meta+'</head><body><p>'+html(spec.body)+'</p>'+headings+script+'</body></html>');
}
export const metadataRequest=(channel,source='raw_http_html',extra={})=>({source_kind:source,retained_metadata_channel:{version:1,channel},options:{max_n:6,case_behavior:'unicode_lowercase',stopwords:[]},view:'totals',n:1,query:'',exact_tokens:null,limit:100,...extra});
const tokens=s=>(s.match(/[\p{L}\p{N}\p{M}]+/gu)||[]).map(s=>s.toLowerCase());
const cmpTokens=(a,b)=>{for(let i=0;i<Math.min(a.length,b.length);i++){const c=Buffer.compare(Buffer.from(a[i]),Buffer.from(b[i]));if(c)return c;}return a.length-b.length;};
export function expected(origin,paths,channel,n,{source='raw_http_html',stopwords=[]}={}){
 const totals=new Map(),sources=[];
 for(const p of paths){const spec=FIXTURES.find(s=>s.path===p);assert(spec);const elements=source==='rendered_dom_html'?spec.dom:spec.raw,complete=(spec.status??200)===200&&Boolean(elements),own=new Map(),spans=[];let count=0;
  if(complete)for(const [i,text]of elements[channel].entries()){const ts=tokens(text);spans.push({retained_element_ordinal:i+1,token_start:count+1,tokens:ts.length,utf8_bytes:Buffer.byteLength(text)});for(let at=0;at+n<=ts.length;at++){const t=ts.slice(at,at+n);if(t.some(t=>stopwords.includes(t)))continue;const k=JSON.stringify(t),r=own.get(k)??{tokens:t,occurrences:0,positions:[]};r.occurrences++;r.positions.push(count+at+1);own.set(k,r);}count+=ts.length;}
  for(const[k,r]of own){const t=totals.get(k)??{n,tokens:r.tokens,occurrences:0,document_frequency:0};t.occurrences+=r.occurrences;t.document_frequency++;totals.set(k,t);}
  sources.push({url:origin+p,complete,elements:complete?elements[channel]:null,spans:complete?spans:null,tokens:count,own});
 }
 return{rows:[...totals.values()].sort((a,b)=>b.occurrences-a.occurrences||a.n-b.n||cmpTokens(a.tokens,b.tokens)),sources};
}
export function assertTotals(rows,want){assert.equal(rows.length,want.rows.length);for(const[r,i]of rows.entries()){const a=i,b=want.rows[r];assert.equal(a.n,b.n);assert.deepEqual(a.tokens,b.tokens);assert.equal(a.occurrences,b.occurrences);assert.equal(a.document_frequency,b.document_frequency);assert.equal(a.source_memberships_complete,true);}}
export function assertSources(rows,want,{kind='raw_http_html',captured=false,report}={}){
 assert.equal(rows.length,want.sources.length);for(const row of rows){const s=row.source,e=want.sources.find(x=>x.url===s.requested_url);assert(e);assert.equal(s.counted,e.complete);const c=s.channel;assert(c);assert.equal(c.source_kind,kind);if(!e.complete){assert.equal(c.elements,null);assert(c.metadata_source==null);assert(c.unavailable_reason);continue;}
  assert.deepEqual(c.elements,e.spans);assert.equal(s.tokens,e.tokens);assert.equal(s.state,e.tokens?'complete':'complete_empty');assert.equal(c.element_set_sha256,sha(Buffer.from(JSON.stringify(e.elements))));assert.equal(c.hash_profile,'serde-json-retained-element-array-v1');assert.equal(c.metadata_source.authority_profile,'retained_html_metadata_v1');assert.match(c.metadata_source.revision,/^[a-f0-9]{64}$/);assert.equal(c.metadata_source.engine,'independent_html5_dom_v1');assert.equal(c.metadata_source.source,kind==='raw_http_html'?'raw_html':'rendered_html');assert.equal(c.source_document_url,e.url);const page=report?.pages.find(p=>p.url===e.url);if(page){assert.equal(c.metadata_source.fetched_at,page.fetched_at);assert.equal(c.metadata_source.rendered_at,kind==='raw_http_html'?null:page.rendered_at);}
  if(!captured){assert(s.source_revision==null);assert(s.capture_config_sha256==null);assert(c.capture_html_sha256==null);assert(c.capture_status==null);}else{assert.equal(c.capture_status,'incomplete');assert.match(c.capture_html_sha256,/^[a-f0-9]{64}$/);assert.notEqual(c.metadata_source.revision,c.capture_html_sha256);}
 }
}
// Independent strict RFC4180-style finite fixture parser, including quoted CRLF.
export function csvRows(bytes){const s=new TextDecoder('utf-8',{fatal:true}).decode(bytes),rows=[];let row=[],v='',quoted=false,closed=false;for(let i=0;i<s.length;i++){const c=s[i];if(quoted){if(c==='"'&&s[i+1]==='"'){v+='"';i++;}else if(c==='"'){quoted=false;closed=true;}else v+=c;}else if(c==='"'){assert(!v&&!closed);quoted=true;}else if(c===','){row.push(v);v='';closed=false;}else if(c==='\r'&&s[i+1]==='\n'){row.push(v);rows.push(row);row=[];v='';closed=false;i++;}else if(c==='\n'){row.push(v);rows.push(row);row=[];v='';closed=false;}else{assert(!closed);v+=c;}}assert(!quoted);assert.equal(v,'');assert.equal(row.length,0);const header=rows.shift();assert(header&&new Set(header).size===header.length);assert.equal(header.length,20);return{header,rows:rows.map(r=>{assert.equal(r.length,header.length);return Object.fromEntries(header.map((k,i)=>[k,r[i]]));})};}
export function assertCsv(bytes,want,{channel}={}){const{header,rows}=csvRows(bytes);assert.deepEqual(header,['snapshot_token','view','n','tokens_json','occurrences','document_frequency','source_ordinal','requested_url','source_revision','capture_config_sha256','text_sha256','basis','observed_generation','source_state','source_counted','first_positions_json','positions_sampled','website_requests','retained_metadata_channel_json','channel_source_json']);assert.equal(rows.length,want.rows.length);for(const[r,i]of rows.entries()){const e=want.rows[r];assert.equal(i.view,'totals');assert.equal(Number(i.n),e.n);assert.deepEqual(JSON.parse(i.tokens_json),e.tokens);assert.equal(Number(i.occurrences),e.occurrences);assert.equal(Number(i.document_frequency),e.document_frequency);assert.equal(i.website_requests,'0');assert.match(i.snapshot_token,/^[a-f0-9]{64}$/);assert.deepEqual(JSON.parse(i.retained_metadata_channel_json),{version:1,channel});}return{rows:rows.length,bytes:bytes.length,sha256:sha(bytes)};}
