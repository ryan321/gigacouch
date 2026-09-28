# V1 Implementation Plan

The current architecture is [docs/GIGACOUCH_ARCHITECTURE_v2.md](docs/GIGACOUCH_ARCHITECTURE_v2.md). The browser slice in progress is the web-1 shell in [runtimes/web](runtimes/web/README.md): a Rust loopback origin, a 16-slot bridge, and an Electron kiosk. The partial CEF source tree on the external drive is not that browser. The Godot player and SDK remain the existing prototype; v2 still treats a managed Godot runtime as a later first-class target, and this plan does not add one.

The checklist below is the existing Godot prototype. That prototype stays in the repo. It is not the V1 shipping runtime.

## Web-1 browser prototype

- [x] `gigacouch.json` package for `runtime: web-1`, with the entry kept inside `web/`.
- [x] Loopback origin that injects the platform scripts, serves the package, and refuses paths outside `web/`.
- [x] Host-owned slots 1–16. South joins and jumps; east held for 1.25s leaves; keyboard Enter/Space joins, Space jumps, Backspace leaves. The page stub replaces `navigator.getGamepads`.
- [x] Save read/write on the host (256 KiB, atomic replace) and `GigaCouch.lifecycle.quit()`.
- [x] Blob Island sample and `couch web-serve` / `couch web-run`. The shell download is `python3 runtimes/web/fetch_shell.py` and is not invoked by tests or the CLI.
- [x] `couch platform` serves accounts, the master library, and package downloads. `couch web-home` signs in with a device code, then can download Blob Island. The server database is local to that process; it is not Neon yet. Godot titles are listed and still launch from the local projects.
- [x] Mac Game Browser app: `python3 scripts/build_browser.py` wraps the downloaded Electron as `Giga Couch.app` with the tile icon, the bundled host, and the Home files, and packs a disk image on `/Volumes/External`. The app starts `couch web-home --serve-only` itself. It is signed ad hoc, so only this Mac opens it; Developer ID signing and notarization are open.
- [x] Stats overlay in the shell (Cmd+I): frame rate and 1% low, GPU/CPU/memory load, GPU and display details, heat, power, and connected plus known controllers. Checked on this Mac with an Xbox pad paired but not pressed; a pad listed by Chromium, Windows, and Linux readings are untested.
- [x] Phones as controllers on the same Wi-Fi: a QR code on Home, a touch pad page with a floating stick and A/B, one WebSocket per phone, and phones merged into the host's player slots. Tests cover the join address, the listener refusing every non-pad route, and a WebSocket phone joining, hearing its player number, and freeing its slot. Checked end to end in the built app with a desktop Chromium window standing in for a phone; not yet tried on a real phone.
- [x] Built-in phone layouts (`stick-2`, `dpad-2`, `stick-4`, `twin-stick`, `one-button`, `quiz-4`) and a package default in `gigacouch.json`. The host carries south/east/west/north/start and a look stick for pads and phones, and `GigaCouch.input` exposes them. Phones switch to the open game's layout and back on the shelf. Tests cover the layout list, the package field, the new buttons, the phone Leave control, and the layout switch; every layout was rendered at phone size in both orientations.
- [x] No limit on phones or players. `players.max` is optional and enforced when a game opens from Home; the shelf has no limit. Tests cover 40 WebSocket phones joining as players 1–40, a game limit dropping and readmitting players, and the raised open-file limit. Not measured: how many phones one Wi-Fi network keeps responsive.
- [x] Five more layouts (`racing`, `paddle`, `touchpad`, `lanes-4`, `two-choice`), `GigaCouch.phone.setLayout` for switching while a game runs, and the Controller Gallery sample on the shelf. The slider and touchpad send positions with no dead zone. The shell no longer pauses a covered or background window. Checked with the gallery and three stand-in phone windows; not yet on real phones.
- [x] Phone sounds: instant host-to-phone messages on a rewritten phone listener, eight stock sounds generated on the phone, a game's own sounds under `phone.sounds` (checked, preloaded, cached by fingerprint), and `GigaCouch.phone.sound`. The gallery plays stock and its own sounds. Tests cover the package checks, sounds reaching and leaving phones with the game, and a sound pushed to one player's phone or all; a stand-in phone decoded all nine gallery sounds and played a custom and a stock sound from button presses. Not yet heard on a real iPhone or Android phone.
- [x] Phone rumble: `GigaCouch.phone.rumble` with presets or on/off patterns, checked by the host and pushed at once. Android phones vibrate; iPhones flash the pad's edges. Phones report whether they can vibrate. The gallery buzzes on some buttons. Tests cover the pattern rules and a rumble reaching one player's phone or all; a stand-in phone vibrated with the gallery's pattern, and flashed with vibration removed. Not yet felt on a real Android phone.
- [x] Phone features games opt into: private screens with images and choices, typed answers, button labels, a phone-to-game event channel, the `draw` layout, a game's own layouts and images in `gigacouch.json`, audience phones, rejoin within two minutes, links to Home people, photos, starting games from a phone, round-trip bars, left-handed mirroring, keep-awake, and pad rumble. Home takes typed names from phones. The Phone Lab sample has a station for each. Tests cover the checks, the event channel, audience targeting, rejoin, profiles, photos, and remote start; stand-in phones ran the lab and Home end to end. Not yet on real phones: keep-awake, photos from a camera, and pad rumble.
- [ ] Phones: tilt and motion, which need an HTTPS fallback through gigacouch.com (which also reaches isolated networks), and Godot games receiving phone input.
- [ ] Windows and Linux app builds in CI, and auto-update.
- [ ] Windows shell, graphics-API launch gate, OS sandbox proof, and the pinned CEF/Chromium build. A package test does not prove those.

Verified on this Mac: `cargo fmt --all -- --check`, `cargo test --workspace --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`. `couch web-run --windowed` opened Blob Island in the Electron shell; Enter joined player 1 and the on-screen hint came from the bridge. That does not verify a physical controller, WebGPU, Windows, or the sandbox. The built `Giga Couch.app` launched windowed on this Mac, started its bundled host, showed Home's sign-in screen, and left no host process after quitting. It was not opened from the disk image on another Mac.

## Public landing page

- [x] Replace the public landing page with an immersive WebGL couch/portal, a 28-second creation loop (wireframe scan → blockout → faceted geometry → smooth surfaces → detailed green upholstery → reset), pointer parallax, three selectable atmospheres, portal pulse, animated concept illustrations, scroll reveals, and account calls to action.
- [x] Add unrestricted couch rotation in every creation stage with mouse/touch dragging, keyboard controls, reset view, and standard-mapped browser controller sticks; keep the scan aligned in couch space and allow direct input while animations are paused.
- [x] Add a procedural aurora/nebula background, perspective grid, scan-timed energy waves, pointer response, and arrival/portal gravity waves with flight trails. Keep the background behind the copy, cap its resolution/update rate, and support static fallback and context recovery.
- [x] Simplify the hero to the couch, portal, and atmospheric effects. Remove the game-world cameos, floating controller models, texture routes/assets, and unused landing-art authoring scripts; retain couch rotation and controller input.
- [x] Replace Open a Portal with Hyperdrive: freeze the build clock, ease the couch into a randomized spin with subtle shake/blur, fly the background and page through a fluid hyperspace tunnel for three seconds, then ease back for half a second and resume the exact build stage. Preserve the previous rotation/pause state, prevent overlapping jumps, suspend hidden tabs, complete active flight when scrolling away, and offer a reduced-motion glow.
- [x] Upgrade Hyperdrive from straight streaks to a translucent fluid volume with warped tunnel walls, blue-green currents, luminous ripples, an acceleration surge, and moving page refraction. Cap the volume resolution/frame rate, keep the couch readable, and recover its graphics context independently.
- [x] Include the embedded WOFF2 fonts in the Docker build context and document manual remote deployment to the existing Fly app. `flyctl config validate` passed; this packaging adjustment has not been remotely built or deployed.
- [x] Keep all landing assets local, isolate styles from account pages, honor reduced motion, pause hidden/offscreen rendering, and provide a WebGL fallback. Illustrations do not imply playable games or runtime qualification.

Local verification for the landing page: the Rust workspace tests, formatting check, and Clippy passed; JavaScript syntax and the bounded, repeating creation timeline were checked. The desktop scene, atmosphere switching, portal action, and pause control were reviewed in Chrome. Rotation verification: the platform build and JavaScript syntax check passed; Chrome exercised native mouse and emulated-touch dragging, scrolling outside the touch target, keyboard rotation/reset, all creation passes, and simulated standard gamepads (all rotation axes, dead zone, frame-rate independence, reset edge, disconnect, pause, and reduced motion). Background verification: Chrome rendered both WebGL layers through the full creation loop without graphics errors; desktop, mobile viewport, atmosphere and portal captures were reviewed. Pause, reduced motion, offscreen suspension, and background context loss/restoration passed. Physical controller hardware remains untested. These checks do not qualify game execution or controller hardware.

Couch-only hero verification: platform build, Rust workspace tests, formatting, Clippy, JavaScript syntax, and diff checks passed. Chrome rendered all creation stages without graphics errors; desktop and phone captures were reviewed. Keyboard rotation/reset, pause, reduced motion, and context restoration passed. The production page makes no side-object requests, and the removed artwork routes return 404.

Hyperdrive verification: the platform build, Rust workspace tests, formatting, Clippy, JavaScript syntax, and diff checks passed. Chrome checked exact build-clock freezing/resumption across all creation stages, randomized rotation and prior-angle restoration, repeat-click protection, the half-second return, paused launches, pause/resume, reduced motion, low-frame-rate timing, and graphics-context recovery. Desktop and phone flight/return captures were reviewed. The fluid volume rendered within a 760 × 650 pixel cap and recovered from context loss; page refraction reset cleanly. Randomized rotation completed 2–2.2 turns per jump, selected a fresh axis, and settled along the shortest arc back to the prior view. The rebuilt production preview completed normal and scrolled-offscreen jumps in about 3.51 seconds, with no browser exceptions.

## Current constraints

- The user authorized the standard Godot 4.7.2 editor for development and dogfooding. Do not install export templates or additional runtime versions implicitly; disk space remains limited.
- Do not automatically install toolchains or download runtimes from build scripts, tests, or CLI commands.
- Keep Rust debug symbols and incremental compilation disabled to limit disk use.
- Develop engine-independent components and run the source sample locally. Physical controller hardware, shared PCK execution, other OS targets, and sandbox validation remain separate required checks.
- Neon is our server database. SQLite and save files live on player computers.
- A TV uses a direct display connection or an existing screen-sharing setup. V1 does not build a streaming system.

## GDK productization — next creator-facing work

See [GDK.md](GDK.md) for the kit contents, current capabilities, proposed commands and acceptance criteria, and [docs/platform-and-gdk.md](docs/platform-and-gdk.md) for the four-way split (platform code, in-game SDK, creator scripts, instructions). This work packages the lessons from the sample games into reusable creator tools; it does not mark runtime/distribution feasibility complete.

- [x] Define the GDK product, SDK/platform responsibilities, technology, current setup path and staged release gates.
- [x] Native macOS Apple Silicon GDK setup preview: standalone AppKit UI, bundled Rust doctor/CLI, existing-editor selection and official download guidance, verified versioned staging/reinstall, installed native launcher, and a Godot Creator Hub that creates independent 3D source projects. Core kit excludes large games/art/engine binaries. This is an internal ad-hoc-signed preview; automatic editor download, public signing/notarization, Windows and full GDK service contracts remain pending. Local verification: 28 native setup and eight Hub/project assertions passed; fresh external installation and generated source-game import/run passed. Native missing-editor, install, Hub handoff and project-to-editor flows reviewed; installed launcher signature and CLI without Cargo/Python on PATH checked. No engine/templates installed.
- [x] Refresh native setup with a dark studio layout, original vector illustration, required/installed comparison cards, readiness badges, disk-space summary and a prominent next action. Detect verified existing kits, show prior versions, flag modified installations and remember the selected parent folder. Preview 0.1.0-preview.4 preserves older installs side by side.
- [x] Share the navy/mint palette and bundled Inter/Space Grotesk fonts across account pages, web home, native apps, Creator Hub, and the Godot library; add focus/hover motion, panel lighting, and cover shading. Native build scripts package the TTF fonts, and web home serves its local WOFF2 assets.
  Local checks for this pass: Rust formatting, workspace tests, and Clippy passed; both native apps passed Swift typechecking; the build scripts parsed; branding packaging checks and Creator Hub import/headless startup passed. The full SDK suite passed on the already-installed Godot 4.7.2, including Haymaker and the Flat World camera correction. This pass did not include a new visual or physical-controller review.
- [x] Optional AI CLI detection and native agent picker; explicit vendor installation, existing-agent declaration and persisted choice. Codex, Claude Code, Grok, Kiro and Cursor catalog; Kiro provisionally maps the requested “kik”. Synthetic installer/detection checks and native picker/cancel review passed. Live vendor downloads and account sign-in remain untested.
- [x] Add a separate local macOS **Giga Couch.app** and Desktop shortcut to open the source library, reuse the bundled doctor CLI without Cargo at launch, and clean up its supervised game/helper on quit. It reuses this checkout and installed Python/Godot; standalone player distribution and account/social services remain pending. Add a top-right close button to the agent picker. Player startup now keeps an animated loading window visible through import and waits for a rendered-library readiness signal before hiding it; per-launch status files are cleaned up and are not inherited by games. All 11 library tests and the full installed-engine SDK suite passed; loading-to-library handoff visually reviewed. Desktop/Dock reopen now activates the existing library or active game; minimizing and reopening the library was verified through Finder.
- [x] Player preview host: `couch host` launches the library without Python; shelf shows samples, Hub games (including missing folders), and unsigned installs; Family/Guest profiles; `COUCH_SAVE_DIR` per profile/game; game info and settings; A/Cross glyphs. Still uses the installed editor as runtime. Accounts, notarized portable app, and playable packed PCKs remain open.
- [ ] Deliver the remaining **Giga Couch** player product: player-only runtime, packed-game play, TV controller qualification, accounts and social flows. See [player-app specification](docs/giga-couch-app.md).
- [x] Create→TV loop (partial): starter `AGENTS.md` + `couch.game.json`; Hub/`couch init` register source projects; Game Player launches them with `--path`; SDK `install_shell` / `save_data` / `quit_to_platform`; `couch doctor --project`. Unsigned local source only. Lobby extraction, 2D starter, `couch pack` / private share remain open.
- [x] GDK creator tools: reusable `install_lobby()`, 2D Flat World starter, `couch check` / `run` / `pack` (no template downloads) / local `publish --visibility private`. Remote accounts and `couch test` harness remain open.
- [ ] Local creator preview leftovers: remapping, rumble, `couch test` harness, prove two unfamiliar projects.
- [ ] Prove reuse with an unfamiliar creator and two independent starter projects; distinguish sixteen-slot synthetic checks from physical hardware acceptance.
- [ ] Package-to-platform preview: real PCKs, qualified shared runtime and OS isolation, Rust host, SQLite-backed playable library, save/update/crash recovery and prebuilt creator tools.
- [ ] Private release: authenticated publishing, validation/signing, authorized recipient installs, licensing and controller/TV qualification.

## First implementation slice: local package library

Build a real, testable content path without executing any game code:

```text
Local manifest + exported PCK
  -> validate metadata and artifact bytes
  -> select target OS/architecture
  -> stage and verify copied content
  -> install immutable release
  -> activate in SQLite
  -> list installed library
```

The initial package contract is provisional until shared-runtime feasibility is demonstrated. Test packages contain synthetic bytes; accepting them proves the content-management path, not Godot compatibility or game safety.

### Deliverables

- [x] Rust workspace with small build profiles and a dependency lockfile.
- [x] Manifest v1 types and JSON Schema, with bounded fields and portable identifiers.
- [x] Streaming SHA-256 and byte-size checks; reject unsafe paths and non-regular artifact files.
- [x] SQLite migration for immutable installed releases and one active release per game/target.
- [x] Staged local installation, idempotent retry, immutable-release conflict detection, and preservation of the previous active release on failure.
- [x] CLI: `doctor`, `validate`, `install`, and `library`, with structured operational JSON output.
- [x] CLI: `couch init` copies a template, writes `couch.game.json`, and registers `creator-projects.json`; `doctor --project` inspects addon/autoload/policy without launching Godot. Neither command downloads an engine.
- [x] Tests for invalid packages, simulated interrupted/retried installation, update failures, persistence, and CLI behavior.
- [x] Manually dispatched Windows/macOS/Linux checks that do not install Godot; push/pull-request triggers removed at user request. Remote runs remain pending.
- [x] README with a small, explicitly non-playable fixture workflow.

Unsigned local imports are developer tooling only. They do not authorize public or private distribution. This slice does not download, install, or launch a runtime.

Verified locally on macOS ARM64 with Rust 1.97.1: 21 tests passed; formatting and Clippy passed. This does not verify other operating systems, actual PCK compatibility, abrupt power-loss recovery, game execution, or sandboxing. See the README for the current guarantees and limits.

## Subsequent milestones

### Runtime detection and SDK onboarding

- [x] Shared policy for standard official Godot 4.7.2 stable, embedded by Rust and read by GDScript.
- [x] Host discovery through explicit path, environment, PATH, and common application locations.
- [x] Missing, unsupported, and unusable states with setup instructions; `doctor --require-godot` for readiness gating.
- [x] Bounded version probes; explicit override precedence; tests for failure, timeout, and selecting a supported installation.
- [x] SDK autoload and editor status panel using the same version policy.
- [x] Explicit SDK test runner that selects Godot through our own doctor command.
- [x] Dogfood missing → installation → supported on this Mac, then run the real SDK checks.

This is a development-engine version check, not certification of the final shared distribution runtime, its signature, or its sandbox.

Dogfood result: the host first reported a missing engine with setup instructions. The official standard macOS archive was verified against its release SHA-256, its app signature was verified, and Godot was installed in the user's Applications directory. Auto-discovery then selected `4.7.2.stable.official.ed1daf0bf` without a path override. The SDK editor plugin imported successfully and seven shared compatibility cases plus the real-engine check passed. The exercise exposed a short first-start timeout and JSON number formatting mismatch; both were addressed. Export templates were not installed.

Verification after this slice: 31 Rust tests passed on macOS ARM64, along with formatting and Clippy. Engine-backed SDK checks passed separately through `scripts/test_sdk.py`. Remote CI and real Windows engine discovery are not yet verified.

### 0. Shared runtime and OS feasibility — engine validation pending

1. Select exact Godot release/build flags and supported Windows/macOS targets.
2. Export two tiny projects plus the launcher and reuse one installed runtime to run them.
3. Prototype the desktop host, readiness handshake, crash recovery, and return to launcher.
4. Demonstrate OS isolation for PCK access, GPU/audio/controllers, restricted communication, scratch storage, and per-game saves.

**Exit:** both games run from the same installed engine on each supported OS, a crash returns to the launcher, and adversarial tests demonstrate the access boundary. Until then, avoid claims that game distribution is safe or that PCKs work across targets.

### 1. SDK and sixteen-player sample

1. GDScript addon, `Platform` autoload, starter project, and documented API.
2. Per-player input state; join, leave, disconnect removal, and fresh rejoin flow.
3. Action remapping, analog dead zones, glyph fallback, and basic rumble.
4. Controller-driven menus, pause, TV-safe layout, and quit-to-platform.
5. Asynchronous local save/load with explicit errors, atomic replacement, and schema migration hooks.

**Exit:** sixteen independent controllers can complete a session, reconnect without stealing another player's slot, and restore a saved game. Wireless Xbox, PlayStation, and mixed groups are required hardware test cases. Check direct TV output and one screen-sharing setup.

#### Little World: first playable slice

- [x] A 3D island with original procedural characters, steps, platforms, a shared camera, movement, jumping, and fall recovery.
- [x] Sixteen player slots in the SDK, package validator, and JSON Schema.
- [x] Device-isolated left-stick/D-pad movement, radial dead zone, and mapped south-button jump (Xbox A / PlayStation Cross).
- [x] Join, hold-to-leave, immediate character/slot removal on disconnect, and one-button rejoin; keyboard fallback uses one of the same sixteen slots.
- [x] Clear held input on focus loss/disconnect; normalize diagonal movement.
- [x] Source-project launch through the platform's installed-engine discovery; no export template downloads.
- [x] Engine-backed synthetic routing tests and actual scene/physics tests; rendered visual inspection on this Mac.
- [x] Wireless setup instructions and a hardware test matrix with untested cases marked pending.
- [x] Maximized startup, 16:9 scaling, and keyboard/controller/button fullscreen toggle; native maximized → fullscreen → maximized verified on this Mac.
- [x] Roku/AirPlay window-sharing instructions using the existing OS flow. Actual TV compatibility, window selection, and latency still need a physical test.
- [x] Experimental Wii family action profiles, per-device F3 layout selection, explicit SDL driver startup, and scoped bare-Remote D-pad mappings; 53 synthetic profile assertions pass.
- [x] Install and open the small Wii pairing helper on this Mac after the Remote Plus hit the standard Bluetooth PIN prompt; archive digest and bundle signature checked. Release, local diagnostic, and game-closed pairing attempts failed; Bluetooth compatibility is a recorded blocker for this Mac/Remote Plus setup.
- [x] User-confirmed movement/jump with one `04e8:7021` Remote via the native macOS report reader. Saved `play_wii_native.py` with private session state and SDK input expiry handling; broader hardware coverage remains pending.
- [x] Cloudbound single-pilot 3D flight sample, native acceleration decoding, factory/approximate scaling, steady-pose calibration, and pause on stale motion. Native compilation, C protocol fixtures, 34 synthetic motion/game assertions, and rendered capture passed. Saved launcher received live motion and factory calibration on this Mac; user confirmed flight, then requested sideways steering and away-to-dive after reporting reversed banking. User confirmed revised sideways flight controls; reconnect acceptance pending.
- [x] Shared Godot controller motion adapter and Cloudbound per-player motion, original Joy-Con separate/paired modes, Switch/Pro family profiles, and explicit stick fallback. 61 synthetic adapter/profile/game assertions pass; Switch hardware qualification pending.
- [x] Ignore macOS duplicate Joy-Con devices (HIDAPI halves vs combined/MFI copy) so one south-face press cannot join two players and a later jump/respawn cannot spawn a ghost that shares input. Synthetic duplicate/paired/MFI/Xbox isolation checks added; physical two-Joy-Con retest pending after the Little World report.
- [x] Native macOS USB reader for vendor-class Xbox 360 XID and Xbox One GIP pads: device catalog, generic `ff:5d:01` / `ff:47:d0` match, multi-pad JSON, SDK consumer, synthetic decode/lookup/input checks. Nacon Compact (`146b:0603`) physically joins Little World on this Mac. HID DualShock/Xbox Bluetooth stay on Godot/SDL. Wired GIP init/decode is implemented; a physical Xbox One USB press is still pending. Rumble and touchpad are out of scope. See `docs/wired-usb-controllers.md`.
- [x] Pocket Rally standalone 3D car game: 1–16 local players, adaptive split-screen chase cameras, per-driver Wii tilt calibration, acceleration/brake/reverse, collisions, checkpoints, and disconnect/rejoin handling. Driving-only native helper supplies sixteen independent observed-variant channels; Cloudbound source and its original reader preserved. Synthetic scene/input checks and rendered 2/4/16-view inspection passed; physical driving and multi-Remote qualification pending.
- [ ] Switch 2 runtime/driver integration and hardware qualification; profile recognition alone is not support. See `docs/switch-controllers.md`.
- [ ] Physical Wii Remote/Plus, Nunchuk, Classic/Pro, and Wii U Pro pairing, driver mapping, reconnect, and mixed-controller tests; integrate verified setup into the desktop host.
- [ ] Specialty Wii accessories, GameCube adapters, Wii U GamePad connection path, motion/IR APIs, and third-party variants. Track individually in `docs/wii-controllers.md`.
- [ ] Physical wireless Xbox, PlayStation, and mixed-controller sessions at 1, 2, 4, 8, and 16 players.
- [ ] Full pause/menu, remapping, haptics, persistence, host lifecycle, and packaged launch.

Verified locally on macOS ARM64: 32 Rust tests, formatting, Clippy, eight runtime checks, 564 synthetic input assertions, 53 Wii profile assertions, 16 native-reader assertions, 69 controller-motion assertions, and 32 headless scene/physics assertions passed. A rendered run passed the same scene tests plus screenshot capture. This validates source-game behavior, not physical wireless connections or a packaged distribution runtime.

### 2. Desktop library and runtime management

- [x] Godot library screen for the source games, mouse/keyboard/standard gamepad navigation, controller setup options, separate game processes, Wii helper selection/cleanup, and return-to-library with selected card restoration. Development Python supervisor reuses the existing Rust runtime doctor. UI/process tests and real headless launches for all three games passed on this Mac.
- [x] Add single-player World 1-1 recreation with original captured NES artwork, tile layout, item/enemy mechanics, underground room, flag/castle completion, and SDK input. Source provenance and deviations documented; game and library checks pass with synthetic input, rendered views inspected, physical playthrough/frame comparison pending. Four-game library uses a 2×2 card layout and all launchers share one catalog. Jump tuning after user feedback increases standing/running height and short-hop height; tests cover all pipe heights without the run button and overhead block hits.
- [x] Gauntlet-inspired first dungeon, The Ember Vault: shared-screen 1–16 local heroes, four selectable classes, enemies/generators, ranged combat/magic, two keyed doors, shared supplies, ally revival, team exit, results/retry, pause/focus recovery, and native Wii fleet integration. Full solo/16-player simulated playthroughs and controller/lifecycle checks pass; rendered game/library inspected. Library now scrolls to accommodate five games. Flying/driving game sources preserved. Physical Gauntlet and sixteen-wireless-controller acceptance pending.
- [x] Gauntlet playtest revision: A/Cross/Wii 2 confirms start after joining (including buffered quick taps), mouse start/resume fallback, individual portal escapes and all-player completion, six-second victory return to the library, fallen-teammate rescue guard, and a lit procedural 3D dungeon with animated heroes/class portraits, stone shaders, torch shadows and portal/spell effects. Synthetic controller/escape/real-process-return tests and rendered inspection completed; physical retest pending.
- [x] Gauntlet art foundation: licensed CC0 textured/rigged heroes and animation library, scanned PBR stone, reflective equipment, animated flames, stone gate arches, closer shared camera/minimap, per-game Forward+ selection with native Metal on macOS, and compatibility launch option. Synthetic rig-animation/camera-fit checks and launcher renderer checks pass. See `docs/gauntlet-art.md` for assets, performance evidence, and remaining art work.
- [x] Gauntlet camera playtest fix: wider default framing, immediate expansion during camera smoothing, fallen teammates included, and F3 full-dungeon overview; camera movement imposes no player boundary. Synthetic map-corner, fallen-hero, transition, and overview checks cover framing.
- [x] Gauntlet character polish: speed-driven blended locomotion with filtered upper-body attacks, alternating swings, authored bow arm poses, separate magic/hit reactions, eased turning/knockdown/revival, and remodeled class equipment, including a robed wizard with a bent felt hat and a repaired face-texture import. Four-class live blend-tree checks cover stride continuity during attacks, stopping, pause, hit/spell events, and recovery. Existing CC0 assets reused; no new downloads. Class silhouettes now include a broader, stockier Warrior, a bareheaded female Valkyrie with paired braids, and a 14% shorter Elf. Headgear follows measured head bounds: a fitted Warrior skullcap, narrower Wizard hat, and no Valkyrie headgear. Proportions apply to the animated visual rig and equipment without changing movement or collision.
- [x] Gauntlet lobby/HUD/menu revision: full-screen visible sixteen-slot party lobby, independent class selection and readiness, all-ready start gate, controller/keyboard/mouse controls, compact joined-player-only status strip, expanded dungeon viewport, minimap confined to the header, and pause options for camera/minimap/lighting/sound/help with confirmed lobby/library exits. Returning to the lobby retains connected players and clears readiness. Automated lifecycle/input/layout checks and rendered small/full-party review cover the new flow; physical controller retest pending.
- [x] Gauntlet controller follow-up: ordinary confirm enters after the full party readies; per-player D-pad focus reaches every lobby button, including controls/fullscreen/library, with profile-aware back actions. Native Wii Plus maps to Start/menu. Tests drive sixteen Wii joins/readiness/start with raw packets rather than calling start directly.
- [x] Gauntlet audio pass: original layered weapon/impact/interface effects, three recorded hurt takes per class, per-player damage routing and reserved hurt voices, and three compressed original music loops with phase fades and hurt ducking. Independent controller-accessible sound/music toggles. Full SDK checks pass; waveform headroom and rendered pause-menu layout checked. The user approved the replacement recorded lobby lines. Hero hurt reactions now use twelve CC0 human recordings from four performers, with checked-in sources and repeatable trimming/loudness preparation; all SDK checks pass (using a longer import/test timeout on this Mac), and sample levels/headroom are verified; listening and broader TV mix balancing remain pending.
- [x] Class-selection utterances: four CC0 recorded lobby lines from Kenney with source/actor/license files, per-player scrolling debounce, interruption of that player’s previous line, and shared-speaker playback. An experimental per-connection Wii output bridge is implemented and packet/routing tested, but speaker streaming is disabled in normal play after physical choppiness and disconnection reports. Xbox, keyboard and Wii selections all use regular speakers.
- [x] Controller hit feedback and softer enemies: actual damage requests 120 ms per-device rumble through Godot/native Wii; passive drain/invulnerability stay silent. Short, quieter grunt/ghost/demon reactions use regular speakers. Rumble has its own menu switch; pause/reset/leave/shutdown stop feedback. The full SDK suite passes, including 103 audio/routing assertions covering recorded-line interruption and disabling Wii audio, 236 gameplay assertions and 350 campaign assertions. The tested Wii Remote stayed connected during rumble-only testing but no vibration was felt; gamepad vibration remains physically unverified.
- [ ] Reliable Wii speaker output: both low-rate synchronous PCM and paced asynchronous ADPCM were physically choppy; the latter coincided with controller disconnects. Experimental code remains isolated behind a disabled speaker flag. Do not advertise native speaker support until sound quality and connection stability are demonstrated on hardware.

- [x] Gauntlet combat feedback and class identity: heavy axe and quicker sword melee with windup/reach/arc/wall checks; modeled Elf arrows; Wizard bolts; distinct damage/cadence. Quiet idle poses and a release guard prevent menu presses from becoming attacks. Actual hits trigger brief flinches/tints and quiet rate-limited sounds; NPC health bars appear only after damage and fade. Single/16-player routes and targeted combat/input/render checks exercise the revised mechanics; physical balance/playtesting remains pending.
- [x] Distinct first-vault keys: Ruby opens the western gate and Sapphire opens the eastern gate; both key pickups, models, labels and minimap use matching colors. Every gate now checks the matching inventory color. Regression cases reject both wrong-key combinations without consuming a key. Full SDK suite passes, including 236 Gauntlet gameplay and 350 campaign assertions.
- [x] Third-vault Sapphire gate: place it across the straight approach before the bend, with solid walls at both ends, so the adjoining room cannot bypass it. Regression checks test every later-level gate independently with all other gates open and verify solid walls at both ends. Full SDK suite passes with a longer timeout on this Mac, including 374 campaign assertions and the existing solo/16-player key-and-exit routes.
- [x] Gate readability: remove overhead gate arches and nearby tall supports, replace thin bars with low saturated panels and thin colored floor inlays, and retain one plain color-name label per gate as its leaves retract into the sides. State is shown by the panels and empty passage, without OPEN/CLOSED text or letter prefixes. All three maps and both gate orientations use the same presentation; collision/key rules are unchanged. Gauntlet gameplay (234 assertions) and campaign (350 assertions) checks pass; closed/open Metal renders were inspected. A subsequent full SDK run passed after the separate Sunbreak compile errors were resolved.
- [x] Three-vault Gauntlet campaign: two larger irregular floor plans (56×34 and 76×48 bounds), varied room dimensions/shapes, crooked corridors, dead-end supplies, horizontal/vertical gates, three then five colored key dependencies, increasing enemy/generator pressure, animated six-second portal transitions, party/color/score carry-over and health/potion recovery. All-party exits advance until the final vault; lobby restarts clear campaign state. Full SDK suite passes, including 234 Gauntlet gameplay assertions and 338 campaign assertions; solo/16-player collision-and-key routes pass independently of combat. Metal renders of both irregular maps and the animated transition were inspected. Later-level combat balance and physical party play remain to be reviewed.
- [x] Potion wave timing: apply class-specific damage once per target as the visible ring reaches enemies and generators, with a shared simulation/render radius. Preserve cast origin, independent overlapping waves, pause, range limits, and reset cleanup. Full SDK suite passes with 234 Gauntlet assertions, including four-class wave timing and matching rendered radius checks.
- [x] Wizard robe movement: remove inherited hip pitch from the skirt, retain hip bounce, and ease the hem/trim opposite actual movement. Four-direction, stopping, and pause regression checks cover the cloth drag. Full SDK suite passes (195 Gauntlet assertions); the Metal walking preview was visually checked.
- [x] Gauntlet player colors: sixteen color swatches with controller color focus, keyboard C, clickable swatches, and live labeled previews. Clothing/shields, player markers, HUD/minimap and magic rings follow the selected color. Color survives class changes and lobby resets; edits clear readiness and do not affect teammates or combat stats. Automated color ownership/persistence/render and native Wii navigation checks pass; physical TV palette review remains pending.
- [ ] Gauntlet AA art completion: bespoke creature models, coherent class-specific armor/weapons, further animation polish and combat effects, final environment dressing, sustained 60 FPS target qualification, and TV playtesting. Current art pass does not establish AAA production quality.
- [x] Sunbreak original solo FPS slice: coastal island scenery, Forward+ / Metal lighting, animated water, mouse/dual-stick controls, carbine aiming/reload/headshots, three robot waves, swept hostile projectiles, solid deployable cover, health/shield drops, shrinking storm, radar, pause/retry/victory, and sixth library card. Real-engine automated physics/combat/lifecycle checks cover the full match; see `sdk/examples/sunbreak/README.md` for scope and limits.
- [x] Haymaker LAN brawler sample: host/join/practice, one third-person view per computer, host-authoritative ENet, melee/loot/shrinking ring, last standing. `Platform.install_lan_session()` plus loopback handshake and combat checks. Two-computer LAN play, discovery, and sandbox network qualification remain separate.
- [x] Haymaker art/combat pass: textured carnival city and wrestling ring, four wrestler builds using existing CC0 humanoid rigs/animation clips, strike combos, grab/slam, dash, block, dropkick, and elbow drop. Generated artwork sources and revised controls are documented in the sample.
- [x] Activate Flat World's camera after adding it to the scene tree, fixing the startup error caught by the SDK suite.
- [x] Sunbreak input correction: no mouse capture on start/resume, explicit right-drag mouse look, controller ownership before menu input consumption, isolated look sources, neutral-stick gate and radial dead zone, and R3 recentering. Synthetic regression coverage added; the user reported improved controller behavior. Broader hardware qualification remains pending.
- [x] Sunbreak visual/motion pass: licensed CC0 rig/animation library and scanned surfaces, original layered armor and rifle/hands, locomotion/aim/fire/hit/death blending, navigation around collision and deployed cover, buffered jumping and landing response, recoil isolated from camera aim, textured scenery and wind foliage, static material batching, and per-game rendering presets. 68 Sunbreak checks pass; the latest full suite stops on a missing audio asset in concurrent Gauntlet work; staged Metal renders inspected. See `docs/sunbreak-art.md` for assets and the limited rendering sample.
- [x] Sunbreak playtest regressions: convex collision for 66 reachable rocks, parent-frame bone turns to keep strafing enemies upright, and independent look-axis dead zones to suppress vertical offset during horizontal turns. Regression checks pass; physical Xbox confirmation for the recurring aim-drift report remains pending.
- [ ] Resolve physical Xbox false upward input: reproduced through both HIDAPI and Apple GCController when moving/firing/jumping without using the right stick. Unsuccessful launcher override removed; 72 Sunbreak checks pass, including real movement/fire/jump with neutral aim. Independent native input comparison remains pending.
- [ ] Sunbreak human playthrough, physical dual-stick controller qualification, sustained frame-rate measurements, and further environment/robot animation polish. Current procedural art does not establish Fortnite production-quality graphics.
- [ ] Connect the library to playable installed releases in SQLite through the production Rust host; source catalog and private-file prototype IPC do not establish distribution or sandbox readiness.


1. Rust desktop host owns SQLite, credentials, installation, and child processes.
2. Godot launcher consumes a versioned local API and shares SDK UI/input components.
3. Runtime inventory, approved artifact verification, automatic reuse, and rollback.
4. Download recovery, disk-space checks, crash recovery, and signed app updates.
5. Local profiles and saves survive game updates/uninstall; offline play needs no Neon connection.

**Exit:** a clean machine installs the app and two games, launches them without the editor, survives an interrupted update, and remains playable offline.

### 3. Private sharing and platform backend

1. Rust/Axum API, Neon migrations, and object storage integration.
2. Managed identity provider selected for PKCE/native login and phone-assisted TV login.
3. Creator-owned drafts, immutable releases, validation jobs, and signed metadata.
4. Family membership, invitations, explicit access grants, and authorized download URLs.
5. Disposable game-validation workers without production credentials.
6. CLI packaging/publishing and stable machine-readable diagnostics.

**Exit:** another authorized account installs a privately shared game on a second computer; unauthorized accounts are denied; retries do not duplicate publication; distributed games pass isolation checks on every supported OS.

### 4. V1 release readiness

1. Signed Windows installers and signed/notarized macOS bundles.
2. Recovery tests for installation, runtime updates, SQLite migrations, and saves.
3. Hardware matrix for controllers, Bluetooth, TV resolutions, audio, sleep/wake, and mirroring.
4. AI-readable SDK docs, starter walkthrough, local validation, and sample projects.
5. Opt-in diagnostic exports, operational backups, and restore drill.

**Exit:** an unfamiliar creator can create/play/share, and a recipient can install/play from the couch without managing Godot or a database.

## Verification strategy

Run Rust formatting, tests, and Clippy locally. Cover actual package corruption, path containment, idempotency, conflicting immutable releases, database persistence, and failed updates. Use subprocess tests for the CLI contract and failure exit codes.

The manually dispatched GitHub Actions workflow repeats engine-independent checks on Windows and macOS. Add headless Godot checks only in an explicitly separate workflow once a runtime is selected; never make local tests install an engine. Real-device testing remains necessary even after headless checks pass.

## Deferred scope

Payments, public discovery, cloud saves, online multiplayer infrastructure, custom streaming, additional engines, and marketplace recommendations remain outside this first V1 build sequence.

### Multiplayer workstream — documented, implementation pending

See [Multiplayer: LAN and managed online relaying](docs/multiplayer.md). Online servers forward messages; a player's computer owns game simulation. Scheduling this work does not change the V1 exit criteria above.

- [x] LAN session helper and Haymaker host/client prototype: address join, host authority, per-computer camera, loopback ENet handshake. Two-computer play and Little World's two-local-players-per-machine proof remain open.
- [ ] Session roster and LAN Little World proof: two computers, two players each, independent views.
- [ ] Authenticated online room and relay prototype with bounded routing and queues.
- [ ] Real-network latency, transport, controller, cross-OS, and sandbox qualification.
- [ ] Reusable SDK templates, usage metering, and a measured monthly developer-service proposal.

### Blender authoring tools

- [x] Installed official Blender 5.2.2 LTS for Apple Silicon (SHA-256 and notarization verified; 907 MiB installed, temporary DMG removed). Created and rendered an editable original chest/flask/brazier study in `art/gauntlet/`, inspected the preview, and loaded all three GLBs through Godot 4.7.2 with materials intact (4,384 / 1,984 / 4,332 triangles). Sources stay outside the runtime project; these art studies are not yet placed in gameplay, and players need no Blender installation.

- [x] Blender cast integration: four hero armor/detail sets, four weapons, two shields, and three modeled creatures with shared mesh parts, local limb animation, combat-event attacks and existing health/hit effects. Preserve hero rigs, proportions, selectable colors and wizard cloth. The SDK run passed all other games; after repairing colored-material lifetime during portrait replacement, all Gauntlet checks passed: 46 cast, 236 gameplay, 103 audio and 374 campaign assertions, plus process return. Final Metal cast render inspected. Thirteen GLBs occupy under 1 MiB; sustained worst-case combat profiling remains pending.

- [x] Gauntlet art/motion and temporary sight polish: engraved/stitched hero equipment, measured movement lean and breathing, articulated enemy knees, eased turns and demon tail motion. Warrior axe grip reverses for alternate strikes; live skeleton checks confirm the cutting edge leads both. Eleven original Blender props/masonry assets add modeled pickups, altars, braziers, carved wall details, beveled batches and varied room-floor motifs without changing navigation. Shared radial fog follows up to sixteen players (seven clear tiles plus three fading tiles), preserves fallen-teammate sight, clears escaped/disconnected bubbles, and limits the minimap/overview without storing exploration history. Full `python3 scripts/test_sdk.py` passed: 53 visibility/prop, 53 cast/animation, 236 gameplay, 103 audio and 374 campaign assertions plus process return and all other SDK/game checks. Metal gathered/split/sixteen-party views and idle/walking/attack cast poses were rendered and inspected; `git diff --check` passed. Physical playtesting and sustained worst-case performance remain pending.

- [x] Gauntlet axe/contact and occluded sight follow-up: replace the Warrior's borrowed sword swings with an authored windup/cleave/recovery, share its blade trajectory and simulation clock with swept hit checks, hold attack direction while moving, and pose the hand onto the wrapped grip. Walls and closed doors now block party sight; cached angular shadow fields, strict octant-boundary rays and a depth-aware fog pass prevent corner/raised-object leaks. Door changes invalidate visibility, and hidden actors/labels/effects are culled. Full `python3 scripts/test_sdk.py` passed (71 visibility, 65 cast, 237 gameplay, 103 audio, 374 campaign assertions and process return, plus other games). Independent grid-ray comparison found no leaks among 11,507 visible floor candidates across three maps. Metal windup/contact/recovery and closed/open-door renders inspected; `git diff --check` passed. Compatibility completed its 71 visibility assertions but reported two GL texture leaks during shutdown; clean Compatibility teardown and physical playtesting remain pending.

- [x] Restore radial-only Gauntlet fog after checkpoint `0cfeb71`: retain the improved axe trajectory/contact, remove wall/door occlusion and mask caching, and use the shared seven-tile clear radius plus three-tile fade for the dungeon and minimap. Nearby enemies/keys remain visible across solid walls and closed/open gates; leaving the radius hides them again. Focused Gauntlet checks passed (58 visibility, 65 cast, 237 gameplay, 103 audio, 374 campaign, plus process return); World 1-1 also passed. The full SDK run passed earlier checks but stopped at the unrelated Sunbreak assertion “WASD input moves the first-person physics body,” so this run is not a full-suite pass. Metal gathered, split-party, sixteen-player and closed/open-gate renders inspected; `git diff --check` passed. Reopened native Wii Gauntlet for playtesting.
