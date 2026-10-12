// Actual browser -> Rust server -> local browser worker -> owned fixture test.
// Requires running Open SEO Spider and renderer; never crawls external sites.
const http=require('node:http'),fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const{pathToFileURL}=require('node:url');
const web=path.resolve(__dirname,'..');
const candidates=[process.env.SPIDER_PLAYWRIGHT_MODULE,path.resolve(web,'../renderer/node_modules/playwright-core'),path.join(process.env.APPDATA||'','npm/node_modules/rea-agents/node_modules/playwright-core')].filter(Boolean);
let playwright;
for(const candidate of candidates){try{playwright=require(candidate);break}catch(_){}}
if(!playwright)throw new Error('Install renderer dependencies or set SPIDER_PLAYWRIGHT_MODULE.');
const app=process.env.SPIDER_APP_URL||'http://127.0.0.1:8787';
const worker=process.env.SPIDER_RENDERER_URL||'http://127.0.0.1:8790/render';
const chrome=process.env.SPIDER_CHROME||path.join(process.env.ProgramFiles||'C:/Program Files','Google/Chrome/Application/chrome.exe');
async function testBrowserPath(){if(process.env.SPIDER_CHROME)return process.env.SPIDER_CHROME;try{return await(await import(pathToFileURL(path.resolve(web,'../renderer/server.mjs')).href)).findBrowserExecutable()}catch(_){return fs.existsSync(chrome)?chrome:null}}
const fixture=http.createServer((req,res)=>{
  const origin=`http://127.0.0.1:${fixture.address().port}`;
  if(req.url==='/robots.txt'){res.setHeader('content-type','text/plain');return res.end('User-agent: *\nAllow: /\n')}
  if(req.url==='/sitemap.xml'||req.url==='/favicon.ico'){res.statusCode=404;return res.end()}
  res.setHeader('content-type','text/html; charset=utf-8');
  if(req.url==='/dynamic')res.setHeader('set-cookie','seo_fixture=private_fixture_value; Path=/; SameSite=Lax');
  if(req.url==='/dynamic')res.end(`<!doctype html><html lang="de"><head><title>Raw SEO title</title><meta name="description" content="Owned rendering fixture"><link rel="canonical" href="${origin}/dynamic"></head><body><main><h1>Raw heading</h1><span class="price">10.00</span><a href="/excluded">Excluded target</a><button></button><img src="data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22/%3E"></main><script>setTimeout(()=>{document.title='Rendered SEO title';document.querySelector('h1').textContent='Rendered heading';document.querySelector('.price').textContent='20.00';const a=document.createElement('a');a.href='/js-only';a.textContent='Discovered through JavaScript';document.querySelector('main').appendChild(a);console.error('owned-renderer-fixture');},50)</script></body></html>`);
  else res.end('<!doctype html><html lang="de"><head><title>Child page</title></head><body><h1>Child page</h1><span class="price">30.00</span></body></html>');
});

(async()=>{
  await new Promise(resolve=>fixture.listen(0,'127.0.0.1',resolve));
  const executable=await testBrowserPath();
  const browser=await playwright.chromium.launch({...(executable?{executablePath:executable}:{}),headless:true});
  const page=await browser.newPage({viewport:{width:1440,height:1000},acceptDownloads:true});
  const consoleErrors=[],checks=[];
  page.on('pageerror',error=>consoleErrors.push(error.message));
  const checked=name=>{checks.push(name);console.log(`PASS ${name}`)};
  const seed=`http://127.0.0.1:${fixture.address().port}/dynamic`;
  try{
    await page.goto(app);await page.locator('#connection-state.connected').waitFor();
    await page.click('#settings-open');
    await page.check('[name=javascript_rendering]');await page.fill('[name=renderer_endpoint]',worker);await page.fill('[name=renderer_wait_ms]','500');await page.check('[name=renderer_accessibility]');await page.check('[name=renderer_screenshot]');
    await page.check('[name=renderer_collect_cookies]');
    const scripts=[{name:'titleAndLinks',script:'await Promise.resolve(); return {title:document.title,links:document.links.length};'},{name:'scriptFailure',script:'throw new Error("Owned script error");'},{name:'laterResult',script:'return 42;'}];
    await page.fill('[name=custom_javascript_json]',JSON.stringify(scripts));
    await page.locator('#advanced-config summary').click();
    await page.fill('[name=custom_extractions_json]',JSON.stringify([{name:'price',selector:'.price',attribute:null},{name:'links',selector:'a',attribute:'href'},{name:'xpathPrice',method:'xpath',selector:'//span[@class="price"]/text()'},{name:'xpathCount',method:'xpath',selector:'count(//a)'},{name:'regexPrice',method:'regex',selector:'class="price">([0-9.]+)',regex_group:1}]));
    await page.fill('[name=exclude_patterns_text]','/excluded');
    await page.click('#settings-form button[type=submit]');await page.fill('#seed-url',seed);await page.click('#start-button');
    await page.waitForFunction(()=>document.getElementById('crawl-state').textContent.includes('abgeschlossen'),{},{timeout:60000});
    const id=await page.locator('#crawl-history').inputValue();
    const response=await fetch(`${app}/api/crawls/${id}/pages?limit=1000`);assert.equal(response.status,200);const data=await response.json();
    const dynamic=data.pages.find(record=>record.url===seed);assert.ok(dynamic,'seed observation');assert.equal(dynamic.rendered,true,dynamic.renderer_error||'rendering was not captured');assert.equal(dynamic.titles[0],'Rendered SEO title');assert.equal(dynamic.raw_metadata.titles[0],'Raw SEO title');assert.equal(dynamic.h1[0],'Rendered heading');
    assert.ok(data.pages.some(record=>record.url.endsWith('/js-only')));assert.ok(!data.pages.some(record=>record.url.endsWith('/excluded')));checked('real renderer discovers JS-only links and respects URL exclusion');
    assert.deepEqual(dynamic.custom_extractions.find(entry=>entry.name==='price').values,['20.00']);assert.equal(dynamic.custom_extractions.find(entry=>entry.name==='price').source,'rendered_html');assert.deepEqual(dynamic.raw_metadata.custom_extractions.find(entry=>entry.name==='price').values,['10.00']);checked('raw and rendered named CSS extraction evidence');
    for(const name of ['xpathPrice','regexPrice']){assert.deepEqual(dynamic.custom_extractions.find(entry=>entry.name===name).values,['20.00']);assert.deepEqual(dynamic.raw_metadata.custom_extractions.find(entry=>entry.name===name).values,['10.00']);assert.equal(dynamic.custom_extractions.find(entry=>entry.name===name).matched_count,1)}assert.deepEqual(dynamic.custom_extractions.find(entry=>entry.name==='xpathCount').values,['2']);assert.equal(dynamic.custom_extractions.find(entry=>entry.name==='xpathCount').result_type,'number');checked('actual XPath text and scalar plus regex capture compare raw and rendered HTML');
    assert.deepEqual(dynamic.custom_javascript_results.find(entry=>entry.name==='titleAndLinks').value,{title:'Rendered SEO title',links:2});assert.match(dynamic.custom_javascript_results.find(entry=>entry.name==='scriptFailure').error,/Owned script error/);assert.equal(dynamic.custom_javascript_results.find(entry=>entry.name==='laterResult').value,42);
    const cookie=dynamic.browser_cookies.find(entry=>entry.name==='seo_fixture');assert.ok(cookie);assert.equal(cookie.sameSite,'Lax');assert.equal(cookie.value_length,21);assert.ok(!Object.hasOwn(cookie,'value'));assert.ok(data.pages.filter(record=>record.url!==seed).every(record=>record.browser_cookies.length===0));checked('Rust persists async JavaScript results individual errors and isolated redacted cookies');
    assert.equal(dynamic.accessibility.engine,'axe-core');assert.ok(dynamic.accessibility.violations.some(violation=>violation.id==='button-name'));assert.ok(dynamic.accessibility.violations.some(violation=>violation.id==='image-alt'));assert.equal(Buffer.from(dynamic.screenshot_base64,'base64').subarray(0,8).toString('hex'),'89504e470d0a1a0a');assert.ok(dynamic.console_errors.length);assert.ok(dynamic.network_requests.length);assert.match(dynamic.browser_version,/^\d+\./);assert.ok(dynamic.rendered_viewport.width>0&&dynamic.rendered_viewport.height>0);assert.equal(dynamic.render_wait_ms,500);assert.equal(dynamic.network_events_dropped,0);checked('actual axe violations screenshot PNG network console and browser provenance observations');
    await page.click('[data-section=titles]');await page.fill('#table-search',seed);await page.waitForFunction(()=>document.querySelectorAll('#results-table tbody tr.click-row').length===1);assert.match(await page.locator('#results-table tbody').innerText(),/Rendered SEO title/);await page.locator('#results-table tbody tr.click-row').click();assert.match(await page.locator('#inspector-content').innerText(),/Raw SEO title/);assert.match(await page.locator('#inspector-content').innerText(),/rendered_html/);await page.locator('.rendered-screenshot summary').click();await page.waitForFunction(()=>document.querySelector('.rendered-screenshot img')?.naturalWidth>0);checked('live UI rendered metadata raw comparison and lazy screenshot preview');
    await page.click('[data-section=extractions]');await page.selectOption('#table-filter','present');assert.match(await page.locator('#results-table tbody').innerText(),/20\.00/);assert.match(await page.locator('#results-table tbody').innerText(),/Gerenderter DOM/);checked('live extraction table displays actual DOM values');
    await page.click('[data-section=javascript]');await page.selectOption('#table-filter','error');assert.match(await page.locator('#results-table tbody').innerText(),/Owned script error/);await page.selectOption('#table-filter','success');assert.match(await page.locator('#results-table tbody').innerText(),/Rendered SEO title/);await page.click('[data-section=cookies]');assert.match(await page.locator('#results-table tbody').innerText(),/seo_fixture/);assert.ok(!(await page.locator('#results-table tbody').innerText()).includes('private_fixture_value'));checked('live JavaScript and cookie tables filter captured results');
    const jsCsv=await(await fetch(`${app}/api/crawls/${id}/javascript.csv`)).text(),cookieCsv=await(await fetch(`${app}/api/crawls/${id}/cookies.csv`)).text();assert.match(jsCsv,/Owned script error/);assert.match(jsCsv,/titleAndLinks/);assert.match(cookieCsv,/seo_fixture/);assert.ok(!cookieCsv.includes('private_fixture_value'));checked('dedicated JavaScript and cookie CSV exports contain persisted observations');
    const pending=page.waitForEvent('download');await page.click('#export-project');await(await pending).saveAs(path.join(__dirname,'ui-renderer-live-project.json'));const report=JSON.parse(fs.readFileSync(path.join(__dirname,'ui-renderer-live-project.json'),'utf8'));const exported=report.pages.find(record=>record.url===seed);assert.equal(exported.rendered,true);assert.equal(exported.browser_version,dynamic.browser_version);assert.deepEqual(exported.rendered_viewport,dynamic.rendered_viewport);assert.equal(exported.render_wait_ms,500);assert.equal(exported.network_events_dropped,0);assert.deepEqual(exported.custom_javascript_results,dynamic.custom_javascript_results);assert.deepEqual(exported.browser_cookies,dynamic.browser_cookies);checked('render evidence scripts cookies and browser provenance survive project export');
    const legacy=JSON.parse(fs.readFileSync(path.join(__dirname,'ui-live-project.json'),'utf8'));await page.setInputFiles('#import-file',{name:'legacy-report.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(legacy))});await page.waitForFunction(()=>document.getElementById('message').textContent.includes('importiert'));assert.equal(await page.locator('#metric-fetched').innerText(),String(legacy.pages.length));checked('legacy pre-extraction JSON report import remains compatible');
    await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.locator('.main-footer').scrollIntoViewIfNeeded();const footer=await page.locator('.main-footer').boundingBox();assert.ok(footer.y<844&&footer.y+footer.height>0);checked('final live mobile no overflow and footer reachable');
    assert.deepEqual(consoleErrors,[]);
    const proof={tested_at:new Date().toISOString(),scope:'Actual final app browser -> Rust -> local Chromium renderer -> owned JS fixture -> persisted exports and legacy import',app_url:app,worker_url:worker,crawl_id:id,fixture_seed:seed,observed_urls:data.total,checks,javascript_runtime_errors:consoleErrors};fs.writeFileSync(path.join(__dirname,'ui-renderer-live-results.json'),JSON.stringify(proof,null,2));console.log(JSON.stringify(proof));
  }finally{await browser.close();await new Promise(resolve=>{fixture.close(resolve);fixture.closeAllConnections()})}
})().catch(error=>{console.error(error);fixture.close();fixture.closeAllConnections();process.exitCode=1});
