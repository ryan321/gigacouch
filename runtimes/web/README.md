# web-1 browser

Giga Couch's game browser is a kiosk, not a normal browser window. The Rust host in `crates/web-host` serves one installed package on `127.0.0.1` and owns controllers, saves, and quit. The shell in `shell/` is an Electron prototype of that kiosk. Architecture: [docs/GIGACOUCH_ARCHITECTURE_v2.md](../../docs/GIGACOUCH_ARCHITECTURE_v2.md).

This is not a certified sandbox. Content-Security-Policy, loopback-only serving, and a navigation lock are the controls that exist today. They have tests for the origin and the slot rules. They do not prove OS isolation, WebGPU behavior, or a physical 16-controller session.

## Platform server

Accounts and the master library live in a separate process. Home talks to it over HTTP and does not open its database. The database file is this server's stand-in for Neon until that host is connected.

```bash
./target/debug/couch platform
```

The sign-in page is `http://127.0.0.1:8787/link`. Home shows a code. Enter that code, a name, and a passphrase on the page. A new name creates the account. Blob Island can then be downloaded onto this Mac. The Godot games are listed in that library and still open from the local projects.

The public site is on that same server. https://gigacouch-platform.fly.dev is the landing page, and https://gigacouch-platform.fly.dev/account creates an account or signs in. Cloudflare holds the private game files, not this website.

The same server is deployed on Fly, in the same style as the education app: one small machine in `iad` that sleeps when idle. Accounts and the library database live on the `gigacouch_data` volume. Point Home at it with:

```bash
./target/debug/couch web-home --windowed --platform https://gigacouch-platform.fly.dev
```

The first request after a sleep can take a few seconds while the machine starts.

## Home

The couch front door is separate from a game. It asks who's playing, then shows the shelf. Family and Guest start on the Mac. Added names are stored in the Home profile file. Account sign-in through a phone is not connected yet. Blob Island plays in this browser. A Godot game opens in its own window and Home stays up; close that window to come back. Home uses an already installed Godot and does not download one.

```bash
./target/debug/couch web-home --windowed
```

Escape during a game returns to Home. Escape on the shelf leaves full screen. Cmd+Q quits on the Mac, and Ctrl+Q quits elsewhere.

## Mac app

```bash
python3 runtimes/web/fetch_shell.py   # once; writes Electron to /Volumes/External
pnpm build                            # app and disk image
pnpm build:app                        # app only, no disk image
pnpm launch                           # open the built app full screen
pnpm launch:windowed                  # run it in a window, logs in this terminal
```

Quit the app before building: the build replaces the whole app bundle, and a copy still running from it opens new pages blank. The build refuses to run while that copy is open.

The pnpm scripts in the root `package.json` have no dependencies. `pnpm build` runs `python3 scripts/build_browser.py`.

The build downloads nothing. It copies that Electron to `/Volumes/External/projects/gigacouch-browser/stage/Giga Couch.app`, renames it, and gives it the Giga Couch tile icon. It bundles the `couch` host, this `shell/`, Home, and Blob Island. It then signs the app ad hoc and writes `GigaCouch-<version>-mac-arm64.dmg` beside it. The Godot samples under `sdk/` are not bundled. Creator projects registered on the Mac still appear.

On launch the app runs its bundled `couch web-home --serve-only` and reads the origin from its JSON line. It points Home at `https://gigacouch-platform.fly.dev` unless `GIGACOUCH_PLATFORM` is set. Quitting the app closes the host's stdin, which stops the host. Chromium's own cache goes under `Application Support/GigaCouch/browser`.

### Phones as controllers

Home shows a "Play on your phone" card with a QR code. A phone on the same Wi-Fi scans it and opens a touch pad. A joins, the phone shows its player number, and the card lists every phone. Every layout has a Menu button in the header. Its sheet offers Back to the shelf while a game is open, which ends the game for everyone and returns the TV to Home, the same as Escape. It also offers Leave the game, Change name, and Reload controller, since the home-screen app has no browser reload button. Holding B does not leave, because B is a game button on some layouts. With the stats overlay open, the card moves left of it.

There is no limit on phones or players. A game can set one with `players.max` in `gigacouch.json`. When that game opens from Home, players past the limit lose their slot and can join again once there is room. Without `max`, everyone who joins plays. The Home card and the stats overlay list the first six phones and then a count. Each phone sends stick movement at most 30 times a second and button presses at once, which keeps a crowded Wi-Fi network responsive. The host raises its open-file limit at start, up to 10240 on macOS, because each phone holds one connection. A typical home router is likely the practical ceiling, somewhere around 30 to 60 busy phones. That number is an estimate, not a measurement.

The phone shows the open game's layout, and the default on the shelf. A game picks one in `gigacouch.json`:

```json
"phone": { "layout": "quiz-4" }
```

| Layout | Controls | Portrait |
| --- | --- | --- |
| `stick-2` (default) | Floating stick, A, B | Yes |
| `dpad-2` | Eight-way d-pad, A, B, Start | Asks to turn sideways |
| `stick-4` | Floating stick, A, B, X, Y, Start | Asks to turn sideways |
| `twin-stick` | Move stick, aim stick, A, Start | Asks to turn sideways |
| `one-button` | One large A | Yes |
| `quiz-4` | Four large A, B, X, Y buttons | Yes |
| `racing` | Steering arrows, Gas (A), Brake (B), Start | Asks to turn sideways |
| `paddle` | A slider that stays where it is left, A | Yes |
| `touchpad` | Finger position on the look axis, A, B | Yes |
| `lanes-4` | Four tall X, A, B, Y lanes | Yes |
| `two-choice` | Two large A and B buttons | Yes |

The paddle slider reports its position on `move`, and the touchpad reports the finger's position on `look`. Both run from -1 to 1 with no dead zone, and both stay where the finger lifts.

A game can change the layout while it runs, for example between a menu and a race:

```javascript
await GigaCouch.phone.setLayout("racing")
const { layout, layouts } = await GigaCouch.phone.info()
```

`setLayout` works only while a game is open; the shelf always shows the default.

#### Phone sounds

A game can play a sound on one player's phone or on every phone:

```javascript
GigaCouch.phone.sound(playerId, "ding")  // that player's phone
GigaCouch.phone.sound("all", "correct")   // every phone
```

Eight stock sounds come with every phone, generated by the pad with no files: `click`, `tick`, `ding`, `success`, `fail`, `buzzer`, `coin`, and `whoosh`. A game adds its own under `phone.sounds` in `gigacouch.json`:

```json
"phone": {
  "layout": "quiz-4",
  "sounds": { "correct": "sounds/correct.mp3", "your-turn": "sounds/turn.m4a" }
}
```

Each file must be inside `web/`, be MP3, M4A, or WAV, and stay under 256 KiB, with at most 32 sounds and 2 MiB together. A game's sound cannot reuse a stock name. When the game opens, the host tells every phone its sounds, and each phone downloads and decodes them so they play at once. The phone listener serves only the open game's declared sounds, named by a fingerprint of their bytes so phones can cache them; nothing else in the game's folder is reachable from the network.

Sound starts after the player's first tap, because browsers block audio until then; joining with A takes care of it. The pad asks iPhone to play through the silent switch, which has not been tried on a real iPhone yet.

#### More from phones

Games opt into these; a game that ignores them still gets plain pads.

```javascript
// A private screen on one phone, every phone, or the audience. A tapped
// choice comes back through events().
GigaCouch.phone.show(2, { id: "hand", title: "Your hand", text: "Only you can see these.",
  choices: [{ id: "sun", label: "Sun", image: "sun" }] })
GigaCouch.phone.show(2, null)                         // take it away

// A question typed on the phone's own keyboard.
GigaCouch.phone.ask("all", { id: "caption", prompt: "Caption this", max: 80 })

// New words on a player's buttons.
GigaCouch.phone.setLabels(2, { south: "Paris", east: "Rome", west: "Oslo", north: "Lima" })

// Everything phones sent since the last call: choices, typed answers, and
// drawing strokes, each with player, audience, name, and a phone token.
for (const event of GigaCouch.phone.events()) { … }

// Players: kind ("pad", "phone", "keyboard"), avatar, profile, away.
GigaCouch.players.list()
```

- **Private screens.** Text, one of the game's images, and up to 12 choices. A choice can also start one of the phone's own actions with `action`: `"photo"` opens the camera, `"profile"` lists Home's people, and `"audience"` switches the phone to the audience. The pad does these from the player's tap; a game cannot start them any other way. Images come from `phone.images` in `gigacouch.json`: PNG, JPG, or WebP, at most 512 KiB each and 4 MiB together. The host checks every field and drops the rest; the pad draws from that data, and no game code runs on a phone.
- **Typing.** Home uses it too: on "Spell the name", any phone can type the name.
- **Drawing.** The built-in `draw` layout has a canvas and a color picker. Strokes arrive as events with points from 0 to 1 across the canvas.
- **The game's own layouts.** `phone.layouts` in `gigacouch.json` names layouts built from stick, dpad, button, arrow, slider, touchpad, canvas, and palette controls, placed in fractions of the screen. `phone.layout` and `setLayout` accept them. The host keeps only the fields the pad draws.
- **Audience.** A phone can choose Watch as the audience from its menu. It takes no player spot, gets screens sent to `"audience"` or `"all"`, and its choices arrive with `audience: true`.
- **Rejoin.** A phone that drops keeps its player number for two minutes, marked `away`, and comes back into it without pressing A.
- **Profiles and photos.** From the phone menu, Who's playing links the phone to a Home person, whose name it then plays under; the links are kept in `phone-profiles.json`. Take a photo sends a 256-pixel JPEG, kept in `phone-avatars/`; games get it as `avatar`.
- **Starting games.** Start a game in the phone menu lists the games on this computer; Home starts the one picked.
- **Signal and comfort.** The pad pings every two seconds and shows the round trip as bars; the host lists it as `rtt`. Left-handed controls mirror the layout. Keep the screen on plays a tiny silent clip on a loop, since the Wake Lock API is off on plain http; it has not been tried on real phones yet.

`examples/phone-lab` shows all of it: the Phone Lab on the shelf has a station for each, and the phone's Next button moves everyone on.

#### Phone rumble

```javascript
GigaCouch.phone.rumble(playerId, "hit")        // a preset
GigaCouch.phone.rumble("all", [60, 40, 60])    // on, off, on, in milliseconds
```

The presets are `tap`, `bump`, `hit`, `long`, `double`, and `heartbeat`. A pattern has 1 to 20 steps of 1 to 2000 ms, and 5 seconds at most in all. Android phones vibrate. iPhones have no vibration for web pages, so the pad flashes its edges for the same length instead. Each phone reports whether it can vibrate, and `/__gigacouch/v1/phones` lists it as `rumble` for each phone. Pads rumble through Chrome's gamepad vibration, where the pad supports it.

The host pushes layout changes, sounds, and rumble to phones as they happen. Each phone's connection checks for queued messages every 10 ms, so a sound reaches the Wi-Fi within a few milliseconds instead of waiting for the phone's next message. The phone listener is its own small server for this reason: it keeps each phone's socket so the host can write to it at any time.

Each reply to a phone carries a fingerprint of the pad the host serves. A phone still running an older pad, because the app was updated and restarted while the page stayed open, reloads itself once to pick up the new one.

The Controller Gallery on the shelf, in `examples/controller-gallery`, shows every layout and plays sounds on the pressing player's phone: stock sounds on most layouts, and its own sounds (a chime, a boing, a horn, engine and tire screech for racing, and a drum kit on the four lanes). The TV lists which button plays which sound and which ones also buzz, and each player's card says whether their phone vibrates or flashes. Each player gets a live card with their sticks and buttons. Holding A for two seconds moves everyone to the next layout, and `]` and `[` step forward and back on the keyboard. It sets no player limit.

An unknown layout fails the package check. The layouts live in `crates/web-host/src/assets/pad.js`, and a test keeps that table and the host's list the same.

The host listens for phones on port 8790, or a free port when 8790 is busy, on every interface. That listener serves only the pad page, its script, and one WebSocket per phone, under a code that changes each time the app starts. Saves, games, Home, and account routes stay on the loopback origin, and a test checks that the phone listener refuses them. The host adds live phones to the device list the page posts, so web games see a phone as another player with no changes. A phone that goes quiet for 1.5 seconds reads as released, and after 8 seconds its slot is freed.

iPhone Safari cannot hide its toolbars for a web page, which leaves little room in landscape. The pad page is an installable web app: Share, then Add to Home Screen, opens it full screen with the Giga Couch icon. The pad shows that tip once on iPhone. Android Chrome goes full screen on the first tap. The join code is kept in `phone-code` next to the Home profiles, so a home-screen pad still works after the app restarts. It stops working if the computer's Wi-Fi address changes.

With the macOS firewall on, the first launch asks whether `couch` may accept incoming connections. Phones need that allowed. The page is plain http, so a phone cannot use tilt, keep its screen awake, or vibrate on iPhone. Networks that isolate devices from each other, such as many guest networks, block phones. Godot games do not receive phone input yet, and a game cannot yet define its own layout.

### Stats overlay

View > Stats Overlay, or Cmd+I (Ctrl+I elsewhere), shows a panel over the shelf or the game. The app remembers the choice. The panel shows:

- Frame rate, time per frame, and the 1% low, with a frame-time line against the display's frame budget. The preload times frames from Electron's isolated world, so the game page cannot see or reach it.
- GPU load and GPU memory in use, CPU load, and memory in use. On a Mac these come from the IORegistry, `vm_stat`, and the memory pressure level. Elsewhere GPU load comes from `nvidia-smi` when an NVIDIA driver provides it; that path has not been run on Windows or Linux yet.
- The GPU and graphics backend, whether WebGL and WebGPU run on the GPU, the processor, display resolution and refresh rate, heat, and power source.
- Controllers. Connected pads come from Chromium's gamepad list, which only includes a pad after one of its buttons is pressed. A Mac controller paired over Bluetooth but not yet pressed shows as "press a button", with its battery level. Known pads that are not connected come from Bluetooth pairings and from `controllers.json`, which remembers pads this app has seen.
- Warnings, each with a label: software rendering, a display under 55 Hz, heat, memory pressure, and running on battery.

The ad hoc signature only works on this Mac. Another Mac blocks the app until it is signed with a Developer ID and notarized.

## Run Blob Island

The shell binary is not in git. On this Mac it is downloaded only by:

```bash
python3 runtimes/web/fetch_shell.py
```

That writes Electron 44.4.4 to `/Volumes/External/projects/gigacouch-electron` and refuses the internal disk. `couch web-run` does not download it.

```bash
cargo build --locked -p couch-cli
./target/debug/couch web-run --package runtimes/web/examples/blob-island --windowed
```

The shell keeps a game's frame loop, timers, and input running while its window is covered or in the background, so phones and pads keep reaching it. Without `--windowed` the shell opens in normal full screen, so app switching and Force Quit still work. `GIGACOUCH_KIOSK=1` asks for Electron's locked kiosk instead. On macOS kiosk mode blocks app switching and Force Quit, so use it only on a dedicated couch machine. `couch web-serve` prints the loopback origin and does not open a window. The first launch can print `sandbox_extension_issue_file failed` for a helper Resources directory; the window still opens.

Enter or the south face joins. Space or the south face jumps. Backspace leaves immediately. Holding the east face for 1.25 seconds leaves. Disconnecting a pad frees its slot. The page reads `GigaCouch.input` and does not use the Gamepad API as the platform source; `input.js` stubs `navigator.getGamepads` after the host has read the devices.

## Package

```text
MyGame/
    gigacouch.json
    web/
        index.html
```

`gigacouch.json` is checked by `crates/web-host` and described by `schemas/web-package.schema.json`. The entry must be an `.html` file inside `web/`. Saves go to the guest directory under Application Support/GigaCouch unless `--save-dir` is set.

The bridge the host injects:

```javascript
GigaCouch.players.list()
GigaCouch.input.action(playerId, "jump")
GigaCouch.input.held(playerId, "west")
GigaCouch.input.axis(playerId, "move")
GigaCouch.input.axis(playerId, "look")
GigaCouch.input.glyph(playerId, "jump")
await GigaCouch.save.read("campaign")
await GigaCouch.save.write("campaign", data)
GigaCouch.lifecycle.quit()
```

`action` is true once per press, and `held` is true while a button is down. Buttons are named by position: `south`, `east`, `west`, `north`, and `start`. `jump` and `primary_action` are `south`, and `secondary_action` is `east`. `move` is the left stick, d-pad, or WASD, and `look` is the right stick or a phone's aim stick. Pads, phones, and the keyboard all report through these names.

## Source build

A partial CEF 7977 / Chromium checkout may still be at `/Volumes/External/projects/gigacouch-chromium`. It stopped during the git-cache download and is not the browser this directory runs. Do not resume `python3 runtimes/web/build_macos.py` unless that compile is explicitly requested. `args.macos.dev.gn` and `args.shipping.gn` remain the notes for that future pin.
