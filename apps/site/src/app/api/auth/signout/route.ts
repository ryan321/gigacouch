import { endSession } from "@/lib/auth";
import { fromSite, json, refused } from "@/lib/http";

export async function POST(request: Request) {
  if (!fromSite(request)) return refused();
  await endSession();
  return json({ ok: true, redirect: "/" });
}
