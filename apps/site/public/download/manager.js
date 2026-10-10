/* Shared by the online React shell and the standalone offline library. No game code runs here. */
(() => {
  const KEY = 'gigacouch-downloads-v2';
  window.createCouchDownloads = function (gamesOrigin) {
    const listeners = new Set();
    let entries = [], jobs = [], active = null, timer, saveTimer;
    let snapshot = {entries, error:'', online:navigator.onLine};
    const valid = item => {
      try { const u = new URL(item.src); return u.origin === gamesOrigin && /^\/g\/[a-f0-9]{32}\/index\.html$/.test(u.pathname) && !u.search && !u.hash && typeof item.title === 'string' && typeof item.slug === 'string'; } catch { return false; }
    };
    const persist = () => {
      clearTimeout(saveTimer);
      try { localStorage.setItem(KEY, JSON.stringify({gamesOrigin, entries})); }
      catch { snapshot = {...snapshot,error:'Browser storage is unavailable. Downloads cannot be kept after this page closes.'}; }
    };
    const emit = (immediate = false) => {
      snapshot = {...snapshot, entries:[...entries], online:navigator.onLine};
      if (immediate) persist(); else { clearTimeout(saveTimer); saveTimer = setTimeout(persist, 300); }
      listeners.forEach(fn => fn());
    };
    const update = (src, patch, immediate = false) => { entries = entries.map(item => item.src === src ? {...item,...patch} : item); emit(immediate); };
    try {
      const stored = JSON.parse(localStorage.getItem(KEY) || '{}');
      entries = (Array.isArray(stored.entries) ? stored.entries : []).filter(valid).map(item => ({...item,
        status:['ready','checking'].includes(item.status) ? 'checking' : 'paused', message:undefined}));
      jobs = entries.filter(item => item.status === 'checking').map(item => ({src:item.src, action:'check'}));
    } catch { /* A missing or damaged index starts an empty library. */ }
    let shell;
    const prepareShell = () => {
      if (!('serviceWorker' in navigator)) return Promise.resolve(null);
      shell ??= navigator.serviceWorker.register('/offline-worker.js', {scope:'/',updateViaCache:'none'})
        .then(() => navigator.serviceWorker.ready).catch(() => { shell = null; return null; });
      return shell;
    };
    void prepareShell();
    const finish = () => {
      clearTimeout(timer);
      active?.frame.remove();
      active = null;
      queueMicrotask(pump);
    };
    const failure = message => {
      if (!active) return;
      update(active.src, {status:'error', message}, true);
      finish();
    };
    const arm = () => { clearTimeout(timer); timer = setTimeout(() => failure('The saved files could not be reached. Reconnect and retry.'), 60000); };
    function pump() {
      if (active || !jobs.length) return;
      const job = jobs.shift();
      if (!entries.some(item => item.src === job.src)) return pump();
      const frame = document.createElement('iframe');
      frame.hidden = true;
      frame.title = 'Game download';
      frame.sandbox = 'allow-scripts allow-same-origin';
      active = {...job,frame};
      update(job.src, {status:job.action === 'download' ? 'downloading' : job.action === 'remove' ? 'removing' : 'checking',message:undefined}, true);
      frame.src = job.src + '?couch=download&action=' + job.action;
      frame.onerror = () => failure('The download could not connect. Reconnect and retry.');
      arm();
      document.body.append(frame);
    }
    window.addEventListener('message', event => {
      if (!active || event.source !== active.frame.contentWindow || event.origin !== gamesOrigin || event.data?.type !== 'gigacouch-download') return;
      const data = event.data;
      arm();
      if (data.status === 'heartbeat') return;
      if (data.status === 'error') return failure(String(data.message || 'Download failed. Please retry.'));
      if (data.status === 'removed') {
        entries = entries.filter(item => item.src !== active.src); emit(true); finish(); return;
      }
      if (!['ready','missing','downloading'].includes(data.status) || !Number.isFinite(data.loaded) || !Number.isFinite(data.total) || data.loaded < 0 || data.total < data.loaded) return;
      const src = active.src;
      const downloadedAt=data.status === 'ready' && active.action === 'download' ? Date.now() : entries.find(item=>item.src===src)?.downloadedAt;
      update(src,{downloadedAt,status:data.status === 'missing' ? 'paused' : data.status, loaded:data.loaded,total:data.total,
        message:data.status === 'missing' ? 'Some saved files are missing. Resume to download them again.' : undefined}, data.status !== 'downloading');
      if (data.status === 'ready' || data.status === 'missing') finish();
    });
    const enqueue = (src,action) => {
      jobs = jobs.filter(job => job.src !== src);
      if (active?.src === src) finish();
      jobs.push({src,action});
      update(src,{status:action === 'download' ? 'queued' : action === 'check' ? 'checking' : 'removing',message:undefined},true);
      pump();
    };
    const api = {
      subscribe(fn) { listeners.add(fn); return () => listeners.delete(fn); },
      getSnapshot() { return snapshot; },
      async download(item) {
        if (!valid(item)) return;
        if (!await prepareShell()) { snapshot = {...snapshot,error:'Offline storage could not start. Check your browser settings and retry.'}; emit(true); return; }
        snapshot = {...snapshot,error:''};
        const previous = entries.find(entry => entry.src === item.src);
        if (previous && ['queued','downloading','removing'].includes(previous.status)) return;
        if (!previous) entries = [...entries,{...item,status:'queued',loaded:0,total:Number.isFinite(item.downloadBytes) ? Math.max(0,item.downloadBytes) : 0}];
        enqueue(item.src,'download');
      },
      pause(src) {
        jobs = jobs.filter(job => job.src !== src);
        if (active?.src === src) finish();
        update(src,{status:'paused',message:'Paused. Completed files are saved.'},true);
      },
      remove(src) { enqueue(src,'remove'); },
      check(src) { if (!active && !jobs.some(job => job.src === src)) enqueue(src,'check'); },
      // Forget only the index when an entire games-origin cache has been evicted.
      forget(src) { jobs = jobs.filter(job => job.src !== src); if(active?.src === src) finish(); entries = entries.filter(item => item.src !== src); emit(true); },
    };
    const connectivity = () => emit(true);
    window.addEventListener('online',connectivity);
    window.addEventListener('offline',connectivity);
    window.addEventListener('pagehide',persist);
    emit(true);
    pump();
    return api;
  };
})();
