(function () {
  var app = document.getElementById("app");
  var screen = "who";
  var profiles = [];
  var active = "family";
  var games = [];
  var focus = 0;
  var shelfRow = 0;
  var shelfIndex = 0;
  var account = { signed_in: false, server: "off", name: "", user_code: "", verification_uri: "" };
  var started = false;
  var askedCode = false;
  var lastAccountPoll = 0;
  var spelling = "";
  var keyFocus = 0;
  var hold = 0;
  var opening = false;
  var notice = "";
  var keys = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").concat(["space", "delete", "done"]);
  var phoneStatus = null;
  var phonesDrawn = "";
  var lastPhonePoll = 0;

  function escapeHtml(text) {
    return String(text).replace(/[&<>"']/g, function (ch) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;" }[ch];
    });
  }

  // Phones join through the host's second listener. The card shows the QR
  // code for it and every phone that has connected, with its player number.
  function pollPhones() {
    fetch("/__gigacouch/v1/phones", { cache: "no-store" })
      .then(function (response) { return response.json(); })
      .then(function (data) { phoneStatus = data; })
      .catch(function () { phoneStatus = null; });
  }

  function drawPhones() {
    var box = document.getElementById("phones");
    // Every screen but the spelling keyboard, which needs the full width.
    var visible = !!(phoneStatus && phoneStatus.enabled) && screen !== "name";
    var html = "";
    if (visible) {
      // A big party can have dozens of phones: list the first few, then a count.
      var SHOWN = 6;
      var all = phoneStatus.phones || [];
      var list = all.slice(0, SHOWN).map(function (phone) {
        var badge = phone.player
          ? '<span class="badge">P' + phone.player + "</span>"
          : '<span class="badge waiting">·</span>';
        var note = phone.quiet ? "<small>reconnecting</small>" : phone.player ? "" : "<small>press A to join</small>";
        return "<li>" + badge + "<span>" + escapeHtml(phone.name) + "</span>" + note + "</li>";
      }).join("");
      if (all.length > SHOWN) {
        list += '<li><span class="badge waiting">+' + (all.length - SHOWN) + "</span><span>more phones</span></li>";
      }
      if (phoneStatus.join_url) {
        html = "<h2>Play on your phone</h2>" +
          '<div class="qr">' + (phoneStatus.qr_svg || "") + "</div>" +
          "<p>Scan with the camera. Use the same Wi-Fi as this computer.</p>" +
          '<p class="address">' + escapeHtml(phoneStatus.join_url.replace(/^https?:\/\//, "")) + "</p>" +
          (list ? "<ul>" + list + "</ul>" : "");
      } else {
        html = "<h2>Play on your phone</h2><p>Connect this computer to Wi-Fi to play with phones.</p>" +
          (list ? "<ul>" + list + "</ul>" : "");
      }
    }
    if (html !== phonesDrawn) {
      box.innerHTML = html;
      box.hidden = !visible;
      document.body.classList.toggle("with-phones", visible);
      phonesDrawn = html;
    }
  }

  function currentName() {
    var person = profiles.find(function (item) { return item.id === active; });
    return person ? person.name : "Family";
  }

  function load() {
    return fetch("/__gigacouch/v1/home", { cache: "no-store" })
      .then(function (response) { return response.json(); })
      .then(function (data) {
        profiles = data.profiles || [];
        active = data.active || "family";
        games = data.games || [];
        account = data.account || account;
        if (!started) {
          started = true;
          focus = Math.max(0, profiles.findIndex(function (item) { return item.id === active; }));
          if (account.server === "ok" && !account.signed_in) {
            screen = "account";
          }
        }
        if (account.signed_in && screen === "account") {
          screen = "who";
          focus = Math.max(0, profiles.findIndex(function (item) { return item.id === active; }));
        }
      });
  }

  function post(path, body) {
    return fetch(path, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    }).then(function () { return load(); });
  }

  function choiceCount() {
    return profiles.length + 1;
  }

  function nudge(x, y) {
    notice = "";
    if (screen === "account") {
      focus = focus === 0 ? 1 : 0;
    } else if (screen === "who") {
      focus = (focus + (x > 0 ? 1 : -1) + choiceCount()) % choiceCount();
    } else if (screen === "shelf") {
      var rows = shelfRows();
      var current = rows[shelfRow] || [];
      if (y !== 0) {
        var nextRow = shelfRow + y;
        if (nextRow >= 0 && nextRow < rows.length && rows[nextRow].length) {
          shelfRow = nextRow;
          shelfIndex = Math.min(shelfIndex, rows[nextRow].length - 1);
        }
      } else if (x !== 0 && current.length) {
        shelfIndex = (shelfIndex + x + current.length) % current.length;
      }
    } else {
      var columns = 7;
      if (x !== 0) {
        keyFocus = Math.max(0, Math.min(keys.length - 1, keyFocus + x));
      }
      if (y !== 0) {
        keyFocus = Math.max(0, Math.min(keys.length - 1, keyFocus + y * columns));
      }
    }
  }

  function activate() {
    if (screen === "account") {
      if (focus === 1) {
        screen = "who";
      }
      return;
    }
    if (screen === "who") {
      if (focus === profiles.length) {
        screen = "name";
        spelling = "";
        keyFocus = 0;
        return;
      }
      post("/__gigacouch/v1/home/use", { id: profiles[focus].id }).then(function () {
        screen = "shelf";
        focus = 0;
      });
      return;
    }
    if (screen === "name") {
      var key = keys[keyFocus];
      if (key === "space" && spelling && !spelling.endsWith(" ")) {
        spelling += " ";
      } else if (key === "delete") {
        spelling = spelling.slice(0, -1);
      } else if (key === "done") {
        var name = spelling.trim();
        if (!name) {
          return;
        }
        post("/__gigacouch/v1/home/add", { name: name }).then(function () {
          screen = "shelf";
          focus = 0;
        });
      } else if (spelling.length < 24) {
        spelling += key;
      }
      return;
    }
    var game = focusedGame();
    if (!game || opening) {
      return;
    }
    notice = "";
    if (game.action === "download") {
      opening = true;
      notice = "Downloading " + game.title + "…";
      drawn = "";
      fetch("/__gigacouch/v1/library/download", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: game.id }),
      })
        .then(function (response) { return response.json(); })
        .then(function (body) {
          opening = false;
          notice = body.ok ? "Downloaded. South starts it in this window." : (body.error || "The download failed.");
          drawn = "";
          return load();
        })
        .catch(function () {
          opening = false;
          notice = "The download failed.";
          drawn = "";
        });
      return;
    }
    if (game.runtime === "web-1") {
      window.location.href = "/play/" + game.id + "/";
      return;
    }
    if (!game.playable) {
      notice = game.reason || "This one cannot be opened from Home.";
      return;
    }
    opening = true;
    notice = "Opening " + game.title + "…";
    drawn = "";
    fetch("/__gigacouch/v1/home/play", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id: game.id }),
    })
      .then(function (response) { return response.json(); })
      .then(function (body) {
        opening = false;
        notice = body.ok ? body.message : (body.error || "The game did not open.");
        drawn = "";
      })
      .catch(function () {
        opening = false;
        notice = "The game did not open.";
        drawn = "";
      });
  }

  function retreat() {
    if (screen === "shelf") {
      screen = "who";
      focus = Math.max(0, profiles.findIndex(function (item) { return item.id === active; }));
    } else if (screen === "name") {
      screen = "who";
      focus = profiles.length;
    }
  }

  var drawn = "";

  function draw() {
    var next = screen === "account" ? accountHtml() : screen === "who" ? whoHtml() : screen === "name" ? nameHtml() : shelfHtml();
    if (next !== drawn) {
      app.innerHTML = next;
      drawn = next;
      var focused = app.querySelector(".cover.focus");
      if (focused && focused.scrollIntoView) {
        focused.scrollIntoView({ inline: "nearest", block: "nearest" });
      }
    }
  }

  function accountHtml() {
    if (!askedCode && account.server === "ok") {
      askedCode = true;
      fetch("/__gigacouch/v1/account/begin", { method: "POST" })
        .then(function (response) { return response.json().then(function (body) { return { ok: response.ok, body: body }; }); })
        .then(function (result) {
          if (!result.ok) {
            account.server = "down";
            screen = "who";
            drawn = "";
            return;
          }
          return load();
        })
        .catch(function () {
          account.server = "down";
          screen = "who";
          drawn = "";
        });
    }
    var code = account.user_code || "……";
    var link = account.verification_uri || "";
    var stay = focus === 1 ? " focus" : "";
    return "<h1>Sign in</h1><p class=\"lede\">On this computer, open the link and enter the code. A new name creates the account.</p><p class=\"code\">" + code + "</p><p class=\"lede\">" + link + "</p><div class=\"people\"><button class=\"choice" + stay + "\">Use this Mac only<small>Skip the shared library</small></button></div><p class=\"hint\">Waiting for that page.</p>";
  }

  function whoHtml() {
    var cards = profiles.map(function (person, index) {
      return '<button class="choice' + (index === focus ? " focus" : "") + '">' + person.name + "<small>On this Mac</small></button>";
    }).join("");
    cards += '<button class="choice' + (focus === profiles.length ? " focus" : "") + '">Add someone<small>Spell a name</small></button>';
    return "<h1>Who's on the couch?</h1><p class=\"lede\">Pick a name. Games and saves stay on this computer.</p><div class=\"people\">" + cards + "</div><p class=\"hint\">Move with the stick. South confirms.</p>";
  }

  function nameHtml() {
    var board = keys.map(function (key, index) {
      var label = key === "space" ? "Space" : key === "delete" ? "Delete" : key === "done" ? "Done" : key;
      return '<button class="key' + (index === keyFocus ? " focus" : "") + '">' + label + "</button>";
    }).join("");
    return "<h1>Spell the name</h1><p class=\"spelling\">" + (spelling || "…") + "</p><div class=\"keys\">" + board + "</div><p class=\"hint\">South adds a letter. East goes back. Or type the name on any phone.</p>";
  }

  function shelfRows() {
    return [
      games.filter(function (game) { return game.place !== "library"; }),
      games.filter(function (game) { return game.place === "library"; }),
    ];
  }

  function clampShelf() {
    var rows = shelfRows();
    if (rows[shelfRow] && rows[shelfRow][shelfIndex]) {
      return;
    }
    shelfRow = rows[0].length ? 0 : 1;
    shelfIndex = 0;
  }

  function focusedGame() {
    clampShelf();
    var rows = shelfRows();
    var row = rows[shelfRow] || [];
    return row[shelfIndex] || null;
  }

  function coverRow(items, rowIndex) {
    if (!items.length) {
      var empty = rowIndex === 0
        ? "Nothing on this Mac yet."
        : (account.server === "down"
          ? "The library server is not running. Games on this Mac still play."
          : account.signed_in
            ? "Nothing else in the library."
            : "Sign in to see the library.");
      return '<p class="empty">' + empty + "</p>";
    }
    return '<div class="row">' + items.map(function (item, index) {
      var selected = rowIndex === shelfRow && index === shelfIndex;
      return '<div class="cover' + (selected ? " focus" : "") + '" style="--cover:' + item.color + '">' + item.title + "</div>";
    }).join("") + "</div>";
  }

  function shelfHtml() {
    var game = focusedGame();
    if (!game) {
      return "<h1>Nothing to play yet</h1>";
    }
    var rows = shelfRows();
    var line = notice || (game.action === "download"
      ? "In the library. South downloads it to this Mac."
      : game.place === "library"
        ? "In the library. No download for this one yet."
        : game.installed
          ? "Downloaded. South starts it in this window."
          : game.runtime === "web-1"
            ? "On this Mac. South starts it in this window."
            : "On this Mac. South opens it in its own window.");
    var who = account.signed_in ? currentName() + ", signed in as " + account.name : currentName();
    return '<section class="shelf"><p class="lede">' + who + '</p><div class="stage"><div class="stage-mark" style="--stage:' + game.color + '"></div><div><h1>' + game.title + "</h1><p>" + game.description + "</p><small>" + game.players + '</small><p class="play-line">' + line + '</p></div></div><div class="bands"><section><h2>On this Mac</h2>' + coverRow(rows[0], 0) + '</section><section><h2>Library</h2>' + coverRow(rows[1], 1) + "</section></div></section><p class=\"hint\">Stick moves. South confirms. East goes back to names.</p>";
  }

  // Phones help Home: the name screen asks every phone to type the name,
  // and a phone can pick a game to start from its menu.
  var askedName = false;
  var lastRemotePoll = 0;
  function phoneHelp() {
    var phone = window.GigaCouch && window.GigaCouch.phone;
    if (!phone) return;
    if (screen === "name" && !askedName) {
      askedName = true;
      phone.ask("all", { id: "home-name", prompt: "Type a name for the couch", placeholder: "Name", max: 24 }).catch(function () {});
    } else if (screen !== "name" && askedName) {
      askedName = false;
      phone.ask("all", null).catch(function () {});
    }
    phone.events().forEach(function (event) {
      if (event.type === "text" && event.ask === "home-name" && screen === "name") {
        var typed = String(event.text || "").trim().slice(0, 24);
        if (typed) {
          post("/__gigacouch/v1/home/add", { name: typed }).then(function () {
            screen = "shelf";
            focus = 0;
          });
        }
      }
    });
    if (performance.now() - lastRemotePoll > 700) {
      lastRemotePoll = performance.now();
      fetch("/__gigacouch/v1/home/remote", { cache: "no-store" })
        .then(function (response) { return response.json(); })
        .then(function (body) { if (body.open) startFromPhone(body.open); })
        .catch(function () {});
    }
  }

  // Starts a game a phone picked, the same way choosing it on the shelf does.
  function startFromPhone(id) {
    var rows = shelfRows();
    for (var row = 0; row < rows.length; row += 1) {
      var index = rows[row].findIndex(function (game) { return game.id === id; });
      if (index !== -1) {
        screen = "shelf";
        shelfRow = row;
        shelfIndex = index;
        drawn = "";
        activate();
        return;
      }
    }
  }

  function frame() {
    phoneHelp();
    var menu = window.GigaCouch && window.GigaCouch.menu;
    if (screen === "account" && performance.now() - lastAccountPoll > 1000) {
      lastAccountPoll = performance.now();
      load().then(function () { drawn = ""; });
    }
    if (menu) {
      var move = menu.move();
      if (hold > 0) {
        hold -= 1;
      } else if (Math.abs(move.x) > 0.45 || Math.abs(move.y) > 0.45) {
        nudge(move.x > 0.45 ? 1 : move.x < -0.45 ? -1 : 0, move.y > 0.45 ? 1 : move.y < -0.45 ? -1 : 0);
        hold = 11;
      }
      if (menu.confirm()) {
        activate();
      }
      if (menu.back()) {
        retreat();
      }
    }
    if (performance.now() - lastPhonePoll > 700) {
      lastPhonePoll = performance.now();
      pollPhones();
    }
    draw();
    drawPhones();
    window.requestAnimationFrame(frame);
  }

  load().then(function () {
    draw();
    window.requestAnimationFrame(frame);
  });
})();
