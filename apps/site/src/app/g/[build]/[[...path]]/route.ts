import { buildFile } from "@/lib/builds";
import { GAMES_ORIGIN, SITE_ORIGIN, hostOf } from "@/lib/config";
import { describe } from "@/lib/mime";
import { sendFile } from "@/lib/send-file";
import { downloadResponse } from "@/lib/download";

type Context = { params: Promise<{ build: string; path?: string[] }> };

const notFound = () => new Response("Not found", { status: 404, headers: { "Content-Type": "text/plain" } });

/**
 * A game's files. They're only served on the games origin: a game opened on
 * the site's own origin could act as whoever is signed in.
 */
async function serve(request: Request, context: Context): Promise<Response> {
  const separate = hostOf(GAMES_ORIGIN) !== hostOf(SITE_ORIGIN);
  if (separate && request.headers.get("host") !== hostOf(GAMES_ORIGIN)) return notFound();
  const { build, path = [] } = await context.params;
  if (path.length === 0) {
    return new Response(null, { status: 308, headers: { Location: `/g/${build}/index.html` } });
  }
  const parts = path.map((part) => decodeURIComponent(part));
  const mode = new URL(request.url).searchParams.get("couch");
  if (mode && parts.length === 1 && parts[0] === "index.html") {
    return downloadResponse(build, mode, request.method) ?? notFound();
  }
  const found = buildFile(build, parts);
  if (!found) return notFound();
  const { type, encoding } = describe(parts[parts.length - 1]);
  const headers: Record<string, string> = {
    "Content-Type": type,
    // Every build has its own address, so its files never change.
    "Cache-Control": "public, max-age=31536000, immutable",
  };
  if (encoding) headers["Content-Encoding"] = encoding;
  // Only the site's play page may show a game in a frame.
  if (type.startsWith("text/html")) headers["Content-Security-Policy"] = `frame-ancestors ${SITE_ORIGIN}`;
  return sendFile(found, headers, request.method);
}

export const GET = serve;
export const HEAD = serve;
