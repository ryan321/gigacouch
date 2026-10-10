(async () => {
  const parentOrigin = document.querySelector('meta[name="parent-origin"]').content;
  const send = data => parent.postMessage({type:'gigacouch-download', ...data}, parentOrigin);
  let loaded = 0;
  try {
    if (!('serviceWorker' in navigator) || !('caches' in window)) throw Error('storage');
    const base = new URL('./', location.href);
    await navigator.serviceWorker.register(new URL('index.html?couch=worker', base), {scope:base.pathname, updateViaCache:'none'});
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller) await new Promise(resolve => navigator.serviceWorker.addEventListener('controllerchange', resolve, {once:true}));
    const response = await fetch('index.html?couch=manifest', {cache:'no-store'});
    if (!response.ok) throw Error('manifest');
    const {files, bytes:total} = await response.json();
    const cache = await caches.open('gigacouch-build-v1:' + base.pathname);
    let completed = 0;
    const report = current => send({status:'downloading', loaded:Math.min(total,loaded + current), total, completed, files:files.length});
    report(0);
    // Two downloads at a time bounds memory and avoids flooding a phone's connection.
    let next = 0;
    const partial = [0,0];
    await Promise.all(partial.map(async (_, slot) => {
      while (next < files.length) {
        const file = files[next++];
        const url = new URL(file.path.split('/').map(encodeURIComponent).join('/'), base);
        if (!await cache.match(url.href)) {
          const result = await fetch(url, {signal:AbortSignal.timeout(180000)});
          if (!result.ok) throw Error('download');
          const reader = result.clone().body.getReader();
          // Consume both branches together: never buffer a whole game in JavaScript.
          await Promise.all([cache.put(url.href, result), (async () => {
            for (;;) {
              const {done,value} = await reader.read();
              if (done) break;
              partial[slot] = Math.min(file.bytes, partial[slot] + value.byteLength);
              report(partial[0] + partial[1]);
            }
          })()]);
        }
        loaded += file.bytes;
        completed++;
        partial[slot] = 0;
        report(partial[0] + partial[1]);
      }
    }));
    send({status:'ready', loaded:total, total, completed, files:files.length});
  } catch (error) {
    send({status:'error', message:error.name === 'QuotaExceededError'
      ? 'There isn’t enough browser storage for this game. Free some space, then retry.'
      : 'The download couldn’t finish. Check your connection and browser storage, then retry.'});
  }
})();
