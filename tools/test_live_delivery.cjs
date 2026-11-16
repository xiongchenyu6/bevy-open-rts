// Real Bevy engine, persistent Chromium profile and offline process restart.
// Run against a built package; this is separate from the small worker fixtures.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const {chromium} = require('playwright-core');
const {rpc} = require('./test_pwa_cache.cjs');
const url = process.env.OPEN_BEVY_GAME_URL || 'http://127.0.0.1:8783/';
const output = path.resolve(process.env.OPEN_BEVY_BROWSER_OUTPUT || 'target/web-live');
const args = ['--enable-unsafe-webgpu','--enable-unsafe-swiftshader','--ignore-gpu-blocklist','--enable-features=Vulkan,UseSkiaRenderer,WebGPU','--use-angle=swiftshader','--ozone-platform=x11','--use-vulkan=swiftshader','--disable-vulkan-surface','--disable-gpu-sandbox','--disable-background-timer-throttling','--disable-renderer-backgrounding','--disable-backgrounding-occluded-windows'];
const report = {url, passed:false, phases:[], errors:[]};
let context;
async function state(page, expected) {
  await page.waitForFunction(s=>document.querySelector('canvas')?.dataset.appScreen===s,expected,{timeout:180000});
}
async function screenshot(page, name) {
  const {captureVisibleFrame} = await import('../scripts/browser-smoke/visible_frame.mjs');
  return captureVisibleFrame(page,path.join(output,name+'.png'),120000);
}
async function launch(profile, offline) {
  context=await chromium.launchPersistentContext(profile,{executablePath:process.env.CHROME_BIN,headless:false,args,viewport:{width:844,height:390},hasTouch:true,deviceScaleFactor:1});
  if(offline) await context.setOffline(true);
  const page=context.pages()[0];
  page.on('pageerror',e=>report.errors.push(String(e)));
  page.on('console',m=>{if(m.type()==='error')report.errors.push(m.text());});
  await page.goto(url,{waitUntil:'domcontentloaded',timeout:60000});
  await page.locator('#start').tap();
  await page.waitForFunction(()=>document.querySelector('canvas')?.dataset.gameAssetsReady==='true',undefined,{timeout:180000});
  await state(page,'MainMenu');
  await page.waitForTimeout(1200);
  const dimensions=await page.locator('canvas').evaluate(c=>({w:c.width,h:c.height,cssW:c.getBoundingClientRect().width,cssH:c.getBoundingClientRect().height,dpr:devicePixelRatio,ready:c.dataset.gameAssetsReady}));
  assert.ok(Math.abs(dimensions.w-dimensions.cssW*dimensions.dpr)<2,JSON.stringify(dimensions));
  assert.ok(Math.abs(dimensions.h-dimensions.cssH*dimensions.dpr)<2,JSON.stringify(dimensions));
  assert.equal(dimensions.ready,'true');
  report.phases.push({name:offline?'cold-offline-menu':'mobile-menu',dimensions,pixels:await screenshot(page,offline?'cold-offline-menu':'mobile-menu')});
  return page;
}
async function setup(page) {
  await page.touchscreen.tap(570,144);
  await state(page,'SkirmishSetup');
  await page.waitForTimeout(700);
}
(async()=>{
  await fs.mkdir(output,{recursive:true});
  const profile=await fs.mkdtemp(path.join(os.tmpdir(),'rts-live-offline-'));
  try {
    let page=await launch(profile,false);
    const before = await rpc(page,'controller','STATUS',{groups:['combat']});
    assert.ok(before.downloadBytes>0,JSON.stringify(before));
    report.combatBefore=before;
    await setup(page);
    report.phases.push({name:'mobile-setup',pixels:await screenshot(page,'mobile-setup')});
    await page.touchscreen.tap(420,306);
    await page.waitForSelector('#downloads-dialog[open]',{timeout:30000});
    await page.waitForFunction(()=>!document.querySelector('#downloads-confirm').disabled);
    report.phases.push({name:'battle-download',text:await page.locator('#downloads-dialog').innerText(),pixels:await screenshot(page,'mobile-download')});
    await page.locator('#downloads-close').tap();
    assert.equal(await page.locator('canvas').getAttribute('data-app-screen'),'SkirmishSetup');
    await page.touchscreen.tap(420,306);
    await page.locator('#downloads-confirm').tap();
    await state(page,'InMatch');
    report.phases.push({name:'mobile-match',pixels:await screenshot(page,'mobile-match')});
    report.combatAfter=await rpc(page,'controller','STATUS',{groups:['combat']});
    assert.equal(report.combatAfter.downloadBytes,0);
    await context.close(); context=null;
    page=await launch(profile,true);
    await setup(page);
    await page.touchscreen.tap(420,306);
    await state(page,'InMatch');
    assert.equal(await page.locator('#downloads-dialog').evaluate(d=>d.open),false);
    report.phases.push({name:'cold-offline-match',pixels:await screenshot(page,'cold-offline-match')});
    assert.deepEqual(report.errors,[]);
    report.passed=true;
  } finally {
    if(context)await context.close();
    await fs.writeFile(path.join(output,'live-delivery.json'),JSON.stringify(report,null,2));
    console.log(JSON.stringify(report,null,2));
  }
})().catch(e=>{console.error(e);process.exitCode=1;});
