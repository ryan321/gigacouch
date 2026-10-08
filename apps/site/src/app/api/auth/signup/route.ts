import { createUser, startSession } from "@/lib/auth";
import { db } from "@/lib/db";
import { fromSite, invalid, json, refused } from "@/lib/http";
import { safeNext } from "@/lib/next-path";
import { checkDisplayName, checkEmail, checkHandle, checkPassword, text, type FieldErrors } from "@/lib/validate";

export async function POST(request: Request) {
  if (!fromSite(request)) return refused();
  const form = await request.formData();
  const email = text(form, "email").toLowerCase();
  const handle = text(form, "handle").toLowerCase().replace(/^@/, "");
  const displayName = text(form, "displayName") || handle;
  const password = typeof form.get("password") === "string" ? (form.get("password") as string) : "";

  const errors: FieldErrors = {};
  const problems: [string, string | null][] = [
    ["email", checkEmail(email)],
    ["handle", checkHandle(handle)],
    ["displayName", checkDisplayName(displayName)],
    ["password", checkPassword(password)],
  ];
  for (const [field, message] of problems) if (message) errors[field] = message;
  if (form.get("oldEnough") !== "on") errors.oldEnough = "You need to be 13 or older to make an account.";
  if (!errors.email && db().prepare("SELECT 1 FROM users WHERE email = ?").get(email))
    errors.email = "That email already has an account. Sign in instead.";
  if (!errors.handle && db().prepare("SELECT 1 FROM users WHERE handle = ?").get(handle))
    errors.handle = "That handle is taken. Try another.";
  if (Object.keys(errors).length) return invalid(errors);

  const id = createUser({ email, handle, displayName, password });
  await startSession(id);
  return json({ ok: true, redirect: safeNext(text(form, "next"), `/u/${handle}`) });
}
