// Controller Gallery: try every phone layout.
//
// The TV shows the current layout and a live card for each player: their
// sticks and buttons as the host reports them. Holding A for two seconds, or
// ] and [ on the keyboard, switches every phone to the next or previous
// layout through GigaCouch.phone.setLayout.
(function () {
  var $ = function (id) { return document.getElementById(id); };
  var HOLD_MS = 2000;
  var BUTTONS = [
    ["south", "A", "var(--mint)"],
    ["east", "B", "var(--coral)"],
    ["west", "X", "var(--blue)"],
    ["north", "Y", "var(--amber)"],
    ["start", "Start", "var(--ink)"],
  ];
  var PLAYER_COLORS = ["#8ce8be", "#9abef7", "#f3c77d", "#ec9a7a", "#c9b3f5", "#f2a7c9", "#7fd6d6", "#d9e58a"];

  // What each layout is for, and which inputs it has, so unused ones dim.
  var ABOUT = {
    "stick-2": { name: "Stick and two buttons", about: "A floating stick with A and B. The default for most games.", move: "Move", buttons: ["south", "east"] },
    "dpad-2": { name: "D-pad", about: "An eight-way d-pad with A, B and Start. For retro and menu-heavy games.", move: "D-pad", buttons: ["south", "east", "start"] },
    "stick-4": { name: "Stick and four buttons", about: "A stick with A, B, X, Y and Start. For games with more actions.", move: "Move", buttons: ["south", "east", "west", "north", "start"] },
    "twin-stick": { name: "Twin sticks", about: "One stick moves, the other aims. For arena shooters.", move: "Move", look: "Aim", buttons: ["south", "start"] },
    "one-button": { name: "One button", about: "One big button. For reaction and party games.", buttons: ["south"] },
    "quiz-4": { name: "Quiz", about: "Four big answer buttons. For trivia and voting.", buttons: ["south", "east", "west", "north"] },
    "racing": { name: "Racing", about: "Steer with the arrows, with Gas and Brake pedals. For driving games.", move: "Steer", buttons: ["south", "east", "start"] },
    "paddle": { name: "Paddle", about: "A slider that stays where you leave it. For Pong and Breakout.", move: "Slider", buttons: ["south"] },
    "touchpad": { name: "Touchpad", about: "Touch anywhere to point. For cursors and drawing.", look: "Point", square: true, buttons: ["south", "east"] },
    "lanes-4": { name: "Four lanes", about: "Four tall lanes. For rhythm games.", buttons: ["west", "south", "east", "north"] },
    "two-choice": { name: "Two choices", about: "Two huge buttons. For this-or-that and yes-or-no.", buttons: ["south", "east"] },
  };

  // Which sound each button plays on the player's own phone. Stock sounds
  // come with every phone; the rest ship with this game (see gigacouch.json).
  var STOCK = ["click", "tick", "ding", "success", "fail", "buzzer", "coin", "whoosh"];
  var SOUNDS = {
    "stick-2": { south: "click", east: "tick" },
    "dpad-2": { south: "coin", east: "tick", start: "success" },
    "stick-4": { south: "ding", east: "fail", west: "chime", north: "boing", start: "whoosh" },
    "twin-stick": { south: "coin", start: "whoosh" },
    "one-button": { south: "boing" },
    "quiz-4": { south: "success", east: "fail", west: "ding", north: "buzzer" },
    "racing": { south: "engine", east: "screech", start: "horn" },
    "paddle": { south: "coin" },
    "touchpad": { south: "click", east: "tick" },
    "lanes-4": { west: "hat", south: "kick", east: "snare", north: "clap" },
    "two-choice": { south: "success", east: "buzzer" },
  };

  // Which buttons also buzz the player's phone, and how. Android phones
  // vibrate; iPhones flash the pad's edges.
  var RUMBLE = {
    "stick-2": { east: "bump" },
    "dpad-2": { east: "bump" },
    "stick-4": { east: "double", north: "bump" },
    "twin-stick": { south: "tap" },
    "one-button": { south: "hit" },
    "quiz-4": { east: "double", north: "long" },
    "racing": { south: "long", east: "hit", start: "double" },
    "paddle": { south: "tap" },
    "touchpad": { east: "bump" },
    "lanes-4": { south: "tap" },
    "two-choice": { east: "double" },
  };
  // Whether each player's phone vibrates, from the host's phone list.
  var vibrates = {};

  var layouts = Object.keys(ABOUT);
  var current = "stick-2";
  var phones = false;
  var switching = false;
  var holdStart = {};
  var mustRelease = {};
  var cards = {};

  function showLayout() {
    var info = ABOUT[current] || { name: current, about: "", buttons: [] };
    $("layout-name").textContent = info.name;
    $("layout-about").textContent = info.about;
    $("how").innerHTML = phones
      ? "Hold <kbd>A</kbd> for two seconds for the next layout. On the keyboard, <kbd>]</kbd> is next and <kbd>[</kbd> goes back."
      : "Phone controllers need the Giga Couch Game Browser. Pads and the keyboard still show here.";
    showSounds();
    $("strip").innerHTML = "";
    layouts.forEach(function (id) {
      var item = document.createElement("li");
      item.textContent = ABOUT[id].name;
      if (id === current) item.className = "on";
      $("strip").append(item);
    });
    Object.keys(cards).forEach(function (id) { shapeCard(cards[id]); });
  }

  // "A · ding (stock)   X · chime (this game)" under the layout name.
  function showSounds() {
    var map = SOUNDS[current] || {};
    var box = $("sounds");
    box.innerHTML = "";
    if (!phones) return;
    BUTTONS.forEach(function (button) {
      var sound = map[button[0]];
      var buzz = (RUMBLE[current] || {})[button[0]];
      if (!sound) return;
      var item = document.createElement("li");
      item.innerHTML = "<b></b> <span></span> <small></small>";
      item.querySelector("b").textContent = button[1];
      item.querySelector("b").style.background = button[2];
      item.querySelector("span").textContent = sound;
      item.querySelector("small").textContent = (STOCK.indexOf(sound) === -1 ? "this game's sound" : "stock") + (buzz ? " + " + buzz + " buzz" : "");
      box.append(item);
    });
  }

  function switchTo(index) {
    if (!phones || switching) return;
    var next = layouts[(index + layouts.length) % layouts.length];
    switching = true;
    window.GigaCouch.phone.setLayout(next).then(function () {
      current = next;
      showLayout();
      window.GigaCouch.phone.sound("all", "whoosh").catch(function () {});
      window.GigaCouch.phone.rumble("all", "tap").catch(function () {});
    }).catch(function () {}).then(function () { switching = false; });
  }

  function step(by) { switchTo(layouts.indexOf(current) + by); }

  window.addEventListener("keydown", function (event) {
    if (event.key === "]" || event.key === "PageDown") step(1);
    if (event.key === "[" || event.key === "PageUp") step(-1);
  });

  function makeCard(player, index) {
    var card = document.createElement("article");
    card.className = "card";
    card.style.setProperty("--player", PLAYER_COLORS[index % PLAYER_COLORS.length]);
    card.innerHTML =
      '<div class="who"><span class="badge"></span><span class="name"></span><span class="feel" hidden></span></div>' +
      '<div class="sticks">' +
      '<div class="stick move"><div class="ring"><div class="dot"></div></div><span></span></div>' +
      '<div class="stick look"><div class="ring"><div class="dot"></div></div><span></span></div>' +
      "</div>" +
      '<div class="buttons"></div><div class="hold"></div>';
    card.querySelector(".badge").textContent = "P" + player.id;
    var row = card.querySelector(".buttons");
    BUTTONS.forEach(function (button) {
      var el = document.createElement("span");
      el.className = "btn";
      el.dataset.key = button[0];
      el.textContent = button[1];
      el.style.setProperty("--key", button[2]);
      row.append(el);
    });
    var entry = { el: card, id: player.id };
    shapeCard(entry);
    return entry;
  }

  // Shows only the inputs the current layout has.
  function shapeCard(entry) {
    var info = ABOUT[current] || {};
    var move = entry.el.querySelector(".stick.move");
    var look = entry.el.querySelector(".stick.look");
    move.hidden = !info.move;
    move.querySelector("span").textContent = info.move || "";
    look.hidden = !info.look;
    look.classList.toggle("square", !!info.square);
    look.querySelector("span").textContent = info.look || "";
    entry.el.querySelectorAll(".btn").forEach(function (el) {
      el.classList.toggle("off", (info.buttons || []).indexOf(el.dataset.key) === -1);
    });
  }

  function place(dot, value) {
    dot.style.transform = "translate(" + (value.x * 160) + "%, " + (value.y * 160) + "%)";
  }

  function frame(now) {
    var api = window.GigaCouch;
    if (api) {
      var list = api.players.list();
      var seen = {};
      list.forEach(function (player, index) {
        seen[player.id] = true;
        var entry = cards[player.id];
        if (!entry) {
          entry = cards[player.id] = makeCard(player, index);
        }
        entry.el.querySelector(".name").textContent = player.name;
        var feel = entry.el.querySelector(".feel");
        var how = vibrates[player.id];
        feel.textContent = how === true ? "vibrates" : how === false ? "flashes" : "";
        feel.hidden = how === undefined;
        place(entry.el.querySelector(".move .dot"), api.input.axis(player.id, "move"));
        place(entry.el.querySelector(".look .dot"), api.input.axis(player.id, "look"));
        entry.el.querySelectorAll(".btn").forEach(function (el) {
          var key = el.dataset.key;
          el.classList.toggle("held", api.input.held(player.id, key));
          if (api.input.action(player.id, key)) {
            var sound = phones && (SOUNDS[current] || {})[key];
            if (sound) api.phone.sound(player.id, sound).catch(function () {});
            var buzz = phones && (RUMBLE[current] || {})[key];
            if (buzz) api.phone.rumble(player.id, buzz).catch(function () {});
            el.classList.add("flash");
            setTimeout(function () { el.classList.remove("flash"); }, 160);
          }
        });
        // Hold A to switch everyone to the next layout.
        var holding = api.input.held(player.id, "south");
        if (!holding) {
          delete holdStart[player.id];
          mustRelease[player.id] = false;
        } else if (!mustRelease[player.id]) {
          holdStart[player.id] = holdStart[player.id] || now;
          if (now - holdStart[player.id] >= HOLD_MS) {
            mustRelease[player.id] = true;
            delete holdStart[player.id];
            step(1);
          }
        }
        var progress = holdStart[player.id] ? Math.min(1, (now - holdStart[player.id]) / HOLD_MS) : 0;
        entry.el.querySelector(".hold").style.width = progress * 100 + "%";
      });
      Object.keys(cards).forEach(function (id) {
        if (!seen[id]) {
          cards[id].el.remove();
          delete cards[id];
        }
      });
      var box = $("players");
      var order = list.map(function (player) { return cards[player.id].el; });
      if (!list.length) {
        if (!box.querySelector(".empty")) {
          box.innerHTML = '<p class="empty">Nobody has joined yet. Scan the code with a phone, then press A. A pad works too.</p>';
        }
      } else {
        var empty = box.querySelector(".empty");
        if (empty) empty.remove();
        order.forEach(function (el, index) {
          if (box.children[index] !== el) box.insertBefore(el, box.children[index] || null);
        });
      }
    }
    requestAnimationFrame(frame);
  }

  // The join code, from the same host that serves this game.
  function showJoin() {
    fetch("/__gigacouch/v1/phones", { cache: "no-store" })
      .then(function (response) { return response.json(); })
      .then(function (status) {
        vibrates = {};
        (status.phones || []).forEach(function (phone) {
          if (phone.player) vibrates[phone.player] = !!phone.rumble;
        });
        var ready = status.enabled && status.join_url;
        $("join").hidden = !ready;
        if (ready) {
          $("join-qr").innerHTML = status.qr_svg || "";
          $("join-url").textContent = status.join_url.replace(/^https?:\/\//, "");
        }
      })
      .catch(function () { $("join").hidden = true; });
  }

  function start() {
    if (!window.GigaCouch || !window.GigaCouch.phone) {
      setTimeout(start, 50);
      return;
    }
    window.GigaCouch.phone.info().then(function (info) {
      phones = !!(info && info.layout);
      if (info && info.layouts) {
        layouts = info.layouts.filter(function (id) { return ABOUT[id]; });
      }
      if (info && info.layout) current = info.layout;
    }).catch(function () {}).then(function () {
      showLayout();
      showJoin();
      setInterval(showJoin, 2000);
      requestAnimationFrame(frame);
    });
  }
  start();
})();
