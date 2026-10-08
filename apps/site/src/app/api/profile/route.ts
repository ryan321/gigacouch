import { currentUser } from "@/lib/auth";
import { db } from "@/lib/db";
import { fromSite, invalid, json, refused, signInFirst } from "@/lib/http";
import { checkImage, deleteImage, saveImage } from "@/lib/media";
import { checkBio, checkDisplayName, file, text, type FieldErrors } from "@/lib/validate";

export async function POST(request: Request) {
  if (!fromSite(request)) return refused();
  const user = await currentUser();
  if (!user) return signInFirst();
  const form = await request.formData();
  const displayName = text(form, "displayName");
  const bio = text(form, "bio");
  const avatar = file(form, "avatar");
  const removeAvatar = form.get("removeAvatar") === "on";

  const errors: FieldErrors = {};
  const nameProblem = checkDisplayName(displayName);
  if (nameProblem) errors.displayName = nameProblem;
  const bioProblem = checkBio(bio);
  if (bioProblem) errors.bio = bioProblem;
  if (avatar) {
    const problem = await checkImage(avatar);
    if (problem) errors.avatar = problem;
  }
  if (Object.keys(errors).length) return invalid(errors);

  let next = user.avatar;
  if (avatar) next = await saveImage(avatar);
  else if (removeAvatar) next = null;
  db().prepare("UPDATE users SET display_name = ?, bio = ?, avatar = ? WHERE id = ?").run(displayName, bio, next, user.id);
  if (next !== user.avatar) deleteImage(user.avatar);
  return json({ ok: true, redirect: `/u/${user.handle}` });
}
