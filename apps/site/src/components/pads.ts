"use client";

/** Chrome shows a page at most four pads. */
export const PAD_SLOTS = 4;

/** A pad's name without the vendor and product codes browsers add. */
export function padName(pad: Gamepad): string {
  const name = pad.id
    .replace(/\s*\(.*\)\s*$/, "")
    .replace(/^[0-9a-f]{4}-[0-9a-f]{4}-/i, "")
    .trim();
  return name || "Controller";
}

export function connectedPads(): (Gamepad | null)[] {
  const list = typeof navigator !== "undefined" && navigator.getGamepads ? Array.from(navigator.getGamepads()) : [];
  return Array.from({ length: PAD_SLOTS }, (_, index) => list[index] ?? null);
}

export function anyPressed(pad: Gamepad): boolean {
  return pad.buttons.some((button) => button.pressed) || pad.axes.some((axis) => Math.abs(axis) > 0.5);
}

type Haptics = { playEffect?: (type: string, params: Record<string, number>) => Promise<unknown> };

export function canRumble(pad: Gamepad): boolean {
  return typeof (pad.vibrationActuator as Haptics | null | undefined)?.playEffect === "function";
}

export function rumble(pad: Gamepad, duration = 250): void {
  const haptics = pad.vibrationActuator as Haptics | null | undefined;
  haptics?.playEffect?.("dual-rumble", { duration, strongMagnitude: 0.8, weakMagnitude: 0.6 })?.catch(() => {});
}
