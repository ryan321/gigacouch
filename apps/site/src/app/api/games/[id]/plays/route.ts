import { currentUser } from "@/lib/auth";
import { canView, gameById, recordPlay } from "@/lib/games";
import { fromSite, json, refused } from "@/lib/http";

/** Count a play when someone starts a game. */
export async function POST(request: Request, context: { params: Promise<{ id: string }> }) {
  if (!fromSite(request)) return refused();
  const game = gameById((await context.params).id);
  const user = await currentUser();
  if (!game || !canView(game, user?.id ?? null)) return json({ ok: false }, 404);
  recordPlay(game.id);
  return json({ ok: true });
}
