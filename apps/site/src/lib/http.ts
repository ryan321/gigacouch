import { GAMES_ORIGIN, hostOf } from "./config";

export function json(data: unknown, status = 200): Response {
  return Response.json(data, { status, headers: { "Cache-Control": "no-store" } });
}

/** A form that didn't pass its checks: one message per field. */
export function invalid(errors: Record<string, string>, status = 400): Response {
  return json({ ok: false, errors }, status);
}

/**
 * Changes must come from the site's own pages. Browsers send Origin on every
 * POST from a page, so a request from another site, or from a game on the
 * games origin, is refused.
 */
export function fromSite(request: Request): boolean {
  const origin = request.headers.get("origin");
  const host = request.headers.get("host");
  if (!origin || !host) return false;
  let originHost: string;
  try {
    originHost = new URL(origin).host;
  } catch {
    return false;
  }
  return originHost === host && host !== hostOf(GAMES_ORIGIN);
}

export function refused(): Response {
  return json({ ok: false, errors: { form: "That request didn't come from this site." } }, 403);
}

export function signInFirst(): Response {
  return json({ ok: false, errors: { form: "Sign in first." } }, 401);
}
