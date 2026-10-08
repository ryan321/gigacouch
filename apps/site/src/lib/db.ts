import fs from "node:fs";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { DATA_DIR } from "./config";

/**
 * Schema changes, in order. Each runs once; PRAGMA user_version records how
 * many have run. Add new ones at the end and never edit one that has run.
 */
const MIGRATIONS: string[] = [
  `CREATE TABLE users (
     id TEXT PRIMARY KEY,
     email TEXT NOT NULL UNIQUE COLLATE NOCASE,
     handle TEXT NOT NULL UNIQUE COLLATE NOCASE,
     display_name TEXT NOT NULL,
     password_hash TEXT NOT NULL,
     bio TEXT NOT NULL DEFAULT '',
     avatar TEXT,
     created_at INTEGER NOT NULL
   );
   CREATE TABLE sessions (
     token_hash TEXT PRIMARY KEY,
     user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
     created_at INTEGER NOT NULL,
     expires_at INTEGER NOT NULL
   );
   CREATE TABLE games (
     id TEXT PRIMARY KEY,
     slug TEXT NOT NULL UNIQUE,
     owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
     title TEXT NOT NULL,
     description TEXT NOT NULL DEFAULT '',
     tags TEXT NOT NULL DEFAULT '[]',
     players_min INTEGER NOT NULL DEFAULT 1,
     players_max INTEGER NOT NULL DEFAULT 1,
     gamepad INTEGER NOT NULL DEFAULT 0,
     keyboard INTEGER NOT NULL DEFAULT 0,
     mouse INTEGER NOT NULL DEFAULT 0,
     visibility TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'unlisted', 'draft')),
     cover TEXT,
     screenshots TEXT NOT NULL DEFAULT '[]',
     build_id TEXT,
     plays INTEGER NOT NULL DEFAULT 0,
     created_at INTEGER NOT NULL,
     updated_at INTEGER NOT NULL
   );
   CREATE TABLE builds (
     id TEXT PRIMARY KEY,
     game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
     files INTEGER NOT NULL,
     bytes INTEGER NOT NULL,
     created_at INTEGER NOT NULL
   );
   CREATE INDEX games_owner ON games(owner_id);
   CREATE INDEX games_listing ON games(visibility, created_at);
   CREATE INDEX sessions_user ON sessions(user_id);`,
];

const holder = globalThis as unknown as { __gigacouchDb?: DatabaseSync };

/** The one database connection, opened and migrated on first use. */
export function db(): DatabaseSync {
  if (!holder.__gigacouchDb) {
    fs.mkdirSync(DATA_DIR, { recursive: true });
    const database = new DatabaseSync(path.join(DATA_DIR, "site.sqlite"));
    database.exec("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;");
    migrate(database);
    holder.__gigacouchDb = database;
  }
  return holder.__gigacouchDb;
}

function migrate(database: DatabaseSync) {
  const row = database.prepare("PRAGMA user_version").get() as { user_version: number };
  for (let step = row.user_version; step < MIGRATIONS.length; step += 1) {
    database.exec("BEGIN");
    try {
      database.exec(MIGRATIONS[step]);
      database.exec(`PRAGMA user_version = ${step + 1}`);
      database.exec("COMMIT");
    } catch (error) {
      database.exec("ROLLBACK");
      throw error;
    }
  }
}

/** Run several statements as one change. */
export function transaction<T>(work: () => T): T {
  const database = db();
  database.exec("BEGIN IMMEDIATE");
  try {
    const result = work();
    database.exec("COMMIT");
    return result;
  } catch (error) {
    database.exec("ROLLBACK");
    throw error;
  }
}

export function now(): number {
  return Math.floor(Date.now() / 1000);
}
