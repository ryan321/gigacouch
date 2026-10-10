// Installed Electron only. Exercises real origin-separated caches and a fresh window offline.
const {app,BrowserWindow}=require('electron');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {spawn,execFileSync}=require('node:child_process');
const path=require('node:path');
let server;
const startServer=async()=>{
 fs.cpSync('public','.next/standalone/public',{recursive:true});
 if(!fs.existsSync('.next/standalone/.next/static'))fs.symlinkSync(path.resolve('.next/static'),'.next/standalone/.next/static','dir');
 server=spawn(process.env.NODE_BINARY||'/Users/ryan/.nvm/versions/node/v22.19.0/bin/node',['.next/standalone/server.js'],{cwd:process.cwd(),env:{...process.env,PORT:'3101',HOSTNAME:'0.0.0.0',DATA_DIR:path.resolve('data'),SITE_ORIGIN:'http://localhost:3101',GAMES_ORIGIN:'http://127.0.0.1:3101'},stdio:['ignore',fs.openSync('/tmp/gigacouch-library-test-server.log','a'),fs.openSync('/tmp/gigacouch-library-test-server.log','a')]});
 for(let i=0;i<100;i++){try{if((await fetch('http://localhost:3101')).ok)return}catch{}await new Promise(r=>setTimeout(r,100))}throw Error('Test server failed');
};
const stopServer=()=>new Promise(resolve=>{if(!server)return resolve();server.once('exit',resolve);server.kill('SIGTERM');});
app.setPath('userData','/Volumes/External/projects/gigacouch-download-check');
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const deadline=setTimeout(()=>{console.error('Timed out');server?.kill('SIGTERM');app.exit(1)},300000);
app.whenReady().then(async()=>{
  if(process.env.ISOLATED_TEST)await startServer();
  if(process.env.WORKER_UPGRADE)for(const name of ['worker','loader'])fs.writeFileSync('.next/standalone/public/download/'+name+'.js',execFileSync('git',['show','972d406:apps/site/public/download/'+name+'.js']));
  const site=process.env.ISOLATED_TEST?'http://localhost:3101':process.env.SITE||'http://localhost:3000';
  const slug=process.env.GAME_SLUG||'star-scramble';
  const partition='persist:library-check-'+Date.now();
  const make=()=>{const w=new BrowserWindow({width:1200,height:850,useContentSize:true,show:false,focusable:false,webPreferences:{partition,backgroundThrottling:false}});w.webContents.on('console-message',e=>{if(e.level==='error')console.error('PAGE',e.message)});w.setOpacity(0);w.setIgnoreMouseEvents(true);w.showInactive();return w};
  let win=make(),session=win.webContents.session;
  let delay=true, delayed=0, injectFailures=false, failedRequests=0, failedURL;
  session.webRequest.onBeforeRequest((details,cb)=>{
    if(injectFailures && details.url.includes('/g/') && !details.url.includes('couch=')) {
      failedURL ??= details.url;
      if(details.url===failedURL && failedRequests<2){failedRequests++;cb({cancel:true});return;}
    }
    if(delay&&details.url.includes('/g/')&&!details.url.includes('couch=')&&delayed++<2)setTimeout(()=>cb({}),1500);else cb({});
  });
  let js=code=>win.webContents.executeJavaScript(code);
  if(process.env.DEBUG_DOWNLOADS)setInterval(()=>js(`({url:location.pathname,entries:window.couchDownloads?.getSnapshot().entries.map(e=>({status:e.status,message:e.message}))})`).then(v=>console.log('PROGRESS',v)).catch(()=>{}),5000);
  const reload=()=>new Promise(resolve=>{win.webContents.once('did-finish-load',resolve);win.reload()});
  const wait=async(code,iterations=600)=>{for(let i=0;i<iterations;i++){if(await js(code))return;await sleep(100)}console.error('STATE',await js(`({url:location.href,text:document.body.innerText,state:window.couchDownloads?.getSnapshot(),frames:[...document.querySelectorAll('iframe')].map(f=>f.src)})`));throw Error('Wait failed: '+code)};
  await win.loadURL(site+'/play/'+slug);
  await wait(`!!window.couchDownloads`);
  assert.match(await js('document.body.innerText'),/Download size: [\d.]+ MB/,'Size shown before download');
  assert.match(await js('document.body.innerText'),/browser’s storage on this device/,'Storage location shown');
  await js(`[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Download').click()`);
  await wait(`window.couchDownloads.getSnapshot().entries.some(e=>e.status==='downloading')`);
  await js(`window.couchDownloads.pause(window.couchDownloads.getSnapshot().entries[0].src)`);
  await reload();
  await wait(`window.couchDownloads?.getSnapshot().entries[0]?.status==='paused'`);
  injectFailures=!!process.env.FAIL_DOWNLOADS;
  await js(`[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Resume download').click()`);
  await wait(`window.couchDownloads.getSnapshot().entries[0].status==='downloading'`);
  let queuedSrc;
  if(process.env.TEST_QUEUE){
    const html=await (await fetch(site+'/play/spooky-game-browser-version')).text();
    const build=html.match(/\/g\/([a-f0-9]{32})\/index\.html/)[1];
    const first=await js('window.couchDownloads.getSnapshot().entries[0]');
    queuedSrc=new URL('/g/'+build+'/index.html',first.src).href;
    await js(`window.couchDownloads.download({...window.couchDownloads.getSnapshot().entries[0],src:${JSON.stringify(queuedSrc)},title:'Second queued game'})`);
    assert.equal(await js(`window.couchDownloads.getSnapshot().entries.find(e=>e.src===${JSON.stringify(queuedSrc)}).status`),'queued','Second download waits its turn');
  }
  await js(`window.__originalDownloads=window.couchDownloads;document.querySelector('a[href="/games"]').click()`);
  await wait(`location.pathname==='/games'`);
  assert.equal(await js('window.__originalDownloads===window.couchDownloads'),true,'Client navigation keeps the manager');
  assert.equal(await js(`!!document.querySelector('iframe[title="Game download"]')`),true,'Download survives navigation');
  await wait(`window.couchDownloads.getSnapshot().entries.some(e=>e.status==='ready')`,1600);
  if(queuedSrc){
    await wait(`window.couchDownloads.getSnapshot().entries.find(e=>e.src===${JSON.stringify(queuedSrc)}).status==='downloading'`);
    await js(`window.couchDownloads.pause(${JSON.stringify(queuedSrc)})`);
    await js(`window.couchDownloads.remove(${JSON.stringify(queuedSrc)})`);
    await wait(`window.couchDownloads.getSnapshot().entries.length===1`);
  }
  delay=false;
  if(process.env.FAIL_DOWNLOADS)assert.equal(failedRequests,2,'Two failed asset transfers recovered automatically');
  await js(`document.querySelector('.download-corner > button').click()`);
  await js(`document.querySelector('a[href="/downloads"]').click()`);
  await wait(`location.pathname==='/downloads'&&!!document.querySelector('.download-card')`);
  assert.match(await js('document.body.innerText'),/Download size: [\d.]+ MB · [\d.]+ MB downloaded/,'Library shows total and downloaded bytes');
  for(const [width,height,name] of [[390,844,'phone'],[844,390,'landscape'],[768,1024,'tablet']]){
    win.setContentSize(width,height);await sleep(250);
    assert.equal(await js('document.documentElement.scrollWidth<=innerWidth'),true,'No overflow '+name);
    fs.writeFileSync('/tmp/gigacouch-library-'+name+'.png',(await win.webContents.capturePage()).toPNG());
  }
  if(process.env.WORKER_UPGRADE){
    for(const name of ['worker','loader'])fs.copyFileSync('public/download/'+name+'.js','.next/standalone/public/download/'+name+'.js');
    await js(`window.couchDownloads.download(window.couchDownloads.getSnapshot().entries[0])`);
    await wait(`window.couchDownloads?.getSnapshot().entries[0]?.status==='ready'`);
  }
  const entry=await js('window.couchDownloads.getSnapshot().entries[0]');
  if(process.env.ONLINE_ONLY){console.log('PASS: live download across navigation and responsive library.');clearTimeout(deadline);app.quit();return;}
  // Close the entire browsing window and reopen with the network already disabled.
  if(process.env.ISOLATED_TEST)await stopServer();
  else session.enableNetworkEmulation({offline:true});
  const old=win;win=make();old.destroy();js=code=>win.webContents.executeJavaScript(code);
  await win.loadURL(site+'/downloads');
  await wait(`!!document.querySelector('#games button')&&document.querySelector('#games button').textContent==='Play'`);
  assert.equal(await js(`document.querySelector('h1').textContent`),'Downloaded games');
  const shellFiles=await js(`(async()=>{const cache=await caches.open('gigacouch-offline-shell-v1');return (await cache.keys()).map(r=>new URL(r.url).pathname).sort()})()`);
  assert.deepEqual(shellFiles,['/download/manager.js','/offline.css','/offline.html','/offline.js'],'Only public shell files are cached on the account origin');
  await js(`document.querySelector('#games button').click()`);
  let game;
  for(let i=0;i<900;i++){
    game=win.webContents.mainFrame.frames.find(f=>f.url===entry.src);
    if(game&&await game.executeJavaScript(slug==='spooky-game-browser-version'?'!!window.__game':`!!document.querySelector('canvas') && document.readyState==='complete'`))break;
    await sleep(100);
  }
  assert(game,'Offline game frame');
  assert.equal(await game.executeJavaScript(slug==='spooky-game-browser-version'?'!!window.__game':`!!document.querySelector('canvas')`),true,'Game starts after a cold offline navigation');
  // Simulate eviction of one asset. The next library check must not claim the build is ready.
  await game.executeJavaScript(`(async()=>{const c=await caches.open('gigacouch-build-v1:'+new URL('./',location.href).pathname);const keys=await c.keys();const asset=keys.find(k=>!k.url.includes('couch=')&&!k.url.endsWith('index.html'));if(asset)await c.delete(asset)})()`);
  await js(`document.getElementById('menu').click();document.getElementById('leave').click()`);
  await reload();
  await wait(`!!document.querySelector('#games button')&&document.querySelector('#games button').textContent==='Resume download'`);
  if(process.env.ISOLATED_TEST)await startServer();
  else session.disableNetworkEmulation();
  await js(`document.querySelector('#games button').click()`);
  await wait(`document.querySelector('#games button')?.textContent==='Play'`,1600);
  await js(`[...document.querySelectorAll('#games button')].find(b=>b.textContent==='Remove').click()`);
  await wait(`window.couchDownloads.getSnapshot().entries.length===0`);
  console.log('PASS: pause/reload/resume; download across navigation; phone/landscape/tablet layouts; cold offline library and play; eviction detected; resume; removal.');
  clearTimeout(deadline);if(server)await stopServer();app.quit();
}).catch(error=>{console.error(error);server?.kill('SIGTERM');app.exit(1)});
