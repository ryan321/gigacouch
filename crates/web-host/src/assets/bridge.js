(function () {
  var latches = new Map();
  var players = [];
  var axes = new Map();
  var looks = new Map();
  var held = new Map();
  var BUTTONS = ["south", "east", "west", "north", "start"];
  // Action names games already use, and the button each one reads.
  var ALIASES = { jump: "south", primary_action: "south", secondary_action: "east" };
  var glyphs = new Map();
  var menu = { move: { x: 0, y: 0 }, confirm: false, back: false };

  function remember(data) {
    var seen = new Set();
    players = data.players || [];
    players.forEach(function (player) {
      seen.add(player.id);
      var latch = latches.get(player.id) || {};
      if (player.edges && player.edges.jump) {
        latch.south = true;
      }
      BUTTONS.forEach(function (button) {
        if (player.pressed && player.pressed[button]) {
          latch[button] = true;
        }
      });
      latches.set(player.id, latch);
      axes.set(player.id, player.move || { x: 0, y: 0 });
      looks.set(player.id, player.look || { x: 0, y: 0 });
      held.set(player.id, player.buttons || {});
      glyphs.set(player.id, player.glyphs || {});
    });
    if (data.menu) {
      menu.move = data.menu.move || { x: 0, y: 0 };
      if (data.menu.confirm) {
        menu.confirm = true;
      }
      if (data.menu.back) {
        menu.back = true;
      }
    }
    Array.from(latches.keys()).forEach(function (id) {
      if (!seen.has(id)) {
        latches.delete(id);
        axes.delete(id);
        looks.delete(id);
        held.delete(id);
        glyphs.delete(id);
      }
    });
  }

  function poll() {
    fetch("/__gigacouch/v1/snapshot", { cache: "no-store" })
      .then(function (response) {
        if (!response.ok) {
          return null;
        }
        return response.json();
      })
      .then(function (data) {
        if (data) {
          remember(data);
        }
      })
      .catch(function () {});
  }

  setInterval(poll, 16);
  poll();

  // True once per press, like a button edge.
  function consume(playerId, button) {
    var latch = latches.get(playerId);
    if (!latch || !latch[button]) {
      return false;
    }
    latch[button] = false;
    return true;
  }

  function buttonFor(name) {
    var button = ALIASES[name] || name;
    return BUTTONS.indexOf(button) === -1 ? null : button;
  }

  window.GigaCouch = {
    api: "1",
    players: {
      list: function () {
        return players.map(function (player) {
          return { id: player.id, name: player.name };
        });
      },
    },
    input: {
      // "jump", "primary_action", "secondary_action", or a button:
      // "south", "east", "west", "north", "start". True once per press.
      action: function (playerId, name) {
        var button = buttonFor(name);
        return button ? consume(playerId, button) : false;
      },
      // True while the button is down.
      held: function (playerId, name) {
        var button = buttonFor(name);
        return !!(button && (held.get(playerId) || {})[button]);
      },
      // "move" (left stick, d-pad, WASD) or "look" (right stick).
      axis: function (playerId, name) {
        var table = name === "move" ? axes : name === "look" ? looks : null;
        var value = (table && table.get(playerId)) || { x: 0, y: 0 };
        return { x: value.x, y: value.y };
      },
      glyph: function (playerId, name) {
        var table = glyphs.get(playerId) || {};
        return table[name] || "";
      },
    },
    save: {
      read: function (slot) {
        return fetch("/__gigacouch/v1/save/read", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ slot: slot }),
        }).then(function (response) {
          return response.json().then(function (body) {
            if (!response.ok) {
              throw new Error((body && body.error) || "save read failed");
            }
            return body.data;
          });
        });
      },
      write: function (slot, data) {
        return fetch("/__gigacouch/v1/save/write", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ slot: slot, data: data }),
        }).then(function (response) {
          return response.json().then(function (body) {
            if (!response.ok) {
              throw new Error((body && body.error) || "save write failed");
            }
          });
        });
      },
    },
    menu: {
      move: function () {
        return { x: menu.move.x || 0, y: menu.move.y || 0 };
      },
      confirm: function () {
        var value = menu.confirm;
        menu.confirm = false;
        return value;
      },
      back: function () {
        var value = menu.back;
        menu.back = false;
        return value;
      },
    },
    // Phone controllers. The open game's gigacouch.json picks the starting
    // layout; a game can switch every phone while it runs.
    phone: {
      // Resolves to { layout, layouts, game }.
      info: function () {
        return fetch("/__gigacouch/v1/phone/layout", { cache: "no-store" }).then(function (response) {
          return response.json();
        });
      },
      // Plays a sound on the phone of one player (a player number) or on
      // every phone ("all"): a stock sound, or one the game lists under
      // phone.sounds in gigacouch.json.
      sound: function (player, name) {
        return fetch("/__gigacouch/v1/phone/sound", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ player: player, name: name }),
        }).then(function (response) {
          return response.json().then(function (body) {
            if (!response.ok) {
              throw new Error((body && body.error) || "could not play that sound");
            }
            return body.phones;
          });
        });
      },
      // Vibrates the phone of one player (a player number) or every phone
      // ("all"). pattern: a preset ("tap", "bump", "hit", "long", "double",
      // "heartbeat"), a length in ms, or [on, off, on, ...] in ms. Android
      // phones vibrate; iPhones flash the pad's edges instead.
      rumble: function (player, pattern) {
        return fetch("/__gigacouch/v1/phone/rumble", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ player: player, pattern: pattern }),
        }).then(function (response) {
          return response.json().then(function (body) {
            if (!response.ok) {
              throw new Error((body && body.error) || "could not rumble");
            }
            return body.phones;
          });
        });
      },
      setLayout: function (layout) {
        return fetch("/__gigacouch/v1/phone/layout", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ layout: layout }),
        }).then(function (response) {
          return response.json().then(function (body) {
            if (!response.ok) {
              throw new Error((body && body.error) || "could not change the phone layout");
            }
          });
        });
      },
    },
    lifecycle: {
      quit: function () {
        fetch("/__gigacouch/v1/quit", { method: "POST" }).catch(function () {});
      },
    },
  };
})();
