import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {createRendererServer} from '../renderer/server.mjs';

// The four opt-in Rust browser fixtures must use an owned worker, never the
// installed user's renderer at8790. Cargo's ordinary run deliberately ignores them.
const root=fileURLToPath(new URL('../',import.meta.url));
const worker=createRendererServer();
const expected=[
  'live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness',
  'live_browser_worker_pause_drains_render_and_retains_rendered_discoveries',
  'live_browser_worker_executes_javascript_for_core_crawl',
  'live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion',
].sort();
let outcome;
try {
  const address=await worker.listen(0);
  assert(![8787,8790].includes(address.port));
  const endpoint=`http://127.0.0.1:${address.port}/render`;
  const args=['test','-p','open-seo-spider-core','--locked','live_browser_worker_','--','--ignored'];
  console.log('Owned Rust browser fixture worker: '+endpoint);
  const child=spawn('cargo',args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','inherit'],
    env:{...process.env,OPEN_SEO_SPIDER_RENDERER:endpoint}});
  let startError,timedOut=false,output='';
  child.stdout.on('data',bytes=>{process.stdout.write(bytes);output+=bytes.toString('utf8');});
  child.once('error',error=>startError=error);
  const timer=setTimeout(()=>{timedOut=true;child.kill();},300000);
  outcome=await new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));
  clearTimeout(timer);
  assert.equal(startError,undefined,'Could not start Cargo');
  assert.equal(timedOut,false,'Rust browser fixture deadline exceeded');
  assert.equal(outcome.code,0,'Rust browser fixtures failed');
  const passed=[...output.replaceAll('\r','').matchAll(/^test (live_browser_worker_\S+) \.\.\. ok$/gm)].map(match=>match[1]).sort();
  assert.deepEqual(passed,expected,'All four named Rust browser fixtures must actually execute');
} finally {
  await worker.close();
}
console.log(JSON.stringify({kind:'owned_renderer_rust_fixtures',result:'PASS',executed:expected,outcome,
  scope:'Explicit opt-in Rust browser tests on an ephemeral owned renderer. Installed ports/stores are untouched.'}));
