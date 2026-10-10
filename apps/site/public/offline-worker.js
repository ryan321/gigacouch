// Cache only the public offline shell. Account pages, API responses and game code never enter this cache.
// Shell revision 2: download sizes and byte progress.
const CACHE = 'gigacouch-offline-shell-v1';
const FILES = ['/offline.html','/offline.css','/offline.js','/download/manager.js'];
self.addEventListener('install',event=>event.waitUntil((async()=>{
  const cache=await caches.open(CACHE);
  await cache.addAll(FILES.map(url=>new Request(url,{cache:'reload'})));
  await self.skipWaiting();
})()));
self.addEventListener('activate',event=>event.waitUntil(self.clients.claim()));
self.addEventListener('fetch',event=>{
  const url=new URL(event.request.url);
  if(url.origin!==location.origin || event.request.method!=='GET' || url.pathname.startsWith('/api/') || url.pathname.startsWith('/g/'))return;
  if(FILES.includes(url.pathname)) {
    event.respondWith((async()=>await (await caches.open(CACHE)).match(url.pathname) || fetch(event.request))()); return;
  }
  if(event.request.mode==='navigate')event.respondWith((async()=>{
    try { const response=await fetch(event.request,{signal:AbortSignal.timeout(4000)}); if(response.status>=500)throw Error('Server unavailable'); return response; }
    catch { return await (await caches.open(CACHE)).match('/offline.html') || Response.error(); }
  })());
});
