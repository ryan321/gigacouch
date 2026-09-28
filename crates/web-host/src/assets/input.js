(function () {
  var nativePads = navigator.getGamepads ? navigator.getGamepads.bind(navigator) : null;
  var keys = new Set();
  var watched = {
    Enter: true,
    NumpadEnter: true,
    Space: true,
    Backspace: true,
    KeyW: true,
    KeyA: true,
    KeyS: true,
    KeyD: true,
    ArrowUp: true,
    ArrowDown: true,
    ArrowLeft: true,
    ArrowRight: true,
  };

  function onKey(event, down) {
    if (!watched[event.code]) {
      return;
    }
    if (down) {
      keys.add(event.code);
    } else {
      keys.delete(event.code);
    }
    event.preventDefault();
    event.stopPropagation();
  }

  window.addEventListener("keydown", function (event) { onKey(event, true); }, true);
  window.addEventListener("keyup", function (event) { onKey(event, false); }, true);
  window.addEventListener("blur", function () { keys.clear(); });

  function family(id) {
    var name = (id || "").toLowerCase();
    if (name.indexOf("xbox") !== -1 || name.indexOf("xinput") !== -1) {
      return "xbox";
    }
    if (
      name.indexOf("dual") !== -1 ||
      name.indexOf("playstation") !== -1 ||
      name.indexOf("sony") !== -1 ||
      name.indexOf("wireless controller") !== -1
    ) {
      return "playstation";
    }
    return "generic";
  }

  function pressed(buttons, index) {
    return !!(buttons[index] && buttons[index].pressed);
  }

  function devices() {
    var list = [
      {
        id: "keyboard",
        kind: "keyboard",
        name: "Keyboard",
        family: "keyboard",
        south: keys.has("Enter") || keys.has("NumpadEnter"),
        jump: keys.has("Space"),
        leave: keys.has("Backspace"),
        analog: false,
        move: {
          x: (keys.has("KeyD") || keys.has("ArrowRight") ? 1 : 0) - (keys.has("KeyA") || keys.has("ArrowLeft") ? 1 : 0),
          y: (keys.has("KeyS") || keys.has("ArrowDown") ? 1 : 0) - (keys.has("KeyW") || keys.has("ArrowUp") ? 1 : 0),
        },
      },
    ];
    if (!nativePads) {
      return list;
    }
    var pads = nativePads() || [];
    for (var i = 0; i < pads.length; i += 1) {
      var pad = pads[i];
      if (!pad) {
        continue;
      }
      var buttons = pad.buttons || [];
      var axes = pad.axes || [];
      var digitalX = (pressed(buttons, 15) ? 1 : 0) - (pressed(buttons, 14) ? 1 : 0);
      var digitalY = (pressed(buttons, 13) ? 1 : 0) - (pressed(buttons, 12) ? 1 : 0);
      var analog = digitalX === 0 && digitalY === 0;
      list.push({
        id: "pad:" + (pad.id || String(pad.index)),
        kind: "pad",
        name: pad.id || "Controller " + (pad.index + 1),
        family: family(pad.id || ""),
        south: pressed(buttons, 0),
        east: pressed(buttons, 1),
        west: pressed(buttons, 2),
        north: pressed(buttons, 3),
        start: pressed(buttons, 9),
        analog: analog,
        look: { x: axes[2] || 0, y: axes[3] || 0 },
        move: analog
          ? { x: axes[0] || 0, y: axes[1] || 0 }
          : { x: digitalX, y: digitalY },
      });
    }
    return list;
  }

  function post() {
    fetch("/__gigacouch/v1/devices", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ devices: devices() }),
    }).catch(function () {});
    window.requestAnimationFrame(post);
  }

  if (nativePads) {
    navigator.getGamepads = function () {
      return [];
    };
  }
  window.requestAnimationFrame(post);
})();
