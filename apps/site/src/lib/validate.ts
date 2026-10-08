import { LIMITS } from "./config";

/** Field name to message, shown next to the field. */
export type FieldErrors = Record<string, string>;

export function text(form: FormData, name: string): string {
  const value = form.get(name);
  return typeof value === "string" ? value.trim() : "";
}

export function file(form: FormData, name: string): File | null {
  const value = form.get(name);
  return value instanceof File && value.size > 0 ? value : null;
}

export function files(form: FormData, name: string): File[] {
  return form.getAll(name).filter((value): value is File => value instanceof File && value.size > 0);
}

export function checkEmail(email: string): string | null {
  if (!email) return "Enter your email address.";
  if (email.length > 254 || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) return "Enter an email address like name@example.com.";
  return null;
}

export function checkHandle(handle: string): string | null {
  if (!handle) return "Choose a handle.";
  if (!/^[a-z0-9_]{3,20}$/.test(handle)) return "Use 3 to 20 lowercase letters, numbers, or underscores.";
  return null;
}

export function checkPassword(password: string): string | null {
  if (password.length < 10) return "Use at least 10 characters.";
  if (password.length > 200) return "Use 200 characters or fewer.";
  return null;
}

export function checkDisplayName(name: string): string | null {
  if (!name) return "Enter the name people will see.";
  if (name.length > 40) return "Use 40 characters or fewer.";
  return null;
}

export function checkBio(bio: string): string | null {
  return bio.length > 300 ? "Use 300 characters or fewer." : null;
}

export type GameDetails = {
  title: string;
  description: string;
  tags: string[];
  playersMin: number;
  playersMax: number;
  gamepad: boolean;
  keyboard: boolean;
  mouse: boolean;
  visibility: "public" | "unlisted" | "draft";
};

/** Read and check a game's details from a form. */
export function readGameDetails(form: FormData): { details: GameDetails; errors: FieldErrors } {
  const errors: FieldErrors = {};
  const title = text(form, "title");
  if (!title) errors.title = "Give the game a title.";
  else if (title.length > 60) errors.title = "Use 60 characters or fewer.";

  const description = text(form, "description");
  if (description.length > 4000) errors.description = "Use 4,000 characters or fewer.";

  const tags = [
    ...new Set(
      text(form, "tags")
        .split(",")
        .map((tag) => tag.trim().toLowerCase().replace(/\s+/g, "-"))
        .filter(Boolean),
    ),
  ];
  if (tags.length > LIMITS.tags) errors.tags = `Use up to ${LIMITS.tags} tags.`;
  else if (tags.some((tag) => !/^[a-z0-9-]{1,24}$/.test(tag)))
    errors.tags = "Tags use letters, numbers, and dashes, up to 24 characters each.";

  const playersMin = Number(text(form, "playersMin") || "1");
  const playersMax = Number(text(form, "playersMax") || "1");
  const inRange = (value: number) => Number.isInteger(value) && value >= 1 && value <= LIMITS.playersMax;
  if (!inRange(playersMin) || !inRange(playersMax) || playersMin > playersMax)
    errors.players = `Choose from 1 to ${LIMITS.playersMax} players, with the fewest no more than the most.`;

  const gamepad = form.get("gamepad") === "on";
  const keyboard = form.get("keyboard") === "on";
  const mouse = form.get("mouse") === "on";
  if (!gamepad && !keyboard && !mouse) errors.controls = "Choose at least one way to play.";

  const visibilityValue = text(form, "visibility") || "public";
  const visibility = (["public", "unlisted", "draft"] as const).find((value) => value === visibilityValue);
  if (!visibility) errors.visibility = "Choose who can see the game.";

  return {
    details: {
      title,
      description,
      tags,
      playersMin,
      playersMax,
      gamepad,
      keyboard,
      mouse,
      visibility: visibility ?? "public",
    },
    errors,
  };
}
