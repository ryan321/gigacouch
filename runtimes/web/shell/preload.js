// Runs in Electron's isolated world. Nothing here is exposed to the page:
// there is no contextBridge, so the game still gets no Node or IPC access.
//
// It times frames and lists gamepads for the stats overlay. requestAnimationFrame
// in the isolated world fires on the same frames as the page's own loop, so
// a slow game shows up as long frame times here. This world also has its own
// navigator, so it sees the real gamepad list even though the page's copy of
// navigator.getGamepads is replaced by the Giga Couch input bridge.
const { ipcRenderer } = require("electron");

function sendPads() {
  const pads = Array.from((navigator.getGamepads && navigator.getGamepads()) || [])
    .filter((pad) => pad && pad.connected)
    .map((pad) => ({ index: pad.index, id: pad.id, mapping: pad.mapping }));
  ipcRenderer.send("gigacouch:pads", pads);
}

if (window.top === window) {
  const frames = [];
  // The first frames after a page load include the load itself.
  let settle = 30;
  let last = 0;
  let sentAt = 0;
  const tick = (now) => {
    if (last && settle > 0) {
      settle -= 1;
    } else if (last) {
      frames.push(now - last);
    }
    last = now;
    if (now - sentAt >= 500 && frames.length) {
      ipcRenderer.send("gigacouch:frames", frames.splice(0));
      sendPads();
      sentAt = now;
    }
    requestAnimationFrame(tick);
  };
  // A hidden page stops rAF; the gap would read as one huge frame.
  document.addEventListener("visibilitychange", () => {
    last = 0;
  });
  requestAnimationFrame(tick);
}
