const artifactPath = require('./artifact-path.cjs');
// Actual browser -> Rust -> owned local HTTP resource fixture -> persisted exports.
// Requires a rebuilt running app. This never crawls a public website.
const http=require('node:http'),fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const web=path.resolve(__dirname,'..');
let playwright;
for(const module of [process.env.SPIDER_PLAYWRIGHT_MODULE,path.resolve(web,'../renderer/node_modules/playwright-core'),'playwright-core'].filter(Boolean)){try{playwright=require(module);break}catch(_){}}
if(!playwright)throw new Error('Run npm ci in renderer or set SPIDER_PLAYWRIGHT_MODULE.');
const app=process.env.SPIDER_APP_URL||'http://127.0.0.1:8787';
const title='Owned PDF <img src=x onerror=window.__resource_xss=true>';
const author=' \t=SUM(1,2)';
const text='Owned PDF resource fixture';
const png=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a3xkAAAAASUVORK5CYII=','base64');
function pdfString(value){return `(${value.replace(/\\/g,'\\\\').replace(/\(/g,'\\(').replace(/\)/g,'\\)').replace(/\t/g,'\\t')})`}
// PDFDocEncoding does not define every control character; Unicode text strings
// explicitly encode the Author TAB so exact metadata and formula protection are testable.
function pdfUnicodeString(value){return `<feff${Buffer.from(value,'utf16le').swap16().toString('hex')}>`}
function originalPDF(){
  const stream=`BT /F1 12 Tf 50 750 Td ${pdfString(text)} Tj ET`;
  const objects=[
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots [6 0 R] >>',
    `<< /Length ${Buffer.byteLength(stream)} >>\nstream\n${stream}\nendstream`,
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>',
    '<< /Type /Annot /Subtype /Link /Rect [0 0 20 20] /A << /S /URI /URI (/from-pdf) >> >>',
    `<< /Title ${pdfString(title)} /Author ${pdfUnicodeString(author)} /CreationDate (D:20261012090000Z) /Producer (Open SEO Spider original fixture) >>`
  ];
  let file='%PDF-1.7\n',offsets=[0];
  for(let i=0;i<objects.length;i++){offsets.push(Buffer.byteLength(file));file+=`${i+1} 0 obj\n${objects[i]}\nendobj\n`}
  const xref=Buffer.byteLength(file);
  file+=`xref\n0 ${objects.length+1}\n0000000000 65535 f \n${offsets.slice(1).map(value=>`${String(value).padStart(10,'0')} 00000 n \n`).join('')}trailer\n<< /Size ${objects.length+1} /Root 1 0 R /Info 7 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(file);
}
const pdf=originalPDF();
const fixture=http.createServer((req,res)=>{
  const origin=`http://127.0.0.1:${fixture.address().port}`;
  if(req.url==='/robots.txt'){res.setHeader('content-type','text/plain');return res.end('User-agent: *\nAllow: /\n')}
  if(req.url==='/sitemap.xml'||req.url==='/favicon.ico'){res.statusCode=404;return res.end()}
  if(req.url==='/pixel.png'){res.setHeader('content-type','image/png');return res.end(png)}
  if(req.url==='/bad.png'){res.setHeader('content-type','image/png');return res.end('invalid owned PNG')}
  if(req.url==='/vector.svg'){res.setHeader('content-type','image/svg+xml');return res.end('<svg xmlns="http://www.w3.org/2000/svg" width="10.5px" height="20px" viewBox="0 0 21 40"><path d="M0 0h1"/></svg>')}
  if(req.url==='/viewbox.svg'){res.setHeader('content-type','image/svg+xml');return res.end('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"/>')}
  if(req.url==='/document.pdf'){res.setHeader('content-type','application/pdf');res.setHeader('Link',`<${origin}/document.pdf>; rel="canonical"`);res.setHeader('X-Robots-Tag','bingbot: noindex');return res.end(pdf)}
  if(req.url==='/bad.pdf'){res.setHeader('content-type','application/pdf');return res.end('%PDF-1.7 invalid owned document')}
  if(req.url==='/redirect-one'){res.statusCode=302;res.setHeader('Location','/redirect-middle');res.setHeader('X-Hop-Only','not retained');return res.end('first redirect body')}
  if(req.url==='/redirect-middle'){res.statusCode=307;res.setHeader('Location','/terminal');return res.end('second redirect body')}
  res.setHeader('content-type','text/html; charset=utf-8');
  if(req.url==='/')return res.end('<!doctype html><html lang="en"><head><title>Owned resource test</title></head><body><h1>Owned fixture</h1><img src="/pixel.png" alt="First occurrence" width="100%" height="auto"><img src="/pixel.png" alt="Second occurrence" width="40" height="30"><img src="/vector.svg" alt="Vector"><img src="/viewbox.svg" alt="Viewbox"><img src="/bad.png"><a href="/document.pdf">Original PDF</a><a href="/bad.pdf">Invalid PDF</a><a href="/redirect-one">Original redirect chain</a></body></html>');
  if(req.url==='/from-pdf')return res.end('<!doctype html><html lang="en"><head><title>PDF annotation target</title></head><body><h1>Found only through PDF URI</h1></body></html>');
  if(req.url==='/terminal')return res.end('<!doctype html><html lang="en"><head><title>Response terminal evidence</title></head><body><h1>Terminal document only</h1></body></html>');
  res.statusCode=404;res.end('unknown local fixture');
});
async function browserPath(){if(process.env.SPIDER_CHROME)return process.env.SPIDER_CHROME;try{return await(await import(pathToFileURL(path.resolve(web,'../renderer/server.mjs')).href)).findBrowserExecutable()}catch(_){return null}}
async function waitForComplete(page,previousId=null){await page.waitForFunction(previous=>document.getElementById('crawl-state').textContent.includes('abgeschlossen')&&(previous==null||document.getElementById('crawl-history').value!==previous),previousId,{timeout:60000});return page.locator('#crawl-history').inputValue()}
async function csvDownload(page,button){const pending=page.waitForEvent('download');await page.click(button);const download=await pending;return fs.readFileSync(await download.path(),'utf8')}
async function tableSearch(page,value,rows){await page.fill('#table-search',value);await page.waitForFunction(expected=>document.querySelectorAll('#results-table tbody tr.click-row').length===expected,rows)}
function csvRows(input){
  const rows=[],cells=[];let value='',quoted=false;
  input=input.replace(/^\uFEFF/,'');
  for(let index=0;index<input.length;index++){
    const ch=input[index];
    if(ch==='"'){
      if(quoted&&input[index+1]==='"'){value+='"';index++}else quoted=!quoted;
    }else if(!quoted&&ch===','){cells.push(value);value=''}
    else if(!quoted&&(ch==='\r'||ch==='\n')){if(ch==='\r'&&input[index+1]==='\n')index++;cells.push(value);rows.push(cells.splice(0));value=''}
    else value+=ch;
  }
  if(value||cells.length){cells.push(value);rows.push(cells.splice(0))}
  assert.equal(quoted,false,'unterminated CSV cell');return rows;
}
(async()=>{
  await new Promise(resolve=>fixture.listen(0,'127.0.0.1',resolve));
  const executable=await browserPath();
  const browser=await playwright.chromium.launch({...(executable?{executablePath:executable}:{}),headless:true});
  const page=await browser.newPage({viewport:{width:1440,height:1000},acceptDownloads:true});
  const errors=[],checks=[];page.on('pageerror',error=>errors.push(error.message));
  const checked=name=>{checks.push(name);console.log(`PASS ${name}`)};
  const origin=`http://127.0.0.1:${fixture.address().port}`;
  try{
    await page.goto(app);await page.locator('#connection-state.connected').waitFor();
    await page.click('#settings-open');await page.uncheck('[name=javascript_rendering]');await page.fill('[name=delay_ms]','0');await page.locator('#advanced-config > summary').click();await page.fill('[name=pdf_analysis_json]','{}');await page.click('#settings-form button[type=submit]');
    await page.fill('#seed-url',`${origin}/`);await page.click('#start-button');
    const id=await waitForComplete(page);
    const response=await fetch(`${app}/api/crawls/${id}/pages?limit=1000`);assert.equal(response.status,200);const data=await response.json();
    const document=data.pages.find(record=>record.url===`${origin}/document.pdf`),pixel=data.pages.find(record=>record.url===`${origin}/pixel.png`),vector=data.pages.find(record=>record.url===`${origin}/vector.svg`),viewbox=data.pages.find(record=>record.url===`${origin}/viewbox.svg`),badImage=data.pages.find(record=>record.url===`${origin}/bad.png`),badPDF=data.pages.find(record=>record.url===`${origin}/bad.pdf`);
    assert.ok(document?.pdf,'new Rust PDF observation');assert.equal(document.pdf.status,'complete');assert.equal(document.pdf.page_count,1);assert.equal(document.pdf.declared_page_count,1);assert.equal(document.pdf.info_metadata.Title,title);assert.equal(document.pdf.info_metadata.Author,author);assert.equal(document.pdf.word_count,4);assert.equal(document.pdf.text_complete,true);assert.match(document.pdf.text_preview,/Owned PDF resource fixture/);assert.deepEqual(document.titles,[]);checked('native PDF parses original metadata verified page count and text without invented HTML title');
    assert.equal(document.pdf.annotation_links.length,1);assert.equal(document.pdf.annotation_links[0].page,1);assert.equal(document.pdf.annotation_links[0].url,`${origin}/from-pdf`);assert.ok(document.links.some(link=>link.kind==='pdf_annotation'&&link.url===`${origin}/from-pdf`));assert.ok(data.pages.some(record=>record.url===`${origin}/from-pdf`));assert.deepEqual(document.header_canonical,[`${origin}/document.pdf`]);assert.equal(document.indexable,true);checked('PDF URI annotation discovers local target and native scoped headers remain separate');
    assert.equal(pixel.image_metadata.status,'complete');assert.equal(pixel.image_metadata.width,1);assert.equal(pixel.image_metadata.height,1);assert.equal(pixel.image_metadata.dimension_unit,'pixel');assert.equal(pixel.image_metadata.encoded_bytes,png.length);assert.equal(vector.image_metadata.width,10.5);assert.equal(vector.image_metadata.height,20);assert.equal(vector.image_metadata.dimension_unit,'css_px');assert.deepEqual(vector.image_metadata.svg_view_box,[0,0,21,40]);checked('real PNG encoded headers and fractional SVG absolute lengths are persisted');
    assert.equal(viewbox.image_metadata.status,'complete');assert.equal(viewbox.image_metadata.width,null);assert.equal(viewbox.image_metadata.height,null);assert.deepEqual(viewbox.image_metadata.svg_view_box,[0,0,200,100]);assert.equal(badImage.image_metadata.status,'error');assert.equal(badImage.image_metadata.width,null);assert.ok(badImage.image_metadata.parse_error);assert.equal(badPDF.pdf.status,'error');assert.equal(badPDF.pdf.page_count,null);assert.ok(badPDF.pdf.parse_error);checked('viewBox is not an invented intrinsic size and invalid resource failures remain explicit');
    await page.click('[data-section=pdf]');assert.equal(await page.locator('#result-count').innerText(),'2 Ergebnisse');await tableSearch(page,'document.pdf',1);await page.locator('#results-table tbody tr.click-row').click();const details=await page.locator('#inspector-content').innerText();assert.match(details,/PDF-Seitenzahl geprüft \/ deklariert/);assert.match(details,/1 \/ 1/);assert.match(details,/Owned PDF resource fixture/);assert.match(details,/PDF-URI-Annotationen/);assert.match(details,/Seite 1/);assert.equal(await page.locator('#inspector-content img').count(),0);assert.equal(await page.evaluate(()=>Boolean(window.__resource_xss)),false);checked('actual PDF table inspector renders metadata as safe text and keeps source page evidence');
    await page.click('[data-section=pdf]');await page.selectOption('#table-filter','error');assert.equal(await page.locator('#result-count').innerText(),'1 Ergebnisse');await page.click('[data-section=imagefiles]');assert.equal(await page.locator('#result-count').innerText(),'4 Ergebnisse');await page.selectOption('#table-filter','dimensions_unknown');assert.equal(await page.locator('#result-count').innerText(),'2 Ergebnisse');await page.selectOption('#table-filter','complete');assert.equal(await page.locator('#result-count').innerText(),'3 Ergebnisse');await tableSearch(page,'vector.svg',1);await page.locator('#results-table tbody tr.click-row').click();assert.match(await page.locator('#inspector-content').innerText(),/10\.5 × 20 css_px/);checked('resource filters sort unknown dimensions separately from parser failure and show fractional SVG measurements');
    await page.click('[data-section=images]');await tableSearch(page,'pixel.png',2);assert.equal(await page.locator('#result-count').innerText(),'2 Ergebnisse');const occurrences=await page.locator('#results-table tbody').innerText();assert.match(occurrences,/First occurrence/);assert.match(occurrences,/Second occurrence/);assert.match(occurrences,/100%/);assert.match(occurrences,/auto/);assert.equal(await page.locator('#results-table th').filter({hasText:'Dateibreite'}).count(),1);checked('same actual image keeps two alt occurrences and HTML attributes distinct from file dimensions');
    await page.click('[data-section=pdf]');const filteredCSV=await csvDownload(page,'#export-csv');assert.match(filteredCSV,/' \t=SUM\(1,2\)/);assert.match(filteredCSV,/Owned PDF <img/);const serverPDF=await csvDownload(page,'#export-resources');assert.match(serverPDF,/Owned PDF/);const pdfCSVRows=csvRows(serverPDF),pdfCSVRecord=pdfCSVRows.slice(1).find(row=>row[0]===document.url),infoJSON=pdfCSVRecord[pdfCSVRows[0].indexOf('Info JSON')];assert.ok(infoJSON.startsWith('{'),'metadata stays inside a JSON object cell, not a formula');assert.equal(JSON.parse(infoJSON).Author,author);assert.equal(JSON.parse(infoJSON).Title,title);await page.click('[data-section=imagefiles]');const serverImages=await csvDownload(page,'#export-resources');assert.match(serverImages,/vector\.svg/);assert.match(serverImages,/10\.5/);checked('filtered UI neutralizes formula text while dedicated Rust CSV retains exact metadata in safe JSON cells');
    const saved=JSON.parse(await(await fetch(`${app}/api/crawls/${id}/project.json`)).text());assert.deepEqual(saved.pages.find(record=>record.url===document.url).pdf,document.pdf);assert.deepEqual(saved.pages.find(record=>record.url===pixel.url).image_metadata,pixel.image_metadata);checked('native resource records survive full project JSON export unchanged');
    const responses=await(await fetch(`${app}/api/crawls/${id}/responses`)).json();const hops=responses.responses.filter(record=>record.requested_url===`${origin}/redirect-one`);assert.deepEqual(hops.map(record=>record.status),[302,307,200]);assert.deepEqual(hops.map(record=>record.response_url),[`${origin}/redirect-one`,`${origin}/redirect-middle`,`${origin}/terminal`]);for(const hop of hops.slice(0,2)){assert.equal(hop.headers,null);assert.equal(hop.body_bytes,null);assert.equal(hop.body_complete,null);assert.equal(hop.document_metadata_available,false)}assert.equal(hops[2].document_metadata_available,true);assert.ok(hops[2].body_bytes>0);assert.ok(!data.pages.some(record=>record.url===`${origin}/redirect-middle`));await page.click('[data-section=responses]');await page.selectOption('#table-filter','redirect_hop');assert.equal(await page.locator('#result-count').innerText(),'2 Ergebnisse');await page.fill('#table-search','redirect-middle');await page.locator('#results-table tbody tr.click-row').last().click();const responseDetails=await page.locator('#inspector-content').innerText();assert.match(responseDetails,/Header nicht gespeichert/);assert.match(responseDetails,/Body nicht gespeichert/);assert.ok(!responseDetails.includes('Response terminal evidence'));await page.locator('[data-response-document]').click();assert.match(await page.locator('#inspector-content').innerText(),/Response terminal evidence/);await page.click('[data-section=responses]');await page.selectOption('#table-filter','headers_unknown');assert.equal(await page.locator('#result-count').innerText(),'2 Ergebnisse');const responsesCSV=await csvDownload(page,'#export-resources');assert.match(responsesCSV,/Response URL/);assert.match(responsesCSV,/redirect-middle/);assert.match(responsesCSV,/307/);checked('actual two-hop redirects appear as native responses with unknown unretained facts and separate terminal document plus CSV');
    // The UI-only unknown and legacy cases are supplied through a real validated import.
    const legacy=JSON.parse(JSON.stringify(saved));for(const record of legacy.pages){delete record.pdf;delete record.image_metadata}
    await page.setInputFiles('#import-file',{name:'legacy-resources.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(legacy))});await page.waitForFunction(()=>document.getElementById('message').textContent.includes('importiert'));await page.click('[data-section=pdf]');await page.selectOption('#table-filter','unavailable');assert.equal(await page.locator('#result-count').innerText(),'2 Ergebnisse');await page.click('[data-section=imagefiles]');await page.selectOption('#table-filter','unavailable');assert.equal(await page.locator('#result-count').innerText(),'4 Ergebnisse');assert.match(await page.locator('#results-table tbody').innerText(),/Nicht erfasst/);checked('legacy import missing new observations remains unknown in actual Rust and UI');
    await page.click('#settings-open');await page.fill('[name=pdf_analysis_json]','{"max_pages":0}');await page.click('#settings-form button[type=submit]');assert.match(await page.locator('#settings-error').innerText(),/PDF-Grenze/);await page.fill('[name=pdf_analysis_json]','{"enabled":false}');await page.click('#settings-form button[type=submit]');await page.selectOption('#crawl-mode','list');await page.fill('#url-list',`${origin}/document.pdf`);const previousId=await page.locator('#crawl-history').inputValue();await page.click('#start-button');const disabledId=await waitForComplete(page,previousId);const disabled=await(await fetch(`${app}/api/crawls/${disabledId}/pages`)).json();assert.equal(disabled.pages[0].pdf.status,'disabled');assert.equal(disabled.pages[0].pdf.page_count,null);await page.click('[data-section=pdf]');await page.selectOption('#table-filter','disabled');assert.equal(await page.locator('#result-count').innerText(),'1 Ergebnisse');checked('PDF configuration validates before start and an actual disabled analysis is not an empty success');
    for(const [width,height]of [[1440,900],[390,844],[320,740]]){await page.setViewportSize({width,height});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.locator('.main-footer').scrollIntoViewIfNeeded();const box=await page.locator('.main-footer').boundingBox();assert.ok(box.y<height&&box.y+box.height>0)}
    await page.click('#settings-open');await page.locator('#settings-dialog .dialog-footer button').scrollIntoViewIfNeeded();const button=await page.locator('#settings-dialog .dialog-footer button').boundingBox();assert.ok(button.y>=0&&button.y+button.height<=740);await page.click('#settings-close');checked('resource views and expanded settings fit narrow and wide browser widths with bottom reachable');
    assert.deepEqual(errors,[]);
    const proof={tested_at:new Date().toISOString(),scope:'Actual browser -> Rust native resource parsing -> original local PDF/PNG/SVG -> UI/filter/export/legacy/disabled configuration',app_url:app,crawl_id:id,fixture_origin:origin,observed_urls:data.total,checks,javascript_runtime_errors:errors};fs.writeFileSync(artifactPath('ui-resource-live-results.json'),JSON.stringify(proof,null,2));console.log(`${checks.length} actual resource checks passed.`);
  }finally{await browser.close();await new Promise(resolve=>{fixture.close(resolve);fixture.closeAllConnections()})}
})().catch(error=>{console.error(error);fixture.close();fixture.closeAllConnections();process.exitCode=1});
