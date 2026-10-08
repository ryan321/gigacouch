import { checkSignIn, startSession } from "@/lib/auth";
import { fromSite, invalid, json, refused } from "@/lib/http";
import { safeNext } from "@/lib/next-path";
import { text } from "@/lib/validate";

export async function POST(request: Request) {
  if (!fromSite(request)) return refused();
  const form = await request.formData();
  const login = text(form, "login").toLowerCase();
  const password = typeof form.get("password") === "string" ? (form.get("password") as string) : "";
  if (!login || !password) return invalid({ form: "Enter your email or handle, and your password." });
  const userId = checkSignIn(login, password);
  if (!userId) return invalid({ form: "That email or handle and password don't match an account." });
  await startSession(userId);
  return json({ ok: true, redirect: safeNext(text(form, "next")) });
}
