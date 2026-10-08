import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Player } from "@/components/Player";
import { currentUser } from "@/lib/auth";
import { GAMES_ORIGIN } from "@/lib/config";
import { controlsList, orList, playersText } from "@/lib/format";
import { canView, gameBySlug } from "@/lib/games";
import { keepPagesOnSite } from "@/lib/site-host";

export const dynamic = "force-dynamic";

type Props = { params: Promise<{ slug: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const game = gameBySlug((await params).slug);
  return game && game.visibility !== "draft" ? { title: `Play ${game.title}` } : {};
}

export default async function PlayPage({ params }: Props) {
  await keepPagesOnSite();
  const game = gameBySlug((await params).slug);
  const user = await currentUser();
  if (!game || !game.buildId || !canView(game, user?.id ?? null)) notFound();
  const controls = `${playersText(game.playersMin, game.playersMax)}. Play with a ${orList(controlsList(game).map((name) => name.toLowerCase()))}.`;
  return (
    <Player
      game={{ id: game.id, slug: game.slug, title: game.title, playersMax: game.playersMax, gamepad: game.gamepad, keyboard: game.keyboard, mouse: game.mouse }}
      src={`${GAMES_ORIGIN}/g/${game.buildId}/index.html`}
      controls={controls}
    />
  );
}
