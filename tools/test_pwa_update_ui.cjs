#!/usr/bin/env node
// Real production startup UI and SW, with only the heavyweight Bevy engine stubbed.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const http = require('node:http');
const {spawnSync} = require('node:child_process');
const {gzipSync} = require('node:zlib');
const {chromium} = require('playwright-core');
const ROOT = path.resolve(__dirname, '..');
const OUTPUT = path.resolve(process.env.PWA_TEST_OUTPUT || 'target/pwa-update-ui-tests');
const results = [];
const pass = text => { results.push(text); console.log(`PASS ${text}`); };

async function main() {
  await fs.mkdir(OUTPUT, {recursive: true});
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'open-rts-pwa-ui-'));
  const fixtures = {};
  for (const revision of ['v1', 'v2', 'v3']) {
    const directory = path.join(temporary, revision);
    await fs.mkdir(directory, {recursive:true});
    for (const name of ['index.html','styles.css','game-cache.js','game-cache.css','downloads.js','pwa.js','pwa.css','touch.js','manifest.webmanifest','icons']) await fs.cp(path.join(ROOT,'web',name),path.join(directory,name),{recursive:true});
    await fs.mkdir(path.join(directory,'pkg'),{recursive:true});
    await fs.mkdir(path.join(directory, 'assets/fonts'), {recursive: true});
    await fs.copyFile(path.join(ROOT, 'assets/fonts/wqy-microhei-ui.ttf'), path.join(directory, 'assets/fonts/wqy-microhei-ui.ttf'));
    await fs.writeFile(path.join(directory, 'changed.json'), revision);
    await fs.mkdir(path.join(directory, 'assets/models'), {recursive: true});
    await fs.writeFile(path.join(directory, 'assets/models/test.glb'), 'shared-combat-fixture');
    await fs.mkdir(path.join(directory, 'THIRD_PARTY_NOTICES'), {recursive: true});
    await fs.writeFile(path.join(directory, 'THIRD_PARTY_NOTICES/test.txt'), 'optional-notice');
    await fs.writeFile(path.join(directory, 'pkg/bevy_open_rts.js'), `export default async function init(options) { if (!options?.module_or_path) throw Error('Missing prepared WASM'); window.fixtureEngineVersion = '${revision}'; const c=document.querySelector('canvas'); c.dataset.assetsTotal='1';c.dataset.assetsDone='1';c.dataset.gameAssetsReady='true'; }`);
    const wasm = Buffer.from([0,97,115,109,1,0,0,0]);
    await fs.writeFile(path.join(directory, 'pkg/bevy_open_rts_bg.wasm'), wasm);
    await fs.writeFile(path.join(directory, 'pkg/bevy_open_rts_bg.wasm.gz'), gzipSync(wasm));
    const generated = spawnSync('python3', ['tools/generate_web_manifest.py', directory, '--revision', revision, '--template', 'web/sw.js'], {cwd: ROOT, encoding: 'utf8'});
    assert.equal(generated.status, 0, generated.stderr);
    fixtures[revision] = {directory, manifest: JSON.parse(await fs.readFile(path.join(directory, 'asset-manifest.json'), 'utf8'))};
  }
  let current = 'v1', failResource = false, legacyMode = false;
  let holdCombat = false, combatHeld = false, releaseCombat;
  let combatGate;
  let releaseDownload;
  const downloadGate = new Promise(resolve => { releaseDownload = resolve; });
  const requests = [];
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, 'http://fixture');
    const transport = url.pathname === '/game/' ? 'index.html' : decodeURIComponent(url.pathname.slice('/game/'.length));
    const immutable = Object.values(fixtures).flatMap(f => f.manifest.files.map(file => ({file, directory: f.directory}))).find(item => item.file.url === transport);
    const relative = immutable?.file.path || transport;
    requests.push({revision: current, path: relative});
    response.setHeader('Cache-Control', 'no-store');
    response.setHeader('Service-Worker-Allowed', '/game/');
    if (!url.pathname.startsWith('/game/') || relative.includes('..')) { response.writeHead(404).end(); return; }
    if (failResource && relative === 'changed.json') { response.writeHead(503).end(); return; }
    if (legacyMode && relative === 'sw.js') {
      response.setHeader('Content-Type', 'text/javascript');
      response.end("self.addEventListener('install',()=>self.skipWaiting()); self.addEventListener('activate',event=>event.waitUntil(self.clients.claim())); self.addEventListener('fetch',event=>{if(event.request.mode==='navigate')event.respondWith(fetch(event.request));});"); return;
    }
    if (legacyMode && relative === 'index.html') {
      response.setHeader('Content-Type', 'text/html');
      response.end('<!doctype html><h1>Legacy network-only game</h1>'); return;
    }
    if (relative === 'changed.json' && current === 'v1') await downloadGate;
    if (relative === 'assets/models/test.glb' && holdCombat) { combatHeld = true; await combatGate; }
    try {
      const body = await fs.readFile(path.join(immutable?.directory || fixtures[current].directory, relative));
      const type = relative.endsWith('.js') ? 'text/javascript' : relative.endsWith('.html') ? 'text/html' : relative.endsWith('.css') ? 'text/css' : relative.endsWith('.png') ? 'image/png' : relative.endsWith('.ttf') ? 'font/ttf' : relative.endsWith('.webmanifest') ? 'application/manifest+json' : 'application/octet-stream';
      response.setHeader('Content-Type', type);
      response.writeHead(200).end(body);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}/game/?lang=en`;
  const launch = (profile = 'profile') => chromium.launchPersistentContext(path.join(temporary, profile), {
    headless: true, executablePath: process.env.CHROMIUM_EXECUTABLE_PATH || undefined,
    viewport: {width: 1280, height: 720}, serviceWorkers: 'allow',
  }).then(async context => {
    // This suite stubs the engine; GPU capability belongs to the live-render gate.
    await context.addInitScript(() => Object.defineProperty(navigator, 'gpu', {value: {requestAdapter: async () => ({})}}));
    return context;
  });
  let context;
  const errors = [];
  const observe = page => page.on('pageerror', error => errors.push(String(error)));
  try {
    context = await launch();
    let page = await context.newPage(); observe(page);
    await page.goto(base);
    await page.locator('#cache-status').filter({hasText: 'Save the launcher first'}).waitFor();
    await page.locator('[data-language="zh-CN"]').click();
    assert.equal(await page.locator('#start').innerText(), '开始游戏');
    await page.locator('[data-language="en"]').click();
    await page.locator('#start').click();
    await page.locator('[data-language="zh-CN"]').click();
    assert.equal(await page.locator('[data-language="zh-CN"]').getAttribute('aria-pressed'), 'true');
    assert.match(await page.locator('#status').innerText(), /[\u4e00-\u9fff]/);
    assert.notEqual(await page.locator('#start').innerText(), '开始游戏');
    await page.screenshot({path: path.join(OUTPUT, 'language-during-loading.png')});
    await page.locator('[data-language="en"]').click();
    assert.equal(await page.evaluate(() => localStorage.getItem('open-rts-language')), 'en');
    await page.setViewportSize({width: 844, height: 390});
    await page.locator('[data-language="zh-CN"]').click();
    assert.match(await page.locator('#cache-status').innerText(), /正在/);
    await page.screenshot({path: path.join(OUTPUT, 'language-during-loading-mobile.png')});
    await page.locator('[data-language="en"]').click();
    await page.setViewportSize({width: 1280, height: 720});
    releaseDownload();
    pass('language switches before Start and during resource preparation without resetting loading state');
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v1');
    assert.equal(requests.filter(r => r.path === 'pkg/bevy_open_rts_bg.wasm.gz').length, 1);
    assert.equal(requests.filter(r => r.path === 'pkg/bevy_open_rts_bg.wasm').length, 0);
    pass('actual Start button prepares resources before engine initialization');

    requests.length = 0;
    await page.reload();
    await page.locator('#cache-status').filter({hasText: 'Launcher saved'}).waitFor();
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v1');
    assert.equal(requests.filter(r => r.path !== 'sw.js').length, 0, JSON.stringify(requests));
    pass('actual second Start initializes from saved resources with zero asset downloads');

    requests.length = 0;
    await page.evaluate(() => window.rtsPrepareMatch(0));
    await page.locator('#downloads-confirm').waitFor({state: 'visible'});
    await page.waitForFunction(() => !document.querySelector('#downloads-confirm').disabled);
    assert.equal(await page.evaluate(() => window.rtsMatchDownload.status), 'downloading');
    assert.equal(requests.filter(r => r.path.startsWith('assets/models/')).length, 0);
    await page.locator('#downloads-close').click();
    assert.equal(await page.evaluate(() => window.rtsMatchDownload.status), 'cancelled');
    pass('chapter preview reports required download without fetching combat and Back cancels before starting');

    await page.evaluate(() => window.rtsPrepareMatch(0));
    await page.waitForFunction(() => !document.querySelector('#downloads-confirm').disabled);
    holdCombat = true;
    combatGate = new Promise(resolve => { releaseCombat = resolve; });
    await page.locator('#downloads-confirm').click();
    for (let attempt = 0; !combatHeld && attempt < 100; attempt++) await new Promise(resolve => setTimeout(resolve, 50));
    assert.ok(combatHeld);
    await page.locator('#downloads-close').click();
    await page.waitForFunction(() => window.rtsMatchDownload.status === 'cancelled');
    assert.equal(await page.locator('#downloads-dialog').evaluate(node => node.open), false);
    holdCombat = false; releaseCombat();
    pass('Back during a blocked chapter transfer cancels the download without entering a run');

    await page.evaluate(() => window.rtsPrepareMatch(0));
    // The last verified file may finish as cancellation arrives. Retry must either
    // offer remaining bytes or immediately prepare the already complete pack.
    await page.waitForFunction(() => window.rtsMatchDownload.status === 'ready' || (document.querySelector('#downloads-dialog').open && !document.querySelector('#downloads-confirm').disabled));
    await page.setViewportSize({width: 844, height: 390});
    await page.evaluate(() => window.rtsSetLanguage('zh-CN'));
    assert.equal(await page.locator('#downloads-title').innerText(), '准备战场');
    const bounds = await page.locator('#downloads-dialog').boundingBox();
    assert.ok(bounds && bounds.x >= 0 && bounds.y >= 0 && bounds.x + bounds.width <= 845 && bounds.y + bounds.height <= 391, JSON.stringify(bounds));
    await page.screenshot({path: path.join(OUTPUT, 'chapter-download-mobile.png')});
    await page.evaluate(() => window.rtsSetLanguage('en'));
    if (await page.evaluate(() => window.rtsMatchDownload.status !== 'ready')) await page.locator('#downloads-confirm').click();
    await page.waitForFunction(() => window.rtsMatchDownload.status === 'ready');
    assert.equal(await page.locator('#downloads-dialog').evaluate(node => node.open), true);
    await page.evaluate(() => window.rtsAssetsDecoded());
    assert.equal(await page.locator('#downloads-dialog').evaluate(node => node.open), false);
    pass('chapter confirmation saves combat before signaling ready; mobile localized modal fits viewport');

    requests.length = 0;
    await page.evaluate(() => window.rtsPrepareMatch(1));
    await page.waitForFunction(() => window.rtsMatchDownload.stage === 1 && window.rtsMatchDownload.status === 'ready');
    assert.equal(requests.filter(r => r.path !== 'sw.js').length, 0);
    pass('another chapter starts from the saved shared pack without another download prompt');

    // Delay only the release transport to reproduce an in-flight return-to-menu
    // request crossing an immediate new chapter entry. The worker/RPC stay real.
    await page.evaluate(() => {
      const original = ServiceWorker.prototype.postMessage;
      ServiceWorker.prototype.postMessage = function(data, ...args) {
        if (data.type === 'PIN_GROUPS' && data.groups?.length === 0) {
          setTimeout(() => original.call(this, data, ...args), 150);
        } else original.call(this, data, ...args);
      };
      window.restoreFixturePostMessage = () => { ServiceWorker.prototype.postMessage = original; };
      window.rtsReleaseMatch();
      window.rtsPrepareMatch(2);
    });
    await page.waitForFunction(() => window.rtsMatchDownload.stage === 2 && window.rtsMatchDownload.status === 'ready');
    await page.evaluate(() => window.restoreFixturePostMessage());
    await page.evaluate(() => window.rtsOpenDownloads());
    await page.waitForFunction(() => !document.querySelector('#downloads-remove').disabled);
    await page.locator('#downloads-remove').click();
    await page.locator('#downloads-summary').filter({hasText: 'Resources in an open match cannot be removed'}).waitFor();
    await page.locator('#downloads-close').click();
    pass('delayed unawaited chapter release cannot clear the next chapter resource protection');


    await page.evaluate(() => { localStorage.setItem('fixture-progress', 'preserve-me'); window.rtsOpenDownloads(); });
    await page.waitForFunction(() => !document.querySelector('#downloads-confirm').disabled);
    await page.locator('#downloads-confirm').click();
    await page.waitForFunction(() => document.querySelector('#downloads-confirm').hidden);
    assert.equal(requests.filter(r => r.path === 'THIRD_PARTY_NOTICES/test.txt').length, 1);
    await page.screenshot({path: path.join(OUTPUT, 'offline-manager-mobile.png')});
    await page.locator('#downloads-remove').click();
    await page.locator('#downloads-summary').filter({hasText: 'Resources in an open match cannot be removed'}).waitFor();
    await page.evaluate(() => window.rtsReleaseMatch());
    await page.locator('#downloads-remove').click();
    await page.locator('#downloads-summary').filter({hasText: 'Extra downloads removed'}).waitFor();
    assert.equal(await page.evaluate(() => localStorage.getItem('fixture-progress')), 'preserve-me');
    const storage = await page.evaluate(async () => {
      const worker = navigator.serviceWorker.controller;
      return new Promise(resolve => { const channel = new MessageChannel(); channel.port1.onmessage = event => resolve(event.data); worker.postMessage({type: 'STATUS', target: 'client', wasmEncoding: 'gzip', groups: ['all']}, [channel.port2]); });
    });
    assert.ok(storage.groups.find(group => group.id === 'core').ready);
    assert.equal(storage.groups.find(group => group.id === 'combat').ready, false);
    assert.equal(storage.groups.find(group => group.id === 'optional').ready, false);
    await page.locator('#downloads-close').click();
    await page.setViewportSize({width: 1280, height: 720});
    pass('offline manager downloads extras, protects live battle assets and removes extras without clearing launcher or progress');

    await page.evaluate(() => {
      const original = ServiceWorker.prototype.postMessage;
      window.fixtureCancelReleaseCount = 0;
      ServiceWorker.prototype.postMessage = function(data, ...args) {
        if (data.type === 'PIN_GROUPS' && data.groups?.length === 0) window.fixtureCancelReleaseCount++;
        if (data.type === 'PIN_GROUPS' && data.groups?.includes('combat')) {
          window.releaseFixtureCombatPin = () => new Promise(resolve => setTimeout(() => {
            original.call(this, data, ...args); resolve();
          }, 150));
        } else original.call(this, data, ...args);
      };
      window.restoreFixturePostMessage = () => { ServiceWorker.prototype.postMessage = original; };
      window.rtsPrepareMatch(3);
    });
    await page.waitForFunction(() => document.querySelector('#downloads-dialog').open && !document.querySelector('#downloads-confirm').disabled);
    await page.locator('#downloads-confirm').click();
    await page.waitForFunction(() => typeof window.releaseFixtureCombatPin === 'function');
    await page.locator('#downloads-close').click();
    assert.equal(await page.evaluate(() => window.rtsMatchDownload.status), 'cancelled');
    await page.evaluate(async () => {
      await window.releaseFixtureCombatPin();
      await window.rtsReleaseMatch(); // Drain pin + cancellation release queue.
      window.restoreFixturePostMessage();
    });
    assert.equal(await page.evaluate(() => window.fixtureCancelReleaseCount), 2, 'Back itself must enqueue a release before the explicit drain release');
    assert.equal(await page.evaluate(() => window.rtsMatchDownload.status), 'cancelled');
    assert.equal(await page.locator('#downloads-dialog').evaluate(node => node.open), false);
    await page.evaluate(() => window.rtsOpenDownloads());
    await page.waitForFunction(() => !document.querySelector('#downloads-remove').disabled);
    await page.locator('#downloads-remove').click();
    await page.locator('#downloads-summary').filter({hasText: 'Extra downloads removed'}).waitFor();
    await page.locator('#downloads-close').click();
    pass('Back while final combat pin is in flight never signals ready afterward and releases the canceled expedition lease');



    current = 'v2';
    await page.reload();
    await page.evaluate(async () => (await navigator.serviceWorker.getRegistration()).update());
    await page.locator('#cache-update').waitFor({state: 'visible'});
    for (const [width, height] of [[1280,720], [844,390], [393,852]]) {
      await page.setViewportSize({width, height});
      for (const selector of ['#start', '#cache-update', '#cache-later']) {
        const box = await page.locator(selector).boundingBox();
        assert.ok(box && box.x >= 0 && box.y >= 0 && box.x + box.width <= width + 1 && box.y + box.height <= height + 1, `${selector} clipped at ${width}x${height}: ${JSON.stringify(box)}`);
      }
      await page.screenshot({path: path.join(OUTPUT, `update-offer-${width}x${height}.png`)});
    }
    pass('update offer and both actions fit desktop, landscape phone and portrait viewports');
    await page.locator('#cache-later').click();
    await page.locator('#cache-update').waitFor({state: 'hidden'});
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v1');
    pass('Later starts the old engine instead of silently upgrading');

    await context.close();
    context = await launch();
    page = await context.newPage(); observe(page);
    await page.goto(base);
    await page.locator('#cache-update').waitFor({state: 'visible'});
    assert.match(await page.locator('#cache-status').textContent(), /new version/);
    pass('update offer survives a full browser restart and natural worker activation');
    await page.locator('#cache-update').click();
    await page.waitForFunction(() => document.querySelector('#cache-status').textContent.includes('Launcher saved'));
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v2');
    pass('Download & update commits, reloads and starts the new engine');

    current = 'v3'; failResource = true;
    await page.reload();
    await page.evaluate(async () => (await navigator.serviceWorker.getRegistration()).update());
    await page.locator('#cache-update').waitFor({state: 'visible'});
    await page.locator('#cache-update').click();
    await page.locator('#cache-status').filter({hasText: 'Update failed'}).waitFor();
    assert.equal(await page.locator('#start').isEnabled(), true);
    assert.match(await page.locator('#cache-update').textContent(), /Retry update/);
    await page.screenshot({path: path.join(OUTPUT, 'failed-update-old-version-available.png')});
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v2');
    pass('failed update shows retry while keeping Start available for previous engine');

    await context.close();
    current = 'v1'; failResource = false; legacyMode = true;
    context = await launch('legacy-profile');
    page = await context.newPage(); observe(page);
    await page.goto(base);
    await page.evaluate(async () => {
      await navigator.serviceWorker.register('./sw.js', {scope: './', updateViaCache: 'none'});
      await navigator.serviceWorker.ready;
    });
    await page.waitForFunction(() => Boolean(navigator.serviceWorker.controller));
    const legacyTab = await context.newPage(); observe(legacyTab);
    await legacyTab.goto(base);
    await legacyTab.waitForFunction(() => Boolean(navigator.serviceWorker.controller));
    legacyMode = false;
    await page.reload();
    await page.locator('#start').waitFor();
    await page.waitForFunction(async () => Boolean((await navigator.serviceWorker.getRegistration()).waiting));
    let migrations = 0;
    page.on('request', request => { if (request.isNavigationRequest() && request.frame() === page.mainFrame()) migrations++; });
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'), null, {timeout: 45000});
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v1');
    assert.equal(migrations, 1, `Unexpected migration reload count: ${migrations}`);
    assert.equal(await page.evaluate(() => sessionStorage.getItem('open-rts-cache-resume-start')), null);
    pass('legacy network-only worker migrates through waiting worker and resumes Start after one reload');
    await context.close();
    context = await launch('activation-race-profile');
    page = await context.newPage(); observe(page);
    await page.addInitScript(() => {
      const register = navigator.serviceWorker.register.bind(navigator.serviceWorker);
      navigator.serviceWorker.register = async (...args) => {
        const actual = await register(...args);
        let activeRead = false, waitingRead = false;
        return new Proxy(actual, {get(target, property) {
          if (window.forceActivationRace && new Error().stack.includes('installedWorker') && property === 'active' && !activeRead) {
            activeRead = true; return null;
          }
          if (window.forceActivationRace && new Error().stack.includes('installedWorker') && property === 'waiting') {
            if (!waitingRead) { waitingRead = true; return target.active; }
            return null;
          }
          const value = Reflect.get(target, property, target);
          return typeof value === 'function' ? value.bind(target) : value;
        }});
      };
    });
    await page.goto(base);
    await page.locator('#cache-status').filter({hasText:'Save the launcher first'}).waitFor();
    requests.length = 0;
    await page.evaluate(async () => { await navigator.serviceWorker.ready; });
    await page.waitForTimeout(100);
    await page.evaluate(() => { window.forceActivationRace = true; });
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.ok(requests.some(r => r.path === 'changed.json'), 'Activation must not skip core preparation');
    pass('worker activating during its STATUS reply retains the inspected reference and prepares core');
    assert.deepEqual(errors, []);
    await fs.writeFile(path.join(OUTPUT, 'results.json'), JSON.stringify({passed: results, pageErrors: errors, engine: 'engine and GPU capability stubbed; production HTML/CSS/PWA/SW and real CacheStorage'}, null, 2));
    console.log(`\n${results.length} real startup-UI scenarios passed; evidence: ${OUTPUT}`);
  } finally {
    releaseCombat?.(); releaseDownload();
    await context?.close();
    await new Promise(resolve => server.close(resolve));
    await fs.rm(temporary, {recursive: true, force: true});
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
