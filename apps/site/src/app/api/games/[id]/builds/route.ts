import { currentUser } from "@/lib/auth";
import { readBuild, saveBuild } from "@/lib/builds";
import { LIMITS } from "@/lib/config";
import { gameById, setLiveBuild } from "@/lib/games";
import { fromSite, invalid, json, refused, signInFirst } from "@/lib/http";
import { file } from "@/lib/validate";

/** Upload a new build. It replaces the live one; the old one is kept. */
export async function POST(request: Request, context: { params: Promise<{ id: string }> }) {
  if (!fromSite(request)) return refused();
  const user = await currentUser();
  if (!user) return signInFirst();
  const game = gameById((await context.params).id);
  if (!game || game.owner.id !== user.id) return json({ ok: false, errors: { form: "That game isn't yours to change." } }, 404);

  const zip = file(await request.formData(), "build");
  if (!zip) return invalid({ build: "Choose the zip of your game's web build." });
  if (zip.size > LIMITS.zipBytes) return invalid({ build: "Zips can be up to 300 MB." });
  const { entries, problems } = readBuild(new Uint8Array(await zip.arrayBuffer()));
  if (problems.length) return invalid({ build: problems.join("\n") });
  setLiveBuild(game.id, saveBuild(game.id, entries));
  return json({ ok: true, redirect: `/games/${game.slug}` });
}
