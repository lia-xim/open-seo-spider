import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';

// AUTHORED / NOT_RUN. Root may compose this into a freshly source-bound native
// browser acceptance. It does not launch a browser/server or grant itself access.
export async function runMembershipCapacityForm(env) {
  const {browser, api, siteOrigin, output, read, checkpoint, record}=env;
  const context=await browser.newContext({serviceWorkers:'block',viewport:{width:1440,height:1000}});
  context.setDefaultTimeout(30000);
  const requests=[],blocked=[],errors=[],probes=[];
  await context.route('**/*',async route=>{
    const req=route.request(),url=new URL(req.url()),allowed=url.origin===api&&(req.method()==='GET'||req.method()==='POST'&&url.pathname==='/api/crawls');
    requests.push({url:url.href,method:req.method(),allowed});
    if(!allowed){blocked.push(req.url());return route.abort('blockedbyclient');}return route.continue();
  });
  try {
    const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
    const input=name=>page.locator('#settings-form [name="'+name+'"]');
    const fields=[['sitemap_membership_max_catalog_json_bytes','max_catalog_json_bytes',33554432],['sitemap_membership_max_row_json_bytes','max_row_json_bytes',262144],['sitemap_membership_max_rows','max_rows',1000000]];
    const storage=()=>page.evaluate(()=>localStorage.getItem('open-seo-spider-settings'));
    async function ready(){await page.locator('#connection-state.connected').waitFor({state:'visible'});checkpoint();}
    async function open(){await page.locator('#settings-open').click();for(const id of ['advanced-config','sitemap-source-processing-settings']){const d=page.locator('#'+id);if(await d.getAttribute('open')===null)await d.locator(':scope > summary').click();}}
    async function save(){await page.locator('#settings-form .dialog-footer button[type="submit"]').click();await page.locator('#settings-dialog').waitFor({state:'hidden'});checkpoint();}
    async function assertFields(enabled,values){assert.equal(await input('sitemap_membership_limits_enabled').isChecked(),enabled);const f=page.locator('#sitemap-membership-limits-fields');assert.equal(await f.evaluate(e=>e.disabled),!enabled);assert.equal(await f.evaluate(e=>e.hidden),!enabled);for(const [name,key]of fields)assert.equal(await input(name).inputValue(),String(values[key]));}
    async function start(label,expected){
      await page.selectOption('#crawl-mode','spider');await page.locator('#seed-url').fill(siteOrigin+'/p404');
      await page.waitForFunction(()=>!document.getElementById('start-button').disabled);
      const responseWait=page.waitForResponse(r=>r.url()===api+'/api/crawls'&&r.request().method()==='POST');await page.locator('#start-button').click();const response=await responseWait;assert.equal(response.status(),200);
      const payload=JSON.parse(response.request().postData());assert.deepEqual(payload.sitemap_urls,[]);assert.equal(payload.obey_robots,false);assert.equal(payload.check_external,false);assert.equal(payload.javascript_rendering,false);const created=await response.json(),id=(created.crawl??created).id;assert.match(id,/^[a-f0-9-]{36}$/);
      for(const name of ['sitemap_membership_limits_enabled',...fields.map(f=>f[0])])assert(!Object.hasOwn(payload,name));
      if(expected===null)assert(!Object.hasOwn(payload,'sitemap_membership_limits'));else assert.deepEqual(payload.sitemap_membership_limits,expected);
      const deadline=Date.now()+60000;let completed;
      while(Date.now()<deadline){checkpoint();const v=(await read('/api/crawls/'+id)).json;assert(!['failed','cancelled'].includes(v.status),v.error??v.status);if(v.status==='completed'){completed=v;break;}await new Promise(r=>setTimeout(r,100));}
      assert(completed);assert.equal(completed.stats.fetched,1);assert.equal(completed.stats.queued,0);assert.equal(completed.stats.in_flight,0);
      const project=(await read('/api/crawls/'+id+'/project.json')).json;assert.deepEqual(project.config.sitemap_urls,[]);assert.equal(project.config.obey_robots,false);assert.equal(project.config.check_external,false);assert(Object.hasOwn(project.config,'renderer'));assert.equal(project.config.renderer,null);
      if(expected===null)assert(!Object.hasOwn(project.config,'sitemap_membership_limits'));else assert.deepEqual(project.config.sitemap_membership_limits,expected);
      assert(project.sitemap_catalog,"Fresh crawl must expose its initialized sitemap catalog");
      assert.equal(project.sitemap_catalog.summary.limits.max_catalog_json_bytes,expected?.max_catalog_json_bytes??33554432);
      const proof={label,id,payload,completed,project_config:project.config};await fs.writeFile(path.join(output,'membership-'+label+'-private.json'),JSON.stringify(proof,null,2)+'\n',{flag:'wx'});probes.push(proof);return proof;
    }
    await page.goto(api+'/',{waitUntil:'networkidle'});await ready();await open();
    const defaults=Object.fromEntries(fields.map(([,key,value])=>[key,value]));await assertFields(false,defaults);
    for(const [name,value]of Object.entries({concurrency:'1',timeout_seconds:'10',max_pages:'0',delay_ms:'0',max_body_mb:'2'}))await input(name).fill(value);
    await input('obey_robots').uncheck();await input('check_external').uncheck();await input('javascript_rendering').uncheck();await input('sitemap_source_processing_enabled').uncheck();await save();await start('absent-default',null);
    await open();await input('sitemap_membership_limits_enabled').check();const high={...defaults,max_catalog_json_bytes:134217728};for(const [name,key]of fields)await input(name).fill(String(high[key]));await save();
    let saved=JSON.parse(await storage());assert.equal(saved.sitemap_membership_limits_enabled,true);assert.equal(saved.sitemap_membership_max_catalog_json_bytes,'134217728');
    await open();await assertFields(true,high);await page.locator('#settings-close').click();await page.reload({waitUntil:'networkidle'});await ready();await open();await assertFields(true,high);await page.locator('#settings-close').click();await start('explicit-128',high);
    await open();const before=await storage();await input('sitemap_membership_max_catalog_json_bytes').fill('100');await input('sitemap_membership_max_row_json_bytes').fill('101');await page.locator('#settings-form .dialog-footer button[type="submit"]').click();assert.equal(await page.locator('#settings-dialog').isVisible(),true);assert.equal(await storage(),before);assert.equal(await page.locator('#settings-error').isVisible(),true);
    await input('sitemap_membership_max_catalog_json_bytes').fill('134217729');assert.equal(await page.locator('#settings-form').evaluate(f=>f.checkValidity()),false);await page.locator('#settings-form .dialog-footer button[type="submit"]').click();assert.equal(await storage(),before);
    await input('sitemap_membership_max_catalog_json_bytes').fill('134217728');await input('sitemap_membership_max_row_json_bytes').fill('262144');await input('sitemap_membership_limits_enabled').uncheck();await save();await start('disabled-again',null);
    assert.deepEqual(blocked,[]);assert.deepEqual(errors,[]);record('membership_capacity_form_save_reopen_reload_payload_and_strict_invalid_controls',{probes:probes.length,limits_independent:true});
    return {result:'PASS_ACTUAL_MEMBERSHIP_FORM',probes,requests,blocked,errors,qualification:'Three genuine tiny UI-started crawls only. Does not prove50k retention, raw-body storage, total RSS or wire-limit changes.'};
  } finally { await context.close(); }
}
