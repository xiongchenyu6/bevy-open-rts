/* The build seals this manifest; install never downloads a new game or activates it. */
const MANIFEST = /* BUILD_MANIFEST */ null;
const SCOPE = self.registration.scope;
const PREFIX = 'open-rts-pwa-' + new URL(SCOPE).pathname;
const BLOBS = PREFIX + '-blobs';
const META = PREFIX + '-manifests';
const CLIENTS = PREFIX + '-clients';
const key = (type, id) => new URL('__pwa/' + type + '/' + encodeURIComponent(id), SCOPE).href;
const blobKey = file => key('sha256', file.sha256);
const groupOf = file => file.group || (
  file.path.startsWith('THIRD_PARTY_NOTICES/') || file.path.startsWith('assets/licenses/') ? 'optional' :
  !file.path.startsWith('assets/') || /^(assets\/(fonts|ui\/icons|ui\/controls|data)\/)/.test(file.path) ? 'core' : 'combat');
const allGroups = manifest => [...new Set(manifest.files.map(groupOf))];
const selected = (manifest, encoding, groups) => manifest.files.filter(f =>
  (!f.encoding || f.encoding === encoding) && (!groups || groups.includes(groupOf(f))));
const encodingOf = data => data.wasmEncoding === 'raw' ? 'raw' : 'gzip';
const send = (port, data) => { try { port.postMessage(data); } catch (_) {} };
let preparation = null;
let preparationAbort;
let preparationOwner;
const cancellations = new Map();
let listeners = new Set();

async function read(cache, url) {
  const response = await (await caches.open(cache)).match(url);
  return response ? response.json() : null;
}
async function write(cache, url, value) {
  await (await caches.open(cache)).put(url, new Response(JSON.stringify(value), {
    headers: {'Content-Type': 'application/json'}
  }));
}
async function remember() {
  if (MANIFEST) await write(META, key('version', MANIFEST.version), MANIFEST);
}
async function committedManifest() {
  const pointer = await read(META, key('committed', 'current'));
  return pointer ? await read(META, key('version', pointer.version)) || MANIFEST : MANIFEST;
}
async function installedGroups(encoding) {
  const saved = await read(META, key('installed', 'groups'));
  if (saved) return saved;
  // Upgrade a previously complete installation without forgetting its offline content.
  const current = await committedManifest();
  const cache = await caches.open(BLOBS);
  const groups = ['core'];
  for (const group of allGroups(current).filter(g => g !== 'core')) {
    const files = selected(current, encoding, [group]);
    if (files.length && (await Promise.all(files.map(f => cache.match(blobKey(f))))).every(Boolean)) groups.push(group);
  }
  await write(META, key('installed', 'groups'), groups);
  return groups;
}
async function requestedGroups(manifest, encoding, requested) {
  const available = allGroups(manifest);
  if (requested?.includes('all')) return available;
  const groups = requested || await installedGroups(encoding);
  if (requested && groups.some(g => !available.includes(g))) throw new Error('Unknown resource group');
  return [...new Set(['core', ...groups.filter(g => available.includes(g))])];
}
async function status(encoding, manifest = MANIFEST, requested) {
  if (!manifest) throw new Error('Game manifest missing; rebuild the web release.');
  const cache = await caches.open(BLOBS);
  // One cache index snapshot avoids thousands of serial disk IPCs on phones.
  const saved = new Set((await cache.keys()).map(request => request.url));
  const installed = await installedGroups(encoding);
  const required = await requestedGroups(manifest, encoding, requested);
  const groups = [];
  for (const id of allGroups(manifest)) {
    let totalBytes = 0, downloadBytes = 0;
    const seen = new Set();
    for (const file of selected(manifest, encoding, [id])) {
      if (seen.has(file.sha256)) continue;
      seen.add(file.sha256); totalBytes += file.bytes;
      if (!saved.has(blobKey(file))) downloadBytes += file.bytes;
    }
    groups.push({id, totalBytes, downloadBytes, ready: downloadBytes === 0, installed: installed.includes(id)});
  }
  let totalBytes = 0, downloadBytes = 0;
  const seen = new Set();
  for (const file of selected(manifest, encoding, required)) {
    if (seen.has(file.sha256)) continue;
    seen.add(file.sha256); totalBytes += file.bytes;
    if (!saved.has(blobKey(file))) downloadBytes += file.bytes;
  }
  return {version: manifest.version, revision: manifest.revision, wasmEncoding: encoding,
    currentVersion: (await committedManifest()).version, groups, installedGroups: installed,
    ready: downloadBytes === 0, downloadBytes, totalBytes};
}
async function verifiedFetch(file, progress, signal) {
  const response = await fetch(new URL(file.url || file.path, SCOPE), {cache: 'no-store', signal});
  if (!response.ok) throw new Error('Download failed: ' + file.path + ' (' + response.status + ')');
  const reader = response.body && response.body.getReader();
  const chunks = [];
  let size = 0;
  if (reader) {
    while (true) {
      const {done, value} = await reader.read();
      if (done) break;
      chunks.push(value); size += value.byteLength;
      if (size > file.bytes) { await reader.cancel(); throw new Error('Version changed: ' + file.path); }
      if (progress) progress(value.byteLength);
    }
  } else {
    const value = new Uint8Array(await response.arrayBuffer());
    chunks.push(value); size = value.length;
    if (progress) progress(size);
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
  const digest = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))]
    .map(n => n.toString(16).padStart(2, '0')).join('');
  if (size !== file.bytes || digest !== file.sha256) throw new Error('Version changed: ' + file.path);
  const headers = new Headers(response.headers);
  headers.delete('content-encoding'); headers.set('content-length', String(file.bytes));
  return new Response(bytes, {status: 200, headers});
}
async function prepare(encoding, manifest, requested, signal) {
  const groups = await requestedGroups(manifest, encoding, requested);
  await remember();
  const before = await status(encoding, manifest, groups);
  let received = 0;
  const report = () => {
    for (const port of listeners) send(port, {type: 'PROGRESS', received, total: before.downloadBytes});
  };
  report();
  const cache = await caches.open(BLOBS);
  for (const file of selected(manifest, encoding, groups)) {
    signal?.throwIfAborted();
    if (await cache.match(blobKey(file))) continue;
    const response = await verifiedFetch(file, count => { received += count; report(); }, signal);
    await cache.put(blobKey(file), response);
  }
  const result = await status(encoding, manifest, groups);
  if (!result.ready) throw new Error('The browser could not retain the complete game.');
  const installed = await installedGroups(encoding);
  await write(META, key('installed', 'groups'), [...new Set([...installed, ...groups])]);
  return {...result, installedGroups: [...new Set([...installed, ...groups])]};
}

self.addEventListener('install', event => event.waitUntil(remember()));
self.addEventListener('message', event => {
  const port = event.ports[0];
  if (!port) return;
  const data = event.data || {};
  const encoding = encodingOf(data);
  event.waitUntil((async () => {
    try {
      const manifest = data.target === 'client' ? await manifestFor(event.source?.id) : data.target === 'current' ? await committedManifest() : MANIFEST;
      if (data.type === 'STATUS') {
        send(port, {type: 'STATUS', ...await status(encoding, manifest, data.groups)});
      } else if (data.type === 'PREPARE') {
        const owner = event.source?.id;
        const epoch = cancellations.get(owner) || 0;
        // Serialize preparations so group commits and progress have one owner.
        while (preparation) await preparation.catch(() => {});
        if (epoch !== (cancellations.get(owner) || 0)) throw new Error('Download cancelled');
        listeners.add(port);
        preparationAbort = new AbortController();
        preparationOwner = event.source?.id;
        const promise = prepare(encoding, manifest, data.groups, preparationAbort.signal);
        preparation = promise;
        try {
          const result = await promise;
          send(port, {type: 'DONE', ...result});
        } finally { if (preparation === promise) preparation = null; }
      } else if (data.type === 'CANCEL') {
        const owner = event.source?.id;
        cancellations.set(owner, (cancellations.get(owner) || 0) + 1);
        if (preparationOwner === owner) preparationAbort?.abort();
        send(port, {type: 'DONE'});
      } else if (data.type === 'PIN_GROUPS') {
        const id = event.source?.id;
        if (id) {
          const existing = await read(CLIENTS, key('client', id));
          await write(CLIENTS, key('client', id), {version: existing?.version || manifest.version, groups: data.groups || []});
        }
        send(port, {type: 'DONE'});
      } else if (data.type === 'REMOVE_GROUPS') {
        if (preparation) throw new Error('A download is still running');
        const remove = data.groups || [];
        if (remove.includes('core')) throw new Error('Startup resources cannot be removed');
        const clients = await self.clients.matchAll({type: 'window'});
        for (const client of clients) {
          const binding = await read(CLIENTS, key('client', client.id));
          if (binding?.groups?.some(g => remove.includes(g))) throw new Error('Close the active match before removing its resources');
        }
        const installed = (await installedGroups(encoding)).filter(g => !remove.includes(g));
        const keep = new Set(selected(manifest, encoding, installed).map(blobKey));
        const cache = await caches.open(BLOBS);
        for (const file of selected(manifest, encoding, remove)) {
          if (!keep.has(blobKey(file))) await cache.delete(blobKey(file));
        }
        await write(META, key('installed', 'groups'), installed);
        send(port, {type: 'DONE', ...await status(encoding, manifest)});
      } else if (data.type === 'ACTIVATE') {
        const result = await status(encoding);
        if (!result.ready) throw new Error('Download the complete update before activating it.');
        await write(META, key('committed', 'current'), {version: MANIFEST.version});
        send(port, {type: 'DONE', ...result, currentVersion: MANIFEST.version});
        await self.skipWaiting();
      }
    } catch (error) {
      send(port, {type: 'ERROR', message: String(error.message || error)});
    } finally {
      listeners.delete(port);
    }
  })());
});

async function bind(clientId, version) {
  if (clientId) {
    const existing = await read(CLIENTS, key('client', clientId));
    await write(CLIENTS, key('client', clientId), {version, groups: existing?.groups || []});
  }
}
async function manifestFor(clientId) {
  const binding = clientId && await read(CLIENTS, key('client', clientId));
  return binding ? await read(META, key('version', binding.version)) : await committedManifest();
}
async function cleanUnused() {
  const clients = (await self.clients.matchAll({type: 'window', includeUncontrolled: true}))
    .filter(client => client.url.startsWith(SCOPE));
  const liveIds = new Set(clients.map(client => client.id));
  const bindings = await caches.open(CLIENTS);
  const used = new Set([MANIFEST.version, (await committedManifest()).version]);
  for (const request of await bindings.keys()) {
    const id = decodeURIComponent(new URL(request.url).pathname.split('/').pop());
    if (!liveIds.has(id)) { await bindings.delete(request); continue; }
    used.add((await (await bindings.match(request)).json()).version);
  }
  // An uncontrolled old tab may not yet have a persisted binding. Keep versions
  // until that tab navigates; deleting early can break its lazy-loaded assets.
  for (const client of clients) {
    if (!(await read(CLIENTS, key('client', client.id)))) return;
  }
  const manifests = await caches.open(META);
  const hashes = new Set();
  for (const request of await manifests.keys()) {
    if (!request.url.startsWith(new URL('__pwa/version/', SCOPE).href)) continue;
    const manifest = await (await manifests.match(request)).json();
    if (!used.has(manifest.version)) { await manifests.delete(request); continue; }
    for (const file of manifest.files) hashes.add(blobKey(file));
  }
  const blobs = await caches.open(BLOBS);
  for (const request of await blobs.keys()) if (!hashes.has(request.url)) await blobs.delete(request);
}
self.addEventListener('activate', event => event.waitUntil((async () => {
  await remember();
  if (!(await read(META, key('committed', 'current')))) {
    await write(META, key('committed', 'current'), {version: MANIFEST.version});
  }
  await self.clients.claim();
  await cleanUnused();
})()));

self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || !url.href.startsWith(SCOPE) || url.origin !== self.location.origin) return;
  const path = decodeURIComponent(url.pathname.slice(new URL(SCOPE).pathname.length));
  if (path === 'sw.js' || path === 'build.json' || path.startsWith('__pwa/') || path.startsWith('_resources/') || path.startsWith('_releases/')) return;
  event.respondWith((async () => {
    const navigation = event.request.mode === 'navigate';
    const manifest = navigation ? await committedManifest() : await manifestFor(event.clientId);
    if (!manifest) return fetch(event.request);
    if (navigation) await bind(event.resultingClientId || event.clientId, manifest.version);
    else if (event.clientId) await bind(event.clientId, manifest.version);
    const file = manifest.files.find(f => f.path === (navigation ? 'index.html' : path));
    if (!file) return fetch(event.request);
    const cache = await caches.open(BLOBS);
    const cached = await cache.match(blobKey(file));
    const complete = !navigation || (await status('gzip', manifest, ['core'])).ready || (await status('raw', manifest, ['core'])).ready;
    if (cached && complete) return cached;
    // Keep uncached first launch online, but never silently mix release bytes.
    // Only PREPARE retains downloaded bytes; merely visiting isn't installation.
    const response = await verifiedFetch(file);
    // On-demand non-startup resources are retained after their first successful use.
    if (!navigation) await cache.put(blobKey(file), response.clone());
    return response;
  })());
});
