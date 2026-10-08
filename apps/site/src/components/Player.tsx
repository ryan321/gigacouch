"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { PAD_SLOTS, anyPressed, connectedPads, padName } from "./pads";

type Props = {
  game: { id: string; slug: string; title: string; playersMax: number; gamepad: boolean; keyboard: boolean; mouse: boolean };
  src: string;
  controls: string;
};

type Slot = { name: string; pressed: boolean } | null;

function readSlots(): Slot[] {
  return connectedPads().map((pad) => (pad ? { name: padName(pad), pressed: anyPressed(pad) } : null));
}

export function Player({ game, src, controls }: Props) {
  const stage = useRef<HTMLDivElement>(null);
  const frame = useRef<HTMLIFrameElement>(null);
  const counted = useRef(false);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [started, setStarted] = useState(false);
  const [slots, setSlots] = useState<Slot[]>(Array(PAD_SLOTS).fill(null));
  const [fullscreen, setFullscreen] = useState(false);
  const [barShown, setBarShown] = useState(true);

  // Give the game the keyboard. Focusing the frame's window, not just the
  // frame element, is what moves focus into a game on another origin.
  const focusGame = useCallback(() => {
    frame.current?.focus();
    frame.current?.contentWindow?.focus();
  }, []);

  // Watch the pads: every frame before the game starts, then twice a second.
  useEffect(() => {
    let last = "";
    let handle = 0;
    const update = () => {
      const next = readSlots();
      const key = JSON.stringify(next);
      if (key !== last) {
        last = key;
        setSlots(next);
      }
    };
    if (!started) {
      const loop = () => {
        update();
        handle = requestAnimationFrame(loop);
      };
      handle = requestAnimationFrame(loop);
      return () => cancelAnimationFrame(handle);
    }
    const timer = setInterval(update, 500);
    return () => clearInterval(timer);
  }, [started]);

  useEffect(() => {
    const changed = () => {
      const now = Boolean(document.fullscreenElement);
      setFullscreen(now);
      setBarShown(!now);
      focusGame();
    };
    document.addEventListener("fullscreenchange", changed);
    return () => document.removeEventListener("fullscreenchange", changed);
  }, [focusGame]);

  const start = useCallback(() => {
    stage.current?.requestFullscreen?.().catch(() => {});
    setStarted(true);
    if (!counted.current) {
      counted.current = true;
      fetch(`/api/games/${game.id}/plays`, { method: "POST", keepalive: true }).catch(() => {});
    }
  }, [game.id]);

  // In full screen the bar hides; moving to the top edge brings it back.
  const peek = () => {
    setBarShown(true);
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      if (document.fullscreenElement) setBarShown(false);
    }, 2500);
  };

  const toggleFullscreen = () => {
    if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
    else stage.current?.requestFullscreen?.().catch(() => {});
  };

  const seen = slots.filter(Boolean).length;
  const shownSlots = slots.slice(0, Math.min(PAD_SLOTS, Math.max(game.playersMax, 1)));

  return (
    <div className="play" ref={stage}>
      {started ? (
        <>
          <iframe
            ref={frame}
            src={src}
            title={game.title}
            allow="gamepad; fullscreen; autoplay"
            sandbox="allow-scripts allow-same-origin allow-pointer-lock allow-forms allow-modals allow-popups allow-downloads"
            onLoad={focusGame}
          />
          {fullscreen && <div aria-hidden="true" onMouseMove={peek} style={{ position: "absolute", top: 0, left: 0, right: 0, height: 14, zIndex: 1 }} />}
          <div className={barShown ? "play-bar" : "play-bar hidden"} onMouseMove={peek}>
            <Link className="button secondary small" href={`/games/${game.slug}`}>Leave game</Link>
            <span className="title">{game.title}</span>
            <span className="spacer" />
            {game.gamepad && <span className="pads">{seen === 1 ? "1 controller" : `${seen} controllers`}</span>}
            <button type="button" className="button secondary small" onClick={toggleFullscreen}>
              {fullscreen ? "Leave full screen" : "Full screen"}
            </button>
          </div>
        </>
      ) : (
        <div className="ready">
          <div className="ready-card">
            <Link className="back" href={`/games/${game.slug}`}>Back to the game page</Link>
            <h1>{game.title}</h1>
            <p className="muted">{controls}</p>
            {game.gamepad && (
              <>
                <div className="player-slots">
                  {shownSlots.map((slot, index) => (
                    <div key={index} className={`player-slot${slot ? " on" : ""}${slot?.pressed ? " pressed" : ""}`}>
                      <strong>Player {index + 1}</strong>
                      <span>{slot ? slot.name : "Press a button on a controller"}</span>
                    </div>
                  ))}
                </div>
                {seen === 0 && <p className="muted">Chrome only sees a controller after one of its buttons is pressed. Chrome shows up to four.</p>}
              </>
            )}
            <div className="form-actions">
              <button type="button" className="button big" autoFocus onClick={start}>Play</button>
              <span className="muted">Press Esc to leave full screen.</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
