"use client";

import { useEffect, useRef, useState } from "react";
import { PAD_SLOTS, anyPressed, canRumble, connectedPads, padName, rumble } from "./pads";

type Snapshot = {
  slots: boolean[];
  chosen: number | null;
  name: string;
  standard: boolean;
  rumble: boolean;
  buttons: number[];
  axes: number[];
};

const EMPTY: Snapshot = { slots: Array(PAD_SLOTS).fill(false), chosen: null, name: "", standard: true, rumble: false, buttons: [], axes: [] };

/** Standard-layout button numbers, as Chrome reports them. */
const B = { south: 0, east: 1, west: 2, north: 3, lb: 4, rb: 5, lt: 6, rt: 7, back: 8, start: 9, l3: 10, r3: 11, up: 12, down: 13, left: 14, right: 15, home: 16 };

/**
 * A drawing of a controller that follows a real pad: buttons light up,
 * sticks move, and triggers fill as they're pulled.
 */
export function PadPanel() {
  const [snap, setSnap] = useState<Snapshot>(EMPTY);
  const picked = useRef<number | null>(null);
  const followed = useRef<number | null>(null);

  useEffect(() => {
    let frame = 0;
    let last = "";
    const tick = () => {
      const pads = connectedPads();
      pads.forEach((pad, index) => {
        if (pad && anyPressed(pad)) followed.current = index;
      });
      let chosen = picked.current ?? followed.current;
      if (chosen === null || !pads[chosen]) chosen = pads.findIndex(Boolean);
      if (chosen < 0) chosen = null;
      const pad = chosen === null ? null : pads[chosen];
      const next: Snapshot = pad
        ? {
            slots: pads.map(Boolean),
            chosen,
            name: padName(pad),
            standard: pad.mapping === "standard",
            rumble: canRumble(pad),
            buttons: pad.buttons.map((button) => Math.round(button.value * 20) / 20),
            axes: pad.axes.map((axis) => Math.round(axis * 20) / 20),
          }
        : EMPTY;
      const key = JSON.stringify(next);
      if (key !== last) {
        last = key;
        setSnap(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, []);

  const on = (index: number) => (snap.buttons[index] ?? 0) > 0.5;
  const part = (index: number) => (on(index) ? "pad-part on" : "pad-part");
  const stick = (x: number, y: number, press: number, axisX: number, axisY: number) => (
    <g>
      <circle cx={x} cy={y} r={21} className="pad-part" />
      <circle cx={x + (snap.axes[axisX] ?? 0) * 8} cy={y + (snap.axes[axisY] ?? 0) * 8} r={13} className={on(press) ? "pad-stick on" : "pad-stick"} />
    </g>
  );
  const trigger = (x: number, index: number) => (
    <g>
      <rect x={x} y={6} width={44} height={13} rx={6} className="pad-part" />
      <rect x={x} y={6} width={44 * (snap.buttons[index] ?? 0)} height={13} rx={6} className="pad-trigger-fill" />
    </g>
  );

  return (
    <div className="pad-panel">
      <svg viewBox="0 0 360 220" role="img" aria-label={snap.chosen === null ? "A controller waiting for a button press" : `${snap.name}, live`}>
        {trigger(84, B.lt)}
        {trigger(232, B.rt)}
        <rect x={72} y={25} width={72} height={13} rx={6} className={part(B.lb)} />
        <rect x={216} y={25} width={72} height={13} rx={6} className={part(B.rb)} />
        <path className="pad-body" d="M92 40H268C300 40 318 58 326 86L350 168C358 196 340 214 318 210C300 207 290 192 276 172L262 152H98L84 172C70 192 60 207 42 210C20 214 2 196 10 168L34 86C42 58 60 40 92 40Z" />
        <rect x={92} y={80} width={17} height={21} rx={3} className={part(B.up)} />
        <rect x={92} y={119} width={17} height={21} rx={3} className={part(B.down)} />
        <rect x={71} y={101} width={21} height={18} rx={3} className={part(B.left)} />
        <rect x={109} y={101} width={21} height={18} rx={3} className={part(B.right)} />
        <rect x={92} y={101} width={17} height={18} className="pad-part" />
        <circle cx={266} cy={80} r={11} className={part(B.north)} />
        <circle cx={266} cy={128} r={11} className={part(B.south)} />
        <circle cx={242} cy={104} r={11} className={part(B.west)} />
        <circle cx={290} cy={104} r={11} className={part(B.east)} />
        <rect x={150} y={86} width={22} height={10} rx={5} className={part(B.back)} />
        <rect x={188} y={86} width={22} height={10} rx={5} className={part(B.start)} />
        <circle cx={180} cy={118} r={8} className={part(B.home)} />
        {stick(138, 150, B.l3, 0, 1)}
        {stick(222, 150, B.r3, 2, 3)}
      </svg>
      <div className="pad-status" aria-live="polite">
        <div>
          {snap.chosen === null ? (
            <>
              <strong>Press any button on a controller</strong>
              <span>Chrome shows a pad to a page after its first button press.</span>
            </>
          ) : (
            <>
              <strong>{snap.name}</strong>
              <span>{snap.standard ? "Standard layout, so games read it the usual way." : "No standard layout, so games may read its buttons differently."}</span>
            </>
          )}
        </div>
        <div className="slots" role="group" aria-label="Controllers Chrome sees">
          {snap.slots.map((connected, index) => (
            <button
              key={index}
              type="button"
              className={`slot${connected ? " connected" : ""}${snap.chosen === index ? " chosen" : ""}`}
              disabled={!connected}
              aria-pressed={snap.chosen === index}
              aria-label={connected ? `Show controller ${index + 1}` : `No controller ${index + 1}`}
              onClick={() => {
                picked.current = index;
                const pad = connectedPads()[index];
                if (pad) rumble(pad, 150);
              }}
            >
              {index + 1}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
