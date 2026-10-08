import fs from "node:fs";
import path from "node:path";
import { unzipSync } from "fflate";
import { DATA_DIR, LIMITS } from "./config";
import { db, now } from "./db";
import { isId, newId } from "./ids";
import { allowedInBuild } from "./mime";

export const BUILDS_DIR = path.join(DATA_DIR, "builds");

type Entry = { path: string; data: Uint8Array };

const JUNK = new Set([".DS_Store", "Thumbs.db", "desktop.ini"]);

function megabytes(bytes: number): string {
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

/**
 * Unpack and check a zipped web build. Returns every problem at once, so a
 * creator can fix them all before uploading again.
 */
export function readBuild(zip: Uint8Array): { entries: Entry[]; problems: string[] } {
  const problems: string[] = [];
  let unpacked = 0;
  let count = 0;
  let tooBig = false;
  let raw: Record<string, Uint8Array>;
  try {
    raw = unzipSync(zip, {
      // Sizes are checked from the zip's directory before anything is unpacked.
      filter(info) {
        if (info.name.endsWith("/")) return false;
        count += 1;
        unpacked += info.originalSize;
        if (info.originalSize > LIMITS.fileBytes) {
          problems.push(`${info.name} is ${megabytes(info.originalSize)}. Each file can be up to ${megabytes(LIMITS.fileBytes)}.`);
          tooBig = true;
        }
        if (unpacked > LIMITS.unpackedBytes || count > LIMITS.files) tooBig = true;
        return !tooBig;
      },
    });
  } catch {
    return { entries: [], problems: ["That file isn't a zip we can open. Zip the folder your engine exported and try again."] };
  }
  if (unpacked > LIMITS.unpackedBytes)
    problems.push(`The game is ${megabytes(unpacked)} unpacked. Games can be up to ${megabytes(LIMITS.unpackedBytes)}.`);
  if (count > LIMITS.files) problems.push(`The zip has ${count} files. Games can have up to ${LIMITS.files}.`);
  if (problems.length) return { entries: [], problems };

  let entries: Entry[] = [];
  for (const [name, data] of Object.entries(raw)) {
    const base = name.split("/").pop() ?? name;
    if (name.startsWith("__MACOSX/") || JUNK.has(base)) continue;
    const parts = name.split("/");
    if (
      name.includes("\\") ||
      name.startsWith("/") ||
      name.length > 300 ||
      parts.some((part) => part === "" || part === "." || part === ".." || part.includes(":"))
    ) {
      problems.push(`${name} has a path that isn't allowed. Paths must stay inside the game's folder.`);
      continue;
    }
    entries.push({ path: name, data });
  }

  // Zipping a folder, instead of its contents, puts everything one level down.
  const hasIndex = entries.some((entry) => entry.path === "index.html");
  if (!hasIndex && entries.length) {
    const top = entries[0].path.split("/")[0];
    if (entries.every((entry) => entry.path.startsWith(`${top}/`)) && entries.some((entry) => entry.path === `${top}/index.html`)) {
      entries = entries.map((entry) => ({ path: entry.path.slice(top.length + 1), data: entry.data }));
    }
  }

  const index = entries.find((entry) => entry.path === "index.html");
  if (!index) problems.push("The zip needs an index.html at its top level. That's the page that starts the game.");

  const unknown = entries.filter((entry) => !allowedInBuild(entry.path)).map((entry) => entry.path);
  if (unknown.length) {
    const shown = unknown.slice(0, 5).join(", ");
    const more = unknown.length > 5 ? `, and ${unknown.length - 5} more` : "";
    problems.push(`These files aren't a type a web game uses: ${shown}${more}. Remove them from the zip.`);
  }

  if (index) {
    const page = new TextDecoder().decode(index.data);
    if (/GODOT_THREADS_ENABLED\s*=\s*true/.test(page)) {
      problems.push(
        "This is a threaded Godot export, which can't run on Giga Couch yet. In Godot's Web export options, turn off Thread Support and export again.",
      );
    }
  }

  return { entries: problems.length ? [] : entries, problems };
}

/** Save a checked build's files and record it for the game. Returns the build id. */
export function saveBuild(gameId: string, entries: Entry[]): string {
  const id = newId();
  const root = path.join(BUILDS_DIR, id);
  let bytes = 0;
  for (const entry of entries) {
    const target = path.join(root, ...entry.path.split("/"));
    if (!target.startsWith(root + path.sep)) throw new Error("build path escaped its folder");
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, entry.data);
    bytes += entry.data.length;
  }
  db()
    .prepare("INSERT INTO builds (id, game_id, files, bytes, created_at) VALUES (?, ?, ?, ?, ?)")
    .run(id, gameId, entries.length, bytes, now());
  return id;
}

export function deleteBuildFiles(buildId: string): void {
  if (isId(buildId)) fs.rmSync(path.join(BUILDS_DIR, buildId), { recursive: true, force: true });
}

/** The file on disk for a path inside a build, or null if there isn't one. */
export function buildFile(buildId: string, filePath: string[]): string | null {
  if (!isId(buildId) || filePath.some((part) => !part || part === "." || part === ".." || part.includes("\\"))) return null;
  const root = path.join(BUILDS_DIR, buildId);
  const target = path.join(root, ...filePath);
  if (!target.startsWith(root + path.sep)) return null;
  try {
    return fs.statSync(target).isFile() ? target : null;
  } catch {
    return null;
  }
}
