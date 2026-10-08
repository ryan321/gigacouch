import fs from "node:fs";
import { Readable } from "node:stream";

/** Stream a file from disk with the given headers. */
export function sendFile(filePath: string, headers: Record<string, string>, method: string): Response {
  const size = fs.statSync(filePath).size;
  const all = { ...headers, "Content-Length": String(size), "X-Content-Type-Options": "nosniff" };
  if (method === "HEAD") return new Response(null, { headers: all });
  const stream = Readable.toWeb(fs.createReadStream(filePath)) as ReadableStream<Uint8Array>;
  return new Response(stream, { headers: all });
}
