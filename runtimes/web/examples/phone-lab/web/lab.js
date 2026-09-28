// Phone Lab: try what phones can do beyond buttons.
//
// Each station shows one feature a game can opt into. Every station's phone
// layout has a Next button (the lab's own layouts, from gigacouch.json), and
// on the keyboard ] and [ step through, or 1 to 9 jump.
(function () {
  var $ = function (id) { return document.getElementById(id); };
  var api = null;
  var players = [];
  var phones = [];
  var station = 0;
  var tick = 0;
  var CARDS = ["sun", "moon", "star", "comet"];
  var PLAYER_COLORS = ["#8ce8be", "#9abef7", "#f3c77d", "#ec9a7a", "#c9b3f5", "#f2a7c9", "#7fd6d6", "#d9e58a"];
  var NEXT = { id: "next", label: "Next station ▸", color: "panel" };

  function el(tag, className, text) {
    var node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  }
  function colorOf(id) { return PLAYER_COLORS[(id - 1) % PLAYER_COLORS.length]; }
  function nameOf(id) {
    var one = players.find(function (player) { return player.id === id; });
    return one ? one.name : "Player " + id;
  }
  function ignore() {}

  // ---- Stations -------------------------------------------------------------

  var STATIONS = [
    // 1. Private hands: each phone sees only its own cards.
    {
      title: "Private hands",
      about: "Each phone shows its own three cards. Only you can see them. Play one, and the TV reveals everyone's once all are in.",
      how: "Tap a card on your phone.",
      layout: "lab-pad",
      enter: function (s) { s.hands = {}; s.played = {}; s.revealAt = 0; },
      exit: function () { api.phone.show("all", null).catch(ignore); },
      frame: function (s) {
        players.forEach(function (player) {
          if (s.hands[player.id]) return;
          var hand = [0, 1, 2].map(function () { return CARDS[Math.floor(Math.random() * CARDS.length)]; });
          s.hands[player.id] = hand;
          api.phone.show(player.id, {
            id: "hand",
            title: "Your hand",
            text: "Play one card. Only you can see these.",
            choices: hand.map(function (card, index) {
              return { id: card + "-" + index, label: card.charAt(0).toUpperCase() + card.slice(1), image: card };
            }).concat([NEXT]),
          }).catch(ignore);
        });
        var everyone = players.length > 0 && players.every(function (player) { return s.played[player.id]; });
        if (everyone && !s.revealAt) s.revealAt = performance.now() + 5000;
        if (s.revealAt && performance.now() > s.revealAt) {
          s.hands = {};
          s.played = {};
          s.revealAt = 0;
        }
      },
      event: function (s, event) {
        if (event.type !== "choice" || event.screen !== "hand" || !event.player) return;
        s.played[event.player] = event.choice.split("-")[0];
        api.phone.show(event.player, { id: "hand", title: "Played", text: "Waiting for everyone else…", choices: [NEXT] }).catch(ignore);
      },
      draw: function (s) {
        if (!players.length) return emptyStage();
        var reveal = !!s.revealAt;
        var grid = el("div", "grid");
        players.forEach(function (player) {
          var tile = playerTile(player);
          var cards = el("div", "cards");
          var card = el("div", "card");
          if (s.played[player.id] && reveal) {
            var img = el("img");
            img.src = "images/" + s.played[player.id] + ".png";
            img.alt = s.played[player.id];
            card.append(img);
          }
          cards.append(card);
          tile.append(cards, el("p", "note", s.played[player.id] ? (reveal ? "Played the " + s.played[player.id] : "Played, face down") : "Choosing…"));
          grid.append(tile);
        });
        return grid;
      },
    },
    // 2. Typing: captions from the phone's own keyboard.
    {
      title: "Captions",
      about: "Everyone writes a caption for the picture on their phone's keyboard. Answers appear here as they come in.",
      how: "Type on your phone. Press A on your phone to write another.",
      layout: "lab-pad",
      enter: function (s) { s.answers = []; askCaption("all"); },
      exit: function () {
        api.phone.ask("all", null).catch(ignore);
        api.phone.show("all", null).catch(ignore);
      },
      frame: function (s) {
        players.forEach(function (player) {
          if (api.input.action(player.id, "south")) askCaption(player.id);
        });
      },
      event: function (s, event) {
        if (event.type === "text" && event.ask === "caption") {
          s.answers.unshift({ name: event.name, text: event.text });
          s.answers = s.answers.slice(0, 8);
          if (event.player) {
            api.phone.show(event.player, {
              id: "sent",
              title: "Sent",
              text: "Your caption is on the TV.",
              choices: [{ id: "again", label: "Write another", color: "mint" }, NEXT],
            }).catch(ignore);
          }
        }
        if (event.type === "choice" && event.choice === "again" && event.player) {
          api.phone.show(event.player, null).catch(ignore);
          askCaption(event.player);
        }
      },
      draw: function (s) {
        var split = el("div", "split");
        var img = el("img", "scene");
        img.src = "images/scene.png";
        img.alt = "A couch on a hill at dusk, under a big moon";
        var list = el("div", "answers");
        if (!s.answers.length) list.append(el("p", "note", "No captions yet."));
        s.answers.forEach(function (answer) {
          var item = el("div", "answer");
          item.append(el("small", "", answer.name), answer.text);
          list.append(item);
        });
        split.append(img, list);
        return split;
      },
    },
    // 3. Drawing: strokes arrive live from each phone's canvas.
    {
      title: "Drawing",
      about: "Draw on your phone and it appears here stroke by stroke. Pick a color on the side, or clear to start over.",
      how: "Draw with a finger.",
      layout: "lab-draw",
      enter: function (s) { s.sketches = {}; },
      event: function (s, event) {
        if (!event.player) return;
        var sketch = s.sketches[event.player] || (s.sketches[event.player] = { strokes: [], dirty: true });
        if (event.type === "clear") sketch.strokes = [];
        if (event.type === "stroke") {
          var last = sketch.strokes[sketch.strokes.length - 1];
          if (!last || last.number !== event.stroke) {
            last = { number: event.stroke, color: event.color, points: [] };
            sketch.strokes.push(last);
          }
          last.points = last.points.concat(event.points);
        }
        sketch.dirty = true;
      },
      draw: function (s) {
        if (!players.length) return emptyStage();
        var grid = el("div", "grid");
        players.forEach(function (player) {
          var tile = playerTile(player);
          var canvas = el("canvas", "sketch");
          canvas.width = 640;
          canvas.height = 400;
          var ink = canvas.getContext("2d");
          ink.lineCap = "round";
          ink.lineJoin = "round";
          ink.lineWidth = 6;
          var colors = { mint: "#8ce8be", blue: "#9abef7", amber: "#f3c77d", coral: "#ec9a7a", panel: "#f2f5f8" };
          ((s.sketches[player.id] || {}).strokes || []).forEach(function (stroke) {
            ink.strokeStyle = colors[stroke.color] || colors.mint;
            ink.beginPath();
            stroke.points.forEach(function (point, index) {
              var x = point[0] * canvas.width;
              var y = point[1] * canvas.height;
              if (index === 0) ink.moveTo(x, y); else ink.lineTo(x, y);
            });
            ink.stroke();
          });
          tile.append(canvas);
          grid.append(tile);
        });
        return grid;
      },
    },
    // 4. Audience vote: players and watching phones vote together.
    {
      title: "Audience vote",
      about: "Everyone votes on their phone, players and audience alike. Anyone can join the audience from the phone menu: Watch as the audience.",
      how: "Tap an answer. Press N on the keyboard for a new question.",
      layout: "lab-pad",
      questions: [
        ["Best couch snack?", ["Popcorn", "Nachos", "Fruit", "Cookies"]],
        ["Who wins a pillow fight?", ["Grandma", "The cat", "The dog", "Nobody"]],
        ["Best game night?", ["Friday", "Saturday", "Sunday", "Every night"]],
      ],
      enter: function (s) { s.round = s.round === undefined ? 0 : s.round; newVote(s); },
      exit: function () { api.phone.show("all", null).catch(ignore); },
      key: function (s, key) { if (key === "n") { s.round += 1; newVote(s); } },
      event: function (s, event) {
        if (event.type !== "choice" || event.screen !== "vote") return;
        s.votes[event.phone] = { choice: event.choice, audience: event.audience };
      },
      draw: function (s) {
        var box = el("div");
        var question = STATIONS[3].questions[s.round % STATIONS[3].questions.length];
        box.append(el("p", "question", question[0]));
        var bars = el("div", "bars");
        var votes = Object.keys(s.votes).map(function (key) { return s.votes[key]; });
        question[1].forEach(function (option, index) {
          var count = votes.filter(function (vote) { return vote.choice === "o" + index; }).length;
          var fromAudience = votes.filter(function (vote) { return vote.choice === "o" + index && vote.audience; }).length;
          var row = el("div", "bar-row");
          var bar = el("div", "bar");
          var fill = el("i");
          fill.style.width = votes.length ? (100 * count) / votes.length + "%" : "0%";
          bar.append(fill);
          row.append(el("span", "", option), bar, el("span", "", count + (fromAudience ? " (" + fromAudience + " audience)" : "")));
          bars.append(row);
        });
        var audienceCount = phones.filter(function (phone) { return phone.audience; }).length;
        box.append(bars, el("p", "note", votes.length + " votes · " + audienceCount + " phones watching as the audience"));
        return box;
      },
    },
    // 5. Button labels: each quiz answer written on a button.
    {
      title: "Answers on the buttons",
      about: "The game writes each answer on a button, so your phone reads Paris, Rome, and so on instead of A, B, X, Y.",
      how: "Press the answer on your phone.",
      layout: "lab-quiz",
      quiz: [
        ["Capital of France?", ["Paris", "Rome", "Oslo", "Lima"], 0],
        ["How many legs does a spider have?", ["Six", "Eight", "Ten", "Twelve"], 1],
        ["Which planet is red?", ["Venus", "Jupiter", "Mars", "Saturn"], 2],
        ["What do bees make?", ["Milk", "Silk", "Wax only", "Honey"], 3],
      ],
      keys: ["south", "east", "west", "north"],
      enter: function (s) { s.round = s.round || 0; newQuiz(s); },
      exit: function () { api.phone.setLabels("all", null).catch(ignore); },
      frame: function (s) {
        var station = STATIONS[4];
        players.forEach(function (player) {
          station.keys.forEach(function (key, index) {
            if (api.input.action(player.id, key) && s.answers[player.id] === undefined && !s.showAt) {
              s.answers[player.id] = index;
            }
          });
        });
        var everyone = players.length > 0 && players.every(function (player) { return s.answers[player.id] !== undefined; });
        if ((everyone || performance.now() - s.startedAt > 12000) && !s.showAt && players.length) {
          s.showAt = performance.now();
        }
        if (s.showAt && performance.now() - s.showAt > 4000) {
          s.round += 1;
          newQuiz(s);
        }
      },
      draw: function (s) {
        var station = STATIONS[4];
        var item = station.quiz[s.round % station.quiz.length];
        var box = el("div");
        box.append(el("p", "question", item[0]));
        var options = el("div", "options");
        var colors = ["var(--mint)", "var(--coral)", "var(--blue)", "var(--amber)"];
        item[1].forEach(function (answer, index) {
          var option = el("div", "option" + (s.showAt && index === item[2] ? " right" : ""), answer);
          option.style.background = colors[index];
          var who = players.filter(function (player) { return s.answers[player.id] === index; }).map(function (player) { return player.name; });
          option.append(el("small", "", s.showAt ? who.join(", ") || "Nobody" : who.length + " answered"));
          options.append(option);
        });
        box.append(options);
        return box;
      },
    },
    // 6. Custom layout: the lab's own Rock, Paper, Scissors buttons.
    {
      title: "Custom layout",
      about: "This layout comes from the game itself, in its gigacouch.json: three tall buttons named Rock, Paper, and Scissors.",
      how: "Pick one on your phone. With one player, the TV plays too.",
      layout: "lab-rps",
      enter: function (s) { s.picks = {}; s.doneAt = 0; s.tv = null; },
      frame: function (s) {
        var names = { south: "Rock", east: "Paper", west: "Scissors" };
        players.forEach(function (player) {
          Object.keys(names).forEach(function (key) {
            if (api.input.action(player.id, key) && !s.doneAt) s.picks[player.id] = names[key];
          });
        });
        var picked = players.filter(function (player) { return s.picks[player.id]; });
        if (!s.doneAt && players.length && picked.length === players.length) {
          if (players.length === 1) s.tv = ["Rock", "Paper", "Scissors"][Math.floor(Math.random() * 3)];
          s.doneAt = performance.now();
        }
        if (s.doneAt && performance.now() - s.doneAt > 3500) { s.picks = {}; s.doneAt = 0; s.tv = null; }
      },
      draw: function (s) {
        if (!players.length) return emptyStage();
        var beats = { Rock: "Scissors", Paper: "Rock", Scissors: "Paper" };
        var grid = el("div", "grid");
        var all = players.map(function (player) { return { name: player.name, pick: s.picks[player.id], id: player.id }; });
        if (s.tv) all.push({ name: "The TV", pick: s.tv });
        all.forEach(function (one) {
          var tile = el("article", "tile");
          tile.append(el("h3", "", one.name));
          tile.append(el("p", "pick", s.doneAt ? one.pick : one.pick ? "Ready" : "…"));
          if (s.doneAt) {
            var wins = all.filter(function (other) { return beats[one.pick] === other.pick; }).length;
            var losses = all.filter(function (other) { return beats[other.pick] === one.pick; }).length;
            tile.append(el("p", "note", wins > losses ? "Wins" : losses > wins ? "Loses" : "Draw"));
          }
          grid.append(tile);
        });
        return grid;
      },
    },
    // 7. Photos and profiles.
    {
      title: "Photos and names",
      about: "Take a photo from the phone menu and it shows here. Link your phone to a name from Home with Who's playing, and your name follows you.",
      how: "On your phone: Take a photo, or Who's playing to pick your name.",
      layout: "lab-pad",
      phoneScreen: {
        id: "photos",
        title: "Your photo and name",
        text: "Take a photo for the TV, or link this phone to a name from Home.",
        choices: [
          { id: "photo", label: "Take a photo", action: "photo", color: "mint" },
          { id: "profile", label: "Who's playing", action: "profile", color: "blue" },
          NEXT,
        ],
      },
      draw: function () {
        if (!players.length) return emptyStage();
        var grid = el("div", "grid");
        players.forEach(function (player) {
          var tile = el("article", "tile");
          var face;
          if (player.avatar) {
            face = el("img", "avatar");
            face.src = player.avatar;
            face.alt = player.name;
          } else {
            face = el("div", "avatar", (player.name || "?").charAt(0).toUpperCase());
            face.style.background = colorOf(player.id);
            face.style.color = "var(--bg)";
          }
          tile.append(face, el("h3", "", player.name));
          tile.append(el("p", "note", (player.profile ? "Linked to " + player.profile + " · " : "") + (player.kind === "phone" ? "phone" : player.kind)));
          grid.append(tile);
        });
        return grid;
      },
    },
    // 8. Connection and rejoin.
    {
      title: "Connection and rejoin",
      about: "Each phone's round trip to the computer, and what happens when a phone drops: it keeps its player number for two minutes.",
      how: "Try it: lock your phone for ten seconds, then open it again. You come back as the same player.",
      layout: "lab-pad",
      phoneScreen: {
        id: "connection",
        title: "Leave and come back",
        text: "Lock your phone for ten seconds, then open it again. You come back as the same player. The bars at the top of this screen show your connection.",
        choices: [NEXT],
      },
      draw: function () {
        if (!phones.length && !players.length) return emptyStage();
        var grid = el("div", "grid");
        players.forEach(function (player) {
          var phone = phones.find(function (one) { return one.player === player.id; });
          var tile = playerTile(player);
          var rtt = phone && phone.rtt;
          var signal = el("span", "signal " + (!rtt ? "" : rtt < 60 ? "good" : rtt < 150 ? "fair" : "poor"));
          signal.append(el("i"), el("i"), el("i"));
          var line = el("p", "note");
          line.append(signal, " " + (player.away ? "Away: holding Player " + player.id : rtt ? rtt + " ms round trip" : player.kind === "phone" ? "Measuring…" : "Not a phone"));
          tile.append(line);
          grid.append(tile);
        });
        phones.filter(function (phone) { return phone.audience; }).forEach(function (phone) {
          var tile = el("article", "tile");
          tile.append(el("h3", "", phone.name), el("p", "note", "Watching as the audience"));
          grid.append(tile);
        });
        return grid;
      },
    },
    // 9. Rumble for phones and pads.
    {
      title: "Rumble",
      about: "One call buzzes whatever a player holds: Android phones vibrate, iPhones flash their edges, and Xbox and PlayStation pads rumble.",
      how: "A is a hit, B a double tap, Y a heartbeat.",
      layout: "lab-pad",
      labels: { south: "Hit", east: "Double", north: "Heartbeat" },
      frame: function (s) {
        var feel = { south: "hit", east: "double", north: "heartbeat" };
        players.forEach(function (player) {
          Object.keys(feel).forEach(function (key) {
            if (api.input.action(player.id, key)) {
              api.phone.rumble(player.id, feel[key]).catch(ignore);
              s.last = { id: player.id, feel: feel[key], at: performance.now() };
            }
          });
        });
      },
      draw: function (s) {
        if (!players.length) return emptyStage();
        var grid = el("div", "grid");
        players.forEach(function (player) {
          var phone = phones.find(function (one) { return one.player === player.id; });
          var tile = playerTile(player);
          var how = player.kind === "pad" ? "Pad: rumbles" : phone ? (phone.rumble ? "Phone: vibrates" : "Phone: flashes its edges") : player.kind;
          tile.append(el("p", "note", how));
          if (s.last && s.last.id === player.id && performance.now() - s.last.at < 600) {
            tile.classList.add("flash");
            tile.append(el("p", "pick", s.last.feel));
          }
          grid.append(tile);
        });
        return grid;
      },
    },
  ];

  var state = {};

  function playerTile(player) {
    var tile = el("article", "tile");
    var head = el("h3");
    var badge = el("span", "badge", "P" + player.id);
    badge.style.background = colorOf(player.id);
    head.append(badge, player.name);
    if (player.away) head.append(el("span", "tag", "away"));
    tile.append(head);
    return tile;
  }

  function emptyStage() {
    return el("p", "empty", "Nobody has joined yet. Scan the code with a phone, then press A.");
  }

  function askCaption(target) {
    api.phone.ask(target, { id: "caption", prompt: "Write a caption for this picture", placeholder: "A caption", max: 80 }).catch(ignore);
  }

  function newVote(s) {
    var question = STATIONS[3].questions[s.round % STATIONS[3].questions.length];
    s.votes = {};
    api.phone.show("all", {
      id: "vote",
      title: question[0],
      text: "Everyone votes: players and audience.",
      choices: question[1].map(function (option, index) {
        return { id: "o" + index, label: option, color: ["mint", "coral", "blue", "amber"][index] };
      }).concat([NEXT]),
    }).catch(ignore);
  }

  function newQuiz(s) {
    var station = STATIONS[4];
    var item = station.quiz[s.round % station.quiz.length];
    s.answers = {};
    s.showAt = 0;
    s.startedAt = performance.now();
    var labels = {};
    station.keys.forEach(function (key, index) { labels[key] = item[1][index]; });
    api.phone.setLabels("all", labels).catch(ignore);
  }

  // ---- Moving between stations ----------------------------------------------

  function go(index) {
    var from = STATIONS[station];
    if (from.exit) from.exit(state[station] || {});
    station = (index + STATIONS.length) % STATIONS.length;
    var to = STATIONS[station];
    state[station] = state[station] || {};
    api.phone.setLayout(to.layout).catch(ignore);
    if (from.phoneScreen) api.phone.show("all", null).catch(ignore);
    if (from.labels) api.phone.setLabels("all", null).catch(ignore);
    if (to.labels) api.phone.setLabels("all", to.labels).catch(ignore);
    state[station].shownTo = -1;
    if (to.enter) to.enter(state[station]);
    api.phone.sound("all", "whoosh").catch(ignore);
    showStation();
  }

  function showStation() {
    var to = STATIONS[station];
    $("title").textContent = to.title;
    $("about").textContent = to.about;
    $("how").innerHTML = "";
    $("how").append(to.how + " ", el("span", "note", "Next station: the Next button on your phone, or "), el("kbd", "", "]"));
    $("strip").innerHTML = "";
    STATIONS.forEach(function (one, index) {
      var item = el("li", index === station ? "on" : "");
      item.append(el("b", "", String(index + 1)), one.title);
      $("strip").append(item);
    });
  }

  window.addEventListener("keydown", function (event) {
    if (event.key === "]" || event.key === "PageDown") go(station + 1);
    else if (event.key === "[" || event.key === "PageUp") go(station - 1);
    else if (/^[1-9]$/.test(event.key)) go(Number(event.key) - 1);
    else if (STATIONS[station].key) STATIONS[station].key(state[station], event.key.toLowerCase());
  });

  // The phone list: audience phones, round trips, who can vibrate.
  function pollPhones() {
    fetch("/__gigacouch/v1/phones", { cache: "no-store" })
      .then(function (response) { return response.json(); })
      .then(function (status) {
        phones = status.phones || [];
        var ready = status.enabled && status.join_url;
        $("join").hidden = !ready;
        if (ready) {
          $("join-qr").innerHTML = status.qr_svg || "";
          $("join-url").textContent = status.join_url.replace(/^https?:\/\//, "");
        }
      })
      .catch(function () { $("join").hidden = true; });
  }

  function frame() {
    // Always come back next frame, so one bad frame can never freeze the lab.
    requestAnimationFrame(frame);
    players = api.players.list();
    var current = STATIONS[station];
    var s = state[station];
    // Next: the lab's layouts put it on Start; screens offer it as a choice.
    // Collect it and move on after this frame, never in the middle of it:
    // the rest of the frame belongs to the station it started in.
    var next = players.some(function (player) { return api.input.action(player.id, "start"); });
    api.phone.events().forEach(function (event) {
      if (event.type === "choice" && event.choice === "next") next = true;
      else if (current.event && !next) current.event(s, event);
    });
    if (next) {
      go(station + 1);
      return;
    }
    // A station's phone screen goes to every phone, again when phones join.
    if (current.phoneScreen && s.shownTo !== phones.length) {
      s.shownTo = phones.length;
      api.phone.show("all", current.phoneScreen).catch(ignore);
    }
    if (current.frame) current.frame(s);
    tick += 1;
    if (tick % 6 === 0) {
      var stage = $("stage");
      stage.innerHTML = "";
      stage.append(current.draw(s));
    }
  }

  function start() {
    api = window.GigaCouch;
    if (!api || !api.phone || !api.phone.show) {
      setTimeout(start, 50);
      return;
    }
    state[0] = {};
    api.phone.setLayout(STATIONS[0].layout).catch(ignore);
    STATIONS[0].enter(state[0]);
    showStation();
    pollPhones();
    setInterval(pollPhones, 1000);
    requestAnimationFrame(frame);
  }
  start();
})();
