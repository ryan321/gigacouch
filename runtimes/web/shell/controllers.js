// Controllers for the stats overlay.
//
// "Connected" is what Chromium's Gamepad API reports: the pads a web game can
// use right now. Chromium only lists a pad after one of its buttons has been
// pressed while the window is open.
//
// "Known" pads are ones seen before but not connected now. They come from two
// places: pads this app has seen (kept in controllers.json under userData),
// and, on macOS, game controllers paired over Bluetooth.
const { execFile } = require("child_process");
const fs = require("fs");
const path = require("path");

// USB vendor IDs of common controller makers, for Bluetooth devices whose
// minor type is not reported as a gamepad.
const PAD_VENDORS = new Set(["045e", "054c", "057e", "2dc8", "0f0d", "20d6", "0e6f", "28de", "046d", "1532", "0079"]);
const PAD_WORDS = /controller|gamepad|joy-?con|dualsense|dualshock|xbox|8bitdo|pro controller|joystick/i;
const MAX_KNOWN = 24;

function hex(value) {
  const text = String(value || "").toLowerCase().replace(/^0x/, "");
  return text ? text.padStart(4, "0") : null;
}

// "Xbox Wireless Controller (STANDARD GAMEPAD Vendor: 045e Product: 0b13)"
// "Xbox 360 Controller (XInput STANDARD GAMEPAD)"
// "045e-0b13-Xbox Wireless Controller"  (Firefox style, just in case)
function parsePadId(id) {
  const text = String(id || "");
  const vendor = /Vendor: ([0-9a-f]{4})/i.exec(text);
  const product = /Product: ([0-9a-f]{4})/i.exec(text);
  const dashed = /^([0-9a-f]{4})-([0-9a-f]{4})-(.+)$/i.exec(text);
  let name = dashed ? dashed[3] : text.replace(/\s*\([^)]*\)\s*$/, "");
  name = name.trim() || "Controller";
  return {
    name,
    vendor: vendor ? hex(vendor[1]) : dashed ? hex(dashed[1]) : null,
    product: product ? hex(product[1]) : dashed ? hex(dashed[2]) : null,
  };
}

function keyFor(pad) {
  return pad.vendor && pad.product ? `${pad.vendor}:${pad.product}` : `name:${pad.name.toLowerCase()}`;
}

let bluetoothCache = { at: 0, pads: [] };

function readBluetooth() {
  return new Promise((resolve) => {
    if (process.platform !== "darwin") {
      resolve([]);
      return;
    }
    execFile("system_profiler", ["SPBluetoothDataType", "-json"], { timeout: 4000 }, (error, stdout) => {
      if (error) {
        resolve([]);
        return;
      }
      try {
        const root = JSON.parse(stdout).SPBluetoothDataType[0] || {};
        const pads = [];
        for (const [group, connected] of [["device_connected", true], ["device_not_connected", false]]) {
          for (const entry of root[group] || []) {
            for (const [name, info] of Object.entries(entry)) {
              const vendor = hex(info.device_vendorID);
              const isPad = /game ?pad|joystick/i.test(info.device_minorType || "")
                || PAD_WORDS.test(name)
                || (vendor && PAD_VENDORS.has(vendor) && !/keyboard|mouse|trackpad|headset|audio/i.test(info.device_minorType || ""));
              if (isPad) {
                pads.push({
                  name,
                  vendor,
                  product: hex(info.device_productID),
                  connected,
                  battery: info.device_batteryLevelMain || null,
                });
              }
            }
          }
        }
        resolve(pads);
      } catch (_error) {
        resolve([]);
      }
    });
  });
}

// Pairing changes rarely; ask the system every 10 seconds at most.
async function bluetoothPads() {
  if (Date.now() - bluetoothCache.at > 10000) {
    bluetoothCache = { at: Date.now(), pads: await readBluetooth() };
  }
  return bluetoothCache.pads;
}

class ControllerHistory {
  constructor(file) {
    this.file = file;
    this.known = {};
    try {
      this.known = JSON.parse(fs.readFileSync(file, "utf8")).known || {};
    } catch (_error) {
      this.known = {};
    }
  }

  // Record the pads seen now. Only writes when something changed.
  note(pads) {
    const today = new Date().toISOString().slice(0, 10);
    const counts = {};
    let changed = false;
    for (const pad of pads) {
      const key = keyFor(pad);
      counts[key] = (counts[key] || 0) + 1;
      const row = this.known[key] || { name: pad.name, most: 0 };
      if (row.lastSeen !== today || row.name !== pad.name || counts[key] > row.most) {
        row.lastSeen = today;
        row.name = pad.name;
        row.most = Math.max(row.most, counts[key]);
        this.known[key] = row;
        changed = true;
      }
    }
    if (changed) {
      const newest = Object.entries(this.known)
        .sort((a, b) => String(b[1].lastSeen).localeCompare(String(a[1].lastSeen)))
        .slice(0, MAX_KNOWN);
      this.known = Object.fromEntries(newest);
      try {
        fs.mkdirSync(path.dirname(this.file), { recursive: true });
        fs.writeFileSync(this.file, JSON.stringify({ known: this.known }, null, 2));
      } catch (_error) {
        /* Forgetting history is harmless. */
      }
    }
  }
}

// gamepads: [{ index, id, mapping }] from the page's frame.
async function summary(gamepads, history) {
  const connected = gamepads
    .map((pad) => ({ ...parsePadId(pad.id), slot: pad.index + 1, standard: pad.mapping === "standard" }))
    .sort((a, b) => a.slot - b.slot);
  history.note(connected);
  const paired = await bluetoothPads();
  for (const pad of connected) {
    const match = paired.find((bt) => bt.connected && (keyFor(bt) === keyFor(pad) || bt.name === pad.name));
    if (match && match.battery) {
      pad.battery = match.battery;
    }
  }

  // Paired and awake, but Chromium lists a pad only after a button press.
  const waiting = paired
    .filter((bt) => bt.connected && !connected.some((pad) => keyFor(pad) === keyFor(bt) || pad.name === bt.name))
    .map((bt) => ({ name: bt.name, battery: bt.battery }));

  const present = [...connected, ...paired.filter((bt) => bt.connected)];
  const online = new Set(present.map(keyFor));
  const onlineNames = new Set(present.map((pad) => pad.name.toLowerCase()));
  const known = new Map();
  for (const bt of paired) {
    if (bt.connected || online.has(keyFor(bt)) || onlineNames.has(bt.name.toLowerCase())) {
      continue;
    }
    known.set(bt.name.toLowerCase(), { name: bt.name, source: "Paired over Bluetooth" });
  }
  for (const [key, row] of Object.entries(history.known)) {
    if (online.has(key) || onlineNames.has(row.name.toLowerCase())) {
      continue;
    }
    const existing = known.get(row.name.toLowerCase());
    known.set(row.name.toLowerCase(), {
      name: row.name,
      source: existing ? existing.source : "Used here before",
      lastSeen: row.lastSeen,
      most: row.most,
    });
  }
  return { connected, known: [...known.values()], waiting };
}

module.exports = { parsePadId, keyFor, ControllerHistory, summary };
