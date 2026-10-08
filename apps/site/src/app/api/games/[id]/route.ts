import { currentUser } from "@/lib/auth";
import { LIMITS } from "@/lib/config";
import { deleteGame, gameById, updateGame } from "@/lib/games";
import { fromSite, invalid, json, refused, signInFirst } from "@/lib/http";
import { checkImage, deleteImage, saveImage } from "@/lib/media";
import { file, files, readGameDetails } from "@/lib/validate";

type Context = { params: Promise<{ id: string }> };

async function ownGame(request: Request, context: Context) {
  if (!fromSite(request)) return { response: refused() };
  const user = await currentUser();
  if (!user) return { response: signInFirst() };
  const game = gameById((await context.params).id);
  if (!game || game.owner.id !== user.id) return { response: json({ ok: false, errors: { form: "That game isn't yours to change." } }, 404) };
  return { game };
}

/** Change a game's details, cover, and screenshots. */
export async function POST(request: Request, context: Context) {
  const { game, response } = await ownGame(request, context);
  if (!game) return response;
  const form = await request.formData();
  const { details, errors } = readGameDetails(form);

  const cover = file(form, "cover");
  if (cover) {
    const problem = await checkImage(cover);
    if (problem) errors.cover = problem;
  }
  const removed = new Set(form.getAll("removeScreenshot").map(String));
  const kept = game.screenshots.filter((name) => !removed.has(name));
  const added = files(form, "screenshots");
  if (kept.length + added.length > LIMITS.screenshots) errors.screenshots = `A game can have up to ${LIMITS.screenshots} screenshots.`;
  for (const shot of added) {
    const problem = await checkImage(shot);
    if (problem) {
      errors.screenshots = `${shot.name}: ${problem}`;
      break;
    }
  }
  if (Object.keys(errors).length) return invalid(errors);

  const removeCover = form.get("removeCover") === "on";
  const coverName = cover ? await saveImage(cover) : removeCover ? null : game.cover;
  const shots = [...kept];
  for (const shot of added) shots.push(await saveImage(shot));
  updateGame(game, details, coverName, shots);
  if (coverName !== game.cover) deleteImage(game.cover);
  for (const name of game.screenshots) if (removed.has(name)) deleteImage(name);
  return json({ ok: true, redirect: `/games/${game.slug}` });
}

export async function DELETE(request: Request, context: Context) {
  const { game, response } = await ownGame(request, context);
  if (!game) return response;
  deleteGame(game);
  return json({ ok: true, redirect: `/u/${game.owner.handle}` });
}
