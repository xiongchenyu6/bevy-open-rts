// Deterministic UI ownership regression. Worker integrity is covered separately.
const {chromium}=require('playwright-core'),fs=require('node:fs/promises'),http=require('node:http'),assert=require('node:assert/strict');
(async()=>{
 const script=await fs.readFile('web/downloads.js');
 const server=http.createServer((req,res)=>{res.setHeader('Content-Type',req.url==='/downloads.js'?'text/javascript':'text/html');res.end(req.url==='/downloads.js'?script:'<!doctype html><body></body>')});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));let browser;
 try{
  browser=await chromium.launch({executablePath:process.env.CHROMIUM_EXECUTABLE_PATH,headless:true});const page=await browser.newPage();await page.goto('http://127.0.0.1:'+server.address().port);
  await page.evaluate(async()=>{const {createDownloadUI}=await import('/downloads.js');window.preparations=[];createDownloadUI({worker:async()=>({}),language:()=> 'en',request:async(_,type)=>{if(type==='PREPARE')return new Promise(resolve=>window.preparations.push(resolve));return {ready:false,downloadBytes:8,groups:[]};}})});
  await page.evaluate(()=>window.rtsPrepareMatch(0));await page.locator('#downloads-confirm').click();await page.waitForFunction(()=>window.preparations.length===1);
  await page.locator('#downloads-close').click();await page.evaluate(()=>window.rtsPrepareMatch(0));await page.locator('#downloads-confirm').click();await page.waitForFunction(()=>window.preparations.length===2);
  await page.evaluate(()=>window.preparations[0]({ready:true,downloadBytes:0}));
  assert.equal(await page.locator('#downloads-confirm').isDisabled(),true,'Old completion must not unlock a newer transfer');
  assert.equal(await page.evaluate(()=>window.rtsMatchDownload.status),'downloading');
  await page.evaluate(()=>window.preparations[1]({ready:true,downloadBytes:0}));await page.waitForFunction(()=>window.rtsMatchDownload.status==='ready');
  console.log('PASS immediate retry opens while canceled preparation is pending; old completion cannot unlock or finish its successor');
 }finally{await browser?.close();server.close();}
})().catch(e=>{console.error(e);process.exitCode=1});
