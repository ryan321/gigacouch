const { app, BrowserWindow, Menu, WebContentsView, dialog, ipcMain } = require("electron");
const { spawn } = require("child_process");
const fs = require("fs");
const path = require("path");
const stats = require("./stats");
const controllers = require("./controllers");

const ORIGIN_PATTERN = /^http:\/\/127\.0\.0\.1:\d+$/;
const PLATFORM = process.env.GIGACOUCH_PLATFORM || "https://gigacouch-platform.fly.dev";

// Set when the couch CLI launched us and already runs the host. The packaged
// app leaves it empty and starts its bundled host instead.
let origin = process.env.GIGACOUCH_ORIGIN || "";
let host = null;

app.commandLine.appendSwitch("autoplay-policy", "no-user-gesture-required");
// Keep Chromium's own cache beside the host's saves instead of a second
// "Giga Couch" folder in Application Support.
app.setPath("userData", path.join(app.getPath("appData"), "GigaCouch", "browser"));

// The packaged app carries the couch binary and the Home files in Resources.
// The host prints one JSON line with its origin, then serves until we close
// its stdin.
function startBundledHost() {
  const resources = process.resourcesPath;
  const binary = path.join(resources, "couch");
  const root = path.join(resources, "gigacouch");
  return new Promise((resolve, reject) => {
    const child = spawn(
      binary,
      ["--json", "web-home", "--serve-only", "--platform", PLATFORM],
      { env: { ...process.env, GIGACOUCH_ROOT: root }, stdio: ["pipe", "pipe", "inherit"] },
    );
    host = child;
    let buffered = "";
    const onData = (chunk) => {
      buffered += chunk.toString("utf8");
      const newline = buffered.indexOf("\n");
      if (newline < 0) {
        return;
      }
      child.stdout.off("data", onData);
      child.stdout.resume();
      try {
        const announced = JSON.parse(buffered.slice(0, newline)).origin || "";
        if (ORIGIN_PATTERN.test(announced)) {
          resolve(announced);
        } else {
          reject(new Error(`The Giga Couch host gave an unexpected address: ${announced}`));
        }
      } catch (error) {
        reject(new Error(`The Giga Couch host did not start: ${error.message}`));
      }
    };
    child.stdout.on("data", onData);
    child.on("error", reject);
    child.on("exit", (code) => {
      host = null;
      if (!origin) {
        reject(new Error(`The Giga Couch host stopped before it was ready (exit ${code}).`));
      } else {
        app.quit();
      }
    });
  });
}

function stopBundledHost() {
  if (host && host.stdin && !host.stdin.destroyed) {
    host.stdin.end();
  }
}

function sameOrigin(url) {
  return url === origin || url.startsWith(`${origin}/`);
}

// ---- Stats overlay -------------------------------------------------------
// A separate view layered over the game. The game page never loads it, sees
// it, or reaches it; it only receives frame timings from our preload.

const OVERLAY_MARGIN = 16;
const OVERLAY_WIDTH = 302;
const MAX_FRAMES = 600;
let mainWindow = null;
let overlay = null;
let overlayTimer = null;
let overlayHeight = 320;
let machine = null;
let frameTimes = [];
let gamepads = [];
let padHistory = null;

function prefsPath() {
  return path.join(app.getPath("userData"), "overlay.json");
}

function overlayWanted() {
  try {
    return JSON.parse(fs.readFileSync(prefsPath(), "utf8")).visible === true;
  } catch (_error) {
    return false;
  }
}

function rememberOverlay(visible) {
  try {
    fs.writeFileSync(prefsPath(), JSON.stringify({ visible }));
  } catch (_error) {
    /* Not remembering the toggle is harmless. */
  }
}

ipcMain.on("gigacouch:frames", (event, list) => {
  if (!mainWindow || event.sender !== mainWindow.webContents || !Array.isArray(list)) {
    return;
  }
  for (const value of list.slice(0, 1000)) {
    if (typeof value === "number" && value > 0 && value < 5000) {
      frameTimes.push(value);
    }
  }
  if (frameTimes.length > MAX_FRAMES) {
    frameTimes = frameTimes.slice(-MAX_FRAMES);
  }
});

ipcMain.on("gigacouch:pads", (event, list) => {
  if (!mainWindow || event.sender !== mainWindow.webContents || !Array.isArray(list)) {
    return;
  }
  gamepads = list
    .slice(0, 16)
    .filter((pad) => pad && Number.isInteger(pad.index) && typeof pad.id === "string")
    .map((pad) => ({ index: pad.index, id: pad.id.slice(0, 200), mapping: String(pad.mapping || "") }));
});

// Frame rate over the last second, and the 1% low over the last few seconds:
// the rate the slowest 1% of frames would sustain.
function frameSummary() {
  let spent = 0;
  let count = 0;
  for (let i = frameTimes.length - 1; i >= 0 && spent < 1000; i -= 1) {
    spent += frameTimes[i];
    count += 1;
  }
  if (!count) {
    return { fps: null, frameMs: null, lowFps: null, frames: [] };
  }
  const recent = frameTimes.slice(-300).sort((a, b) => a - b);
  const slowest = recent[Math.min(recent.length - 1, Math.floor(recent.length * 0.99))];
  return {
    fps: (count * 1000) / spent,
    frameMs: spent / count,
    lowFps: 1000 / slowest,
    frames: frameTimes.slice(-120),
  };
}

function layoutOverlay() {
  if (!overlay || !mainWindow || mainWindow.isDestroyed()) {
    return;
  }
  const [width, height] = mainWindow.getContentSize();
  // A short window scrolls the panel instead of cutting it off.
  overlay.setBounds({
    x: Math.max(0, width - OVERLAY_WIDTH - OVERLAY_MARGIN),
    y: OVERLAY_MARGIN,
    width: OVERLAY_WIDTH,
    height: Math.max(80, Math.min(overlayHeight, height - OVERLAY_MARGIN * 2)),
  });
}

async function refreshOverlay() {
  if (!overlay || !mainWindow || mainWindow.isDestroyed()) {
    return;
  }
  try {
    machine = machine || (await stats.identity());
    padHistory = padHistory || new controllers.ControllerHistory(path.join(app.getPath("userData"), "controllers.json"));
    const [reading, pads] = await Promise.all([
      stats.sample(mainWindow),
      controllers.summary(gamepads, padHistory),
    ]);
    if (!overlay || overlay.webContents.isDestroyed()) {
      return;
    }
    const height = await overlay.webContents.executeJavaScript(
      `window.gigacouchRender(${JSON.stringify({ ...machine, ...reading, ...frameSummary(), pads })})`,
    );
    if (Number.isFinite(height) && height > 0 && height !== overlayHeight) {
      overlayHeight = height;
      layoutOverlay();
    }
  } catch (_error) {
    /* A missed reading is skipped; the next tick tries again. */
  }
}

function showOverlay() {
  if (overlay || !mainWindow) {
    return;
  }
  overlay = new WebContentsView({
    webPreferences: { sandbox: true, contextIsolation: true, nodeIntegration: false },
  });
  overlay.setBackgroundColor("#00000000");
  overlay.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  overlay.webContents.on("will-navigate", (event) => event.preventDefault());
  mainWindow.contentView.addChildView(overlay);
  layoutOverlay();
  overlay.webContents.loadFile(path.join(__dirname, "overlay.html")).then(refreshOverlay);
  overlayTimer = setInterval(refreshOverlay, 1000);
  mainWindow.webContents.focus();
}

function hideOverlay() {
  clearInterval(overlayTimer);
  overlayTimer = null;
  const view = overlay;
  overlay = null;
  if (!view) {
    return;
  }
  // During quit the window may already be gone; touching it would throw and
  // leave an error dialog blocking the quit.
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.contentView.removeChildView(view);
  }
  if (!view.webContents.isDestroyed()) {
    view.webContents.close();
  }
}

function setOverlay(visible) {
  if (visible) {
    showOverlay();
  } else {
    hideOverlay();
  }
  rememberOverlay(visible);
  const item = Menu.getApplicationMenu() && Menu.getApplicationMenu().getMenuItemById("stats-overlay");
  if (item) {
    item.checked = visible;
  }
}

// Quit and full screen always have keyboard shortcuts: Cmd+Q / Ctrl+Q and
// Ctrl+Cmd+F / F11. The menu bar itself stays hidden in full screen.
function installMenu() {
  const template = [
    {
      label: app.name,
      submenu: [
        ...(process.platform === "darwin" ? [{ role: "hide" }, { type: "separator" }] : []),
        { role: "quit" },
      ],
    },
    {
      label: "View",
      submenu: [
        { role: "togglefullscreen" },
        {
          id: "stats-overlay",
          label: "Stats Overlay",
          type: "checkbox",
          accelerator: "CmdOrCtrl+I",
          checked: overlayWanted(),
          click: (item) => setOverlay(item.checked),
        },
      ],
    },
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

function createWindow() {
  const windowed = process.env.GIGACOUCH_WINDOWED === "1";
  // Kiosk locks the whole computer: on macOS it blocks app switching and
  // Force Quit. Only a dedicated couch machine should ask for it.
  const kiosk = !windowed && process.env.GIGACOUCH_KIOSK === "1";
  installMenu();
  const win = new BrowserWindow({
    width: 1280,
    height: 720,
    fullscreen: !windowed,
    kiosk,
    autoHideMenuBar: true,
    backgroundColor: "#101722",
    title: process.env.GIGACOUCH_TITLE || "Giga Couch",
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: true,
      webSecurity: true,
      devTools: process.env.GIGACOUCH_DEVTOOLS === "1",
    },
  });

  win.webContents.on("before-input-event", (event, input) => {
    if (input.type !== "keyDown" || input.key !== "Escape") {
      return;
    }
    const current = win.webContents.getURL();
    if (current.includes("/play/")) {
      event.preventDefault();
      win.loadURL(`${origin}/`);
    } else if (win.isFullScreen() && !kiosk) {
      // On the shelf, Escape leaves full screen like any other Mac app.
      event.preventDefault();
      win.setFullScreen(false);
    }
  });
  win.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  win.webContents.on("will-navigate", (event, url) => {
    if (!sameOrigin(url)) {
      event.preventDefault();
    }
  });
  win.webContents.on("will-redirect", (event, url) => {
    if (!sameOrigin(url)) {
      event.preventDefault();
    }
  });
  win.webContents.on("will-attach-webview", (event) => event.preventDefault());
  win.webContents.session.setPermissionRequestHandler((_contents, _permission, callback) => {
    callback(false);
  });

  const quitPoll = setInterval(async () => {
    try {
      const response = await fetch(`${origin}/__gigacouch/v1/control`);
      const body = await response.json();
      if (body.quit) {
        clearInterval(quitPoll);
        app.quit();
      }
    } catch (_error) {
      /* The origin is local; a refused connection means it is already gone. */
    }
  }, 400);

  win.webContents.on("did-finish-load", async () => {
    win.focus();
    win.webContents.focus();
    if (process.env.GIGACOUCH_DEMO_JOIN === "1" || process.env.GIGACOUCH_DEMO_RIGHT === "1") {
      await new Promise((resolve) => setTimeout(resolve, 800));
    }
    if (process.env.GIGACOUCH_DEMO_JOIN === "1") {
      win.webContents.sendInputEvent({ type: "keyDown", keyCode: "Return" });
      await new Promise((resolve) => setTimeout(resolve, 250));
      win.webContents.sendInputEvent({ type: "keyUp", keyCode: "Return" });
    }
    if (process.env.GIGACOUCH_DEMO_RIGHT === "1") {
      await new Promise((resolve) => setTimeout(resolve, 400));
      win.webContents.sendInputEvent({ type: "keyDown", keyCode: "Right" });
      await new Promise((resolve) => setTimeout(resolve, 120));
      win.webContents.sendInputEvent({ type: "keyUp", keyCode: "Right" });
    }
    if (process.env.GIGACOUCH_SCREENSHOT) {
      await new Promise((resolve) => setTimeout(resolve, 700));
      const image = await win.webContents.capturePage();
      fs.writeFileSync(process.env.GIGACOUCH_SCREENSHOT, image.toPNG());
      if (overlay) {
        await new Promise((resolve) => setTimeout(resolve, 2500));
        await refreshOverlay();
        const panel = await overlay.webContents.capturePage();
        fs.writeFileSync(process.env.GIGACOUCH_SCREENSHOT.replace(/\.png$/, "") + "-overlay.png", panel.toPNG());
      }
      if (process.env.GIGACOUCH_SCREENSHOT_QUIT === "1") {
        app.quit();
      }
    }
  });

  mainWindow = win;
  win.on("resize", layoutOverlay);
  // Clean up while the window still exists, not after it is destroyed.
  win.on("close", hideOverlay);
  win.on("closed", () => {
    mainWindow = null;
  });
  win.loadURL(`${origin}/`);
  if (overlayWanted()) {
    showOverlay();
  }
}

async function start() {
  if (!origin && app.isPackaged) {
    try {
      origin = await startBundledHost();
    } catch (error) {
      dialog.showErrorBox("Giga Couch could not start", error.message);
      app.quit();
      return;
    }
  }
  if (!ORIGIN_PATTERN.test(origin)) {
    console.error("GIGACOUCH_ORIGIN must be http://127.0.0.1:<port>");
    app.exit(1);
    return;
  }
  createWindow();
}

app.whenReady().then(start);
app.on("window-all-closed", () => app.quit());
app.on("will-quit", stopBundledHost);
