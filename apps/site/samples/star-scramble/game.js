// Star Scramble: a Giga Couch sample. Up to four players on controllers,
// plus one on the keyboard. It reads pads with Chrome's standard gamepad API
// and keeps the best score in localStorage.
(function () {
  "use strict";
  var canvas = document.getElementById("game");
  var ctx = canvas.getContext("2d");
  var COLORS = ["#8ce8be", "#9abef7", "#f3c77d", "#f29a8e", "#c5a0d9"];
  var ROUND = 45;
  var players = [];
  var stars = [];
  var keys = {};
  var state = "lobby";
  var timeLeft = ROUND;
  var last = performance.now();
  var best = 0;
  var held = {};
  // Enter or Space pressed since the last frame, so a quick tap is never missed.
  var enterTapped = false;

  try { best = Number(localStorage.getItem("star-scramble-best")) || 0; } catch (e) { best = 0; }

  function resize() {
    var ratio = window.devicePixelRatio || 1;
    canvas.width = Math.floor(innerWidth * ratio);
    canvas.height = Math.floor(innerHeight * ratio);
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  }
  addEventListener("resize", resize);
  resize();

  addEventListener("keydown", function (event) {
    keys[event.code] = true;
    if ((event.code === "Enter" || event.code === "Space") && !event.repeat) enterTapped = true;
    if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space"].indexOf(event.code) !== -1) event.preventDefault();
  });
  addEventListener("keyup", function (event) { keys[event.code] = false; });

  function find(source) {
    for (var i = 0; i < players.length; i++) if (players[i].source === source) return players[i];
    return null;
  }

  function join(source, name) {
    if (find(source) || players.length >= 5) return;
    players.push({ source: source, name: name, color: COLORS[players.length], x: innerWidth / 2 + (players.length - 2) * 60, y: innerHeight / 2, score: 0 });
  }

  function spawnStar() {
    stars.push({ x: 40 + Math.random() * (innerWidth - 80), y: 90 + Math.random() * (innerHeight - 130), spin: Math.random() * 6 });
  }

  function startRound() {
    state = "playing";
    timeLeft = ROUND;
    stars = [];
    for (var i = 0; i < 6; i++) spawnStar();
    players.forEach(function (p) { p.score = 0; });
  }

  // "Pressed this frame" for a button, so holding it doesn't repeat.
  function edge(id, down) {
    var was = held[id];
    held[id] = down;
    return down && !was;
  }

  function readInput() {
    var pads = navigator.getGamepads ? navigator.getGamepads() : [];
    var startPressed = false;
    for (var i = 0; i < pads.length; i++) {
      var pad = pads[i];
      if (!pad) continue;
      var source = "pad" + pad.index;
      var any = pad.buttons.some(function (b) { return b.pressed; });
      if (edge(source + ":any", any) && !find(source)) join(source, "Player " + (players.length + 1));
      var player = find(source);
      if (!player) continue;
      var x = pad.axes[0] || 0;
      var y = pad.axes[1] || 0;
      if (pad.buttons[14] && pad.buttons[14].pressed) x = -1;
      if (pad.buttons[15] && pad.buttons[15].pressed) x = 1;
      if (pad.buttons[12] && pad.buttons[12].pressed) y = -1;
      if (pad.buttons[13] && pad.buttons[13].pressed) y = 1;
      player.dx = Math.abs(x) > 0.18 ? x : 0;
      player.dy = Math.abs(y) > 0.18 ? y : 0;
      player.pad = pad;
      var a = pad.buttons[0] && pad.buttons[0].pressed;
      var startButton = pad.buttons[9] && pad.buttons[9].pressed;
      if (edge(source + ":a", a) || edge(source + ":start", startButton)) startPressed = true;
    }
    var enter = enterTapped;
    enterTapped = false;
    if (enter) {
      if (!find("kb")) join("kb", "Keyboard");
      else startPressed = true;
    }
    var kb = find("kb");
    if (kb) {
      kb.dx = (keys.ArrowRight || keys.KeyD ? 1 : 0) - (keys.ArrowLeft || keys.KeyA ? 1 : 0);
      kb.dy = (keys.ArrowDown || keys.KeyS ? 1 : 0) - (keys.ArrowUp || keys.KeyW ? 1 : 0);
    }
    return startPressed;
  }

  function rumble(pad) {
    var haptics = pad && pad.vibrationActuator;
    if (haptics && haptics.playEffect) haptics.playEffect("dual-rumble", { duration: 90, strongMagnitude: 0.6, weakMagnitude: 0.4 }).catch(function () {});
  }

  function update(dt) {
    var startPressed = readInput();
    if (state !== "playing") {
      if (startPressed && players.length) startRound();
      return;
    }
    timeLeft -= dt;
    if (timeLeft <= 0) {
      timeLeft = 0;
      state = "results";
      var top = Math.max.apply(null, players.map(function (p) { return p.score; }));
      if (top > best) {
        best = top;
        try { localStorage.setItem("star-scramble-best", String(best)); } catch (e) { /* storage may be off */ }
      }
      return;
    }
    players.forEach(function (p) {
      p.x = Math.max(24, Math.min(innerWidth - 24, p.x + (p.dx || 0) * 360 * dt));
      p.y = Math.max(84, Math.min(innerHeight - 24, p.y + (p.dy || 0) * 360 * dt));
      for (var i = stars.length - 1; i >= 0; i--) {
        var s = stars[i];
        if (Math.hypot(s.x - p.x, s.y - p.y) < 34) {
          stars.splice(i, 1);
          p.score += 1;
          rumble(p.pad);
          spawnStar();
        }
      }
    });
    stars.forEach(function (s) { s.spin += dt * 2; });
  }

  function drawStar(x, y, r, spin) {
    ctx.beginPath();
    for (var i = 0; i < 10; i++) {
      var radius = i % 2 ? r * 0.45 : r;
      var angle = spin + (i * Math.PI) / 5;
      ctx.lineTo(x + Math.cos(angle) * radius, y + Math.sin(angle) * radius);
    }
    ctx.closePath();
    ctx.fillStyle = "#f3c77d";
    ctx.fill();
  }

  function text(value, x, y, size, color, align) {
    ctx.font = "600 " + size + "px system-ui, sans-serif";
    ctx.fillStyle = color;
    ctx.textAlign = align || "center";
    ctx.fillText(value, x, y);
  }

  function draw() {
    ctx.fillStyle = "#101722";
    ctx.fillRect(0, 0, innerWidth, innerHeight);
    stars.forEach(function (s) { drawStar(s.x, s.y, 16, s.spin); });
    players.forEach(function (p) {
      ctx.beginPath();
      ctx.arc(p.x, p.y, 22, 0, Math.PI * 2);
      ctx.fillStyle = p.color;
      ctx.fill();
    });
    // Scores along the top.
    players.forEach(function (p, i) {
      text(p.name + "  " + p.score, 24 + i * 170, 40, 20, p.color, "left");
    });
    if (state === "playing") text(Math.ceil(timeLeft) + "s", innerWidth - 32, 40, 22, "#f2f5f8", "right");
    var mid = innerHeight / 2;
    if (state === "lobby") {
      text("Star Scramble", innerWidth / 2, mid - 70, 56, "#f2f5f8");
      text("Press any button on a controller to join. Keyboard: press Enter.", innerWidth / 2, mid - 20, 20, "#a0afbf");
      text(players.length ? "Press A, Start, or Enter to begin" : "Up to four controllers and a keyboard", innerWidth / 2, mid + 20, 20, players.length ? "#8ce8be" : "#a0afbf");
      if (best) text("Best: " + best + " stars", innerWidth / 2, mid + 64, 18, "#a0afbf");
    } else if (state === "results") {
      var sorted = players.slice().sort(function (a, b) { return b.score - a.score; });
      text(sorted[0].name + " wins with " + sorted[0].score + " stars", innerWidth / 2, mid - 20, 40, sorted[0].color);
      text("Press A, Start, or Enter to play again. Best: " + best, innerWidth / 2, mid + 26, 20, "#a0afbf");
    }
  }

  function frame(now) {
    var dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    update(dt);
    draw();
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
})();
