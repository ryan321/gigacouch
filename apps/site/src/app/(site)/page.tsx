import Link from "next/link";
import { GameTiles } from "@/components/GameTile";
import { PadPanel } from "@/components/PadPanel";
import { listGames } from "@/lib/games";

export default function Home() {
  const newest = listGames({ sort: "new", limit: 8 });
  const popular = listGames({ sort: "popular", limit: 4 }).filter((game) => game.plays > 0);
  return (
    <div className="page">
      <section className="hero">
        <div>
          <h1>Web games you play with a controller.</h1>
          <p className="lede">
            Upload a game you made for the web. Anyone can play it in Chrome, with a pad in hand and the game on the big screen.
          </p>
          <div className="actions">
            <Link className="button big" href="/games">Browse games</Link>
            <Link className="button big secondary" href="/upload">Upload a game</Link>
          </div>
        </div>
        <PadPanel />
      </section>

      <section className="section">
        <div className="section-head">
          <h2>New games</h2>
          {newest.length > 0 && <Link href="/games">See all games</Link>}
        </div>
        {newest.length > 0 ? (
          <GameTiles games={newest} />
        ) : (
          <div className="empty">
            <p>No games yet. Yours could be the first one people play here.</p>
            <Link className="button" href="/upload">Upload a game</Link>
          </div>
        )}
      </section>

      {popular.length > 0 && (
        <section className="section">
          <div className="section-head">
            <h2>Most played</h2>
            <Link href="/games?sort=popular">See more</Link>
          </div>
          <GameTiles games={popular} />
        </section>
      )}

      <section className="section">
        <div className="section-head">
          <h2>Put your game on Giga Couch</h2>
        </div>
        <div className="how">
          <div>
            <h3>Export it for the web</h3>
            <p>Godot, Unity, three.js, Babylon.js, and plain JavaScript all work. In Godot, turn off Thread Support when you export.</p>
          </div>
          <div>
            <h3>Zip it and upload</h3>
            <p>Put index.html at the top of the zip. Add a cover, and say how many people can play and with what.</p>
          </div>
          <div>
            <h3>Share the link</h3>
            <p>Anyone with Chrome can play it right away. Games read controllers with Chrome&apos;s standard gamepad support.</p>
          </div>
        </div>
      </section>
    </div>
  );
}
