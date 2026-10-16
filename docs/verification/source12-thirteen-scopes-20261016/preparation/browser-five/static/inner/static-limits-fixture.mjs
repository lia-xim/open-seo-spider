// AUTHORED_NOT_RUN. Calling start() performs only a new owned loopback fixture.
import http from 'node:http';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const html=body=>Buffer.from('<!doctype html><html><head><meta charset="utf-8"><title>Owned static limits fixture</title></head><body>'+body+'</body></html>\r\n');
export async function startStaticLimitsFixture({checkpoint}){
  const hits=[],errors=[],sockets=new Set(),responses=new Map();let opened=0,closed=0,siteClosed=false;
  const many=Array.from({length:500},(_,i)=>'/many-leaf-'+String(i).padStart(3,'0'));
  const folder=['/folder-zero','/folder/one','/folder//two','/encoded%2Fslash','/trailing/'];
  const query=['/q-none','/q-empty?','/q-equal?a=1&a=2','/q-over?a&&','/q-encoded?a=%26','/q-semi?a;b;c'];
  const bytes=['/bytes/aa','/bytes/aaa','/bytes/aaaa'];
  function add(target,body=html('<p>Stored owned response.</p>'),status=200,type='text/html; charset=utf-8',location=null){assert(!responses.has(target));responses.set(target,{body,status,type,location});}
  add('/many',html(many.map(p=>'<a href="'+p+'">'+p+'</a>').join('\n')));for(const p of many)add(p);
  for(const p of [...folder,...query,...bytes,'/deep/path','/deep/target','/alias?keep=3','/alias?drop=1&drop=2&keep=3','/unicode-%C3%BC','/fragment-target'])add(p);
  add('/links',html('<a href="/folder-zero">allowed</a><a href="/deep/path">denied first</a><a href="/deep/path">denied second</a><a href="/q-equal?a=1&amp;a=2">query equal</a><a href="/q-over?a&amp;&amp;">query over</a>'));
  add('/redir',Buffer.alloc(0),302,'text/plain; charset=utf-8','/deep/target');
  add('/policy-redir',Buffer.alloc(0),302,'text/plain; charset=utf-8','/alias?drop=1&drop=2&keep=3');
  add('/resources',html('<link rel="stylesheet" href="/style.css"><img src="/image" alt="owned">'));
  add('/style.css',Buffer.from("/* owned CSS */\r\na{background:url('/deep/image.png')}\r\n"),200,'text/css; charset=utf-8');
  add('/image',Buffer.alloc(0),302,'image/png','/deep/final.png');
  add('/deep/image.png',Buffer.from('owned refused CSS target'),200,'image/png');add('/deep/final.png',Buffer.from('owned refused image redirect'),200,'image/png');
  add('/robots.txt',Buffer.from('User-agent: *\nAllow: /\n'),200,'text/plain; charset=utf-8');
  const server=http.createServer((req,res)=>{
    try{
      checkpoint();assert.equal(req.method,'GET','Unexpected method on owned website');
      const target=req.url;assert(typeof target==='string'&&target.startsWith('/')&&!target.startsWith('//'));
      let response=responses.get(target);if(target==='/map.xml?a=1&a=2')response={body:Buffer.from('<?xml version="1.0"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>'+origin+'/q-over?a&amp;&amp;</loc></url></urlset>'),status:200,type:'application/xml; charset=utf-8',location:null};
      assert(response,'Unplanned owned website target '+target);
      const hit={sequence:hits.length+1,method:req.method,raw_target:target,status:response.status,response_bytes:response.body.length,response_sha256:sha(response.body),request_end:false,response_finish:false,response_close:false,premature_close:false};hits.push(hit);
      req.once('end',()=>{hit.request_end=true;});req.once('aborted',()=>errors.push('Aborted fixture request '+target));req.once('error',e=>errors.push(String(e)));req.resume();
      res.once('finish',()=>{hit.response_finish=true;});res.once('close',()=>{hit.response_close=true;hit.premature_close=!res.writableFinished;if(hit.premature_close)errors.push('Premature fixture response '+target);});res.once('error',e=>errors.push(String(e)));
      res.writeHead(response.status,{'content-type':response.type,'content-length':response.body.length,connection:'close',...(response.location?{location:response.location}:{})});res.end(response.body);
    }catch(error){errors.push(String(error.stack??error));if(!res.headersSent)res.writeHead(500,{'content-length':0,connection:'close'});res.end();}
  });
  server.on('connection',socket=>{opened++;sockets.add(socket);socket.once('close',()=>{closed++;sockets.delete(socket);});socket.once('error',e=>errors.push(String(e)));});server.on('error',e=>errors.push(String(e)));
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});const origin='http://127.0.0.1:'+server.address().port;
  const snapshot=()=>structuredClone(hits);
  const complete=()=>{assert.deepEqual(errors,[]);assert(hits.every(h=>h.request_end&&h.response_finish&&h.response_close&&!h.premature_close),'Each actual GET must close its exact response normally');};
  return{origin,hits,errors,many,folder,query,bytes,snapshot,complete,
    fixtureBytes:()=>[...responses].map(([target,r])=>({target,status:r.status,content_type:r.type,location:r.location,bytes:r.body.length,sha256:sha(r.body)})),
    async close(){await new Promise((resolve,reject)=>server.close(e=>e?reject(e):resolve()));siteClosed=true;assert.equal(sockets.size,0);assert.equal(opened,closed);complete();return{actual_closed:siteClosed,sockets_opened:opened,sockets_closed:closed,retained:sockets.size,errors:structuredClone(errors)};},
    async failureClose(){server.closeAllConnections();await new Promise(resolve=>server.close(resolve));siteClosed=true;return{actual_closed:siteClosed,sockets_opened:opened,sockets_closed:closed,retained:sockets.size,errors:structuredClone(errors)};}
  };
}
