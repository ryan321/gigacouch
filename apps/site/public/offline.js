(() => {
  const $ = id => document.getElementById(id);
  const size = bytes => bytes < 1024*1024 ? Math.ceil(bytes/1024)+' KB' : (bytes/1024/1024).toFixed(1)+' MB';
  let stored;
  try { stored=JSON.parse(localStorage.getItem('gigacouch-downloads-v2')||'{}'); } catch { stored={}; }
  let origin;
  try { const u=new URL(stored.gamesOrigin); if(u.protocol==='https:' || u.protocol==='http:' && ['127.0.0.1','localhost'].includes(u.hostname)) origin=u.origin; } catch {}
  if(!origin) { $('notice').textContent='No games saved yet. Connect to the internet, browse a game, and choose Download.'; return; }
  const manager=window.couchDownloads=window.createCouchDownloads(origin);
  let playing=null,frame=null;
  const savedBuild=location.pathname.match(/^\/saved\/([a-f0-9]{32})$/)?.[1];
  let wanted=new URL(location.href).searchParams.get('play') || (savedBuild ? origin+'/g/'+savedBuild+'/index.html' : null);
  const node=(tag,text,cls)=>{const el=document.createElement(tag);el.textContent=text;if(cls)el.className=cls;return el};
  const button=(label,fn,cls)=>{const el=node('button',label,cls);el.onclick=fn;return el};
  const menuButtons=[$('resume'),$('restart'),$('fullscreen'),$('leave')];
  const focus=()=>{frame?.focus();frame?.contentWindow?.focus()};
  const closeMenu=()=>{$('game-menu').hidden=true;focus()};
  const openMenu=()=>{$('game-menu').hidden=false;menuButtons[0].focus()};
  function launch(item) {
    if(item.status!=='ready')return;
    playing=item;
    $('library').hidden=true;$('player').hidden=false;$('title').textContent=item.title;$('menu-title').textContent=item.title;
    frame=document.createElement('iframe');frame.src=item.src;frame.title=item.title;
    frame.allow='gamepad; fullscreen; autoplay';frame.sandbox='allow-scripts allow-same-origin allow-pointer-lock allow-forms allow-modals allow-popups allow-downloads';
    frame.onload=focus;$('game-frame').replaceChildren(frame);closeMenu();
    $('player').requestFullscreen?.().catch(()=>{});
  }
  function render() {
    const {entries,error,online}=manager.getSnapshot();
    $('connection').textContent=online?'Downloaded games · on this device':'Offline · saved games';
    $('notice').textContent=error || (!entries.length?'No downloaded games yet. Connect and browse games to get started.':'');
    $('storage').textContent=size(entries.reduce((n,item)=>n+item.loaded,0))+' saved. Remove games to free space.';
    $('games').replaceChildren(...entries.map(item=>{
      const card=node('article','','card');card.append(node('h2',item.title),node('p',item.controls,'muted'));
      const status={ready:'Ready to play offline',checking:'Checking saved files…',paused:'Paused',queued:'Queued',removing:'Removing…',error:'Needs attention',downloading:'Downloading · '+(item.total?Math.min(99,Math.floor(item.loaded/item.total*100)):0)+'%'}[item.status];
      card.append(node('p',status));
      if(item.downloadedAt)card.append(node('p','Saved '+new Date(item.downloadedAt).toLocaleString(),'muted'));
      if(item.status==='downloading'){const bar=document.createElement('progress');bar.max=item.total||1;bar.value=item.loaded;bar.setAttribute('aria-label',item.title+' download');card.append(bar)}
      card.append(node('p',size(item.loaded)+(item.total?' / '+size(item.total):''),'muted'));
      if(item.message)card.append(node('p',item.message));
      const actions=node('div','','actions');
      if(item.status==='ready')actions.append(button('Play',()=>launch(item)));
      if(['paused','error'].includes(item.status))actions.append(button('Resume download',()=>manager.download(item)));
      if(['downloading','queued'].includes(item.status))actions.append(button('Pause',()=>manager.pause(item.src),'secondary'));
      if(!['removing','checking'].includes(item.status))actions.append(button('Remove',()=>manager.remove(item.src),'danger'));
      if(item.status==='error')actions.append(button('Forget listing',()=>manager.forget(item.src),'secondary'));
      card.append(actions);return card;
    }));
    if(wanted){const item=entries.find(item=>item.src===wanted&&item.status==='ready');if(item){wanted=null;launch(item)}}
  }
  manager.subscribe(render);render();
  $('menu').onclick=openMenu;$('resume').onclick=closeMenu;
  $('restart').onclick=()=>{if(playing)launch(playing)};
  $('fullscreen').onclick=()=>{if(document.fullscreenElement)document.exitFullscreen().catch(()=>{});else $('player').requestFullscreen?.().catch(()=>{});closeMenu()};
  $('leave').onclick=()=>{if(document.fullscreenElement)document.exitFullscreen().catch(()=>{});frame?.remove();frame=null;playing=null;$('player').hidden=true;$('library').hidden=false;render()};
  $('game-menu').onkeydown=event=>{
    const index=menuButtons.indexOf(document.activeElement);
    if(event.key==='Escape')closeMenu();
    else if(event.key==='ArrowDown')menuButtons[(index+1)%4].focus();
    else if(event.key==='ArrowUp')menuButtons[(index+3)%4].focus();else return;
    event.preventDefault();
  };
  // Same center-button chord as the online player. Held buttons don't become a new press on entry.
  const previous=new Map();let holdSince=null,used=false;
  function pads(){
    const now=performance.now();let chord=false;
    for(const pad of navigator.getGamepads?.()||[]){
      if(!pad)continue;
      const down=pad.buttons.map(b=>b.pressed);down[12] ||= pad.axes[1]<-.6;down[13] ||= pad.axes[1]>.6;
      const before=previous.get(pad.index)||down;const pressed=n=>down[n]&&!before[n];previous.set(pad.index,down);
      if(!playing)continue;
      chord ||= down[8]&&down[9];
      if(pressed(16)){$('game-menu').hidden?openMenu():closeMenu()}
      if(!$('game-menu').hidden){
        const index=Math.max(0,menuButtons.indexOf(document.activeElement));
        if(pressed(12))menuButtons[(index+3)%4].focus();if(pressed(13))menuButtons[(index+1)%4].focus();
        if(pressed(0))menuButtons[index].click();if(pressed(1))closeMenu();
      }
    }
    if(chord){holdSince??=now;if(!used&&now-holdSince>=800){used=true;$('game-menu').hidden?openMenu():closeMenu()}}
    else {holdSince=null;used=false}
    requestAnimationFrame(pads);
  }
  requestAnimationFrame(pads);
})();
