"use client";
import Link from "next/link";
import { useDownloads } from "./Downloads";
import { Player } from "./Player";
export function SavedPlayer({build}:{build:string}) {
  const {manager,state}=useDownloads();
  const item=state.entries.find(item=>new URL(item.src).pathname===`/g/${build}/index.html`);
  if(!manager)return <div className="page library-page">Opening your downloads…</div>;
  if(!item)return <div className="page library-page stack"><h1>Game not saved</h1><Link href="/downloads">Open downloaded games</Link></div>;
  return <Player game={item} src={item.src} controls={item.controls}/>;
}
