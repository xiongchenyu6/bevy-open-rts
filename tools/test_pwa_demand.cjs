#!/usr/bin/env node
// Real HTTP, Chromium and CacheStorage: request receipts prove on-demand behavior.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const http = require('node:http');
const {spawnSync} = require('node:child_process');
const {gzipSync} = require('node:zlib');
const {chromium} = require('playwright-core');
const {rpc: fullRpc, waiting, install} = require('./test_pwa_cache.cjs');
const rpc = (page, target, type, extra = {}) => fullRpc(page, target, type, {groups: undefined, ...extra});
const ROOT = path.resolve(__dirname, '..');
const OUTPUT = path.resolve(process.env.PWA_TEST_OUTPUT || 'target/pwa-demand-tests');
const passed = [], receipts = [];
const pass = message => { passed.push(message); console.log('PASS ' + message); };
const read = (page, file) => page.evaluate(file => fetch('./' + file).then(async r => { if (!r.ok) throw Error(String(r.status)); return r.text(); }), file);
const candidate = async page => { await page.evaluate(async () => (await navigator.serviceWorker.getRegistration()).update()); await waiting(page); };
async function accept(page, expectedInstalled) {
  const prepared = await rpc(page, 'waiting', 'PREPARE');
  assert.equal(prepared.type, 'DONE', JSON.stringify(prepared));
  assert.equal((await rpc(page, 'waiting', 'ACTIVATE')).type, 'DONE');
  await page.waitForFunction(async () => { const reg = await navigator.serviceWorker.getRegistration(); return !reg.waiting && reg.active?.state === 'activated' && navigator.serviceWorker.controller === reg.active; });
  if (expectedInstalled) {
    const ledger = await page.evaluate(async () => {
      const name = (await caches.keys()).find(name => name.endsWith('-manifests'));
      const cache = await caches.open(name);
      const request = (await cache.keys()).find(request => request.url.endsWith('/__pwa/installed/groups'));
      return request ? (await cache.match(request)).json() : null;
    });
    assert.deepEqual(ledger?.sort(), [...expectedInstalled].sort(), 'installed group ledger must survive activation before any navigation/status can reconstruct it');
  }
  await page.reload();
}
async function main() {
  await fs.mkdir(OUTPUT, {recursive: true});
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'rogue-demand-'));
  const fixtures = {};
  for (const revision of ['v1', 'v2', 'v3', 'v4']) {
    const directory = path.join(temporary, revision);
    await fs.mkdir(path.join(directory, 'pkg'), {recursive: true});
    await fs.mkdir(path.join(directory, 'assets/models'), {recursive: true});
    await fs.mkdir(path.join(directory, 'assets/data'), {recursive: true});
    await fs.mkdir(path.join(directory, 'THIRD_PARTY_NOTICES'), {recursive: true});
    const shell = revision === 'v1' ? 'v1' : 'v2';
    await fs.writeFile(path.join(directory, 'index.html'), `<!doctype html><html><head><link rel="icon" href="data:,"></head><body><h1>${shell}</h1><p>On-demand update integration fixture</p></body></html>`);
    await fs.writeFile(path.join(directory, 'assets/models/enemy.glb'), revision === 'v1' ? 'enemy-v1' : revision === 'v4' ? 'enemy-v4' : 'enemy-v2');
    await fs.writeFile(path.join(directory, 'assets/models/shared.glb'), 'shared-model');
    await fs.writeFile(path.join(directory, 'assets/data/campaign.json'), JSON.stringify({damage: revision === 'v1' || revision === 'v2' ? 1 : 2}));
    await fs.writeFile(path.join(directory, 'THIRD_PARTY_NOTICES/license.txt'), 'optional-license-' + revision);
    const engine = Buffer.from('same-engine-for-data-only-update');
    await fs.writeFile(path.join(directory, 'pkg/bevy_open_rts_bg.wasm'), engine);
    await fs.writeFile(path.join(directory, 'pkg/bevy_open_rts_bg.wasm.gz'), gzipSync(engine));
    const generated = spawnSync('python3', ['tools/generate_web_manifest.py', directory, '--revision', revision, '--template', 'web/sw.js'], {cwd: ROOT, encoding: 'utf8'});
    assert.equal(generated.status, 0, generated.stderr);
    fixtures[revision] = {directory, manifest: JSON.parse(await fs.readFile(path.join(directory, 'asset-manifest.json'), 'utf8'))};
  }
  const allFiles = Object.values(fixtures).flatMap(f => f.manifest.files.map(file => ({file, directory: f.directory})));
  let current = 'v1', corrupt = false, holdCombat = false, heldCombat = false, releaseCombat;
  let combatGate;
  const requests = [];
  const server = http.createServer(async (request, response) => {
    const pathname = decodeURIComponent(new URL(request.url, 'http://fixture').pathname);
    const transport = pathname === '/game/' ? 'index.html' : pathname.slice('/game/'.length);
    const immutable = allFiles.find(item => item.file.url === transport);
    const relative = immutable?.file.path || transport;
    requests.push({revision: current, path: relative, transport});
    response.setHeader('Cache-Control', 'no-store');
    response.setHeader('Service-Worker-Allowed', '/game/');
    if (!pathname.startsWith('/game/') || transport.includes('..')) { response.writeHead(404).end(); return; }
    try {
      let bytes = await fs.readFile(path.join(immutable?.directory || fixtures[current].directory, relative));
      if (corrupt && immutable?.file.path === 'assets/models/enemy.glb' && immutable.directory === fixtures.v4.directory) bytes = Buffer.alloc(bytes.length, 88);
      response.setHeader('Content-Type', relative.endsWith('.js') ? 'text/javascript' : relative.endsWith('.html') ? 'text/html' : relative.endsWith('.json') ? 'application/json' : 'application/octet-stream');
      if (holdCombat && relative === 'assets/models/shared.glb') {
        heldCombat = true;
        response.writeHead(200); response.write(bytes.subarray(0, 1));
        await combatGate; response.end(bytes.subarray(1));
      } else response.writeHead(200).end(bytes);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}/game/`;
  const launch = () => chromium.launchPersistentContext(path.join(temporary, 'profile'), {headless: true, executablePath: process.env.CHROMIUM_EXECUTABLE_PATH || undefined, viewport: {width: 1000, height: 700}, serviceWorkers: 'allow'});
  let context;
  const errors = [];
  const observe = page => page.on('pageerror', error => errors.push(String(error)));
  const save = label => { receipts.push({label, requests: [...requests]}); requests.length = 0; };
  try {
    context = await launch(); let page = await context.newPage(); observe(page);
    await page.goto(base); await install(page); requests.length = 0;
    assert.equal((await rpc(page, 'controller', 'PREPARE', {groups: ['core']})).type, 'DONE');
    assert.equal(requests.filter(r => r.path.startsWith('assets/models/') || r.path.startsWith('THIRD_PARTY_NOTICES/')).length, 0);
    assert.equal(requests.filter(r => r.path === 'pkg/bevy_open_rts_bg.wasm.gz').length, 1);
    assert.equal(requests.filter(r => r.path === 'pkg/bevy_open_rts_bg.wasm').length, 0);
    assert.deepEqual((await rpc(page, 'controller', 'STATUS')).installedGroups, ['core']);
    save('first-core-only'); pass('first Start saves shell/config/engine without combat or optional resources');

    const oldTab = await context.newPage(); observe(oldTab); await oldTab.goto(base);
    current = 'v2'; await candidate(page); requests.length = 0;
    await accept(page);
    assert.equal(await page.locator('h1').textContent(), 'v2');
    assert.equal(requests.filter(r => r.path.startsWith('assets/') || r.path.includes('.wasm') || r.path.startsWith('THIRD_PARTY_NOTICES/')).length, 0);
    save('core-only-upgrade'); pass('updating a core-only installation does not acquire changed combat or optional packs');

    assert.equal(await read(oldTab, 'assets/models/enemy.glb'), 'enemy-v1');
    assert.equal(requests.filter(r => r.path === 'assets/models/enemy.glb').length, 1);
    assert.ok(requests.find(r => r.path === 'assets/models/enemy.glb').transport.startsWith('_resources/'));
    save('old-version-first-lazy-fetch'); pass('old open version lazily fetches its original immutable model after a new release');
    await oldTab.close();

    holdCombat = true;
    combatGate = new Promise(resolve => { releaseCombat = resolve; });
    const canceledPreparation = rpc(page, 'controller', 'PREPARE', {target: 'client', groups: ['combat']});
    for (let attempt = 0; !heldCombat && attempt < 100; attempt++) await new Promise(resolve => setTimeout(resolve, 50));
    assert.ok(heldCombat, 'combat transfer reached controlled partial response');
    const queuedTab = await context.newPage(); observe(queuedTab); await queuedTab.goto(base);
    const queuedPreparation = rpc(queuedTab, 'controller', 'PREPARE', {target: 'client', groups: ['optional']});
    await rpc(queuedTab, 'controller', 'STATUS');
    assert.equal((await rpc(queuedTab, 'controller', 'CANCEL')).type, 'DONE');
    assert.equal((await rpc(page, 'controller', 'CANCEL')).type, 'DONE');
    assert.equal((await canceledPreparation).type, 'ERROR');
    assert.equal((await queuedPreparation).type, 'ERROR');
    assert.equal(requests.filter(r => r.path.startsWith('THIRD_PARTY_NOTICES/')).length, 0);
    await queuedTab.close();
    pass('canceling a queued download in another tab never starts its optional transfer');
    assert.deepEqual((await rpc(page, 'controller', 'STATUS')).installedGroups, ['core']);
    holdCombat = false; releaseCombat();
    assert.equal((await rpc(page, 'controller', 'PREPARE', {target: 'client', groups: ['combat']})).type, 'DONE');
    assert.equal(requests.filter(r => r.path === 'assets/models/enemy.glb').length, 1, 'verified earlier file survives cancel and is reused');
    assert.equal(requests.filter(r => r.path === 'assets/models/shared.glb').length, 2, 'partial file is fetched again');
    pass('canceling chapter download leaves launcher usable and retry reuses already verified files');
    assert.deepEqual([...new Set(requests.filter(r => r.path !== 'sw.js').map(r => r.path))].sort(), ['assets/models/enemy.glb', 'assets/models/shared.glb']);
    assert.equal(await read(page, 'assets/models/enemy.glb'), 'enemy-v2');
    save('first-chapter-combat');
    assert.equal((await rpc(page, 'controller', 'PREPARE', {target: 'current', groups: ['combat']})).downloadBytes, 0);
    assert.equal(requests.filter(r => r.path !== 'sw.js').length, 0);
    save('second-chapter-shared'); pass('first chapter fetches combat once; another chapter reuses shared combat with zero downloads');

    current = 'v3'; await candidate(page); requests.length = 0;
    await accept(page, ['core', 'combat']);
    pass('installed group ledger survives worker activation before navigation or status can reconstruct it');
    assert.deepEqual(requests.filter(r => r.path !== 'sw.js').map(r => r.path), ['assets/data/campaign.json'], JSON.stringify(requests));
    assert.deepEqual(JSON.parse(await read(page, 'assets/data/campaign.json')), {damage: 2});
    save('data-only-upgrade'); pass('chapter balance-only update downloads one JSON file and no engine, model or unused optional files');

    current = 'v4'; await candidate(page); corrupt = true; requests.length = 0;
    const rejected = await rpc(page, 'waiting', 'PREPARE');
    assert.equal(rejected.type, 'ERROR', JSON.stringify(rejected));
    assert.equal((await rpc(page, 'waiting', 'ACTIVATE')).type, 'ERROR');
    assert.equal(await read(page, 'assets/models/enemy.glb'), 'enemy-v2');
    await context.setOffline(true); await page.reload();
    assert.equal(await read(page, 'assets/models/enemy.glb'), 'enemy-v2');
    save('installed-pack-corruption'); pass('installed combat updates must pass integrity before activation; failed update retains offline old chapter');
    await context.setOffline(false); corrupt = false;
    await accept(page);
    assert.equal(await read(page, 'assets/models/enemy.glb'), 'enemy-v4');
    save('installed-pack-retry');

    assert.equal((await rpc(page, 'controller', 'PREPARE', {target: 'current', groups: ['all']})).type, 'DONE');
    assert.equal(requests.filter(r => r.path === 'THIRD_PARTY_NOTICES/license.txt').length, 1);
    save('full-offline-opt-in');
    await context.close(); context = await launch(); await context.setOffline(true);
    page = await context.newPage(); observe(page); await page.goto(base);
    assert.equal(await read(page, 'assets/models/enemy.glb'), 'enemy-v4');
    assert.equal(await read(page, 'THIRD_PARTY_NOTICES/license.txt'), 'optional-license-v4');
    pass('explicit full offline download survives complete browser process restart with networking disabled');

    assert.equal((await rpc(page, 'controller', 'PIN_GROUPS', {groups: ['combat']})).type, 'DONE');
    assert.equal((await rpc(page, 'controller', 'REMOVE_GROUPS', {groups: ['combat']})).type, 'ERROR');
    assert.equal((await rpc(page, 'controller', 'REMOVE_GROUPS', {groups: ['core']})).type, 'ERROR');
    assert.equal((await rpc(page, 'controller', 'REMOVE_GROUPS', {groups: ['optional']})).type, 'DONE');
    const cleaned = await rpc(page, 'controller', 'STATUS', {groups: ['all']});
    assert.equal(cleaned.groups.find(g => g.id === 'optional').ready, false);
    assert.equal(await read(page, 'assets/models/enemy.glb'), 'enemy-v4');
    assert.ok(cleaned.groups.find(g => g.id === 'core').ready);
    pass('storage cleanup can remove optional bytes but refuses core and active battle resources');
    assert.deepEqual(errors, []);
    await page.screenshot({path: path.join(OUTPUT, 'offline-after-demand-update.png')});
    await fs.writeFile(path.join(OUTPUT, 'results.json'), JSON.stringify({passed, requests: receipts, errors}, null, 2));
    console.log(`\n${passed.length} demand-update scenarios passed; request receipts: ${OUTPUT}`);
  } finally { releaseCombat?.(); await context?.close(); await new Promise(resolve => server.close(resolve)); await fs.rm(temporary, {recursive: true, force: true}); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
