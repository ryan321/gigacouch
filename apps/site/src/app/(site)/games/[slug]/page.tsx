import { buildDownloadManifest } from "@/lib/download";
import { downloadSize } from "@/lib/download-format";
import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Avatar } from "@/components/Avatar";
import { Cover } from "@/components/Cover";
import { currentUser } from "@/lib/auth";
import { controlsList, countText, playersText } from "@/lib/format";
import { canView, gameBySlug } from "@/lib/games";

type Props = { params: Promise<{ slug: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const game = gameBySlug((await params).slug);
  return game && game.visibility !== "draft" ? { title: game.title, description: game.description.slice(0, 160) } : {};
}

const NOTICES = {
  draft: "This is a draft. Only you can see it. Change who can see it in Edit game.",
  unlisted: "This game is unlisted. Anyone with the link can play it, but it doesn't show up in browsing or search.",
};

export default async function GamePage({ params }: Props) {
  const game = gameBySlug((await params).slug);
  const user = await currentUser();
  if (!game || !canView(game, user?.id ?? null)) notFound();
  const mine = user?.id === game.owner.id;

  return (
    <div className="page">
      <div className="game-top">
        <Cover game={game} large />
        <div className="game-side">
          <h1>{game.title}</h1>
          <Link className="byline" href={`/u/${game.owner.handle}`}>
            <Avatar user={game.owner} size={32} />
            {game.owner.displayName}
          </Link>
          <div className="form-actions">
            <Link className="button big" href={`/play/${game.slug}`}>Play</Link>
            {mine && <Link className="button secondary" href={`/games/${game.slug}/edit`}>Edit game</Link>}
          </div>
          {mine && game.visibility !== "public" && <p className="notice">{NOTICES[game.visibility]}</p>}
          <dl className="facts">
            <dt>Players</dt>
            <dd>{playersText(game.playersMin, game.playersMax)}</dd>
            <dt>Plays with</dt>
            <dd>{controlsList(game).join(", ")}</dd>
            {game.buildId && <><dt>Download size</dt><dd>{downloadSize(buildDownloadManifest(game.buildId).bytes)}</dd></>}
            <dt>Played</dt>
            <dd>{countText(game.plays, "time", "times")}</dd>
          </dl>
          <p className="muted">Downloads are saved in this browser’s storage on this device, not your Downloads folder. Clearing browser data removes saved games.</p>
          {game.tags.length > 0 && (
            <div className="chips">
              {game.tags.map((tag) => <Link key={tag} className="chip" href={`/games?tag=${encodeURIComponent(tag)}`}>{tag}</Link>)}
            </div>
          )}
        </div>
      </div>
      {game.description && (
        <section className="section">
          <h2 style={{ marginBottom: 14 }}>About this game</h2>
          <p className="description">{game.description}</p>
        </section>
      )}
      {game.screenshots.length > 0 && (
        <section className="section">
          <h2 style={{ marginBottom: 14 }}>Screenshots</h2>
          <div className="shots">
            {game.screenshots.map((name, index) => <img key={name} src={`/media/${name}`} alt={`${game.title}, screenshot ${index + 1}`} />)}
          </div>
        </section>
      )}
    </div>
  );
}
