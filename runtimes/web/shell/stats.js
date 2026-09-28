// Collects machine readings for the stats overlay. Everything here uses
// Electron, Node, or tools that ship with the operating system. Nothing is
// installed. A reading this machine cannot provide is left null, and the
// overlay leaves it out rather than guessing.
const { app, powerMonitor, screen } = require("electron");
const { execFile } = require("child_process");
const os = require("os");

function run(command, args) {
  return new Promise((resolve) => {
    execFile(command, args, { timeout: 1500 }, (error, stdout) => resolve(error ? "" : stdout));
  });
}

// Chromium reports the GPU through ANGLE, for example
//   ANGLE (Apple, ANGLE Metal Renderer: Apple M1, Version 27.0 (Build 26A428))
//   ANGLE (NVIDIA, NVIDIA GeForce RTX 3070 Direct3D11 vs_5_0 ps_5_0, D3D11)
function parseRenderer(raw) {
  const text = String(raw || "");
  if (!text.startsWith("ANGLE (")) {
    return { name: text || null, backend: null };
  }
  // Vendor first, backend last; the renderer in between may contain commas.
  const parts = text.slice(7, -1).split(", ");
  const renderer = parts.slice(1, -1).join(", ");
  const metal = /ANGLE (\w+) Renderer: (.+)/.exec(renderer);
  if (metal) {
    return { name: metal[2], backend: metal[1] };
  }
  const name = renderer
    .replace(/\s+(Direct3D|OpenGL|Vulkan)\S*(\s+\S+_\d+_\d+)*$/, "")
    .replace(/\s*\(.*\)$/, "")
    .trim();
  const backend = (parts[parts.length - 1] || "").replace(/\s+[\d.]+$/, "").replace(/^D3D/, "Direct3D ");
  return { name: name || text, backend: backend || null };
}

function enabled(status) {
  return typeof status === "string" && status.startsWith("enabled");
}

async function identity() {
  const info = await app.getGPUInfo("complete").catch(() => ({}));
  const features = app.getGPUFeatureStatus();
  const renderer = parseRenderer(info.auxAttributes && info.auxAttributes.glRenderer);
  return {
    gpu: renderer.name,
    backend: renderer.backend,
    hardware: enabled(features.gpu_compositing) && enabled(features.webgl),
    webgl: enabled(features.webgl),
    webgpu: enabled(features.webgpu),
    cpu: (os.cpus()[0] || {}).model || null,
    cores: os.cpus().length,
    totalMemory: os.totalmem(),
  };
}

// Activity Monitor's "Memory Used": app memory plus wired plus compressed.
async function macMemoryUsed() {
  const text = await run("vm_stat", []);
  const page = Number((/page size of (\d+)/.exec(text) || [])[1]);
  const pages = (label) => Number((new RegExp(`${label}:\\s+(\\d+)`).exec(text) || [])[1] || 0);
  if (!page) {
    return null;
  }
  const app = pages("Anonymous pages") - pages("Pages purgeable");
  return (app + pages("Pages wired down") + pages("Pages occupied by compressor")) * page;
}

async function macPressure() {
  const level = Number((await run("sysctl", ["-n", "kern.memorystatus_vm_pressure_level"])).trim());
  return { 1: "normal", 2: "warning", 4: "critical" }[level] || null;
}

// Apple GPUs publish live counters in the IORegistry; no admin rights needed.
async function macGpuLoad() {
  const text = await run("ioreg", ["-r", "-d", "1", "-c", "IOAccelerator"]);
  const number = (label) => {
    const match = new RegExp(`"${label}"=(\\d+)`).exec(text);
    return match ? Number(match[1]) : null;
  };
  return { load: number("Device Utilization %"), memory: number("In use system memory"), memoryTotal: null };
}

// NVIDIA drivers ship nvidia-smi. Other GPUs on Windows and Linux report no
// live load here yet. Not yet tried on a real Windows or Linux machine.
async function nvidiaGpuLoad() {
  const text = await run("nvidia-smi", [
    "--query-gpu=utilization.gpu,memory.used,memory.total",
    "--format=csv,noheader,nounits",
  ]);
  const [load, used, total] = text.split("\n")[0].split(",").map((value) => Number(value.trim()));
  if (!Number.isFinite(load)) {
    return { load: null, memory: null, memoryTotal: null };
  }
  return { load, memory: used * 1048576, memoryTotal: total * 1048576 };
}

let lastCpu = null;
function cpuLoad() {
  const totals = os.cpus().reduce(
    (sum, cpu) => {
      const times = cpu.times;
      sum.idle += times.idle;
      sum.all += times.user + times.nice + times.sys + times.idle + times.irq;
      return sum;
    },
    { idle: 0, all: 0 },
  );
  const previous = lastCpu;
  lastCpu = totals;
  if (!previous || totals.all === previous.all) {
    return null;
  }
  return Math.round(100 * (1 - (totals.idle - previous.idle) / (totals.all - previous.all)));
}

async function sample(win) {
  const mac = process.platform === "darwin";
  const [gpu, used, pressure] = await Promise.all([
    mac ? macGpuLoad() : nvidiaGpuLoad(),
    mac ? macMemoryUsed() : Promise.resolve(os.totalmem() - os.freemem()),
    mac ? macPressure() : Promise.resolve(null),
  ]);
  const metrics = app.getAppMetrics();
  const display = screen.getDisplayMatching(win.getBounds());
  const thermal = typeof powerMonitor.getCurrentThermalState === "function"
    ? powerMonitor.getCurrentThermalState()
    : "unknown";
  return {
    cpuLoad: cpuLoad(),
    appCpu: Math.round(metrics.reduce((sum, m) => sum + m.cpu.percentCPUUsage, 0) / os.cpus().length),
    memoryUsed: used,
    appMemory: metrics.reduce((sum, m) => sum + m.memory.workingSetSize, 0) * 1024,
    pressure,
    gpuLoad: gpu.load,
    gpuMemory: gpu.memory,
    gpuMemoryTotal: gpu.memoryTotal,
    display: {
      width: Math.round(display.size.width * display.scaleFactor),
      height: Math.round(display.size.height * display.scaleFactor),
      hz: display.displayFrequency || null,
      scale: display.scaleFactor,
    },
    thermal: thermal === "unknown" ? null : thermal,
    battery: powerMonitor.isOnBatteryPower(),
  };
}

module.exports = { identity, sample, parseRenderer };
