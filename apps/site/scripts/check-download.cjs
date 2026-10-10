// Run with an already-installed Electron; never installs a browser.
const {app,BrowserWindow}=require('electron');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const timer=setTimeout(()=>{console.error('Download check timed out');app.exit(1)},300000);
app.whenReady().then(async()=>{
  const win=new BrowserWindow({width:1200,height:800,show:false,focusable:false,webPreferences:{backgroundThrottling:false,partition:'download-check-'+Date.now()}});
  win.setOpacity(0);win.setIgnoreMouseEvents(true);win.showInactive();
  const js=code=>win.webContents.executeJavaScript(code);
  const wait=async(code)=>{for(let i=0;i<1000;i++){if(await js(code))return;await sleep(200)}throw Error('Timed out: '+code)};
  const site=process.env.SITE||'http://localhost:3000';
  const slug=process.env.GAME_SLUG||'star-scramble';
  let failManifest=Boolean(process.env.TEST_RETRY);
  win.webContents.session.webRequest.onBeforeRequest((details,callback)=>{
    if(failManifest && details.url.includes('couch=manifest')) { failManifest=false; callback({cancel:true}); }
    else callback({});
  });
  await win.loadURL(site+'/play/'+slug);
  await wait(`!![...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Download')`);
  assert.equal(await js(`document.querySelectorAll('iframe').length`),0,'No downloads or game before clicking');
  await js(`[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Download').click()`);
  await wait(`!!document.querySelector('progress')`);
  assert.equal(await js(`!![...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Downloading…'&&b.disabled)`),true);
  if(process.env.TEST_RETRY){
    await wait(`!![...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Retry download')`);
    await js(`[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Retry download').click()`);
  }
  await wait(`!![...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Play')`);
  assert.equal(await js(`document.querySelectorAll('iframe').length`),0,'Game still does not run when download completes');
  fs.writeFileSync('/tmp/gigacouch-download-ready.png',(await win.webContents.capturePage()).toPNG());
  // Cache must be sufficient even with the network unavailable at launch.
  win.webContents.session.enableNetworkEmulation({offline:true});
  await js(`[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Play').click()`);
  let frame;
  for(let i=0;i<600;i++){
    frame=win.webContents.mainFrame.frames.find(f=>f.url.includes('/g/'));
    if(frame && await frame.executeJavaScript(`document.readyState==='complete' && !!document.querySelector('canvas')`)) break;
    await sleep(200);
  }
  assert(frame,'Game iframe exists');
  assert.equal(await frame.executeJavaScript(`!!document.querySelector('canvas')`),true,'Game launches from downloaded files while offline');
  if(slug==='spooky-game-browser-version'){
    for(let i=0;i<600;i++){if(await frame.executeJavaScript('!!window.__game'))break;await sleep(200)}
    assert.equal(await frame.executeJavaScript('!!window.__game'),true,'Spooky runtime starts offline');
  }
  win.webContents.session.disableNetworkEmulation();
  console.log('PASS: Download → progress → Play; no early game execution; cached launch while offline.');
  clearTimeout(timer);app.quit();
}).catch(error=>{console.error(error);app.exit(1)});
