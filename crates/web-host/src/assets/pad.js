// The phone pad. Served as its own file: the host's content policy allows
// scripts from this origin only, never inline ones.
//
// The host picks a layout (the open game's, or the default) and names it in
// every reply. This page draws that layout from the table below. Every layout
// maps onto the same five buttons (south, east, west, north, start) and two
// sticks (move, look) that the host gives every player.
(function () {
  var code = location.pathname.split("/")[2] || "";
  var $ = function (id) { return document.getElementById(id); };
  var RADIUS = 56;

  // Positions are fractions of the control area. Round buttons give a centre
  // (x, y) and a size as a fraction of the area's shorter side; square
  // buttons and stick zones give a rect [x, y, width, height]. A layout with
  // no portrait list asks for the phone to be turned sideways.
  var LAYOUTS = {
    "stick-2": {
      landscape: [
        { type: "stick", axis: "move", rect: [0, 0, 0.55, 1], hint: "Drag anywhere here to move" },
        { type: "button", key: "south", label: "A", hint: "join · jump", color: "mint", x: 0.85, y: 0.66, size: 0.4 },
        { type: "button", key: "east", label: "B", color: "amber", x: 0.64, y: 0.38, size: 0.3 },
      ],
      portrait: [
        { type: "stick", axis: "move", rect: [0, 0.45, 1, 0.55], hint: "Drag anywhere here to move" },
        { type: "button", key: "south", label: "A", hint: "join · jump", color: "mint", x: 0.7, y: 0.24, size: 0.4 },
        { type: "button", key: "east", label: "B", color: "amber", x: 0.28, y: 0.18, size: 0.3 },
      ],
    },
    "dpad-2": {
      landscape: [
        { type: "dpad", x: 0.24, y: 0.56, size: 0.7 },
        { type: "button", key: "start", label: "Start", small: true, x: 0.5, y: 0.14, size: 0.16 },
        { type: "button", key: "south", label: "A", color: "mint", x: 0.86, y: 0.64, size: 0.34 },
        { type: "button", key: "east", label: "B", color: "amber", x: 0.67, y: 0.46, size: 0.3 },
      ],
    },
    "stick-4": {
      landscape: [
        { type: "stick", axis: "move", rect: [0, 0, 0.5, 1], hint: "Drag anywhere here to move" },
        { type: "button", key: "start", label: "Start", small: true, x: 0.5, y: 0.1, size: 0.14 },
        { type: "button", key: "south", label: "A", color: "mint", x: 0.77, y: 0.78, size: 0.25 },
        { type: "button", key: "east", label: "B", color: "coral", x: 0.91, y: 0.52, size: 0.25 },
        { type: "button", key: "west", label: "X", color: "blue", x: 0.63, y: 0.52, size: 0.25 },
        { type: "button", key: "north", label: "Y", color: "amber", x: 0.77, y: 0.26, size: 0.25 },
      ],
    },
    "twin-stick": {
      landscape: [
        { type: "stick", axis: "move", rect: [0, 0, 0.5, 1], hint: "Drag to move" },
        { type: "stick", axis: "look", rect: [0.5, 0, 0.5, 1], hint: "Drag to aim" },
        { type: "button", key: "south", label: "A", color: "mint", x: 0.5, y: 0.84, size: 0.2 },
        { type: "button", key: "start", label: "Start", small: true, x: 0.5, y: 0.1, size: 0.14 },
      ],
    },
    "one-button": {
      landscape: [
        { type: "button", key: "south", label: "A", hint: "tap", color: "mint", x: 0.5, y: 0.5, size: 0.86 },
      ],
      portrait: [
        { type: "button", key: "south", label: "A", hint: "tap", color: "mint", x: 0.5, y: 0.5, size: 0.86 },
      ],
    },
    "quiz-4": {
      landscape: [
        { type: "button", key: "south", label: "A", color: "mint", rect: [0.02, 0.04, 0.47, 0.44] },
        { type: "button", key: "east", label: "B", color: "coral", rect: [0.51, 0.04, 0.47, 0.44] },
        { type: "button", key: "west", label: "X", color: "blue", rect: [0.02, 0.52, 0.47, 0.44] },
        { type: "button", key: "north", label: "Y", color: "amber", rect: [0.51, 0.52, 0.47, 0.44] },
      ],
      portrait: [
        { type: "button", key: "south", label: "A", color: "mint", rect: [0.04, 0.02, 0.44, 0.47] },
        { type: "button", key: "east", label: "B", color: "coral", rect: [0.52, 0.02, 0.44, 0.47] },
        { type: "button", key: "west", label: "X", color: "blue", rect: [0.04, 0.51, 0.44, 0.47] },
        { type: "button", key: "north", label: "Y", color: "amber", rect: [0.52, 0.51, 0.44, 0.47] },
      ],
    },
    "racing": {
      landscape: [
        { type: "arrow", dir: -1, label: "◀", x: 0.13, y: 0.6, size: 0.42 },
        { type: "arrow", dir: 1, label: "▶", x: 0.36, y: 0.6, size: 0.42 },
        { type: "button", key: "start", label: "Start", small: true, x: 0.5, y: 0.1, size: 0.14 },
        { type: "button", key: "east", label: "Brake", color: "coral", rect: [0.56, 0.34, 0.18, 0.6] },
        { type: "button", key: "south", label: "Gas", color: "mint", rect: [0.77, 0.12, 0.21, 0.82] },
      ],
    },
    "paddle": {
      landscape: [
        { type: "slider", rect: [0.04, 0.34, 0.92, 0.42], hint: "Slide to move. It stays where you leave it." },
        { type: "button", key: "south", label: "A", color: "mint", x: 0.9, y: 0.15, size: 0.24 },
      ],
      portrait: [
        { type: "slider", rect: [0.05, 0.56, 0.9, 0.3], hint: "Slide to move" },
        { type: "button", key: "south", label: "A", color: "mint", x: 0.5, y: 0.24, size: 0.4 },
      ],
    },
    "touchpad": {
      landscape: [
        { type: "touchpad", rect: [0.02, 0.04, 0.66, 0.92], hint: "Touch to point" },
        { type: "button", key: "east", label: "B", color: "amber", x: 0.84, y: 0.28, size: 0.26 },
        { type: "button", key: "south", label: "A", color: "mint", x: 0.84, y: 0.7, size: 0.34 },
      ],
      portrait: [
        { type: "touchpad", rect: [0.04, 0.02, 0.92, 0.62], hint: "Touch to point" },
        { type: "button", key: "south", label: "A", color: "mint", x: 0.32, y: 0.83, size: 0.34 },
        { type: "button", key: "east", label: "B", color: "amber", x: 0.72, y: 0.83, size: 0.26 },
      ],
    },
    "lanes-4": {
      landscape: [
        { type: "button", key: "west", label: "X", color: "blue", rect: [0.01, 0.04, 0.235, 0.92] },
        { type: "button", key: "south", label: "A", color: "mint", rect: [0.2575, 0.04, 0.235, 0.92] },
        { type: "button", key: "east", label: "B", color: "coral", rect: [0.5075, 0.04, 0.235, 0.92] },
        { type: "button", key: "north", label: "Y", color: "amber", rect: [0.7575, 0.04, 0.235, 0.92] },
      ],
      portrait: [
        { type: "button", key: "west", label: "X", color: "blue", rect: [0.02, 0.02, 0.23, 0.96] },
        { type: "button", key: "south", label: "A", color: "mint", rect: [0.265, 0.02, 0.23, 0.96] },
        { type: "button", key: "east", label: "B", color: "coral", rect: [0.51, 0.02, 0.23, 0.96] },
        { type: "button", key: "north", label: "Y", color: "amber", rect: [0.755, 0.02, 0.23, 0.96] },
      ],
    },
    "two-choice": {
      landscape: [
        { type: "button", key: "south", label: "A", color: "mint", rect: [0.02, 0.04, 0.47, 0.92] },
        { type: "button", key: "east", label: "B", color: "coral", rect: [0.51, 0.04, 0.47, 0.92] },
      ],
      portrait: [
        { type: "button", key: "south", label: "A", color: "mint", rect: [0.04, 0.02, 0.92, 0.47] },
        { type: "button", key: "east", label: "B", color: "coral", rect: [0.04, 0.51, 0.92, 0.47] },
      ],
    },
    "draw": {
      landscape: [
        { type: "canvas", rect: [0.01, 0.03, 0.84, 0.94] },
        { type: "palette", rect: [0.87, 0.03, 0.12, 0.94] },
      ],
      portrait: [
        { type: "canvas", rect: [0.02, 0.01, 0.96, 0.84] },
        { type: "palette", rect: [0.02, 0.87, 0.96, 0.12] },
      ],
    },
  };
  var DEFAULT_LAYOUT = "stick-2";
  // The host fills this in. When the host's pad is newer (it was updated and
  // restarted while this page stayed open), reload once to pick it up.
  var PAD_VERSION = "{{PAD_VERSION}}";
  var BUTTONS = ["south", "east", "west", "north", "start", "leave", "shelf"];

  // ---- Sound -------------------------------------------------------------
  // Browsers allow sound only after a tap, so the audio engine starts on the
  // first touch. Stock sounds are generated here; a game's own sounds are
  // downloaded and decoded when the game opens, so they play at once.
  var audio = null;
  var master = null;
  var soundBytes = {};
  var soundBuffers = {};
  try {
    // Lets iPhone play pad sounds with the silent switch on.
    if (navigator.audioSession) navigator.audioSession.type = "playback";
  } catch (e) { /* older Safari */ }

  function startAudio() {
    var Engine = window.AudioContext || window.webkitAudioContext;
    if (!Engine) return;
    if (!audio) {
      audio = new Engine();
      master = audio.createGain();
      master.gain.value = 0.8;
      master.connect(audio.destination);
      Object.keys(soundBytes).forEach(decode);
    }
    if (audio.state === "suspended") audio.resume();
  }
  document.addEventListener("pointerdown", startAudio, { capture: true });
  document.addEventListener("touchend", startAudio, { capture: true });

  function decode(name) {
    if (!audio || !soundBytes[name]) return;
    var bytes = soundBytes[name].slice(0);
    audio.decodeAudioData(bytes, function (buffer) { soundBuffers[name] = buffer; }, function () {});
  }

  // The open game's own sounds: name to address on this host.
  function loadSounds(map) {
    soundBytes = {};
    soundBuffers = {};
    Object.keys(map || {}).forEach(function (name) {
      fetch(map[name]).then(function (response) {
        return response.ok ? response.arrayBuffer() : null;
      }).then(function (bytes) {
        if (!bytes) return;
        soundBytes[name] = bytes;
        decode(name);
      }).catch(function () {});
    });
  }

  function tone(type, from, to, start, length, level) {
    var osc = audio.createOscillator();
    var gain = audio.createGain();
    var t = audio.currentTime + start;
    osc.type = type;
    osc.frequency.setValueAtTime(from, t);
    if (to !== from) osc.frequency.exponentialRampToValueAtTime(to, t + length);
    gain.gain.setValueAtTime(0.0001, t);
    gain.gain.exponentialRampToValueAtTime(level, t + 0.005);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + length);
    osc.connect(gain);
    gain.connect(master);
    osc.start(t);
    osc.stop(t + length + 0.02);
  }

  function noise(start, length, from, to, level) {
    var frames = Math.ceil(audio.sampleRate * length);
    var buffer = audio.createBuffer(1, frames, audio.sampleRate);
    var data = buffer.getChannelData(0);
    for (var i = 0; i < frames; i += 1) data[i] = Math.random() * 2 - 1;
    var source = audio.createBufferSource();
    var filter = audio.createBiquadFilter();
    var gain = audio.createGain();
    var t = audio.currentTime + start;
    source.buffer = buffer;
    filter.type = "bandpass";
    filter.Q.value = 1.2;
    filter.frequency.setValueAtTime(from, t);
    filter.frequency.exponentialRampToValueAtTime(to, t + length);
    gain.gain.setValueAtTime(0.0001, t);
    gain.gain.exponentialRampToValueAtTime(level, t + length * 0.3);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + length);
    source.connect(filter);
    filter.connect(gain);
    gain.connect(master);
    source.start(t);
  }

  // Stock sounds any game can play without shipping a file. Keep the names
  // in step with STOCK_SOUNDS in the host's package.rs.
  var STOCK_SOUNDS = {
    click: function () { tone("square", 1400, 1400, 0, 0.03, 0.25); },
    tick: function () { tone("sine", 2000, 2000, 0, 0.025, 0.3); },
    ding: function () { tone("sine", 1319, 1319, 0, 0.6, 0.5); tone("sine", 2637, 2637, 0, 0.3, 0.12); },
    success: function () { [523, 659, 784, 1047].forEach(function (f, i) { tone("triangle", f, f, i * 0.07, 0.18, 0.45); }); },
    fail: function () { tone("sawtooth", 330, 110, 0, 0.45, 0.25); },
    buzzer: function () { tone("square", 150, 140, 0, 0.5, 0.3); },
    coin: function () { tone("square", 988, 988, 0, 0.07, 0.25); tone("square", 1319, 1319, 0.07, 0.3, 0.25); },
    whoosh: function () { noise(0, 0.4, 300, 3000, 0.6); },
  };

  // ---- Rumble ------------------------------------------------------------
  // Android phones vibrate. iPhones have no vibration for web pages, so the
  // pad flashes its edges for the same length instead.
  var canVibrate = typeof navigator.vibrate === "function";
  var flashTimer = null;
  function rumble(pattern) {
    if (!Array.isArray(pattern) || !pattern.length) return;
    if (canVibrate) {
      try { if (navigator.vibrate(pattern)) return; } catch (e) { /* fall back to the flash */ }
    }
    var total = pattern.reduce(function (sum, ms) { return sum + ms; }, 0);
    document.body.classList.add("rumble");
    clearTimeout(flashTimer);
    flashTimer = setTimeout(function () { document.body.classList.remove("rumble"); }, Math.max(120, Math.min(total, 1500)));
  }

  function playSound(name) {
    if (!audio || audio.state !== "running") return;
    if (soundBuffers[name]) {
      var source = audio.createBufferSource();
      source.buffer = soundBuffers[name];
      source.connect(master);
      source.start();
    } else if (STOCK_SOUNDS[name]) {
      STOCK_SOUNDS[name]();
    }
  }

  function stored(key, fallback) {
    try { return localStorage.getItem(key) || fallback; } catch (e) { return fallback; }
  }
  function store(key, value) {
    try { localStorage.setItem(key, value); } catch (e) { /* private mode */ }
  }

  var id = stored("gigacouch.phone.id", "");
  if (!/^[A-Za-z0-9_-]{8,64}$/.test(id)) {
    var bytes = new Uint8Array(12);
    crypto.getRandomValues(bytes);
    id = Array.prototype.map.call(bytes, function (b) { return ("0" + b.toString(16)).slice(-2); }).join("");
    store("gigacouch.phone.id", id);
  }
  var ua = navigator.userAgent;
  var name = stored("gigacouch.phone.name", /iPhone/.test(ua) ? "iPhone" : /iPad/.test(ua) ? "iPad" : /Android/.test(ua) ? "Android phone" : "Phone");

  var pad = {};
  var latch = {};
  function releaseAll() {
    pad = { south: false, east: false, west: false, north: false, start: false, leave: false, x: 0, y: 0, lx: 0, ly: 0, digital: false, absolute: false };
    latch = {};
  }
  releaseAll();

  var socket = null;
  var open = false;
  var dirty = true;
  var sentAt = 0;
  // Stick movement goes out at most 30 times a second; button presses go
  // out at once. With dozens of phones on one Wi-Fi network, the number of
  // messages matters more than their size.
  var STICK_INTERVAL_MS = 33;
  var failures = 0;
  var player = null;
  var inGame = false;
  var layoutName = DEFAULT_LAYOUT;
  var drawnAs = "";

  $("name").textContent = name;

  function status(text, on) {
    $("status").textContent = text;
    $("dot").className = "dot" + (on ? " on" : "");
  }

  // A tap shorter than one message still counts: a press stays latched
  // until it has been sent once.
  function send() {
    if (!open) return;
    var message = { name: name, x: round(pad.x), y: round(pad.y), lx: round(pad.lx), ly: round(pad.ly), digital: pad.digital, absolute: pad.absolute, rumble: canVibrate, audience: audience, rtt: Math.round(rtt) };
    BUTTONS.forEach(function (key) {
      message[key] = !!(pad[key] || latch[key]);
    });
    latch = {};
    dirty = false;
    sentAt = performance.now();
    try { socket.send(JSON.stringify(message)); } catch (e) { /* reconnect handles it */ }
  }
  function round(value) { return Math.round(value * 1000) / 1000; }

  function showPlayer() {
    $("menu-leave").hidden = !player;
    $("menu-shelf").hidden = !inGame;
    $("menu-games").hidden = inGame;
    $("audience-badge").hidden = !audience;
    $("menu-audience").firstChild.textContent = audience ? "Play as a player" : "Watch as the audience";
    if (audience) {
      $("player").hidden = true;
      status(open ? "Watching" : "Connecting…", open);
    } else if (player) {
      $("player").hidden = false;
      $("player").textContent = "Player " + player;
      status("You're in", true);
    } else {
      $("player").hidden = true;
      status(open ? "Press A to join" : "Connecting…", open);
    }
  }

  function connect() {
    if (socket && socket.readyState <= 1) return;
    var scheme = location.protocol === "https:" ? "wss://" : "ws://";
    socket = new WebSocket(scheme + location.host + "/p/" + code + "/ws?id=" + id);
    socket.onopen = function () {
      open = true;
      failures = 0;
      showPlayer();
      send();
    };
    socket.onmessage = function (event) {
      try {
        var reply = JSON.parse(event.data);
        if (reply.pad && reply.pad !== PAD_VERSION && stored("gigacouch.phone.reloaded", "") !== reply.pad) {
          store("gigacouch.phone.reloaded", reply.pad);
          location.reload();
          return;
        }
        if (reply.sounds) loadSounds(reply.sounds);
        if (reply.sound) {
          playSound(reply.sound);
          return;
        }
        if (reply.rumble) {
          rumble(reply.rumble);
          return;
        }
        if (reply.pong !== undefined) { heard(reply.pong); return; }
        if (reply.notice) toast(reply.notice);
        if (reply.profiles) showList("Who's playing", reply.profiles, pickProfile, "Not linked");
        if (reply.shelf) showList("Start a game", reply.shelf, openGame);
        if ("profile" in reply) showProfile(reply.profile);
        if ("photo" in reply) $("menu-photo-note").textContent = reply.photo ? "Your photo is on the TV. Tap to retake." : "Shows next to your name";
        if (reply.images) preloadImages(reply.images);
        if ("labels" in reply) { labels = reply.labels || {}; drawnAs = ""; draw(); }
        if ("screen" in reply) { gameScreen = reply.screen; picked = null; renderScreen(); }
        // The same question again (a reconnect) keeps what is being typed.
        if ("ask" in reply && JSON.stringify(reply.ask) !== JSON.stringify(question)) { question = reply.ask; renderAsk(); }
        // A game's own layout comes with its drawing data.
        if (reply.layout && reply.layout_spec) LAYOUTS[reply.layout] = reply.layout_spec;
        if ("player" in reply) player = reply.player || null;
        if ("game" in reply) inGame = reply.game === true;
        if (reply.layout && LAYOUTS[reply.layout] && reply.layout !== layoutName) {
          layoutName = reply.layout;
          draw();
        }
        showPlayer();
      } catch (e) { /* ignore */ }
    };
    socket.onclose = function () {
      var was = open;
      open = false;
      player = null;
      showPlayer();
      failures += 1;
      status(was ? "Reconnecting…" : "Connecting…", false);
      if (failures === 3) {
        // A changed code means the shelf restarted; the page itself is gone.
        fetch(location.pathname, { cache: "no-store" }).then(function (response) {
          if (response.status === 404) $("gone").hidden = false;
        }).catch(function () {});
      }
      setTimeout(connect, Math.min(3000, 400 * failures));
    };
  }

  // The pad's own tap feedback, on the shelf only. Inside a game the game
  // decides what a press feels like: a pad buzz right before the game's
  // rumble can make Android start the game's vibration late.
  function buzz() {
    if (inGame || !navigator.vibrate) return;
    try { navigator.vibrate(15); } catch (e) { /* not on iPhone */ }
  }

  function capture(el, event) {
    try { el.setPointerCapture(event.pointerId); } catch (e) { /* older browsers */ }
  }

  function makeButton(spec, box) {
    var el = document.createElement("button");
    el.type = "button";
    el.className = "face" + (spec.rect ? " rect" : "") + (spec.small ? " small" : "");
    el.innerHTML = "";
    el.append((spec.key && labels[spec.key]) || spec.label);
    if (spec.hint) {
      var small = document.createElement("small");
      small.textContent = spec.hint;
      el.append(small);
    }
    if (spec.rect) {
      el.style.left = spec.rect[0] * box.width + "px";
      el.style.top = spec.rect[1] * box.height + "px";
      el.style.width = spec.rect[2] * box.width + "px";
      el.style.height = spec.rect[3] * box.height + "px";
      el.style.fontSize = Math.min(spec.rect[2] * box.width, spec.rect[3] * box.height) * 0.32 + "px";
    } else {
      var size = spec.size * Math.min(box.width, box.height);
      el.style.left = spec.x * box.width - size / 2 + "px";
      el.style.top = spec.y * box.height - (spec.small ? size / 4 : size / 2) + "px";
      el.style.width = (spec.small ? size * 1.8 : size) + "px";
      el.style.height = (spec.small ? size / 2 : size) + "px";
      el.style.fontSize = size * 0.34 + "px";
    }
    if (spec.color) el.style.background = "var(--" + spec.color + ")";
    // A dark "panel" button, such as Next, takes light words.
    if (spec.color === "panel") el.classList.add("dark");
    el.setAttribute("aria-label", spec.label);
    var holding = null;
    el.addEventListener("pointerdown", function (event) {
      event.preventDefault();
      event.stopPropagation();
      holding = event.pointerId;
      capture(el, event);
      pad[spec.key] = true;
      latch[spec.key] = true;
      // Only players' buttons reach a game; say so instead of doing nothing.
      if (!player && !audience && spec.key !== "south" && BUTTONS.indexOf(spec.key) !== -1) {
        toast("Press A to join first");
      }
      el.classList.add("down");
      buzz();
      send();
    });
    function release(event) {
      if (event.pointerId !== holding) return;
      holding = null;
      pad[spec.key] = false;
      el.classList.remove("down");
      dirty = true;
    }
    el.addEventListener("pointerup", release);
    el.addEventListener("pointercancel", release);
    return el;
  }

  // A floating stick: it appears where the thumb lands.
  function makeStick(spec, box) {
    var zone = document.createElement("div");
    zone.className = "zone";
    zone.style.left = spec.rect[0] * box.width + "px";
    zone.style.top = spec.rect[1] * box.height + "px";
    zone.style.width = spec.rect[2] * box.width + "px";
    zone.style.height = spec.rect[3] * box.height + "px";
    zone.innerHTML = '<div class="hint"></div><div class="base" hidden></div><div class="knob" hidden></div>';
    zone.querySelector(".hint").textContent = spec.hint || "";
    var base = zone.querySelector(".base");
    var knob = zone.querySelector(".knob");
    var hint = zone.querySelector(".hint");
    var xKey = spec.axis === "look" ? "lx" : "x";
    var yKey = spec.axis === "look" ? "ly" : "y";
    var touch = null;
    var origin = { x: 0, y: 0 };
    function place(el, x, y) { el.style.left = x + "px"; el.style.top = y + "px"; }
    zone.addEventListener("pointerdown", function (event) {
      if (touch !== null) return;
      event.preventDefault();
      touch = event.pointerId;
      capture(zone, event);
      var rect = zone.getBoundingClientRect();
      origin = { x: event.clientX - rect.left, y: event.clientY - rect.top };
      place(base, origin.x, origin.y);
      place(knob, origin.x, origin.y);
      base.hidden = false;
      knob.hidden = false;
      hint.hidden = true;
    });
    zone.addEventListener("pointermove", function (event) {
      if (event.pointerId !== touch) return;
      var rect = zone.getBoundingClientRect();
      var dx = event.clientX - rect.left - origin.x;
      var dy = event.clientY - rect.top - origin.y;
      var length = Math.sqrt(dx * dx + dy * dy);
      if (length > RADIUS) {
        dx = (dx / length) * RADIUS;
        dy = (dy / length) * RADIUS;
      }
      place(knob, origin.x + dx, origin.y + dy);
      pad[xKey] = dx / RADIUS;
      pad[yKey] = dy / RADIUS;
      dirty = true;
    });
    function letGo(event) {
      if (event.pointerId !== touch) return;
      touch = null;
      pad[xKey] = 0;
      pad[yKey] = 0;
      base.hidden = true;
      knob.hidden = true;
      dirty = true;
      send();
    }
    zone.addEventListener("pointerup", letGo);
    zone.addEventListener("pointercancel", letGo);
    return zone;
  }

  // An eight-way d-pad. It sends whole steps, so the host skips the stick
  // dead zone.
  function makeDpad(spec, box) {
    var size = spec.size * Math.min(box.width, box.height);
    var el = document.createElement("div");
    el.className = "dpad";
    el.style.left = spec.x * box.width - size / 2 + "px";
    el.style.top = spec.y * box.height - size / 2 + "px";
    el.style.width = size + "px";
    el.style.height = size + "px";
    var third = size / 3;
    var arms = {};
    [["up", third, 0], ["down", third, 2 * third], ["left", 0, third], ["right", 2 * third, third], ["mid", third, third]].forEach(function (arm) {
      var b = document.createElement("b");
      b.style.left = arm[1] + "px";
      b.style.top = arm[2] + "px";
      b.style.width = third + "px";
      b.style.height = third + "px";
      el.append(b);
      arms[arm[0]] = b;
    });
    var touch = null;
    function aim(event) {
      var rect = el.getBoundingClientRect();
      var dx = event.clientX - rect.left - size / 2;
      var dy = event.clientY - rect.top - size / 2;
      var length = Math.sqrt(dx * dx + dy * dy);
      var x = 0;
      var y = 0;
      if (length > size * 0.12) {
        x = Math.abs(dx) > length * 0.38 ? Math.sign(dx) : 0;
        y = Math.abs(dy) > length * 0.38 ? Math.sign(dy) : 0;
      }
      if (x !== pad.x || y !== pad.y) buzz();
      pad.x = x;
      pad.y = y;
      arms.up.className = y < 0 ? "on" : "";
      arms.down.className = y > 0 ? "on" : "";
      arms.left.className = x < 0 ? "on" : "";
      arms.right.className = x > 0 ? "on" : "";
      dirty = true;
    }
    el.addEventListener("pointerdown", function (event) {
      if (touch !== null) return;
      event.preventDefault();
      touch = event.pointerId;
      capture(el, event);
      aim(event);
    });
    el.addEventListener("pointermove", function (event) {
      if (event.pointerId === touch) aim(event);
    });
    function letGo(event) {
      if (event.pointerId !== touch) return;
      touch = null;
      pad.x = 0;
      pad.y = 0;
      Object.keys(arms).forEach(function (key) { arms[key].className = ""; });
      dirty = true;
      send();
    }
    el.addEventListener("pointerup", letGo);
    el.addEventListener("pointercancel", letGo);
    return el;
  }

  // iPhone Safari cannot hide its toolbars for a web page, so point people
  // at the home-screen web app, which opens full screen. Android Chrome can
  // go full screen from a tap, so it does that instead.
  var standalone = window.navigator.standalone === true ||
    (window.matchMedia && (window.matchMedia("(display-mode: standalone)").matches ||
      window.matchMedia("(display-mode: fullscreen)").matches));
  var apple = /iPhone|iPad|iPod/.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1);
  if (apple && !standalone && stored("gigacouch.phone.tip", "") !== "hidden") {
    $("tip").hidden = false;
  }
  $("tip-close").addEventListener("click", function () {
    $("tip").hidden = true;
    store("gigacouch.phone.tip", "hidden");
    drawnAs = "";
    draw();
  });
  function goFullScreen(event) {
    // Only from the controls. Asking for full screen uses up the tap, and a
    // menu item or a game's choice may need it, for example to open the
    // camera for a photo.
    if (!event || !event.target || !event.target.closest || !event.target.closest("#pad-controls")) return;
    var root = document.documentElement;
    if (standalone || apple || document.fullscreenElement || !root.requestFullscreen) return;
    // Never while typing: going full screen can close the phone's keyboard.
    var focused = document.activeElement;
    if (question || (focused && /^(INPUT|TEXTAREA)$/.test(focused.tagName))) return;
    root.requestFullscreen({ navigationUI: "hide" }).then(function () {
      var layout = LAYOUTS[layoutName] || LAYOUTS[DEFAULT_LAYOUT];
      if (!layout.portrait && screen.orientation && screen.orientation.lock) {
        screen.orientation.lock("landscape").catch(function () {});
      }
    }).catch(function () {});
  }
  // Browsers allow full screen only from a finished tap (the finger lifting),
  // not from the touch starting. Keep trying on each tap until it takes, so
  // it also comes back after a swipe out of full screen.
  document.addEventListener("pointerup", goFullScreen, { capture: true });
  document.addEventListener("touchend", goFullScreen, { capture: true });

  function placeRect(el, rect, box) {
    el.style.left = rect[0] * box.width + "px";
    el.style.top = rect[1] * box.height + "px";
    el.style.width = rect[2] * box.width + "px";
    el.style.height = rect[3] * box.height + "px";
  }

  // Steering arrows: holding one sets move x to -1 or 1; both cancel out.
  var arrowsHeld = { "-1": false, "1": false };
  function makeArrow(spec, box) {
    var el = makeButton({ key: "arrow" + spec.dir, label: spec.label, color: "panel", x: spec.x, y: spec.y, size: spec.size }, box);
    el.classList.add("arrow");
    function update() {
      pad.x = (arrowsHeld["1"] ? 1 : 0) - (arrowsHeld["-1"] ? 1 : 0);
      dirty = true;
    }
    el.addEventListener("pointerdown", function () { arrowsHeld[spec.dir] = true; update(); send(); });
    el.addEventListener("pointerup", function () { arrowsHeld[spec.dir] = false; update(); });
    el.addEventListener("pointercancel", function () { arrowsHeld[spec.dir] = false; update(); });
    return el;
  }

  // A slider that stays where the finger leaves it: move x from -1 to 1.
  function makeSlider(spec, box) {
    var track = document.createElement("div");
    track.className = "slider";
    placeRect(track, spec.rect, box);
    track.innerHTML = '<div class="hint"></div><div class="rail"></div><div class="thumb"></div>';
    track.querySelector(".hint").textContent = spec.hint || "";
    var thumb = track.querySelector(".thumb");
    function show() { thumb.style.left = ((pad.x + 1) / 2) * 100 + "%"; }
    show();
    var touch = null;
    function aim(event) {
      var rect = track.getBoundingClientRect();
      var value = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      pad.x = Math.max(-1, Math.min(1, value));
      show();
      dirty = true;
    }
    track.addEventListener("pointerdown", function (event) {
      if (touch !== null) return;
      event.preventDefault();
      touch = event.pointerId;
      capture(track, event);
      aim(event);
    });
    track.addEventListener("pointermove", function (event) { if (event.pointerId === touch) aim(event); });
    function letGo(event) { if (event.pointerId === touch) { touch = null; send(); } }
    track.addEventListener("pointerup", letGo);
    track.addEventListener("pointercancel", letGo);
    return track;
  }

  // A touchpad: the finger's position on the look axis, -1 to 1 each way,
  // kept where the finger lifts.
  function makeTouchpad(spec, box) {
    var pad_ = document.createElement("div");
    pad_.className = "touchpad";
    placeRect(pad_, spec.rect, box);
    pad_.innerHTML = '<div class="hint"></div><div class="spot"></div>';
    pad_.querySelector(".hint").textContent = spec.hint || "";
    var spot = pad_.querySelector(".spot");
    function show() {
      spot.style.left = ((pad.lx + 1) / 2) * 100 + "%";
      spot.style.top = ((pad.ly + 1) / 2) * 100 + "%";
    }
    show();
    var touch = null;
    function aim(event) {
      var rect = pad_.getBoundingClientRect();
      pad.lx = Math.max(-1, Math.min(1, ((event.clientX - rect.left) / rect.width) * 2 - 1));
      pad.ly = Math.max(-1, Math.min(1, ((event.clientY - rect.top) / rect.height) * 2 - 1));
      show();
      dirty = true;
    }
    pad_.addEventListener("pointerdown", function (event) {
      if (touch !== null) return;
      event.preventDefault();
      touch = event.pointerId;
      capture(pad_, event);
      pad_.classList.add("down");
      pad_.querySelector(".hint").hidden = true;
      aim(event);
    });
    pad_.addEventListener("pointermove", function (event) { if (event.pointerId === touch) aim(event); });
    function letGo(event) {
      if (event.pointerId !== touch) return;
      touch = null;
      pad_.classList.remove("down");
      send();
    }
    pad_.addEventListener("pointerup", letGo);
    pad_.addEventListener("pointercancel", letGo);
    return pad_;
  }

  function draw() {
    var layout = LAYOUTS[layoutName] || LAYOUTS[DEFAULT_LAYOUT];
    var portrait = window.innerHeight > window.innerWidth;
    var controls = portrait ? layout.portrait : layout.landscape;
    if (controls && mirrored) controls = controls.map(mirror);
    var key = layoutName + (portrait ? ":portrait" : ":landscape") + ":" + window.innerWidth + "x" + window.innerHeight + (mirrored ? ":mirror" : "");
    if (key === drawnAs) return;
    drawnAs = key;
    releaseAll();
    dirty = true;
    var area = $("pad-controls");
    area.innerHTML = "";
    $("turn").hidden = !!controls;
    if (!controls) return;
    var box = area.getBoundingClientRect();
    arrowsHeld = { "-1": false, "1": false };
    pad.digital = controls.some(function (spec) { return spec.type === "dpad" || spec.type === "arrow"; });
    pad.absolute = controls.some(function (spec) { return spec.type === "slider" || spec.type === "touchpad"; });
    controls.forEach(function (spec) {
      if (spec.type === "stick") area.append(makeStick(spec, box));
      else if (spec.type === "dpad") area.append(makeDpad(spec, box));
      else if (spec.type === "arrow") area.append(makeArrow(spec, box));
      else if (spec.type === "slider") area.append(makeSlider(spec, box));
      else if (spec.type === "touchpad") area.append(makeTouchpad(spec, box));
      else if (spec.type === "canvas") area.append(makeCanvas(spec, box));
      else if (spec.type === "palette") area.append(makePalette(spec, box, portrait));
      else area.append(makeButton(spec, box));
    });
  }

  // ---- Features a game can opt into ----------------------------------------

  var labels = {};
  var gameScreen = null;
  var picked = null;
  var question = null;
  var audience = stored("gigacouch.phone.audience", "") === "1";
  var mirrored = stored("gigacouch.phone.mirror", "") === "1";
  var keepAwake = stored("gigacouch.phone.awake", "1") === "1";
  var rtt = 0;
  var penColor = "mint";
  var strokeNumber = 0;
  var COLORS = { mint: "#8ce8be", blue: "#9abef7", amber: "#f3c77d", coral: "#ec9a7a", panel: "#f2f5f8" };

  // A discrete message: an event for the game, a ping, a request.
  function sendKind(message) {
    if (!open) return;
    try { socket.send(JSON.stringify(message)); } catch (e) { /* reconnect handles it */ }
  }
  function sendEvent(event) { sendKind({ kind: "event", event: event }); }

  var toastTimer = null;
  function toast(text) {
    $("toast").textContent = text;
    $("toast").hidden = false;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () { $("toast").hidden = true; }, 3000);
  }

  // Left-handed: controls swap sides; steering arrows keep their direction.
  function mirror(spec) {
    var copy = {};
    Object.keys(spec).forEach(function (key) { copy[key] = spec[key]; });
    if (copy.rect) copy.rect = [1 - copy.rect[0] - copy.rect[2], copy.rect[1], copy.rect[2], copy.rect[3]];
    if (typeof copy.x === "number") copy.x = 1 - copy.x;
    if (copy.type === "arrow") {
      copy.dir = -copy.dir;
      copy.label = copy.dir < 0 ? "◀" : "▶";
    }
    return copy;
  }

  // The game's private screen: text, an image, and choices to tap.
  function renderScreen() {
    var old = $("screen");
    if (old) old.remove();
    if (!gameScreen) return;
    var box = document.createElement("section");
    box.id = "screen";
    box.className = "overlay";
    if (gameScreen.title) { var h = document.createElement("h2"); h.textContent = gameScreen.title; box.append(h); }
    if (gameScreen.image) { var img = document.createElement("img"); img.className = "art"; img.src = gameScreen.image; img.alt = ""; box.append(img); }
    if (gameScreen.text) { var p = document.createElement("p"); p.textContent = gameScreen.text; box.append(p); }
    if (gameScreen.choices && gameScreen.choices.length) {
      var grid = document.createElement("div");
      grid.className = "choices";
      gameScreen.choices.forEach(function (choice) {
        var button = document.createElement("button");
        button.type = "button";
        button.className = "choice" + (picked === choice.id ? " picked" : "");
        if (choice.color) button.style.background = COLORS[choice.color] || "";
        if (choice.image) { var art = document.createElement("img"); art.src = choice.image; art.alt = ""; button.append(art); }
        button.append(choice.label);
        if (choice.detail) { var small = document.createElement("small"); small.textContent = choice.detail; button.append(small); }
        button.addEventListener("click", function () {
          picked = choice.id;
          sendEvent({ type: "choice", choice: choice.id, screen: gameScreen.id });
          buzz();
          // A phone action runs from this tap, which the camera needs.
          if (choice.action === "photo") $("photo-input").click();
          else if (choice.action === "profile") sendKind({ kind: "request", what: "profiles" });
          else if (choice.action === "audience") toggleAudience();
          renderScreen();
        });
        grid.append(button);
      });
      box.append(grid);
    }
    $("controls").append(box);
  }

  // A question from the game, answered with the phone's own keyboard.
  function renderAsk() {
    var old = $("ask");
    if (old) old.remove();
    if (!question) return;
    var box = document.createElement("section");
    box.id = "ask";
    box.className = "overlay";
    var form = document.createElement("form");
    var label = document.createElement("label");
    label.textContent = question.prompt;
    label.htmlFor = "ask-input";
    var input = document.createElement(question.multiline ? "textarea" : "input");
    input.id = "ask-input";
    input.maxLength = question.max || 80;
    input.placeholder = question.placeholder || "";
    input.autocomplete = "off";
    var button = document.createElement("button");
    button.type = "submit";
    button.textContent = "Send";
    // A question never traps the player: Not now closes it, and the
    // controls underneath (Next included) are back.
    var later = document.createElement("button");
    later.type = "button";
    later.className = "later";
    later.textContent = "Not now";
    later.addEventListener("click", function () {
      input.blur();
      question = null;
      renderAsk();
    });
    form.append(label, input, button, later);
    form.addEventListener("submit", function (event) {
      event.preventDefault();
      var text = input.value.trim();
      if (!text) return;
      sendEvent({ type: "text", text: text });
      input.blur();
      question = null;
      renderAsk();
      toast("Sent");
    });
    box.append(form);
    $("controls").append(box);
    setTimeout(function () { try { input.focus(); } catch (e) {} }, 50);
  }

  function preloadImages(map) {
    Object.keys(map || {}).forEach(function (key) { var img = new Image(); img.src = map[key]; });
  }

  // Drawing: strokes go to the game as batches of points, 0 to 1 across the
  // canvas, with the pen color. The phone draws them too, straight away.
  function makeCanvas(spec, box) {
    var canvas = document.createElement("canvas");
    canvas.className = "canvas";
    placeRect(canvas, spec.rect, box);
    var width = spec.rect[2] * box.width;
    var height = spec.rect[3] * box.height;
    var scale = window.devicePixelRatio || 1;
    canvas.width = Math.round(width * scale);
    canvas.height = Math.round(height * scale);
    var ink = canvas.getContext("2d");
    ink.scale(scale, scale);
    ink.lineCap = "round";
    ink.lineJoin = "round";
    ink.lineWidth = 6;
    var touch = null;
    var last = null;
    var batch = [];
    var flushTimer = null;
    function flush(end) {
      if (!batch.length && !end) return;
      sendEvent({ type: "stroke", stroke: strokeNumber, points: batch.splice(0, 128), color: penColor, end: !!end });
    }
    function point(event) {
      var rect = canvas.getBoundingClientRect();
      return [Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)), Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height))];
    }
    canvas.addEventListener("pointerdown", function (event) {
      if (touch !== null) return;
      event.preventDefault();
      touch = event.pointerId;
      capture(canvas, event);
      strokeNumber += 1;
      last = point(event);
      batch = [last];
      ink.strokeStyle = COLORS[penColor];
      flushTimer = setInterval(function () { flush(false); }, 50);
    });
    canvas.addEventListener("pointermove", function (event) {
      if (event.pointerId !== touch) return;
      var next = point(event);
      ink.beginPath();
      ink.moveTo(last[0] * width, last[1] * height);
      ink.lineTo(next[0] * width, next[1] * height);
      ink.stroke();
      last = next;
      batch.push(next);
      if (batch.length >= 128) flush(false);
    });
    function letGo(event) {
      if (event.pointerId !== touch) return;
      touch = null;
      clearInterval(flushTimer);
      flush(true);
    }
    canvas.addEventListener("pointerup", letGo);
    canvas.addEventListener("pointercancel", letGo);
    canvas.clearInk = function () { ink.clearRect(0, 0, width, height); };
    return canvas;
  }

  function makePalette(spec, box, portrait) {
    var bar = document.createElement("div");
    bar.className = "palette" + (portrait ? " row" : "");
    placeRect(bar, spec.rect, box);
    ["mint", "blue", "amber", "coral"].forEach(function (color) {
      var swatch = document.createElement("button");
      swatch.type = "button";
      swatch.style.background = COLORS[color];
      swatch.setAttribute("aria-label", color + " pen");
      if (color === penColor) swatch.className = "on";
      swatch.addEventListener("click", function () {
        penColor = color;
        bar.querySelectorAll("button").forEach(function (other) { other.classList.remove("on"); });
        swatch.className = "on";
      });
      bar.append(swatch);
    });
    var clear = document.createElement("button");
    clear.type = "button";
    clear.className = "clear";
    clear.textContent = "Clear";
    clear.addEventListener("click", function () {
      var canvas = $("controls").querySelector(".canvas");
      if (canvas && canvas.clearInk) canvas.clearInk();
      sendEvent({ type: "clear" });
    });
    bar.append(clear);
    return bar;
  }

  // Connection quality: a ping every two seconds; the round trip shows as
  // bars and goes to the host so the TV can show it too.
  var pingAt = {};
  function ping() {
    var t = Math.round(performance.now());
    pingAt[t] = true;
    sendKind({ kind: "ping", t: t });
  }
  function heard(t) {
    if (!pingAt[t]) return;
    delete pingAt[t];
    var sample = performance.now() - t;
    rtt = rtt ? rtt * 0.7 + sample * 0.3 : sample;
    $("signal").className = rtt < 60 ? "good" : rtt < 150 ? "fair" : "poor";
    $("signal").title = Math.round(rtt) + " ms";
  }
  setInterval(ping, 2000);

  // Keep the screen on: a tiny silent clip on a loop, started by a tap.
  // The Wake Lock API is off on plain http, so this is the stand-in.
  var awakeVideo = null;
  function startAwake() {
    if (!keepAwake) return;
    if (!awakeVideo) {
      awakeVideo = document.createElement("video");
      awakeVideo.className = "keep-awake";
      awakeVideo.setAttribute("playsinline", "");
      awakeVideo.loop = true;
      awakeVideo.src = "/p/" + code + "/awake.mp4";
      document.body.append(awakeVideo);
    }
    if (awakeVideo.paused) awakeVideo.play().catch(function () {});
  }
  function stopAwake() { if (awakeVideo) awakeVideo.pause(); }
  document.addEventListener("pointerup", startAwake, { capture: true });
  document.addEventListener("touchend", startAwake, { capture: true });

  // A list in the menu: Home's people, or the games on the shelf.
  function showList(title, items, pick, none) {
    $("menu-sub-title").textContent = title;
    var list = $("menu-list");
    list.innerHTML = "";
    items.forEach(function (item) {
      var button = document.createElement("button");
      button.type = "button";
      button.className = "item small";
      button.textContent = item.name || item.title;
      button.addEventListener("click", function () { pick(item.id); });
      list.append(button);
    });
    if (none) {
      var off = document.createElement("button");
      off.type = "button";
      off.className = "item small";
      off.textContent = none;
      off.addEventListener("click", function () { pick(null); });
      list.append(off);
    }
    if (!items.length && !none) list.textContent = "Nothing to show yet.";
    $("menu-main").hidden = true;
    $("menu-sub").hidden = false;
    $("menu").hidden = false;
  }
  function pickProfile(profile) { sendKind({ kind: "profile", id: profile }); closeMenu(); }
  function openGame(game) { sendKind({ kind: "open", id: game }); closeMenu(); }
  function showProfile(profile) {
    $("menu-profile-note").textContent = profile ? "This phone is " + profile.name : "Link this phone to a name on the couch";
    if (profile) {
      name = profile.name;
      $("name").textContent = name;
      dirty = true;
    }
  }

  // A photo for the TV: the camera or a picture, cut to a square here and
  // sent as a small JPEG.
  $("photo-input").addEventListener("change", function () {
    var file = $("photo-input").files && $("photo-input").files[0];
    if (!file) return;
    toast("Saving photo…");
    var reader = new FileReader();
    reader.onload = function () {
      var img = new Image();
      img.onload = function () {
        var side = Math.min(img.width, img.height);
        var canvas = document.createElement("canvas");
        canvas.width = 256;
        canvas.height = 256;
        canvas.getContext("2d").drawImage(img, (img.width - side) / 2, (img.height - side) / 2, side, side, 0, 0, 256, 256);
        canvas.toBlob(function (blob) {
          if (!blob) return;
          fetch("/p/" + code + "/avatar?id=" + id, { method: "POST", headers: { "content-type": "image/jpeg" }, body: blob })
            .then(function (response) {
              toast(response.ok ? "Photo saved" : "That photo didn't save");
              if (response.ok) $("menu-photo-note").textContent = "Your photo is on the TV. Tap to retake.";
            })
            .catch(function () { toast("That photo didn't save"); });
        }, "image/jpeg", 0.85);
      };
      img.src = reader.result;
    };
    reader.readAsDataURL(file);
    $("photo-input").value = "";
  });

  function showToggles() {
    $("menu-mirror-note").textContent = mirrored ? "On: the stick is on the right" : "Off";
    $("menu-awake-note").textContent = keepAwake ? "On" : "Off: the phone may dim and lock";
  }
  showToggles();

  // The platform menu, on every layout: back to the shelf, leave, rename.
  // Opening it releases every control so nothing stays held in the game.
  function closeMenu() {
    $("menu").hidden = true;
    $("menu-main").hidden = false;
    $("menu-sub").hidden = true;
  }
  $("menu-back").addEventListener("click", function () {
    $("menu-main").hidden = false;
    $("menu-sub").hidden = true;
  });
  $("menu-games").addEventListener("click", function () { sendKind({ kind: "request", what: "shelf" }); });
  $("menu-profile").addEventListener("click", function () { sendKind({ kind: "request", what: "profiles" }); });
  $("menu-photo").addEventListener("click", function () { closeMenu(); $("photo-input").click(); });
  function toggleAudience() {
    audience = !audience;
    store("gigacouch.phone.audience", audience ? "1" : "");
    dirty = true;
    send();
    showPlayer();
  }
  $("menu-audience").addEventListener("click", function () {
    toggleAudience();
    closeMenu();
  });
  $("menu-mirror").addEventListener("click", function () {
    mirrored = !mirrored;
    store("gigacouch.phone.mirror", mirrored ? "1" : "");
    showToggles();
    drawnAs = "";
    draw();
  });
  $("menu-awake").addEventListener("click", function () {
    keepAwake = !keepAwake;
    store("gigacouch.phone.awake", keepAwake ? "1" : "");
    if (keepAwake) startAwake(); else stopAwake();
    showToggles();
  });
  $("menu-open").addEventListener("click", function () {
    releaseAll();
    drawnAs = "";
    draw();
    showPlayer();
    $("menu").hidden = false;
  });
  $("menu-close").addEventListener("click", closeMenu);
  $("menu").addEventListener("click", function (event) {
    if (event.target === $("menu")) closeMenu();
  });
  $("menu-shelf").addEventListener("click", function () {
    latch.shelf = true;
    buzz();
    send();
    closeMenu();
  });
  $("menu-leave").addEventListener("click", function () {
    latch.leave = true;
    buzz();
    send();
    closeMenu();
  });
  // The home-screen app has no browser reload button, so the menu offers one.
  $("menu-reload").addEventListener("click", function () {
    location.reload();
  });
  $("menu-name").addEventListener("click", function () {
    closeMenu();
    rename();
  });

  function rename() {
    var next = window.prompt("Your name on the couch", name);
    if (next && next.trim()) {
      name = next.trim().slice(0, 24);
      store("gigacouch.phone.name", name);
      $("name").textContent = name;
      dirty = true;
    }
  }
  $("name").addEventListener("click", rename);

  // Send changes on the next frame, and a heartbeat so the couch knows the
  // phone is still there. Each reply may switch the layout.
  function frame(now) {
    if (dirty && now - sentAt >= STICK_INTERVAL_MS) send();
    requestAnimationFrame(frame);
  }
  setInterval(send, 400);

  document.addEventListener("gesturestart", function (event) { event.preventDefault(); });
  document.addEventListener("contextmenu", function (event) { event.preventDefault(); });
  window.addEventListener("resize", draw);
  window.addEventListener("pagehide", function () {
    if (socket) try { socket.close(); } catch (e) {}
  });
  document.addEventListener("visibilitychange", function () {
    if (document.visibilityState === "visible" && socket && socket.readyState > 1) connect();
  });

  draw();
  connect();
  requestAnimationFrame(frame);
})();
