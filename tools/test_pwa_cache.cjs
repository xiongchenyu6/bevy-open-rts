#!/usr/bin/env node
// Real Chromium + Service Worker integration. No mocked CacheStorage or fetch.
// npm install --no-save playwright
// CHROMIUM_EXECUTABLE_PATH=/path/to/chromium node tools/test_pwa_cache.cjs
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const http = require('node:http');
const os = require('node:os');
const {spawnSync} = require('node:child_process');
const {gzipSync} = require('node:zlib');
const {chromium} = require('playwright-core');

const ROOT = path.resolve(__dirname, '..');
const OUTPUT = path.resolve(process.env.PWA_TEST_OUTPUT || 'target/pwa-cache-tests');
const passed = [];
const log = message => { passed.push(message); console.log(`PASS ${message}`); };
const timeout = 20000;

async function rpc(page, target, type, extra = {}) {
  return page.evaluate(async ({target, type, extra, timeout}) => {
    const registration = await navigator.serviceWorker.getRegistration();
    const worker = target === 'candidate' ? (registration.waiting || registration.active) : target === 'controller' ? navigator.serviceWorker.controller : registration[target];
    if (!worker) throw new Error(`No ${target} service worker`);
    return new Promise((resolve, reject) => {
      const channel = new MessageChannel();
      const progress = [];
      const timer = setTimeout(() => { channel.port1.close(); reject(new Error(`Timed out: ${type}`)); }, timeout);
      channel.port1.onmessage = event => {
        if (event.data.type === 'PROGRESS') { progress.push(event.data); return; }
        clearTimeout(timer);
        channel.port1.close();
        resolve({...event.data, progress});
      };
      worker.postMessage({type, wasmEncoding: 'gzip', groups: ['all'], ...extra}, [channel.port2]);
    });
  }, {target, type, extra, timeout});
}
async function waiting(page) {
  await page.evaluate(async () => { await (await navigator.serviceWorker.getRegistration()).update(); });
  await page.waitForFunction(async () => Boolean((await navigator.serviceWorker.getRegistration())?.waiting), null, {timeout});
}
async function install(page) {
  await page.evaluate(async () => {
    await navigator.serviceWorker.register('./sw.js', {scope: './', updateViaCache: 'none'});
    await navigator.serviceWorker.ready;
  });
  await page.waitForFunction(() => Boolean(navigator.serviceWorker.controller), null, {timeout});
}
const readAsset = page => page.evaluate(() => fetch('./assets/changing.txt').then(r => r.text()));
const version = page => page.locator('h1').textContent();

async function main() {
  await fs.mkdir(OUTPUT, {recursive: true});
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'open-rts-pwa-test-'));
  const fixtures = {};
  for (const revision of ['v1', 'v2', 'v3', 'v4']) {
    const directory = path.join(temporary, revision);
    await fs.mkdir(path.join(directory, 'pkg'), {recursive: true});
    await fs.mkdir(path.join(directory, 'assets'), {recursive: true});
    await fs.writeFile(path.join(directory, 'index.html'), `<!doctype html><html><head><meta charset="utf-8"><link rel="icon" href="data:,"></head><body><h1>${revision}</h1><p>Real service worker integration fixture</p></body></html>`);
    await fs.writeFile(path.join(directory, 'assets/changing.txt'), `model-${revision}`);
    await fs.writeFile(path.join(directory, 'assets/unchanged.txt'), 'unchanged-texture-and-audio');
    await fs.writeFile(path.join(directory, 'assets/duplicate.txt'), 'unchanged-texture-and-audio');
    const wasm = Buffer.from('same-engine-wasm-fixture');
    await fs.writeFile(path.join(directory, 'pkg/bevy_open_rts_bg.wasm'), wasm);
    await fs.writeFile(path.join(directory, 'pkg/bevy_open_rts_bg.wasm.gz'), gzipSync(wasm));
    const generated = spawnSync('python3', ['tools/generate_web_manifest.py', directory, '--revision', revision, '--template', 'web/sw.js'], {cwd: ROOT, encoding: 'utf8'});
    assert.equal(generated.status, 0, generated.stderr);
    fixtures[revision] = {directory, manifest: JSON.parse(await fs.readFile(path.join(directory, 'asset-manifest.json'), 'utf8'))};
  }
  let current = 'v1';
  let failure = null;
  const requests = [];
  const server = http.createServer(async (request, response) => {
    const pathname = decodeURIComponent(new URL(request.url, 'http://fixture').pathname);
    const transport = pathname === '/game/' ? 'index.html' : pathname.slice('/game/'.length);
    const immutable = Object.values(fixtures).flatMap(f => f.manifest.files.map(file => ({file, directory: f.directory}))).find(item => item.file.url === transport);
    const relative = immutable?.file.path || transport;
    requests.push({revision: current, path: relative, transport});
    response.setHeader('Cache-Control', 'no-store');
    response.setHeader('Service-Worker-Allowed', '/game/');
    if (!pathname.startsWith('/game/') || relative.includes('..')) { response.writeHead(404).end(); return; }
    try {
      let data = await fs.readFile(path.join(immutable?.directory || fixtures[current].directory, relative));
      if (failure?.path === relative) {
        if (failure.kind === 'network') { response.writeHead(503).end('Unavailable'); return; }
        data = Buffer.alloc(data.length, 'X'); // Same length: tests SHA-256, not only size.
      }
      response.setHeader('Content-Type', relative.endsWith('.js') ? 'text/javascript' : relative.endsWith('.html') ? 'text/html' : 'application/octet-stream');
      response.writeHead(200).end(data);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}/game/`;
  const launch = (profile = 'profile') => chromium.launchPersistentContext(path.join(temporary, profile), {
    headless: true,
    executablePath: process.env.CHROMIUM_EXECUTABLE_PATH || undefined,
    viewport: {width: 1000, height: 700},
    serviceWorkers: 'allow',
  });
  let context;
  const pageErrors = [];
  const observe = page => page.on('pageerror', error => pageErrors.push(String(error)));
  try {
    context = await launch();
    let page = await context.newPage(); observe(page);
    await page.goto(base);
    await install(page);
    assert.equal(requests.filter(r => r.path.startsWith('assets/') || r.path.includes('.wasm')).length, 0);
    const before = await rpc(page, 'controller', 'STATUS');
    assert.equal(before.ready, false);
    const [prepared, concurrent] = await Promise.all([rpc(page, 'controller', 'PREPARE'), rpc(page, 'controller', 'PREPARE')]);
    assert.equal(concurrent.type, 'DONE');
    assert.equal(prepared.type, 'DONE', JSON.stringify(prepared));
    assert.equal(prepared.ready, true);
    assert.ok(prepared.progress.some(p => p.received === p.total && p.total > 0));
    assert.equal(requests.filter(r => r.path === 'pkg/bevy_open_rts_bg.wasm').length, 0);
    assert.equal(requests.filter(r => r.path === 'pkg/bevy_open_rts_bg.wasm.gz').length, 1);
    assert.equal(await readAsset(page), 'model-v1');
    assert.equal(requests.filter(r => ['assets/duplicate.txt', 'assets/unchanged.txt'].includes(r.path)).length, 1);
    log('first concurrent preparation verifies and persists assets, deduplicates identical hashes and downloads gzip only');

    requests.length = 0;
    await page.reload();
    assert.equal(await version(page), 'v1');
    assert.equal(await readAsset(page), 'model-v1');
    await page.evaluate(() => fetch('./assets/unchanged.txt').then(r => r.text()));
    assert.equal(requests.filter(r => r.path !== 'sw.js').length, 0, JSON.stringify(requests));
    assert.equal((await rpc(page, 'controller', 'STATUS')).downloadBytes, 0);
    log('re-entry performs zero shell, model, texture or engine network requests');

    await context.close();
    context = await launch();
    await context.setOffline(true);
    page = await context.newPage(); observe(page);
    await page.goto(base);
    assert.equal(await version(page), 'v1');
    assert.equal(await readAsset(page), 'model-v1');
    assert.equal((await rpc(page, 'controller', 'STATUS')).ready, true);
    log('browser process restart preserves cache and cold offline navigation works');
    await context.setOffline(false);
    const oldTab = await context.newPage(); observe(oldTab);
    await oldTab.goto(base);
    assert.equal(await readAsset(oldTab), 'model-v1');

    current = 'v2'; requests.length = 0;
    await waiting(page);
    const update = await rpc(page, 'waiting', 'STATUS');
    assert.equal(update.revision, 'v2');
    assert.equal(update.ready, false);
    assert.equal(requests.filter(r => r.path.startsWith('assets/') || r.path.includes('.wasm')).length, 0);
    assert.equal(await readAsset(page), 'model-v1');
    await page.reload();
    assert.equal(await version(page), 'v1');
    assert.equal(await readAsset(page), 'model-v1');
    log('detecting/deferring an update downloads no game assets and keeps old version playable');

    await context.close();
    context = await launch();
    await context.setOffline(true);
    page = await context.newPage(); observe(page);
    await page.goto(base);
    assert.equal(await version(page), 'v1');
    assert.equal(await readAsset(page), 'model-v1');
    const deferredRestart = await rpc(page, 'candidate', 'STATUS');
    assert.equal(deferredRestart.revision, 'v2');
    assert.equal(deferredRestart.currentVersion, fixtures.v1.manifest.version);
    log('closing every tab after deferring does not silently commit the naturally activated update');
    await context.setOffline(false);
    const pinnedOldTab = await context.newPage(); observe(pinnedOldTab);
    await pinnedOldTab.goto(base);
    assert.equal(await readAsset(pinnedOldTab), 'model-v1');

    requests.length = 0;
    const upgraded = await rpc(page, 'candidate', 'PREPARE');
    assert.equal(upgraded.type, 'DONE', JSON.stringify(upgraded));
    const oldHashes = new Set(fixtures.v1.manifest.files.map(f => f.sha256));
    const expected = fixtures.v2.manifest.files.filter(f => f.encoding !== 'raw' && !oldHashes.has(f.sha256));
    assert.deepEqual(requests.filter(r => r.path !== 'sw.js').map(r => r.path).sort(), expected.map(f => f.path).sort());
    assert.equal(update.downloadBytes, expected.reduce((n, f) => n + f.bytes, 0));
    assert.equal(upgraded.progress.at(-1).received, update.downloadBytes);
    const accepted = await rpc(page, 'candidate', 'ACTIVATE');
    assert.equal(accepted.type, 'DONE', JSON.stringify(accepted));
    await page.waitForFunction(async () => !(await navigator.serviceWorker.getRegistration()).waiting, null, {timeout});
    await page.reload();
    assert.equal(await version(page), 'v2');
    assert.equal(await readAsset(page), 'model-v2');
    assert.equal(await version(pinnedOldTab), 'v1');
    assert.equal(await readAsset(pinnedOldTab), 'model-v1');
    const cdp = await context.newCDPSession(page);
    await cdp.send('ServiceWorker.enable');
    await cdp.send('ServiceWorker.stopAllWorkers');
    assert.equal(await readAsset(pinnedOldTab), 'model-v1');
    assert.equal(await readAsset(page), 'model-v2');
    await cdp.detach();
    log('accepted update downloads only changed hashes; old open tab remains pinned to old assets');
    await pinnedOldTab.close();

    current = 'v3';
    await waiting(page);
    failure = {path: 'assets/changing.txt', kind: 'hash'};
    const bad = await rpc(page, 'waiting', 'PREPARE');
    assert.equal(bad.type, 'ERROR', JSON.stringify(bad));
    assert.match(bad.message, /Version changed|hash|integrity/i);
    assert.equal((await rpc(page, 'waiting', 'STATUS')).ready, false);
    assert.equal((await rpc(page, 'waiting', 'ACTIVATE')).type, 'ERROR');
    assert.equal(await readAsset(page), 'model-v2');
    await page.reload();
    assert.equal(await version(page), 'v2');
    log('same-size hash corruption prevents activation and leaves old cached version intact');
    failure = null;
    const retried = await rpc(page, 'waiting', 'PREPARE');
    assert.equal(retried.type, 'DONE', JSON.stringify(retried));
    assert.equal((await rpc(page, 'waiting', 'ACTIVATE')).type, 'DONE');
    await page.waitForFunction(async () => !(await navigator.serviceWorker.getRegistration()).waiting, null, {timeout});
    await page.reload();
    assert.equal(await readAsset(page), 'model-v3');
    log('retry after integrity failure recovers and activates complete version');

    current = 'v4';
    await waiting(page);
    failure = {path: 'assets/changing.txt', kind: 'network'};
    assert.equal((await rpc(page, 'waiting', 'PREPARE')).type, 'ERROR');
    assert.equal((await rpc(page, 'waiting', 'ACTIVATE')).type, 'ERROR');
    assert.equal(await readAsset(page), 'model-v3');
    await context.setOffline(true);
    await page.reload();
    assert.equal(await version(page), 'v3');
    assert.equal(await readAsset(page), 'model-v3');
    log('interrupted update retains the old complete version for offline use');
    assert.deepEqual(pageErrors, []);
    await page.screenshot({path: path.join(OUTPUT, 'offline-after-failed-update.png')});
    await context.close();
    current = 'v1'; failure = null; requests.length = 0;
    context = await launch('raw-profile');
    page = await context.newPage(); observe(page);
    await page.goto(base); await install(page);
    assert.equal((await rpc(page, 'controller', 'PREPARE', {wasmEncoding: 'raw'})).type, 'DONE');
    assert.equal(requests.filter(r => r.path === 'pkg/bevy_open_rts_bg.wasm').length, 1);
    assert.equal(requests.filter(r => r.path === 'pkg/bevy_open_rts_bg.wasm.gz').length, 0);
    await context.setOffline(true); await page.reload();
    assert.equal(await page.evaluate(() => fetch('./pkg/bevy_open_rts_bg.wasm').then(r => r.text())), 'same-engine-wasm-fixture');
    log('raw-WASM capability path saves only raw engine and works offline');

    await fs.writeFile(path.join(OUTPUT, 'results.json'), JSON.stringify({passed, pageErrors, browser: context.browser()?.version() || 'persistent Chromium'}, null, 2));
    console.log(`\n${passed.length} real-browser scenarios passed; evidence: ${OUTPUT}`);
  } finally {
    await context?.close();
    await new Promise(resolve => server.close(resolve));
    await fs.rm(temporary, {recursive: true, force: true});
  }
}
module.exports = {rpc, waiting, install};
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
