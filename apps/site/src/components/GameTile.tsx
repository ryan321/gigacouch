import Link from "next/link";
import type { Game } from "@/lib/games";
import { playersText } from "@/lib/format";
import { Cover } from "./Cover";

const HIDDEN: Record<string, string> = { draft: "Draft", unlisted: "Unlisted" };

export function GameTile({ game, showOwner = true }: { game: Game; showOwner?: boolean }) {
  return (
    <Link className="tile" href={`/games/${game.slug}`}>
      <Cover game={game} />
      <h3>{game.title}</h3>
      <div className="meta">
        {showOwner && <span>{game.owner.displayName}</span>}
        <span>{playersText(game.playersMin, game.playersMax)}</span>
        {HIDDEN[game.visibility] && <span className="badge">{HIDDEN[game.visibility]}</span>}
      </div>
    </Link>
  );
}

export function GameTiles({ games, showOwner = true }: { games: Game[]; showOwner?: boolean }) {
  return (
    <div className="tiles">
      {games.map((game) => (
        <GameTile key={game.id} game={game} showOwner={showOwner} />
      ))}
    </div>
  );
}
