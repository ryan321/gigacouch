"use client";
import Link from "next/link";
import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

export type SavedGame = { id:string; slug:string; title:string; src:string; controls:string; playersMax:number; gamepad:boolean; keyboard:boolean; mouse:boolean };
export type Download = SavedGame & {status:"queued" | "downloading" | "checking" | "ready" | "paused" | "error" | "removing"; loaded:number; total:number; downloadedAt?:number; message?:string};
type Snapshot = {entries:Download[]; error:string; online:boolean};
type Manager = {subscribe:(fn:()=>void)=>()=>void; getSnapshot:()=>Snapshot; download:(game:SavedGame)=>Promise<void>; pause:(src:string)=>void; remove:(src:string)=>void; check:(src:string)=>void; forget:(src:string)=>void};
declare global { interface Window { createCouchDownloads?:(origin:string)=>Manager; couchDownloads?:Manager } }
const Context = createContext<{manager:Manager|null; state:Snapshot}>({manager:null,state:{entries:[],error:"",online:true}});
export const useDownloads = () => useContext(Context);
export const percent = (entry:Download) => entry.status === "ready" ? 100 : entry.total ? Math.min(99,Math.floor(entry.loaded/entry.total*100)) : 0;
export const size = (bytes:number) => bytes < 1024*1024 ? `${Math.ceil(bytes/1024)} KB` : `${(bytes/1024/1024).toFixed(1)} MB`;
export const statusText = (item:Download) => ({queued:"Queued",downloading:`Downloading · ${percent(item)}%`,checking:"Checking saved files…",ready:"Ready to play offline",paused:"Paused",error:"Needs attention",removing:"Removing…"})[item.status];

export function DownloadProvider({children,gamesOrigin}:{children:ReactNode;gamesOrigin:string}) {
  const [manager,setManager] = useState<Manager|null>(null);
  const [state,setState] = useState<Snapshot>({entries:[],error:"",online:true});
  const [open,setOpen] = useState(false);
  useEffect(() => {
    let cancelled=false, unsubscribe:(()=>void)|undefined;
    const loaded=() => {
      if(cancelled || !window.createCouchDownloads) return;
      window.couchDownloads ??= window.createCouchDownloads(gamesOrigin);
      const current=window.couchDownloads;
      setManager(current); setState(current.getSnapshot());
      unsubscribe=current.subscribe(()=>setState(current.getSnapshot()));
    };
    if(window.createCouchDownloads) loaded();
    else {
      const script=document.createElement("script"); script.src="/download/manager.js"; script.onload=loaded;
      script.onerror=()=>setState(previous=>({...previous,error:"Downloads could not start. Reload to retry."}));
      document.head.append(script);
    }
    return()=>{cancelled=true;unsubscribe?.()};
  },[gamesOrigin]);
  const running=state.entries.find(item=>item.status==="downloading");
  const queued=state.entries.filter(item=>["queued","downloading"].includes(item.status)).length;
  return <Context.Provider value={{manager,state}}>
    {children}
    <aside className="download-corner" aria-label="Downloads">
      <button className="button secondary" aria-expanded={open} aria-controls="download-panel" onClick={()=>setOpen(!open)}>
        ↓ {running ? `${percent(running)}%` : "Downloads"}{queued>1 ? ` · ${queued}` : ""}
      </button>
      {open && <section className="download-panel stack" id="download-panel" onKeyDown={event=>{if(event.key==="Escape")setOpen(false)}}>
        <h2>Your downloads</h2>
        <p className="muted">Keep browsing while games download. Closing this tab pauses unfinished downloads.</p>
        {state.error && <p role="alert">{state.error}</p>}
        {!state.entries.length && <p>No downloads yet. Open a game and choose Download.</p>}
        {state.entries.map(item=><div className="download-summary" key={item.src}>
          <strong>{item.title}</strong><span className="muted">{statusText(item)}</span>
          {item.status==="downloading" && <progress aria-label={`${item.title} download`} max={100} value={percent(item)}/>}
        </div>)}
        {state.online ? <Link className="button" href="/downloads" onClick={()=>setOpen(false)}>Open downloaded games</Link> : <a className="button" href="/offline.html">Open offline library</a>}
      </section>}
    </aside>
  </Context.Provider>;
}

export function DownloadLibrary() {
  const {manager,state}=useDownloads();
  const bytes=state.entries.reduce((sum,item)=>sum+item.loaded,0);
  return <div className="page library-page stack">
    <h1>Downloaded games</h1>
    <p className="lede">Your games, saved on this device. Download while you browse and play when you’re ready.</p>
    <p className="muted">{size(bytes)} saved · {state.entries.length} saved {state.entries.length===1?"version":"versions"}. Browsers may clear saved files when space runs low.</p>
    <a href="/offline.html" className="button secondary">Open offline library</a>
    {state.error && <p role="alert">{state.error}</p>}
    {!state.entries.length && <div className="download-card stack"><h2>Your library starts here</h2><p>Choose a game, then select Download on its play screen.</p><Link className="button" href="/games">Browse games</Link></div>}
    <div className="download-grid">{state.entries.map(item=><article className="download-card stack" key={item.src}>
      <h2>{item.title}</h2><p className="muted">{item.controls}</p>
      <p>{statusText(item)}</p>
      {item.downloadedAt && <p className="muted">Saved {new Date(item.downloadedAt).toLocaleString()}</p>}
      {["queued","downloading"].includes(item.status) && <progress aria-label={`${item.title} download`} max={100} value={percent(item)}/>}
      <p className="muted">{size(item.loaded)}{item.total ? ` / ${size(item.total)}` : ""}</p>
      {item.message && <p role={item.status==="error"?"alert":undefined}>{item.message}</p>}
      {state.entries.some(other=>other.id===item.id && other.src!==item.src) && <p className="muted">Multiple versions saved. Remove the older one when you no longer need it.</p>}
      <div className="download-actions">
        {item.status==="ready" && <Link className="button" href={`/saved/${new URL(item.src).pathname.split("/")[2]}`}>Play saved game</Link>}
        {["paused","error"].includes(item.status) && <button className="button" onClick={()=>manager?.download(item)}>Resume download</button>}
        {["downloading","queued"].includes(item.status) && <button className="button secondary" onClick={()=>manager?.pause(item.src)}>Pause</button>}
        {!["removing","checking"].includes(item.status) && <button className="button danger" onClick={()=>manager?.remove(item.src)}>Remove</button>}
        {item.status==="error" && <button className="button secondary" onClick={()=>manager?.forget(item.src)}>Forget listing</button>}
        <Link href={`/games/${item.slug}`} className="button secondary">Game page</Link>
      </div>
    </article>)}</div>
    <p className="muted">Touch controls and offline support depend on the game. Games that use online services still need a connection. New versions download separately, so your saved version remains playable.</p>
  </div>;
}
