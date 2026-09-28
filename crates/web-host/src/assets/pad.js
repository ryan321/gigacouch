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
  };
  var DEFAULT_LAYOUT = "stick-2";
  var BUTTONS = ["south", "east", "west", "north", "start", "leave", "shelf"];

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
    pad = { south: false, east: false, west: false, north: false, start: false, leave: false, x: 0, y: 0, lx: 0, ly: 0, digital: false };
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
    var message = { name: name, x: round(pad.x), y: round(pad.y), lx: round(pad.lx), ly: round(pad.ly), digital: pad.digital };
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
    if (player) {
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
        player = reply.player || null;
        inGame = reply.game === true;
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

  function buzz() {
    if (navigator.vibrate) {
      try { navigator.vibrate(8); } catch (e) { /* not on iPhone */ }
    }
  }

  function capture(el, event) {
    try { el.setPointerCapture(event.pointerId); } catch (e) { /* older browsers */ }
  }

  function makeButton(spec, box) {
    var el = document.createElement("button");
    el.type = "button";
    el.className = "face" + (spec.rect ? " rect" : "") + (spec.small ? " small" : "");
    el.innerHTML = "";
    el.append(spec.label);
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
    el.setAttribute("aria-label", spec.label);
    var holding = null;
    el.addEventListener("pointerdown", function (event) {
      event.preventDefault();
      event.stopPropagation();
      holding = event.pointerId;
      capture(el, event);
      pad[spec.key] = true;
      latch[spec.key] = true;
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
  function goFullScreen() {
    var root = document.documentElement;
    if (standalone || apple || document.fullscreenElement || !root.requestFullscreen) return;
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

  function draw() {
    var layout = LAYOUTS[layoutName] || LAYOUTS[DEFAULT_LAYOUT];
    var portrait = window.innerHeight > window.innerWidth;
    var controls = portrait ? layout.portrait : layout.landscape;
    var key = layoutName + (portrait ? ":portrait" : ":landscape") + ":" + window.innerWidth + "x" + window.innerHeight;
    if (key === drawnAs) return;
    drawnAs = key;
    releaseAll();
    dirty = true;
    var area = $("controls");
    area.innerHTML = "";
    $("turn").hidden = !!controls;
    if (!controls) return;
    var box = area.getBoundingClientRect();
    pad.digital = controls.some(function (spec) { return spec.type === "dpad"; });
    controls.forEach(function (spec) {
      if (spec.type === "stick") area.append(makeStick(spec, box));
      else if (spec.type === "dpad") area.append(makeDpad(spec, box));
      else area.append(makeButton(spec, box));
    });
  }

  // The platform menu, on every layout: back to the shelf, leave, rename.
  // Opening it releases every control so nothing stays held in the game.
  function closeMenu() { $("menu").hidden = true; }
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
