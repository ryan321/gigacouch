import { db, now, transaction } from "./db";
import { newId } from "./ids";
import { deleteBuildFiles } from "./builds";
import { deleteImage } from "./media";
import type { GameDetails } from "./validate";

export type Visibility = "public" | "unlisted" | "draft";

export type Game = {
  id: string;
  slug: string;
  title: string;
  description: string;
  tags: string[];
  playersMin: number;
  playersMax: number;
  gamepad: boolean;
  keyboard: boolean;
  mouse: boolean;
  visibility: Visibility;
  cover: string | null;
  screenshots: string[];
  buildId: string | null;
  plays: number;
  createdAt: number;
  updatedAt: number;
  owner: { id: string; handle: string; displayName: string; avatar: string | null };
};

type Row = {
  id: string;
  slug: string;
  title: string;
  description: string;
  tags: string;
  players_min: number;
  players_max: number;
  gamepad: number;
  keyboard: number;
  mouse: number;
  visibility: Visibility;
  cover: string | null;
  screenshots: string;
  build_id: string | null;
  plays: number;
  created_at: number;
  updated_at: number;
  owner_id: string;
  owner_handle: string;
  owner_name: string;
  owner_avatar: string | null;
};

const SELECT = `
  SELECT games.*, users.handle AS owner_handle, users.display_name AS owner_name, users.avatar AS owner_avatar
  FROM games JOIN users ON users.id = games.owner_id`;

function toGame(row: Row): Game {
  return {
    id: row.id,
    slug: row.slug,
    title: row.title,
    description: row.description,
    tags: JSON.parse(row.tags) as string[],
    playersMin: row.players_min,
    playersMax: row.players_max,
    gamepad: row.gamepad === 1,
    keyboard: row.keyboard === 1,
    mouse: row.mouse === 1,
    visibility: row.visibility,
    cover: row.cover,
    screenshots: JSON.parse(row.screenshots) as string[],
    buildId: row.build_id,
    plays: row.plays,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    owner: { id: row.owner_id, handle: row.owner_handle, displayName: row.owner_name, avatar: row.owner_avatar },
  };
}

export type Browse = {
  q?: string;
  players?: number;
  control?: "gamepad" | "keyboard" | "mouse";
  tag?: string;
  sort?: "new" | "popular";
  limit?: number;
};

/** Public games for browsing and the home page. Unlisted and draft games never show here. */
export function listGames(options: Browse = {}): Game[] {
  const where = ["games.visibility = 'public'", "games.build_id IS NOT NULL"];
  const values: (string | number)[] = [];
  if (options.q) {
    const like = `%${options.q.replace(/[%_\\]/g, (match) => `\\${match}`)}%`;
    where.push(
      `(games.title LIKE ? ESCAPE '\\' OR games.description LIKE ? ESCAPE '\\' OR users.handle LIKE ? ESCAPE '\\'
        OR users.display_name LIKE ? ESCAPE '\\' OR EXISTS (SELECT 1 FROM json_each(games.tags) WHERE value = ?))`,
    );
    values.push(like, like, like, like, options.q.toLowerCase());
  }
  if (options.players) {
    // Games that can be played by exactly this many people.
    where.push("games.players_min <= ? AND games.players_max >= ?");
    values.push(options.players, options.players);
  }
  if (options.control) where.push(`games.${options.control} = 1`);
  if (options.tag) {
    where.push("EXISTS (SELECT 1 FROM json_each(games.tags) WHERE value = ?)");
    values.push(options.tag);
  }
  const order = options.sort === "popular" ? "games.plays DESC, games.created_at DESC" : "games.created_at DESC";
  const rows = db()
    .prepare(`${SELECT} WHERE ${where.join(" AND ")} ORDER BY ${order} LIMIT ?`)
    .all(...values, options.limit ?? 60) as Row[];
  return rows.map(toGame);
}

export function gameBySlug(slug: string): Game | null {
  const row = db().prepare(`${SELECT} WHERE games.slug = ?`).get(slug) as Row | undefined;
  return row ? toGame(row) : null;
}

export function gameById(id: string): Game | null {
  const row = db().prepare(`${SELECT} WHERE games.id = ?`).get(id) as Row | undefined;
  return row ? toGame(row) : null;
}

/** A person's games. Others see only public ones; the owner sees everything. */
export function gamesByOwner(ownerId: string, includeHidden: boolean): Game[] {
  const hidden = includeHidden ? "" : "AND games.visibility = 'public' AND games.build_id IS NOT NULL";
  const rows = db()
    .prepare(`${SELECT} WHERE games.owner_id = ? ${hidden} ORDER BY games.created_at DESC`)
    .all(ownerId) as Row[];
  return rows.map(toGame);
}

/** Who may open a game's page and play it. */
export function canView(game: Game, viewerId: string | null): boolean {
  return game.visibility !== "draft" || game.owner.id === viewerId;
}

function slugFor(title: string): string {
  const base =
    title
      .toLowerCase()
      .normalize("NFKD")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 48) || "game";
  const taken = db().prepare("SELECT 1 FROM games WHERE slug = ?");
  if (!taken.get(base)) return base;
  for (let n = 2; ; n += 1) {
    const candidate = `${base}-${n}`;
    if (!taken.get(candidate)) return candidate;
  }
}

export function createGame(ownerId: string, details: GameDetails, cover: string | null, screenshots: string[]): { id: string; slug: string } {
  const id = newId();
  const slug = slugFor(details.title);
  const time = now();
  db()
    .prepare(
      `INSERT INTO games (id, slug, owner_id, title, description, tags, players_min, players_max, gamepad, keyboard, mouse,
                          visibility, cover, screenshots, created_at, updated_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    )
    .run(
      id, slug, ownerId, details.title, details.description, JSON.stringify(details.tags),
      details.playersMin, details.playersMax, Number(details.gamepad), Number(details.keyboard), Number(details.mouse),
      details.visibility, cover, JSON.stringify(screenshots), time, time,
    );
  return { id, slug };
}

export function updateGame(game: Game, details: GameDetails, cover: string | null, screenshots: string[]): void {
  db()
    .prepare(
      `UPDATE games SET title = ?, description = ?, tags = ?, players_min = ?, players_max = ?, gamepad = ?, keyboard = ?,
                        mouse = ?, visibility = ?, cover = ?, screenshots = ?, updated_at = ?
       WHERE id = ?`,
    )
    .run(
      details.title, details.description, JSON.stringify(details.tags), details.playersMin, details.playersMax,
      Number(details.gamepad), Number(details.keyboard), Number(details.mouse), details.visibility, cover,
      JSON.stringify(screenshots), now(), game.id,
    );
}

/** Make a build the one players get. The previous build is kept for rolling back later. */
export function setLiveBuild(gameId: string, buildId: string): void {
  db().prepare("UPDATE games SET build_id = ?, updated_at = ? WHERE id = ?").run(buildId, now(), gameId);
}

export function deleteGame(game: Game): void {
  const builds = db().prepare("SELECT id FROM builds WHERE game_id = ?").all(game.id) as { id: string }[];
  transaction(() => {
    db().prepare("DELETE FROM games WHERE id = ?").run(game.id);
  });
  for (const build of builds) deleteBuildFiles(build.id);
  deleteImage(game.cover);
  for (const shot of game.screenshots) deleteImage(shot);
}

export function recordPlay(gameId: string): void {
  db().prepare("UPDATE games SET plays = plays + 1 WHERE id = ?").run(gameId);
}

export function popularTags(limit = 12): string[] {
  const rows = db()
    .prepare(
      `SELECT value AS tag, count(*) AS uses FROM games, json_each(games.tags)
       WHERE games.visibility = 'public' AND games.build_id IS NOT NULL
       GROUP BY value ORDER BY uses DESC, value LIMIT ?`,
    )
    .all(limit) as { tag: string }[];
  return rows.map((row) => row.tag);
}
