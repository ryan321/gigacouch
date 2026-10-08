import crypto from "node:crypto";
import { cookies } from "next/headers";
import { cache } from "react";
import { db, now } from "./db";
import { newId } from "./ids";

const COOKIE = "gc_session";
const SESSION_DAYS = 30;

export type User = {
  id: string;
  email: string;
  handle: string;
  displayName: string;
  bio: string;
  avatar: string | null;
};

export function hashPassword(password: string): string {
  const salt = crypto.randomBytes(16);
  const hash = crypto.scryptSync(password, salt, 64);
  return `scrypt$${salt.toString("base64")}$${hash.toString("base64")}`;
}

export function verifyPassword(password: string, stored: string): boolean {
  const [scheme, salt, hash] = stored.split("$");
  if (scheme !== "scrypt" || !salt || !hash) return false;
  const expected = Buffer.from(hash, "base64");
  const actual = crypto.scryptSync(password, Buffer.from(salt, "base64"), expected.length);
  return crypto.timingSafeEqual(expected, actual);
}

function tokenHash(token: string): string {
  return crypto.createHash("sha256").update(token).digest("hex");
}

/** Sign this browser in as the user. */
export async function startSession(userId: string): Promise<void> {
  const token = crypto.randomBytes(32).toString("base64url");
  const created = now();
  db()
    .prepare("INSERT INTO sessions (token_hash, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)")
    .run(tokenHash(token), userId, created, created + SESSION_DAYS * 86400);
  (await cookies()).set(COOKIE, token, {
    httpOnly: true,
    sameSite: "lax",
    path: "/",
    secure: process.env.COOKIE_SECURE === "1",
    maxAge: SESSION_DAYS * 86400,
  });
}

export async function endSession(): Promise<void> {
  const jar = await cookies();
  const token = jar.get(COOKIE)?.value;
  if (token) db().prepare("DELETE FROM sessions WHERE token_hash = ?").run(tokenHash(token));
  jar.delete(COOKIE);
}

type UserRow = {
  id: string;
  email: string;
  handle: string;
  display_name: string;
  bio: string;
  avatar: string | null;
};

export function toUser(row: UserRow): User {
  return {
    id: row.id,
    email: row.email,
    handle: row.handle,
    displayName: row.display_name,
    bio: row.bio,
    avatar: row.avatar,
  };
}

/** The signed-in user for this request, or null. */
export const currentUser = cache(async (): Promise<User | null> => {
  const token = (await cookies()).get(COOKIE)?.value;
  if (!token) return null;
  const row = db()
    .prepare(
      `SELECT users.id, users.email, users.handle, users.display_name, users.bio, users.avatar
       FROM sessions JOIN users ON users.id = sessions.user_id
       WHERE sessions.token_hash = ? AND sessions.expires_at > ?`,
    )
    .get(tokenHash(token), now()) as UserRow | undefined;
  return row ? toUser(row) : null;
});

export function createUser(input: {
  email: string;
  handle: string;
  displayName: string;
  password: string;
}): string {
  const id = newId();
  db()
    .prepare(
      `INSERT INTO users (id, email, handle, display_name, password_hash, created_at)
       VALUES (?, ?, ?, ?, ?, ?)`,
    )
    .run(id, input.email, input.handle, input.displayName, hashPassword(input.password), now());
  return id;
}

/** Find an account by email or handle and check its password. */
export function checkSignIn(login: string, password: string): string | null {
  const row = db()
    .prepare("SELECT id, password_hash FROM users WHERE email = ? OR handle = ?")
    .get(login, login.replace(/^@/, "")) as { id: string; password_hash: string } | undefined;
  if (!row) {
    // Spend the same time as a real check, so timing doesn't reveal accounts.
    verifyPassword(password, hashPassword("not a real password"));
    return null;
  }
  return verifyPassword(password, row.password_hash) ? row.id : null;
}

export function userByHandle(handle: string): User | null {
  const row = db()
    .prepare("SELECT id, email, handle, display_name, bio, avatar FROM users WHERE handle = ?")
    .get(handle) as UserRow | undefined;
  return row ? toUser(row) : null;
}
