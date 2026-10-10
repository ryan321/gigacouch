// One immutable build per scope. The download frame fills this cache before launch.
const cacheName = 'gigacouch-build-v1:' + new URL(self.registration.scope).pathname;
self.addEventListener('install', event => event.waitUntil(self.skipWaiting()));
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || !url.href.startsWith(self.registration.scope) || url.searchParams.has('couch')) return;
  event.respondWith((async () => {
    const cache = await caches.open(cacheName);
    return await cache.match(event.request, {ignoreSearch:true}) || fetch(event.request);
  })());
});
