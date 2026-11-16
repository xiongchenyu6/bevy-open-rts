const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const http=require('node:http');
const {chromium}=require('playwright-core');
(async()=>{
 const server=http.createServer(async(req,res)=>{res.setHeader('Content-Type',req.url.endsWith('.js')?'text/javascript':'text/html');res.end(req.url.endsWith('.js')?await fs.readFile('web/touch.js'): `<!doctype html><style>body{margin:0}canvas{width:844px;height:390px;touch-action:none}</style><canvas data-app-screen="InMatch"></canvas><script type="module">import {installTouchControls} from './touch.js';const c=document.querySelector('canvas');installTouchControls(c);window.events=[];window.wheels=[];for(const type of ['pointerdown','pointerup'])c.addEventListener(type,e=>events.push({type,button:e.button,pointer:e.pointerType}));c.addEventListener('wheel',e=>wheels.push(e.deltaY));window.rtsTouchWorldEnabled=true;window.ready=true;</script>`);});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_EXECUTABLE_PATH});
 try{
 const context=await browser.newContext({viewport:{width:844,height:390},hasTouch:true});const page=await context.newPage();await page.goto(`http://127.0.0.1:${server.address().port}`);await page.waitForFunction(()=>window.ready);const cdp=await context.newCDPSession(page);
 const touch=(type,points=[])=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:points.map((p,i)=>({x:p[0],y:p[1],id:i}))});
 const clear=()=>page.evaluate(()=>{events.length=0;window.rtsTouchCamera={x:0,y:0,zoom:0};});
 const result=()=>page.evaluate(()=>({events,camera:rtsTouchCamera}));
 await touch('touchStart',[[300,200]]);await touch('touchEnd');await page.waitForFunction(()=>events.length===2);assert.deepEqual((await result()).events.map(e=>e.button),[0,0]);
 await clear();await touch('touchStart',[[300,200]]);await page.waitForTimeout(550);await touch('touchEnd');await page.waitForFunction(()=>events.length===2);assert.deepEqual((await result()).events.map(e=>e.button),[2,2]);
 await clear();await touch('touchStart',[[300,200]]);await touch('touchMove',[[380,240]]);await touch('touchEnd');assert.equal((await result()).events.length,0);assert.ok((await result()).camera.x>0);
 await clear();await touch('touchStart',[[300,200]]);await touch('touchCancel');assert.equal((await result()).events.length,0);
 await clear();await page.evaluate(()=>window.rtsTouchHudRects=[[250,100,450,300]]);await touch('touchStart',[[300,200]]);await touch('touchMove',[[380,240]]);await touch('touchEnd');assert.deepEqual(await result(),{events:[],camera:{x:0,y:0,zoom:0}});
 await clear();await page.evaluate(()=>document.querySelector('canvas').style.marginLeft='40px');await touch('touchStart',[[480,200]]);await touch('touchMove',[[530,240]]);await touch('touchEnd');assert.deepEqual(await result(),{events:[],camera:{x:0,y:0,zoom:0}});await page.evaluate(()=>document.querySelector('canvas').style.marginLeft='0');
 await clear();await page.evaluate(()=>window.rtsTouchHudRects=[]);await touch('touchStart',[[300,180],[450,180]]);await touch('touchMove',[[280,180],[470,180]]);await touch('touchMove',[[260,180],[490,180]]);await touch('touchEnd');assert.equal((await result()).events.length,0);assert.ok((await result()).camera.zoom<0);
 await page.evaluate(()=>document.querySelector('canvas').dataset.appScreen='OptionsMenu');await clear();await touch('touchStart',[[300,280]]);await touch('touchMove',[[300,160]]);await touch('touchEnd');assert.equal((await result()).events.length,0);assert.ok(await page.evaluate(()=>wheels.some(v=>v>0)));
 console.log('PASS 8 real CDP touch scenarios: tap, hold order, pan, cancellation, HUD isolation, safe-area coordinates, pinch, menu scroll');
 }finally{await browser.close();server.close();}
})().catch(e=>{console.error(e);process.exitCode=1});
