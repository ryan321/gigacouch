import crypto from "node:crypto";

/** A random id that is safe in URLs and file names: 32 hex characters. */
export function newId(): string {
  return crypto.randomBytes(16).toString("hex");
}

export function isId(value: string): boolean {
  return /^[0-9a-f]{32}$/.test(value);
}
