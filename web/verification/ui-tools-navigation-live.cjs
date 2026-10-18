'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs');
const {chromium}=require('../../renderer/node_modules/playwright-core');
const artifactPath=require('./artifact-path.cjs');
const app=process.env.SPIDER_APP_URL;
assert(app&&new URL(app).hostname==='127.0.0.1'&&new URL(app).port!=='8787','Use an isolated E2E app');
(async()=>{
  const executablePath=process.env.SPIDER_CHROME||process.env.OPEN_SEO_BROWSER_PATH||await(await import('../../renderer/server.mjs')).findBrowserExecutable();
  const browser=await chromium.launch({headless:true,executablePath});
  const page=await browser.newPage(),errors=[],checks=[];
  page.on('pageerror',error=>errors.push(error.message));
  try{
    for(const width of [1440,320]){
      await page.setViewportSize({width,height:900});await page.goto(app);
      await page.locator('#connection-state.connected').waitFor();
      const links=await page.locator('.tools-menu a').evaluateAll(nodes=>nodes.map(n=>({href:n.getAttribute('href'),text:n.textContent.trim()})));
      assert.equal(links.length,9);assert.equal(new Set(links.map(x=>x.href)).size,9);
      for(const link of links){
        await page.locator('.tools-menu summary').click();
        const navigation=page.waitForResponse(r=>r.url()===app+link.href&&r.request().isNavigationRequest());
        await page.locator(`.tools-menu a[href="${link.href}"]`).click();
        assert.equal((await navigation).status(),200);
        await page.waitForURL(app+link.href);
        assert(await page.locator('h1').isVisible(),link.href);
        assert(await page.locator('a[href="/"]').first().isVisible(),link.href+' return link');
        assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),link.href+' overflow at '+width);
        await page.locator('a[href="/"]').first().click();
        await page.locator('#connection-state.connected').waitFor();
        checks.push({path:link.href,width,round_trip:true});
      }
    }
    assert.deepEqual(errors,[]);
    fs.writeFileSync(artifactPath('tools-navigation.json'),JSON.stringify({result:'PASS',checks,errors,scope:'Actual redesigned menu and native HTML routes at desktop/mobile. Tool feature correctness is tested in separate workflow suites.'},null,2)+'\n');
    console.log(`${checks.length} actual native navigation round trips passed`);
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
