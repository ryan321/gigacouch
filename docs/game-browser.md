# Giga Couch Game Browser

The Giga Couch Game Browser is the living-room app for gigacouch.com. It is a Chromium shell aimed at a TV and a pile of controllers. A web game still runs in Chrome. The Game Browser is the better place to play it on a couch, and the place a Godot game is launched from.

## Motivation

Creators can ship a normal web game: HTML, JavaScript, WebGL or WebGPU, the standard gamepad calls, and ordinary browser storage. That game should be playable by opening gigacouch.com in Chrome, Safari, or Firefox. Those browsers are built for a desk. On a shared TV they leave the tabs and address bar on screen, treat every person as one browser profile, stop seeing pads past the operating system's limits, and store progress as website data that a cache clear or a private window can drop.

Giga Couch's job is the part around the game. Someone should be able to browse the shelf from the couch, play with the controllers in the room, trust the graphics setup, keep a separate save for each person, and keep playing after the network is unplugged. The Game Browser is that player. It uses a Chromium engine so a web game does not have to be rewritten, and it changes the screen, the pads, the graphics setup, and the save files around that engine.

Compiling a private Chromium is not required for this. The value is the shell and the host around a pinned Chromium: a full-screen shelf, a known GPU path, cleaned-up controllers, and household save files. A from-source build is worth considering only if the shipped shell is too large, or if a sandbox or graphics switch cannot be set any other way. The partial CEF checkout on the external drive is not this browser.

## How someone uses it

gigacouch.com is the catalog in any browser. A web game offers two actions: **Play** runs it on the site, and **Get Giga Couch** downloads the Game Browser. Inside the app, the same site is the shelf.

* **Play** uses the pinned graphics stack and the cleaned-up controllers.
* **Download** keeps a copy on the computer. That copy launches with the network unplugged, through the same host, into the same save file.
* A **Godot** game is listed on the same shelf. Choosing it starts the game in its own window. The shelf stays open, and closing the game returns to it. A native Godot game is not played inside the page. The site can show it and offer the app.

The account is the same one on the website and in the app, so the shelf matches. The app opens gigacouch.com and installed games. It stays a full-screen shelf: no address bar, no tabs, and no general web.

A web game remains one package with two ways to run it. On the public site it uses the pads and storage of whatever browser opened the page. In the Game Browser, and from the downloaded copy, the same files get the couch controllers, the family saves, and offline launch. Join, leave, button glyphs, and quit-to-shelf can be extra calls for games that want them. The game still plays through the standard gamepad and storage calls when those extras are absent.

## What is better than Chrome

### On the TV

The shelf fills the screen, one game at a time, and quit returns to the shelf. The stick or pad moves, and the south button chooses. Type, cards, and buttons are sized for a couch. The app stays on the HDMI display, and a game does not lose the screen because another window took focus.

### Playing

Web games run in one pinned Chromium. They use hardware WebGL and WebGPU, and the strong GPU when the PC has two. A machine that cannot do that does not start the game. Sound and the frame loop keep going for the session, and a permission popup does not interrupt play. A Godot game opens beside the shelf and comes back to it when it closes.

### Controllers

The host presents up to 16 pads through the normal gamepad calls, with one button layout. More than four Xbox pads still appear. Duplicate Joy-Cons are ignored. Wired Xbox pads that the operating system hides are included. The south button joins and confirms. Holding the east button leaves. On-screen buttons can show the glyph for the pad in that person's hands.

### Saves and the shelf

The shelf asks who is playing. Family, Guest, and added names each keep their own progress. Each game has its own folder. A write finishes fully or the previous save stays. The files remain after an update, a logout, or the network dropping. Playing from the shelf and playing a downloaded copy use the same file.

## Saves

Chrome keeps a save in `localStorage` or IndexedDB for that site, inside that Chrome profile. On a shared TV that is one pile for everybody who uses the computer. Games hosted on the same site share that pile. Clearing browsing data, using a private window, or a crash during a write can drop it. The website and a file opened from disk are different sites, so they do not see each other's saves.

The Game Browser asks who is playing and stores a JSON file on the computer:

```text
Application Support/GigaCouch/saves/<person>/<game>/campaign.json
```

* Family, Guest, and any added name are separate. A visitor can play without writing over someone else's file.
* Each game has its own folder, so one game cannot read or erase another.
* The host writes a temporary file, flushes it to disk, then renames it over the previous save. Power loss in the middle keeps the last good file.
* The save is not browsing data. It survives a cache clear, an app update, logout, and an unplugged network.
* A slot name is a short plain name, the body is JSON, and the limit is 256 KiB, which fits progress, unlocks, and settings. Too big, a bad name, or a missing file comes back as an error the game can show.
* A Godot game is given that same person's directory, so web games and Godot games keep saves the same way.

A game played in ordinary Chrome can still save with browser storage. That save belongs to that browser on that machine. The Game Browser copy belongs to the person on the couch. Copying a website save into the app would be an account feature. Play does not depend on it.

## Ideas beyond this

These are not built or promised. They are candidates for making the Game Browser clearly better than a Chrome tab, roughly ordered by how much each one sets it apart.

### Biggest differentiators

* **Phones as controllers.** The shelf shows a QR code. A phone on the same Wi-Fi opens a pad page served by the host and appears as another pad. Nobody sits out because the house has only two controllers.
* **Remote play with a friend.** The host streams the TV picture over WebRTC to a friend's browser. The friend's input comes back as another local pad, so the game does not know that player is remote.
* **A guide-button overlay.** The home button pauses the game by holding its frame loop and muting it. The overlay offers resume, quit to shelf, volume, player swap, and screenshot. A game cannot block it.
* **One lobby for every game.** Before launch, the host runs a shared join screen where each person picks a name and a color. The game receives that roster instead of building its own join flow, and each pad is already tied to a person's save.

### Feel and polish

* **Fuller controller features.** Normalize rumble, set pad lights to the player's color, and expose gyro and adaptive triggers where the hardware has them.
* **TV integration.** Wake the TV and switch its input over HDMI-CEC. Keep the display awake during play, and match the refresh rate or turn on variable refresh where the system allows it.
* **No stutter on first launch.** Warm the shader and GPU caches when a game downloads, and keep a cache for each game.
* **Replay capture.** Keep a rolling buffer of the last 30 seconds so someone can save a clip of what just happened.
* **Crash recovery.** A watchdog notices a hung or crashed game process and returns to the shelf with a clear message. The last good save stays.

### Household

* **Settings that follow the person.** Button remapping, hold-to-toggle, and text size belong to each person. A copilot mode lets two pads drive one player, for kids or anyone who wants help.
* **Parental controls.** Content ratings, playtime limits, and a profile limited to approved games.
* **Save sync between household computers.** Sync saves through the platform backend, with clear handling when two copies disagree.
* **Couch history.** Who played, who won, and time played for each person. The shelf can use it to order games.

### Trust and creator hooks

* **A permission list for each game.** A downloaded game gets no network by default and keeps its own storage. The game declares what it needs, instead of relying on browser popups.
* **A device hint.** The host tells the game the GPU tier, the TV-safe area, and the pad count, so it can pick quality settings without guessing.
* **Signed updates in the background.** Download only the changed files, and check the package before launch so what runs matches what was approved.

The strongest first set is phones as controllers, the guide-button overlay, and the shared lobby. Together they make a party game playable by everyone on the couch with no extra hardware. Remote play is the most striking idea and also the most work.

## Where this stands

The Mac shell already runs full screen and can play a web game while the host reads the pads. Blob Island is the sample. Home can list a Godot game and open it in its own window. Saves for a web game are already atomic JSON files, capped at 256 KiB, owned by the host.

`python3 scripts/build_browser.py` builds a Mac `Giga Couch.app` and disk image with the couch-tile icon. The app starts its own host, so it no longer needs the command line. It is signed only for this Mac. Cmd+I shows a stats overlay with frame rate, GPU, CPU and memory load, the graphics card, the display's refresh rate, and connected and known controllers.

Still ahead of a promise creators can rely on: Developer ID signing and notarization, Windows and Linux builds, Play on gigacouch.com itself, the app as a window onto that site, the launch check that refuses a machine with no hardware GPU, offline downloads of the catalog, and a tested session with controllers and WebGPU on both systems. The current sample moves only when the Giga Couch page API is present, and inside the shell the normal gamepad list is replaced. The shelf described above feeds that list with the cleaned-up pads instead, so the same game code runs on the website and in the app.
