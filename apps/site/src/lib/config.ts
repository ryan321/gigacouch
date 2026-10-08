import path from "node:path";

const port = process.env.PORT ?? "3000";

function origin(value: string | undefined, fallback: string): string {
  return (value ?? fallback).replace(/\/+$/, "");
}

/** Where people use the site. Sign-in cookies live here. */
export const SITE_ORIGIN = origin(process.env.SITE_ORIGIN, `http://localhost:${port}`);

/**
 * Where uploaded games are served from. It must be a different site from
 * SITE_ORIGIN, so a game can't read or use anyone's sign-in. Locally,
 * 127.0.0.1 and localhost are different sites to Chrome.
 */
export const GAMES_ORIGIN = origin(process.env.GAMES_ORIGIN, `http://127.0.0.1:${port}`);

export const DATA_DIR = process.env.DATA_DIR ?? path.join(process.cwd(), "data");

export const LIMITS = {
  /** The zip as uploaded. */
  zipBytes: 300 * 1024 * 1024,
  /** Everything in the zip once unpacked. */
  unpackedBytes: 500 * 1024 * 1024,
  /** One file once unpacked. */
  fileBytes: 200 * 1024 * 1024,
  files: 5000,
  imageBytes: 5 * 1024 * 1024,
  screenshots: 6,
  tags: 5,
  playersMax: 4,
};

export function hostOf(url: string): string {
  return new URL(url).host;
}
