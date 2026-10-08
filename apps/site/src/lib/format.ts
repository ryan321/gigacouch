/** The address of an uploaded image. Safe to use in the browser. */
export function mediaUrl(name: string | null): string | null {
  return name ? `/media/${name}` : null;
}

export function playersText(min: number, max: number): string {
  if (min === max) return min === 1 ? "1 player" : `${min} players`;
  return `${min}–${max} players`;
}

export function controlsList(game: { gamepad: boolean; keyboard: boolean; mouse: boolean }): string[] {
  return [game.gamepad && "Controller", game.keyboard && "Keyboard", game.mouse && "Mouse"].filter(
    (value): value is string => Boolean(value),
  );
}

const TINTS = ["var(--mint)", "var(--blue)", "var(--amber)"];

/** A stable brand color for a game without a cover. */
export function tintFor(id: string): string {
  let total = 0;
  for (const char of id) total = (total * 31 + char.charCodeAt(0)) >>> 0;
  return TINTS[total % TINTS.length];
}

/** "a", "a or b", "a, b, or c". */
export function orList(items: string[]): string {
  if (items.length < 3) return items.join(" or ");
  return `${items.slice(0, -1).join(", ")}, or ${items[items.length - 1]}`;
}

export function countText(count: number, one: string, many: string): string {
  return `${count.toLocaleString("en-US")} ${count === 1 ? one : many}`;
}
