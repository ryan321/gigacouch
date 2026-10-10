"use client";

import { downloadSize, downloadProgress } from "@/lib/download-format";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import { useDownloads, type Download } from "./Downloads";
import { BUTTON, PAD_SLOTS, PressWatcher, buttonNames, connectedPads, padName, type ButtonNames } from "./pads";

type Props = {
  game: { downloadBytes?: number; id: string; slug: string; title: string; playersMax: number; gamepad: boolean; keyboard: boolean; mouse: boolean };
  src: string;
  controls: string;
};

type Slot = { name: string; pressed: boolean } | null;

type MenuItem = "resume" | "restart" | "fullscreen" | "leave";
const MENU: MenuItem[] = ["resume", "restart", "fullscreen", "leave"];

/** How long both center buttons are held to open the menu. */
const HOLD_MS = 800;

export function Player({ game, src, controls }: Props) {
  const router = useRouter();
  const stage = useRef<HTMLDivElement>(null);
  const frame = useRef<HTMLIFrameElement>(null);
  const launchButton = useRef<HTMLButtonElement>(null);
  const menuButtons = useRef<(HTMLButtonElement | null)[]>([]);
  const counted = useRef(false);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [started, setStarted] = useState(false);
  const {manager, state:downloads} = useDownloads();
  const download: Download | {status:"idle"; loaded:number; total:number; message?:string} = downloads.entries.find(item=>item.src===src) ?? {status:"idle",loaded:0,total:game.downloadBytes ?? 0};
  const waiting = ["queued","downloading","checking","removing"].includes(download.status);
  const [run, setRun] = useState(0);
  const [slots, setSlots] = useState<Slot[]>(Array(PAD_SLOTS).fill(null));
  const [names, setNames] = useState<ButtonNames>(buttonNames(null));
  const [fullscreen, setFullscreen] = useState(false);
  const [barShown, setBarShown] = useState(true);
  const [menuOpen, setMenuOpen] = useState(false);
  const [selected, setSelected] = useState(0);
  const [hold, setHold] = useState(0);
  const [hint, setHint] = useState(false);
  const [note, setNote] = useState("");

  // The pad loop reads these without restarting when they change.
  const live = useRef({ started, menuOpen, selected });
  live.current = { started, menuOpen, selected };

  // Give the game the keyboard. Focusing the frame's window, not just the
  // frame element, is what moves focus into a game on another origin.
  const focusGame = useCallback(() => {
    frame.current?.focus();
    frame.current?.contentWindow?.focus();
  }, []);

  const start = useCallback(() => {
    if (!manager || waiting) return;
    if (download.status !== "ready") {
      void manager.download({...game,src,controls});
      return;
    }
    // Recent Chrome counts a controller press as a user action, so full
    // screen usually works from a pad too. If it's refused, the game starts
    // in the window.
    stage.current?.requestFullscreen?.().catch(() => {});
    setStarted(true);
    if (!counted.current) {
      counted.current = true;
      fetch(`/api/games/${game.id}/plays`, { method: "POST", keepalive: true }).catch(() => {});
    }
  }, [game, download.status, manager, waiting, src, controls]);

  useEffect(() => {
    if (download.status === "ready" || download.status === "error") launchButton.current?.focus();
  }, [download.status]);

  const openMenu = useCallback(() => {
    setNote("");
    setSelected(0);
    setMenuOpen(true);
  }, []);

  const closeMenu = useCallback(() => {
    setMenuOpen(false);
    setNote("");
    focusGame();
  }, [focusGame]);

  const choose = useCallback(
    (item: MenuItem, fromPad: boolean) => {
      if (item === "resume") closeMenu();
      else if (item === "restart") {
        setRun((count) => count + 1);
        closeMenu();
      } else if (item === "leave") {
        if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
        router.push(`/games/${game.slug}`);
      } else if (document.fullscreenElement) {
        document.exitFullscreen().catch(() => {});
        closeMenu();
      } else {
        stage.current
          ?.requestFullscreen?.()
          .then(closeMenu)
          .catch(() =>
            setNote(fromPad ? "Chrome needs a click or a key press to go full screen. Click Full screen, or press Enter." : "Chrome didn't allow full screen. Try again."),
          );
      }
    },
    [closeMenu, game.slug, router],
  );

  // One loop reads the pads for the whole page: the player list before the
  // game, starting with the bottom face button, and the system menu.
  useEffect(() => {
    const watcher = new PressWatcher();
    let handle = 0;
    let lastSlots = "";
    let holdSince: number | null = null;
    let holdUsed = false;
    let singleSince: number | null = null;
    const loop = () => {
      const now = performance.now();
      const pads = watcher.read();
      const { started: playing, menuOpen: open, selected: index } = live.current;

      const next = connectedPads().map((pad) => (pad ? { name: padName(pad), pressed: pad.buttons.some((b) => b.pressed) } : null));
      const key = JSON.stringify(next);
      if (key !== lastSlots) {
        lastSlots = key;
        setSlots(next);
        setNames(buttonNames(connectedPads().find(Boolean)));
      }

      const any = (button: number) => pads.some((entry) => entry.pressed.has(button));
      if (!playing) {
        if (any(BUTTON.south)) start();
      } else {
        // Holding both center buttons opens or closes the menu.
        const chord = pads.some((entry) => entry.held.has(BUTTON.select) && entry.held.has(BUTTON.start));
        // Holding just one of the two buttons shows how to open the menu.
        const single = !chord && !open && pads.some((entry) => entry.held.has(BUTTON.select) || entry.held.has(BUTTON.start));
        if (single) singleSince ??= now;
        else singleSince = null;
        setHint(singleSince !== null && now - singleSince > 350);
        if (!chord) {
          holdSince = null;
          holdUsed = false;
          setHold(0);
        } else if (!holdUsed) {
          holdSince ??= now;
          const progress = Math.min(1, (now - holdSince) / HOLD_MS);
          setHold(progress);
          if (progress >= 1) {
            holdUsed = true;
            setHold(0);
            if (open) closeMenu();
            else openMenu();
          }
        }
        if (any(BUTTON.home)) {
          if (open) closeMenu();
          else openMenu();
        } else if (open && !chord) {
          if (any(BUTTON.up)) setSelected((index + MENU.length - 1) % MENU.length);
          if (any(BUTTON.down)) setSelected((index + 1) % MENU.length);
          if (any(BUTTON.south)) choose(MENU[index], true);
          if (any(BUTTON.east)) closeMenu();
        }
      }
      handle = requestAnimationFrame(loop);
    };
    handle = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(handle);
  }, [choose, closeMenu, openMenu, start]);

  // The selected menu item has focus, so the keyboard works there too.
  useEffect(() => {
    if (menuOpen) menuButtons.current[selected]?.focus();
  }, [menuOpen, selected]);

  useEffect(() => {
    const changed = () => {
      const now = Boolean(document.fullscreenElement);
      setFullscreen(now);
      setBarShown(!now);
      if (!live.current.menuOpen) focusGame();
    };
    document.addEventListener("fullscreenchange", changed);
    return () => document.removeEventListener("fullscreenchange", changed);
  }, [focusGame]);

  // In full screen the bar hides; moving to the top edge brings it back.
  const peek = () => {
    setBarShown(true);
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      if (document.fullscreenElement) setBarShown(false);
    }, 2500);
  };

  const seen = slots.filter(Boolean).length;
  const shownSlots = slots.slice(0, Math.min(PAD_SLOTS, Math.max(game.playersMax, 1)));
  const labels: Record<MenuItem, string> = {
    resume: "Resume",
    restart: "Restart game",
    fullscreen: fullscreen ? "Leave full screen" : "Full screen",
    leave: "Leave game",
  };

  return (
    <div className="play" ref={stage}>
      {started ? (
        <>
          <iframe
            key={run}
            ref={frame}
            src={src}
            title={game.title}
            allow="gamepad; fullscreen; autoplay"
            sandbox="allow-scripts allow-same-origin allow-pointer-lock allow-forms allow-modals allow-popups allow-downloads"
            onLoad={focusGame}
          />
          {fullscreen && <div aria-hidden="true" onMouseMove={peek} style={{ position: "absolute", top: 0, left: 0, right: 0, height: 14, zIndex: 1 }} />}
          <div className={barShown && !menuOpen ? "play-bar" : "play-bar hidden"} onMouseMove={peek}>
            <button type="button" className="button secondary small" onClick={openMenu}>Menu</button>
            <span className="title">{game.title}</span>
            <span className="spacer" />
            {game.gamepad && <span className="pads">Menu on a controller: hold {names.menu}</span>}
          </div>
          {hint && !menuOpen && hold <= 0.15 && (
            <div className="hold-toast" role="status">
              <span>Hold {names.menu} together to open the menu</span>
            </div>
          )}
          {hold > 0.15 && !menuOpen && (
            <div className="hold-toast" role="status">
              <span>Keep holding to open the menu</span>
              <div className="progress" aria-hidden="true"><i style={{ width: `${Math.round(hold * 100)}%` }} /></div>
            </div>
          )}
          {menuOpen && (
            <div className="system-menu" role="dialog" aria-modal="true" aria-labelledby="system-menu-title">
              <div
                className="menu-panel"
                onKeyDown={(event) => {
                  if (event.key === "ArrowDown") setSelected((selected + 1) % MENU.length);
                  else if (event.key === "ArrowUp") setSelected((selected + MENU.length - 1) % MENU.length);
                  else if (event.key === "Escape") closeMenu();
                  else return;
                  event.preventDefault();
                }}
              >
                <h2 id="system-menu-title">{game.title}</h2>
                <div className="menu-items">
                  {MENU.map((item, index) => (
                    <button
                      key={item}
                      ref={(element) => {
                        menuButtons.current[index] = element;
                      }}
                      type="button"
                      className={index === selected ? "menu-item selected" : "menu-item"}
                      onFocus={() => setSelected(index)}
                      onClick={() => choose(item, false)}
                    >
                      {labels[item]}
                    </button>
                  ))}
                </div>
                {note && <p className="menu-note" role="alert">{note}</p>}
                <p className="muted menu-help">
                  {names.south} to choose. {names.east} to go back to the game.
                </p>
              </div>
            </div>
          )}
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
              <button ref={launchButton} type="button" className="button big" autoFocus disabled={!manager || waiting} onClick={start}>
                {download.status === "ready" ? "Play" : download.status === "idle" ? "Download" : download.status === "paused" ? "Resume download" : download.status === "error" ? "Retry download" : download.status === "checking" ? "Checking…" : download.status === "queued" ? "Queued…" : download.status === "removing" ? "Removing…" : "Downloading…"}
              </button>
              <span className="muted">
                {game.gamepad ? `Or press ${names.south} on a controller.` : "Or press Enter."}
              </span>
            </div>
            {!!(download.total || game.downloadBytes) && <p className="muted">Download size: {downloadSize(download.total || game.downloadBytes || 0)}</p>}
            <p className="muted">Saved in this browser’s storage on this device, not your Downloads folder. Clearing browser data removes saved games.</p>
            {downloads.error && <p role="alert">{downloads.error}</p>}
            {download.status !== "idle" && <div className="game-download" aria-live="polite">
              {download.status === "downloading" && <>
                <progress aria-label="Game download" max={100} value={download.total ? Math.min(99, Math.floor(download.loaded / download.total * 100)) : 0} />
                <p className="muted">{downloadProgress(download.loaded, download.total)}</p>
              </>}
              {download.status === "ready" && <p>Download complete. Ready to play.</p>}
              {download.status === "error" && <p role="alert">{download.message}</p>}
            </div>}
            {!started && <Link className="button secondary" href="/games">Keep browsing games</Link>}
            {download.status === "idle" && <p className="muted">{downloads.entries.some(item=>item.id===game.id && item.src!==src) ? "A new version is available. Download it when you’re ready; your saved version stays in Downloaded games." : "Download the game first, then press Play when you’re ready."}</p>}
            <p className="muted">
              {game.gamepad
                ? `During the game, hold ${names.menu} together for the menu. Esc leaves full screen.`
                : "During the game, use Menu at the top of the screen. Esc leaves full screen."}
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
