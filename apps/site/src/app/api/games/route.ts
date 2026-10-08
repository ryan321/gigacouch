import { currentUser } from "@/lib/auth";
import { readBuild, saveBuild } from "@/lib/builds";
import { LIMITS } from "@/lib/config";
import { createGame, setLiveBuild } from "@/lib/games";
import { fromSite, invalid, json, refused, signInFirst } from "@/lib/http";
import { checkImage, saveImage } from "@/lib/media";
import { file, files, readGameDetails } from "@/lib/validate";

/** Create a game from its details and its first build. */
export async function POST(request: Request) {
  if (!fromSite(request)) return refused();
  const user = await currentUser();
  if (!user) return signInFirst();
  const form = await request.formData();
  const { details, errors } = readGameDetails(form);

  const cover = file(form, "cover");
  if (cover) {
    const problem = await checkImage(cover);
    if (problem) errors.cover = problem;
  }
  const shots = files(form, "screenshots");
  if (shots.length > LIMITS.screenshots) errors.screenshots = `Add up to ${LIMITS.screenshots} screenshots.`;
  for (const shot of shots) {
    const problem = await checkImage(shot);
    if (problem) {
      errors.screenshots = `${shot.name}: ${problem}`;
      break;
    }
  }

  const zip = file(form, "build");
  let problems: string[] = [];
  let entries: ReturnType<typeof readBuild>["entries"] = [];
  if (!zip) errors.build = "Choose the zip of your game's web build.";
  else if (zip.size > LIMITS.zipBytes) errors.build = "Zips can be up to 300 MB.";
  else {
    ({ entries, problems } = readBuild(new Uint8Array(await zip.arrayBuffer())));
    if (problems.length) errors.build = problems.join("\n");
  }
  if (Object.keys(errors).length) return invalid(errors);

  const coverName = cover ? await saveImage(cover) : null;
  const shotNames: string[] = [];
  for (const shot of shots) shotNames.push(await saveImage(shot));
  const game = createGame(user.id, details, coverName, shotNames);
  setLiveBuild(game.id, saveBuild(game.id, entries));
  return json({ ok: true, redirect: `/games/${game.slug}`, slug: game.slug, id: game.id });
}
