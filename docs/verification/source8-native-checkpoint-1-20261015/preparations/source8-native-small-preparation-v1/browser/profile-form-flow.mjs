import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';

const fields=[
  {name:'sitemap_source_max_post_http_entity_bytes',native:'max_post_http_entity_bytes',initial:8388608,maximum:67108864},
  {name:'sitemap_source_max_decoded_xml_bytes',native:'max_decoded_xml_bytes',initial:8388608,maximum:67108864},
  {name:'sitemap_source_max_xml_events',native:'max_xml_events',initial:500000,maximum:2000000}
];
const storageKey='open-seo-spider-settings';
const customProfile={max_post_http_entity_bytes:16777216,max_decoded_xml_bytes:33554432,max_xml_events:1250000};

// Genuine DOM inputs and production HTTP only. These two deliberately mutating,
// tiny crawls run AFTER the original 405-page readonly snapshot interval.
export async function runSitemapSourceProfileForm(env){
  const {browser,api,siteOrigin,output,read,checkpoint,quiet,record,verifySavedConfig}=env;
  const context=await browser.newContext({serviceWorkers:'block',viewport:{width:1440,height:1000}});
  context.setDefaultTimeout(30000);
  const requests=[],blocked=[],pageErrors=[],probes=[];let result;
  await context.route('**/*',async route=>{
    const request=route.request(),url=new URL(request.url()),method=request.method();
    const allowed=url.origin===api&&(method==='GET'||method==='POST'&&url.pathname==='/api/crawls');
    const item={url:url.href,method,allowed};requests.push(item);
    if(!allowed){blocked.push(item);return route.abort('blockedbyclient');}
    return route.continue();
  });
  try{
    const page=await context.newPage();page.on('pageerror',error=>pageErrors.push(error.message));
    const input=name=>page.locator('#settings-form [name="'+name+'"]');
    const enabled=input('sitemap_source_processing_enabled');
    const stored=()=>page.evaluate(key=>localStorage.getItem(key),storageKey);
    async function ready(){await page.locator('#connection-state.connected').waitFor({state:'visible'});checkpoint();}
    async function open(){
      await page.locator('#settings-open').click();await page.locator('#settings-dialog').waitFor({state:'visible'});
      for(const selector of ['#advanced-config','#sitemap-source-processing-settings']){
        const details=page.locator(selector);
        if(await details.getAttribute('open')===null)await details.locator(':scope > summary').click();
      }
      checkpoint();
    }
    async function save(){await page.locator('#settings-form .dialog-footer button[type="submit"]').click();await page.locator('#settings-dialog').waitFor({state:'hidden'});checkpoint();}
    async function assertProfile(expectedEnabled,values){
      assert.equal(await enabled.isChecked(),expectedEnabled);
      assert.equal(await page.locator('#sitemap-source-processing-fields').evaluate(element=>element.disabled),!expectedEnabled);
      assert.equal(await page.locator('#sitemap-source-processing-fields').evaluate(element=>element.hidden),!expectedEnabled);
      for(const field of fields){assert.equal(await input(field.name).inputValue(),String(values[field.native]));assert.equal(await input(field.name).isDisabled(),!expectedEnabled);}
    }
    async function reload(){await page.reload({waitUntil:'networkidle'});await ready();}
    async function startProbe(name,profile){
      await page.waitForFunction(()=>!document.getElementById('start-button').disabled);
      await page.selectOption('#crawl-mode','spider');await page.locator('#seed-url').fill(siteOrigin+'/p404');
      const pending=page.waitForResponse(response=>response.url()===api+'/api/crawls'&&response.request().method()==='POST');
      await page.locator('#start-button').click();const response=await pending;assert.equal(response.status(),200);
      const payload=JSON.parse(response.request().postData()),created=await response.json(),id=(created.crawl??created).id;
      assert.match(id,/^[a-f0-9-]{36}$/);assert.equal(payload.max_body_bytes,2097152);assert.deepEqual(payload.seed_urls,[siteOrigin+'/p404']);
      assert.equal(payload.mode,'spider');assert.equal(payload.obey_robots,false);assert.equal(payload.javascript_rendering,false);assert.equal(payload.renderer,null);
      for(const key of ['sitemap_source_processing_enabled',...fields.map(field=>field.name)])assert.equal(Object.hasOwn(payload,key),false,'UI-only key must not enter native request: '+key);
      if(profile===null)assert.equal(Object.hasOwn(payload,'sitemap_source_processing'),false);
      else assert.deepEqual(payload.sitemap_source_processing,profile);
      const stop=Date.now()+60000;let completed;
      while(Date.now()<stop){
        checkpoint();const current=(await read('/api/crawls/'+id)).json;
        assert(!['failed','cancelled'].includes(current.status),current.error??current.status);
        if(current.status==='completed'){completed=current;break;}
        await new Promise(resolve=>setTimeout(resolve,100));
      }
      assert(completed,'Actual tiny UI-started crawl must complete');assert.equal(completed.stats.fetched,1);assert.equal(completed.stats.queued,0);assert.equal(completed.stats.in_flight,0);
      const saved=await verifySavedConfig(id,profile);
      const proof={name,id,request:payload,completed_status:completed.status,fetched:completed.stats.fetched,saved_config:saved};
      await fs.writeFile(path.join(output,'profile-'+name+'-actual-request.json'),JSON.stringify(proof,null,2)+'\n',{flag:'wx'});
      probes.push(proof);await reload();return proof;
    }
    await page.goto(api+'/',{waitUntil:'networkidle'});await ready();await open();
    const defaults=Object.fromEntries(fields.map(field=>[field.native,field.initial]));
    await assertProfile(false,defaults);
    for(const [name,value] of Object.entries({concurrency:'1',timeout_seconds:'10',max_pages:'0',delay_ms:'0',max_body_mb:'2'}))await input(name).fill(value);
    await input('obey_robots').uncheck();await input('javascript_rendering').uncheck();await save();
    let settings=JSON.parse(await stored());assert.equal(settings.sitemap_source_processing_enabled,false);assert.equal(settings.max_body_mb,2);
    for(const field of fields)assert.equal(settings[field.name],String(field.initial));
    await open();await assertProfile(false,defaults);await page.locator('#settings-close').click();await reload();await open();await assertProfile(false,defaults);
    record('actual_source_profile_off_defaults_save_reopen_reload',{enabled:false,fields_disabled_hidden:true,defaults,page_max_body_bytes:2097152,local_storage_persisted:true});

    await enabled.check();await assertProfile(true,defaults);
    const boundaries=[];
    for(const field of fields){
      assert.equal(await input(field.name).getAttribute('min'),'1');assert.equal(await input(field.name).getAttribute('max'),String(field.maximum));assert.equal(await input(field.name).getAttribute('step'),'1');assert.equal(await input(field.name).getAttribute('required'),'');
      for(const value of [1,field.maximum]){await input(field.name).fill(String(value));assert.equal(await input(field.name).evaluate(element=>element.checkValidity()),true);boundaries.push({field:field.native,value,valid:true});}
      await input(field.name).fill(String(field.initial));
    }
    record('actual_source_profile_enabled_defaults_exact_html_bounds',{defaults,boundaries,limits_are_form_validation_only:true});

    const unchangedStorage=await stored(),createsBefore=requests.filter(item=>item.method==='POST').length,invalid=[];
    for(const field of fields){
      for(const value of ['0',String(field.maximum+1),'','1.5']){
        await input(field.name).fill(value);assert.equal(await page.locator('#settings-form').evaluate(form=>form.checkValidity()),false);
        await page.locator('#settings-form .dialog-footer button[type="submit"]').click();await quiet();
        assert.equal(await page.locator('#settings-dialog').isVisible(),true);assert.equal(await stored(),unchangedStorage,'Invalid native form value must not save settings');
        assert.equal(requests.filter(item=>item.method==='POST').length,createsBefore,'Invalid save must not start any crawl');
        invalid.push({field:field.native,value,save_blocked:true});await input(field.name).fill(String(field.initial));
      }
    }
    record('actual_source_profile_invalid_range_empty_fraction_blocks_save',{cases:invalid,local_storage_unchanged:true,no_crawl_posts:true});

    await enabled.uncheck();await save();
    const off=await startProbe('off',null);
    record('actual_source_profile_off_real_start_native_omission_and_saved_config',{request_profile_field_present:false,saved_config:off.saved_config,job_id:off.id,page_max_body_bytes:2097152});

    await open();await enabled.check();for(const field of fields)await input(field.name).fill(String(customProfile[field.native]));await save();
    settings=JSON.parse(await stored());assert.equal(settings.sitemap_source_processing_enabled,true);assert.equal(settings.max_body_mb,2);
    for(const field of fields)assert.equal(settings[field.name],String(customProfile[field.native]));
    await open();await assertProfile(true,customProfile);await page.locator('#settings-close').click();await reload();await open();await assertProfile(true,customProfile);assert.equal(await input('max_body_mb').inputValue(),'2');await page.locator('#settings-close').click();
    const on=await startProbe('on',customProfile);
    record('actual_source_profile_on_exact_three_values_save_reopen_reload_real_start',{profile:customProfile,local_storage_persisted:true,saved_config:on.saved_config,job_id:on.id,page_max_body_bytes:2097152});
    assert.notEqual(off.id,on.id);assert.equal(blocked.length,0,JSON.stringify(blocked));assert.deepEqual(pageErrors,[]);
    result={authored_native_groups:5,actual_profile_crawls:2,probes,requests,blocked_requests:blocked,page_errors:pageErrors,no_route_fulfill:true,large_xml_fetch_and_effective_limits:'OPEN_NOT_EXERCISED'};
  }finally{await context.close();}
  result.context_closed=true;return result;
}
