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

The pnpm scripts in the root `package.json` have no dependencies. `pnpm build` runs `python3 scripts/build_browser.py`.

The build downloads nothing. It copies that Electron to `/Volumes/External/projects/gigacouch-browser/stage/Giga Couch.app`, renames it, and gives it the Giga Couch tile icon. It bundles the `couch` host, this `shell/`, Home, and Blob Island. It then signs the app ad hoc and writes `GigaCouch-<version>-mac-arm64.dmg` beside it. The Godot samples under `sdk/` are not bundled. Creator projects registered on the Mac still appear.

On launch the app runs its bundled `couch web-home --serve-only` and reads the origin from its JSON line. It points Home at `https://gigacouch-platform.fly.dev` unless `GIGACOUCH_PLATFORM` is set. Quitting the app closes the host's stdin, which stops the host. Chromium's own cache goes under `Application Support/GigaCouch/browser`.

### Phones as controllers

Home shows a "Play on your phone" card with a QR code. A phone on the same Wi-Fi scans it and opens a touch pad. A joins, the phone shows its player number, and the card lists every phone. Every layout has a Menu button in the header. Its sheet offers Back to the shelf while a game is open, which ends the game for everyone and returns the TV to Home, the same as Escape. It also offers Leave the game and Change name. Holding B does not leave, because B is a game button on some layouts. With the stats overlay open, the card moves left of it.

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

An unknown layout fails the package check. The layouts live in `crates/web-host/src/assets/pad.js`, and a test keeps that table and the host's list the same.

The host listens for phones on port 8790, or a free port when 8790 is busy, on every interface. That listener serves only the pad page, its script, and one WebSocket per phone, under a code that changes each time the app starts. Saves, games, Home, and account routes stay on the loopback origin, and a test checks that the phone listener refuses them. The host adds live phones to the device list the page posts, so web games see a phone as another player with no changes. A phone that goes quiet for 1.5 seconds reads as released, and after 8 seconds its slot is freed.

iPhone Safari cannot hide its toolbars for a web page, which leaves little room in landscape. The pad page is an installable web app: Share, then Add to Home Screen, opens it full screen with the Giga Couch icon. The pad shows that tip once on iPhone. Android Chrome goes full screen on the first tap. The join code is kept in `phone-code` next to the Home profiles, so a home-screen pad still works after the app restarts. It stops working if the computer's Wi-Fi address changes.

With the macOS firewall on, the first launch asks whether `couch` may accept incoming connections. Phones need that allowed. The page is plain http, so a phone cannot use tilt, keep its screen awake, or vibrate on iPhone. Networks that isolate devices from each other, such as many guest networks, block phones. Godot games do not receive phone input yet, and a game cannot yet define its own layout or switch layouts mid-game.

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

Without `--windowed` the shell opens in normal full screen, so app switching and Force Quit still work. `GIGACOUCH_KIOSK=1` asks for Electron's locked kiosk instead. On macOS kiosk mode blocks app switching and Force Quit, so use it only on a dedicated couch machine. `couch web-serve` prints the loopback origin and does not open a window. The first launch can print `sandbox_extension_issue_file failed` for a helper Resources directory; the window still opens.

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
