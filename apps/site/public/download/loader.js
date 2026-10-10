(async () => {
  const parentOrigin = document.querySelector('meta[name="parent-origin"]').content;
  const send = data => parent.postMessage({type:'gigacouch-download', ...data}, parentOrigin);
  const heartbeat = setInterval(() => send({status:'heartbeat'}),5000);
  const action = new URL(location.href).searchParams.get('action') || 'download';
  let loaded = 0, lastReport = 0, phase = 'starting browser storage';
  const retryable = error => !['QuotaExceededError','SecurityError','NotAllowedError','AbortError'].includes(error.name)
    && (!error.status || error.status === 408 || error.status === 429 || error.status >= 500);
  const responseOK = response => {
    if(!response.ok) throw Object.assign(Error('Server returned HTTP '+response.status),{status:response.status});
    return response;
  };
  const retry = async task => {
    for(let attempt=0;;attempt++) {
      try { return await task(); }
      catch(error) {
        if(attempt === 2 || !retryable(error)) throw error;
        await new Promise(resolve=>setTimeout(resolve,1000 * (attempt+1)));
      }
    }
  };
  const fetchSmall = url => retry(async()=>responseOK(await fetch(url,{cache:'no-store',signal:AbortSignal.timeout(30000)})));
  try {
    if (!('serviceWorker' in navigator) || !('caches' in window)) throw new DOMException('Browser storage is unavailable','SecurityError');
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
    phase = 'reading the download list';
    const manifestURL = new URL('index.html?couch=manifest',base).href;
    const saved = await cache.match(manifestURL);
    const response = action === 'download' ? await fetchSmall(manifestURL) : saved || await fetchSmall(manifestURL);
    const {files, bytes:total} = await response.clone().json();
    let completed = 0;
    const report = current => {
      if(performance.now()-lastReport < 150) return;
      lastReport = performance.now();
      send({status:'downloading',loaded:Math.min(total,loaded + current),total});
    };
    if(action === 'download') {
      send({status:'downloading',loaded:0,total});
      phase = 'saving the offline launcher';
      await cache.put(manifestURL,response);
      // Save the helper itself: offline checks, resuming and removal must work after reopening.
      for (const mode of ['download','loader']) {
        const url = new URL('index.html?couch='+mode,base);
        const helper = await fetchSmall(url);
        await cache.put(url.href,helper);
      }
    }
    phase = 'downloading game files';
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
            try {
              await retry(async()=>{
                if(abort.signal.aborted) throw new DOMException('Download stopped','AbortError');
                partial[slot] = 0;
                const transfer = new AbortController();
                let stalled;
                // A large file can take minutes on mobile. Only time out if bytes stop arriving.
                const activity = () => { clearTimeout(stalled); stalled=setTimeout(()=>transfer.abort(new DOMException('No download progress','TimeoutError')),60000); };
                activity();
                try {
                  const result = responseOK(await fetch(url,{signal:AbortSignal.any([abort.signal,transfer.signal])}));
                  const reader = result.clone().body.getReader();
                  const tasks = [cache.put(url.href,result),(async () => {
                    for(;;) {
                      const {done,value} = await reader.read(); if(done) break;
                      activity();
                      partial[slot] = Math.min(file.bytes,partial[slot]+value.byteLength);
                      report(partial[0]+partial[1]);
                    }
                    clearTimeout(stalled);
                  })()];
                  try { await Promise.all(tasks); }
                  catch(error) { transfer.abort(); await Promise.allSettled(tasks); throw error; }
                } finally { clearTimeout(stalled); }
              });
            } catch(error) { error.file = file.path; throw error; }
          }
          loaded += file.bytes; completed++; partial[slot] = 0;
          if(action === 'download') report(partial[0]+partial[1]);
        }
      }));
    } catch(error) { abort.abort(); throw error; }
    send({status:completed === files.length ? 'ready' : 'missing',loaded,total});
  } catch(error) {
    const detail = error.file ? 'downloading '+error.file : phase;
    console.error('Giga Couch download failed:',detail,error.name,error.message);
    let message;
    if(error.name === 'QuotaExceededError') message = 'There isn’t enough browser storage. Free some space or remove a downloaded game, then retry.';
    else if(['SecurityError','NotAllowedError'].includes(error.name)) message = 'Browser privacy settings blocked game storage. Allow site data for Giga Couch and its games, then retry.';
    else message = `Could not finish ${detail}${error.status ? ' (HTTP '+error.status+')' : ' ('+error.name+')'}. Completed files are saved. Retry to continue.`;
    send({status:'error',message});
  } finally { clearInterval(heartbeat); }
})();
