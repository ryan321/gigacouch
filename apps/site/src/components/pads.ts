"use client";

/** Chrome shows a page at most four pads. */
export const PAD_SLOTS = 4;

/** Standard-layout button numbers, as Chrome reports them. */
export const BUTTON = { south: 0, east: 1, west: 2, north: 3, select: 8, start: 9, up: 12, down: 13, left: 14, right: 15, home: 16 };

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

/** What the buttons are called on the pad in someone's hands. */
export type ButtonNames = { south: string; east: string; menu: string; home: string };

export function buttonNames(pad: Gamepad | null | undefined): ButtonNames {
  const id = pad?.id.toLowerCase() ?? "";
  if (/xbox|xinput|045e/.test(id)) return { south: "A", east: "B", menu: "View and Menu", home: "the Xbox button" };
  if (/dualsense|0ce6|0df2/.test(id)) return { south: "Cross", east: "Circle", menu: "Create and Options", home: "the PS button" };
  if (/dualshock|playstation|054c/.test(id)) return { south: "Cross", east: "Circle", menu: "Share and Options", home: "the PS button" };
  // Nintendo pads report B in the bottom spot and A on the right.
  if (/pro controller|joy-con|057e/.test(id)) return { south: "B", east: "A", menu: "− and +", home: "the Home button" };
  return { south: "A", east: "B", menu: "Select and Start", home: "the Home button" };
}

/**
 * Turns pad state into presses, one pad at a time. A button counts once when
 * it goes down. Buttons already held when watching starts are ignored, so a
 * press from the previous page doesn't carry over.
 */
export class PressWatcher {
  private previous = new Map<number, boolean[]>();
  private readonly startedAt = performance.now();

  /** The buttons pressed since the last call, on each connected pad. */
  read(): { pad: Gamepad; pressed: Set<number>; held: Set<number> }[] {
    const now = performance.now();
    const out: { pad: Gamepad; pressed: Set<number>; held: Set<number> }[] = [];
    for (const pad of connectedPads()) {
      if (!pad) continue;
      const down = pad.buttons.map((button) => button.pressed);
      // The left stick works like the d-pad for moving through menus.
      const y = pad.axes[1] ?? 0;
      down[BUTTON.up] = down[BUTTON.up] || y < -0.6;
      down[BUTTON.down] = down[BUTTON.down] || y > 0.6;
      const before = this.previous.get(pad.index) ?? (now - this.startedAt < 400 ? down : []);
      const pressed = new Set<number>();
      const held = new Set<number>();
      down.forEach((isDown, index) => {
        if (!isDown) return;
        held.add(index);
        if (!before[index]) pressed.add(index);
      });
      this.previous.set(pad.index, down);
      out.push({ pad, pressed, held });
    }
    return out;
  }
}
