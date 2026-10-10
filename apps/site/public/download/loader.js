(async () => {
  const parentOrigin = document.querySelector('meta[name="parent-origin"]').content;
  const send = data => parent.postMessage({type:'gigacouch-download', ...data}, parentOrigin);
  const heartbeat = setInterval(() => send({status:'heartbeat'}),5000);
  const action = new URL(location.href).searchParams.get('action') || 'download';
  let loaded = 0, lastReport = 0;
  try {
    if (!('serviceWorker' in navigator) || !('caches' in window)) throw Error('storage');
    const base = new URL('./', location.href);
    let registration = await navigator.serviceWorker.getRegistration(base.href);
    if (!registration || navigator.onLine) {
      try {
        registration = await navigator.serviceWorker.register(new URL('index.html?couch=worker', base), {scope:base.pathname, updateViaCache:'none'});
        if(registration.active) await registration.update();
      }
      catch(error) { if(!registration) throw error; }
    }
    // An older build worker may still be active during this upgrade. Wait until the new worker
    // can serve the offline helper before declaring an already-cached build ready.
    const pending = registration.installing || registration.waiting;
    if(pending && pending.state !== 'activated') await new Promise((resolve,reject) => {
      const changed = () => {
        if(pending.state === 'activated' || pending.state === 'redundant') {
          pending.removeEventListener('statechange',changed);
          pending.state === 'activated' ? resolve() : reject(Error('Worker update failed'));
        }
      };
      pending.addEventListener('statechange',changed); changed();
    });
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller) await new Promise(resolve => navigator.serviceWorker.addEventListener('controllerchange', resolve, {once:true}));
    const cacheName = 'gigacouch-build-v1:' + base.pathname;
    if (action === 'remove') {
      await caches.delete(cacheName);
      await registration.unregister();
      send({status:'removed'}); return;
    }
    const cache = await caches.open(cacheName);
    const manifestURL = new URL('index.html?couch=manifest',base).href;
    const saved = await cache.match(manifestURL);
    const response = saved || await fetch(manifestURL,{cache:'no-store',signal:AbortSignal.timeout(30000)});
    if (!response.ok) throw Error('manifest');
    const {files, bytes:total} = await response.clone().json();
    let completed = 0;
    const report = current => {
      if(performance.now()-lastReport < 150) return;
      lastReport = performance.now();
      send({status:'downloading',loaded:Math.min(total,loaded + current),total});
    };
    if(action === 'download') {
      await cache.put(manifestURL,response);
      // Save the helper itself: offline checks, resuming and removal must work after reopening.
      for (const mode of ['download','loader']) {
        const url = new URL('index.html?couch='+mode,base);
        const helper = await fetch(url,{signal:AbortSignal.timeout(30000)});
        if(!helper.ok) throw Error('helper');
        await cache.put(url.href,helper);
      }
    }
    let next = 0;
    const partial = [0,0];
    const abort = new AbortController();
    try {
      await Promise.all(partial.map(async (_,slot) => {
        while(next < files.length && !abort.signal.aborted) {
          const file = files[next++];
          const url = new URL(file.path.split('/').map(encodeURIComponent).join('/'),base);
          if(!await cache.match(url.href)) {
            if(action === 'check') continue;
            const result = await fetch(url,{signal:AbortSignal.any([abort.signal,AbortSignal.timeout(180000)])});
            if(!result.ok) throw Error('download');
            const reader = result.clone().body.getReader();
            await Promise.all([cache.put(url.href,result),(async () => {
              for(;;) {
                const {done,value} = await reader.read(); if(done) break;
                partial[slot] = Math.min(file.bytes,partial[slot]+value.byteLength);
                report(partial[0]+partial[1]);
              }
            })()]);
          }
          loaded += file.bytes; completed++; partial[slot] = 0;
          if(action === 'download') report(partial[0]+partial[1]);
        }
      }));
    } catch(error) { abort.abort(); throw error; }
    send({status:completed === files.length ? 'ready' : 'missing',loaded,total});
  } catch(error) {
    send({status:'error',message:error.name === 'QuotaExceededError'
      ? 'There isn’t enough browser storage. Remove a downloaded game, then retry.'
      : 'The saved files could not be reached. Check your connection and browser storage, then retry.'});
  } finally { clearInterval(heartbeat); }
})();
