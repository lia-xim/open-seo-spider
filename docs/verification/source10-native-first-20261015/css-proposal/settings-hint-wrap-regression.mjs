// AUTHORED NOT_RUN. Invoke only in an owned browser after the genuine invalid settings submit.
import assert from 'node:assert/strict';
export async function verifySettingsHintWrapping({page,sourceRequests}) {
  assert.equal(typeof sourceRequests,'function');
  const before=sourceRequests();const layouts=[];
  for(const width of [1440,320]) {
    await page.setViewportSize({width,height:1000});
    await page.evaluate(()=>document.fonts.ready);
    const row=await page.evaluate(()=>{
      const dialog=document.querySelector('#settings-dialog'),form=document.querySelector('#settings-form'),error=document.querySelector('#settings-error');
      const visible=e=>!!e&&e.getClientRects().length>0;
      const measure=e=>({client:e.clientWidth,scroll:e.scrollWidth,bounds:e.getBoundingClientRect().toJSON(),overflowWrap:getComputedStyle(e).overflowWrap,whiteSpace:getComputedStyle(e).whiteSpace,overflowX:getComputedStyle(e).overflowX});
      const hints=[...dialog.querySelectorAll('.settings-grid small')].filter(visible);
      const target=hints.filter(e=>e.textContent.includes('"max_title_chars"')&&e.textContent.includes('"max_title_pixels"'));
      return {width:innerWidth,document:document.documentElement.scrollWidth,dialogOpen:dialog.open,errorVisible:visible(error),errorText:error.textContent,dialog:measure(dialog),form:measure(form),hints:hints.map(measure),target:target.map(e=>({...measure(e),text:e.textContent}))};
    });
    assert.equal(row.width,width);assert.equal(row.dialogOpen,true);assert.equal(row.errorVisible,true);
    assert.match(row.errorText,/mindestens eine Darstellung/);assert.equal(row.target.length,1);
    assert(row.document<=width+1);
    for(const item of [row.dialog,row.form,...row.hints]) {assert(item.client>0);assert(item.scroll<=item.client+1);assert(item.bounds.left>=-1&&item.bounds.right<=width+1);}
    assert.equal(row.target[0].overflowWrap,'anywhere');assert.equal(row.target[0].whiteSpace,'normal');
    assert(!['hidden','clip'].includes(row.target[0].overflowX));layouts.push(row);
  }
  assert.deepEqual(sourceRequests(),before,'Responsive settings reads must make no website requests');
  return {status:'PASS_ACTUAL_SETTINGS_HINT_WRAP_1440_320',layouts,new_website_requests:0};
}
