import http from 'node:http';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {bound} from '../source18-directive-native-smoke-preparation-v6/runtime.mjs';

export const PAGE_PATHS=['/root','/visible','/hidden','/missing-page'];
export const GET_PATHS=[...PAGE_PATHS,'/sheet.css','/missing.css','/root-only.png','/shared.png','/visible-only.png','/hidden-only.png','/css-only.png'];
const sha=b=>createHash('sha256').update(b).digest('hex');
export function config(origin,mask) {
  assert([0,7].includes(mask));const sw={store:false,fetch:true};
  return {seed_urls:[origin+'/root'],crawl_links:true,concurrency:1,timeout_seconds:10,max_pages:0,max_depth:0,delay_ms:0,max_body_bytes:1048576,obey_robots:false,discover_sitemaps:false,include_subdomains:false,check_external:false,indexability_agent:'googlebot',user_agent:'Open SEO Spider owned broken-source native cases',exclude_patterns:['/not-followed$'],resource_discovery:{resources:{css:sw,images:sw},navigation:{internal_anchors:{store:true,fetch:true,expand:true}},css:{stylesheets:true,style_elements:false,style_attributes:false}},...(mask?{directive_reporting:{version:1,respect_noindex:true,respect_canonical:true,respect_next_prev:true}}:{})};
}
export function body(origin,p) {
  if(p==='/sheet.css')return {status:200,type:'text/css; charset=utf-8',text:'.owned{background-image:url("/css-only.png")}'};
  if(p==='/missing.css')return {status:404,type:'text/css; charset=utf-8',text:'/* Owned missing stylesheet. No declarations. */'};
  if(p.endsWith('.png'))return {status:404,type:'image/png',text:'Owned missing image '+p};
  if(p==='/missing-page')return {status:404,type:'text/html; charset=utf-8',text:'<!doctype html><html lang="en"><head><title>Owned404</title></head><body>Owned missing page</body></html>'};
  const a=(target,text)=>'<a href="'+target+'">'+text+'</a>',img=target=>'<img src="'+target+'" alt="Owned missing">';
  let head='<title>Owned '+p+'</title>',content='<h1>Owned '+p+'</h1>';
  if(p==='/root'){head+='<link rel="stylesheet" href="/sheet.css"><link rel="stylesheet" href="/missing.css">';content+=a('/visible','Visible owner')+a('/hidden','Hidden owner')+a('/missing-page','Broken root')+a('/not-followed','Retained excluded target')+img('/root-only.png')+img('/shared.png')+img('/shared.png');}
  else if(p==='/visible')content+=a('/missing-page','Broken visible')+img('/shared.png')+img('/visible-only.png');
  else {assert.equal(p,'/hidden');head+='<meta name="googlebot" content="noindex,follow"><link rel="canonical" href="'+origin+'/root"><link rel="prev" href="'+origin+'/root">';content+=a('/missing-page','Broken hidden first')+a('/missing-page','Broken hidden second')+img('/hidden-only.png')+img('/shared.png');}
  return {status:200,type:'text/html; charset=utf-8',text:'<!doctype html><html lang="en"><head>'+head+'</head><body>'+content+'</body></html>'};
}
export async function fixture(checkpoint) {
  const rows=[],failures=[],sockets=new Set();let origin=null,listenerClosed=false,closed=false;
  const server=http.createServer((req,res)=>{try{checkpoint('BROKEN_FIXTURE_REQUEST');const u=new URL(req.url,origin),allowed=req.method==='GET'&&!u.search&&GET_PATHS.includes(u.pathname);if(!allowed)failures.push('Unexpected owned fixture request '+req.method+' '+u.pathname+u.search);const response=allowed?body(origin,u.pathname):{status:500,type:'text/plain',text:'Unexpected owned fixture request'},raw=Buffer.from(response.text),row={method:req.method,path:u.pathname,query:u.search,status:response.status,body_sha256:sha(raw),body_bytes:raw.length,request_complete:false,response_finished:false,response_closed:false};rows.push(row);req.once('end',()=>{row.request_complete=req.complete;});req.resume();res.once('finish',()=>{row.response_finished=true;});res.once('close',()=>{row.response_closed=true;});res.writeHead(response.status,{'content-type':response.type,'content-length':raw.length,'cache-control':'no-store',connection:'close'});res.end(raw);}catch(e){failures.push(String(e));res.destroy(e);}});
  server.on('connection',s=>{sockets.add(s);s.once('close',()=>sockets.delete(s));s.on('error',e=>failures.push('fixture-socket:'+e));});server.on('error',e=>failures.push('fixture-listener:'+e));
  const owner={server,sockets,rows,failures,get origin(){return origin;},get closed(){return closed;},async close(){if(closed)return;await bound(new Promise((resolve,reject)=>server.close(e=>{listenerClosed=true;e?reject(e):resolve();})),10000,'Broken fixture actual listener close');assert.equal(sockets.size,0);closed=true;},receipt(){return {origin,rows,failures,socket_retained:sockets.size,listener_closed:listenerClosed,closed};}};
  checkpoint('BROKEN_FIXTURE_OWNER_RESERVED',owner);await bound(new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);}),10000,'Broken fixture listen');const address=server.address();assert(address&&typeof address==='object'&&!['8787','8790'].includes(String(address.port)));origin='http://127.0.0.1:'+address.port;return owner;
}
