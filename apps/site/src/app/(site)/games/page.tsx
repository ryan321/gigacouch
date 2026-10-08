import type { Metadata } from "next";
import Link from "next/link";
import { Field } from "@/components/Field";
import { GameTiles } from "@/components/GameTile";
import { countText } from "@/lib/format";
import { listGames, popularTags, type Browse } from "@/lib/games";
import { one, type SearchParams } from "@/lib/params";

export const metadata: Metadata = { title: "Games" };

const CONTROLS = { gamepad: "Controller", keyboard: "Keyboard", mouse: "Mouse" } as const;

export default async function BrowsePage({ searchParams }: { searchParams: SearchParams }) {
  const params = await searchParams;
  const q = one(params.q).slice(0, 80);
  const playersValue = Number(one(params.players));
  const players = [1, 2, 3, 4].includes(playersValue) ? playersValue : undefined;
  const controlValue = one(params.control);
  const control = controlValue in CONTROLS ? (controlValue as Browse["control"]) : undefined;
  const tag = one(params.tag).toLowerCase();
  const sort = one(params.sort) === "popular" ? "popular" : "new";
  const games = listGames({ q: q || undefined, players, control, tag: tag || undefined, sort });
  const tags = popularTags();
  const filtered = Boolean(q || players || control || tag);

  const tagLink = (value: string) => {
    const next = new URLSearchParams();
    if (q) next.set("q", q);
    if (players) next.set("players", String(players));
    if (control) next.set("control", control);
    if (sort === "popular") next.set("sort", sort);
    if (value !== tag) next.set("tag", value);
    const query = next.toString();
    return query ? `/games?${query}` : "/games";
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>Games</h1>
        <p className="lede">Every public game on Giga Couch. Plug in a controller, pick one, and press Play.</p>
      </div>
      <form className="filters" action="/games" role="search">
        <Field label="Search" className="grow">
          <input type="search" name="q" defaultValue={q} placeholder="Title, creator, or tag" />
        </Field>
        <Field label="Players">
          <select name="players" defaultValue={players ?? ""}>
            <option value="">Any number</option>
            <option value="1">1 player</option>
            <option value="2">2 players</option>
            <option value="3">3 players</option>
            <option value="4">4 players</option>
          </select>
        </Field>
        <Field label="Plays with">
          <select name="control" defaultValue={control ?? ""}>
            <option value="">Anything</option>
            {Object.entries(CONTROLS).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
          </select>
        </Field>
        <Field label="Sort">
          <select name="sort" defaultValue={sort}>
            <option value="new">Newest</option>
            <option value="popular">Most played</option>
          </select>
        </Field>
        {tag && <input type="hidden" name="tag" value={tag} />}
        <button className="button" type="submit">Show games</button>
      </form>
      {tags.length > 0 && (
        <div className="chips" style={{ marginTop: 16 }} aria-label="Popular tags">
          {tags.map((value) => (
            <Link key={value} className={value === tag ? "chip on" : "chip"} href={tagLink(value)} aria-current={value === tag ? "true" : undefined}>
              {value}
            </Link>
          ))}
        </div>
      )}
      <p className="results-note" aria-live="polite">{countText(games.length, "game", "games")}{tag ? ` tagged ${tag}` : ""}</p>
      {games.length > 0 ? (
        <GameTiles games={games} />
      ) : (
        <div className="empty">
          {filtered ? (
            <>
              <p>No games match those filters.</p>
              <Link className="button secondary" href="/games">Clear the filters</Link>
            </>
          ) : (
            <>
              <p>No games yet. Upload one and it shows up here.</p>
              <Link className="button" href="/upload">Upload a game</Link>
            </>
          )}
        </div>
      )}
    </div>
  );
}
