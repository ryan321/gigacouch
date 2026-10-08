import fs from "node:fs";
import path from "node:path";
import { DATA_DIR, LIMITS } from "./config";
import { newId } from "./ids";

export const MEDIA_DIR = path.join(DATA_DIR, "media");

const KINDS: { extension: string; type: string; matches: (bytes: Uint8Array) => boolean }[] = [
  { extension: "png", type: "image/png", matches: (b) => b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47 },
  { extension: "jpg", type: "image/jpeg", matches: (b) => b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff },
  { extension: "gif", type: "image/gif", matches: (b) => b[0] === 0x47 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x38 },
  {
    extension: "webp",
    type: "image/webp",
    matches: (b) =>
      b[0] === 0x52 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x46 &&
      b[8] === 0x57 && b[9] === 0x45 && b[10] === 0x42 && b[11] === 0x50,
  },
];

/** Check an uploaded image without saving it. Returns a message, or null when it's fine. */
export async function checkImage(file: File): Promise<string | null> {
  if (file.size > LIMITS.imageBytes) return "Use an image under 5 MB.";
  const head = new Uint8Array(await file.slice(0, 12).arrayBuffer());
  return KINDS.some((kind) => kind.matches(head)) ? null : "Use a PNG, JPEG, GIF, or WebP image.";
}

/** Save a checked image and return its file name. */
export async function saveImage(file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const kind = KINDS.find((candidate) => candidate.matches(bytes));
  if (!kind) throw new Error("not an image");
  const name = `${newId()}.${kind.extension}`;
  fs.mkdirSync(MEDIA_DIR, { recursive: true });
  fs.writeFileSync(path.join(MEDIA_DIR, name), bytes);
  return name;
}

export function deleteImage(name: string | null | undefined): void {
  if (!name || !isMediaName(name)) return;
  fs.rmSync(path.join(MEDIA_DIR, name), { force: true });
}

export function isMediaName(name: string): boolean {
  return /^[0-9a-f]{32}\.(png|jpg|gif|webp)$/.test(name);
}

export function mediaType(name: string): string {
  const extension = name.split(".").pop();
  return KINDS.find((kind) => kind.extension === extension)?.type ?? "application/octet-stream";
}
