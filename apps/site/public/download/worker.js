const cacheName = 'gigacouch-build-v1:' + new URL(self.registration.scope).pathname;
self.addEventListener('install', event => event.waitUntil(self.skipWaiting()));
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || !url.href.startsWith(self.registration.scope)) return;
  const helper = url.searchParams.get('couch');
  if (helper === 'worker') return;
  event.respondWith((async () => {
    const cache = await caches.open(cacheName);
    // Helpers are exact query-keyed entries; a game index must never resolve to its downloader.
    if (helper) {
      url.search = '?couch=' + encodeURIComponent(helper);
      // Prefer fresh helper code online, retaining an offline fallback.
      try { return await fetch(event.request,{signal:AbortSignal.timeout(10000)}); }
      catch { return await cache.match(url.href) || Response.error(); }
    }
    url.search = '';
    return await cache.match(url.href) || fetch(event.request);
  })());
});
