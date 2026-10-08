import fs from "node:fs";
import path from "node:path";
import { MEDIA_DIR, isMediaName, mediaType } from "@/lib/media";
import { sendFile } from "@/lib/send-file";

async function serve(request: Request, context: { params: Promise<{ name: string }> }): Promise<Response> {
  const { name } = await context.params;
  const target = path.join(MEDIA_DIR, name);
  if (!isMediaName(name) || !fs.existsSync(target)) return new Response("Not found", { status: 404 });
  return sendFile(target, { "Content-Type": mediaType(name), "Cache-Control": "public, max-age=31536000, immutable" }, request.method);
}

export const GET = serve;
export const HEAD = serve;
