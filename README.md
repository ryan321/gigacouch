# Giga Couch

A website for making, sharing, and playing web games in Chrome with a game controller. The current V1 playtest is
at [gigacouch-platform.fly.dev](https://gigacouch-platform.fly.dev).

**V1 is the website: accounts, profiles, uploading web games, browsing, and playing in Chrome. See [docs/v1.md](docs/v1.md).** It lives in [apps/site](apps/site/README.md). To run it locally, run `pnpm site`, open http://localhost:3000, and run `pnpm site:seed` for a demo account and game. The Game Browser app, phones as controllers, the Godot SDK and player, and the local library described below are research and prototypes from before that decision. They stay in the repo and are not V1.

- [V1: web games in Chrome](docs/v1.md)
- [Product vision](PRODUCT.md)
- [Game Development Kit: contents, workflow, and delivery plan](GDK.md)
- [What the platform and GDK provide: code, scripts, and instructions](docs/platform-and-gdk.md)
- [Giga Couch player app: TV library, controllers, accounts and social](docs/giga-couch-app.md)
- [Giga Couch Game Browser: couch play for web games, and why it is better than Chrome](docs/game-browser.md)
- [Phone controller ideas: what to build next](docs/phone-controller-ideas.md)
- [Phone views: reusable, game-defined phone screens](docs/phone-views.md)
- [Runtime architecture](docs/GIGACOUCH_ARCHITECTURE_v2.md)
- [Technical architecture and stack](TECH_STACK.md)
- [Implementation plan: V1 checklist, then R&D](IMPLEMENTATION_PLAN.md)
- [Multiplayer design: LAN and managed online relaying](docs/multiplayer.md)
- [Pricing strategy options: paid platform, subscriptions, and creator marketplace](docs/pricing-strategy.md)
- [Package manifest schema](schemas/manifest.schema.json)

The account pages, web library, native setup/player, Creator Hub, and Godot library share the navy-and-mint palette, Inter UI text, and Space Grotesk headings described in [the brand guide](brand/README.md). Fonts are bundled locally; the native build scripts include them in each app.

## Public landing page

`couch platform` serves the landing page at `/` and the existing account flow at `/account`. The page source lives in `crates/platform/site/`: a responsive, full-screen WebGL couch and portal, a 28-second creation loop (wireframe scan → blockout → faceted geometry → smooth surfaces → detailed upholstery → reset), pointer parallax, three selectable atmospheres, a Hyperdrive jump, animated concept worlds, and scroll reveals. Assets and fonts are served locally; there are no CDN or JavaScript framework dependencies. These are illustrative concepts, not playable game previews.

The hero background is a procedural WebGL environment: flowing aurora filaments, drifting nebula clouds and distant stars over a receding grid. Energy waves follow the couch’s scan stages, pointer motion bends the currents, and arrival or changing the atmosphere sends a gravity wave through the scene. The background renders behind the headline, caps its resolution and update rate, and falls back to static gradients if its graphics context is unavailable.

The couch is the only rendered object in the hero, surrounded by the portal and atmospheric effects. There are no side-object textures or animation atlases to load.

**Hyperdrive** freezes the couch at its exact creation stage, eases it into a randomized spin, and adds a small vibration and blur. Each jump picks a new rotation axis and completes about two full turns, then settles along the shortest arc instead of rewinding those turns. A full-viewport fluid tunnel surrounds it: translucent blue-green folds, twisting currents, and luminous refraction fronts rush past as the camera accelerates. The headline and intro bend through a moving displacement filter as the page stretches and blurs. The volume uses a capped-resolution WebGL layer that draws only during flight; it needs no downloaded artwork or animation files. The jump lasts three seconds, followed by a half-second eased return to the previous viewing angle; the build then resumes where it stopped. Repeated clicks are ignored during flight. Pause freezes the flight, hidden tabs suspend it, and launching from a paused scene restores that pause afterward. An active jump finishes even if you scroll past the hero, so the rest of the page returns to normal. Reduced motion uses a gentle button glow without the tunnel, shake, or page movement.

The final creation pass adds sage green linen shading, a geometry-locked woven texture, inflated cushions, and rounded seam piping; earlier stages retain their simpler materials.

Every couch stage is a full 3D model. Drag over the couch to rotate in any direction; arrow keys turn it, Q/E roll it, and Home or Reset View restores the original view. A browser-standard controller uses the left stick to turn, the right stick horizontally to roll, and A/Cross to reset. Press a controller button to make it available to the browser. Rotation works while paused or with reduced motion enabled; touch scrolling remains available outside the couch.

The motion control pauses the scene and CSS animations. Reduced-motion preferences are respected, normal scene rendering stops when the hero is offscreen or the tab is hidden, and a branded fallback appears when WebGL is unavailable. The landing page has its own stylesheet; account styling remains separate.


### Deploy V1 to Fly

The existing `gigacouch-platform` app now runs the Next.js website from `apps/site`, with the game files routed
through `gigacouch-games`. With the installed Fly CLI authenticated:

```sh
cd apps/site
flyctl deploy --remote-only --ha=false
flyctl status --app gigacouch-platform
```

See [the site deployment notes](apps/site/README.md#fly-playtest-deployment) for the games gateway, data migration
and checks. The root Dockerfile and Fly configuration are historical Rust-platform files; deploying them to the
same app would replace V1. Pushing to GitHub does not automatically deploy either app.

## GDK setup preview

Build and open the native macOS Apple Silicon installer:

```sh
python3 scripts/build_gdk.py
open '.gigacouch/gdk/Giga Couch GDK Setup.app'
```

The native setup screen works without Godot. Its editor and kit cards compare what you need with what is on your Mac, show readiness and disk space, and recognize an already-installed kit. An optional AI assistant card detects Codex, Claude Code, Grok, Kiro and Cursor CLIs, accepts your existing agent, or guides you through a chosen vendor’s CLI installation. It reuses a supported editor or offers the official download page and an editor picker, then installs a small versioned GDK. Creator Hub can create an independent 3D game, open it in Godot, and run it after its first import. The built app includes the CLI and needs no Rust/Python on the creator's machine. This is an ad-hoc-signed internal preview; Windows, public signing/notarization and automatic editor downloads remain planned. [Build, testing and installation details](apps/gdk-setup/README.md).

## Current implementation

The first V1 slice is **local package validation and installation**:

- Manifest parsing and validation for provisional Windows/macOS package targets.
- Streaming artifact size and SHA-256 verification.
- Staged installation with immutable release identities.
- SQLite library persistence and one active release per game/target.
- Idempotent retry, previous-release retention, and recovery by retry after a filesystem rename succeeds but database activation fails.
- A CLI with human-readable output and `--json` for automation.
- Shared Godot discovery/version policy, setup instructions, and an initial SDK editor plugin/autoload that checks the running engine.
- A playable **Little World** 3D sample, with 16 player slots, device-isolated input, jumping, and reconnect/leave handling through the SDK.

**Godot is not required for package/library commands and is not installed or downloaded by them.** `doctor` probes discovered executables with `--headless --version`. Package validation executes no game code: success means that metadata and bytes match, not that a package is playable, signed, or safe.

A source-game library screen is implemented. The production desktop host, full input/save SDK, runtime installation, packaged game launch, OS sandbox, Neon API, sign-in, downloads, and sharing remain planned work. Little World is a playable source prototype, not a complete V1 platform release.

`couch web-serve` and `couch web-run` host a local web-1 package. Blob Island is the sample. The Electron shell is fetched separately onto the external drive; package tests do not download it or prove the game runs. See [runtimes/web/README.md](runtimes/web/README.md).

## Open the game library

On this Mac, double-click **Giga Couch.app** on the Desktop. This separate player app opens the existing controller-navigable game library; **Giga Couch Creator.app** opens Creator Hub. Rebuild the local player shortcut with `python3 scripts/build_player.py --desktop`. The preview reuses this checkout, existing Python and Godot, and a bundled doctor CLI; opening it does not build Rust or duplicate the game assets. A loading window stays visible through resource preparation until the library has rendered. Reopening the app brings the existing library or active game forward and restores a minimized library. [Player app details](apps/player/README.md).

```sh
python3 scripts/library.py
```

Click **Little World**, **Cloudbound**, **Pocket Rally**, **Super Mario Bros. — World 1-1**, **Gauntlet**, **Sunbreak**, or **Haymaker** to play. Each game runs in its own process; closing it returns you to the library with the same card selected. Keyboard arrows + Enter and standard gamepad D-pad + A / Cross also navigate the cards. F11 toggles fullscreen.

On this Mac, **Wii Remote + gamepads** uses our existing native Wii reader automatically for the selected game. Choose **Gamepads / keyboard** to play without it. Close any standalone game/reader first. See [library controls and implementation](sdk/launcher/README.md).

This first library lists the seven included source games in a scrolling grid. The production Rust supervisor and launching installed releases from SQLite remain planned.

## Play Haymaker — LAN last-one-standing brawler

```sh
python3 scripts/play.py --game haymaker
python3 scripts/play.py --game haymaker --host
python3 scripts/play.py --game haymaker --join 192.168.1.12
```

Or select **Haymaker** in the library. One computer hosts; every other player runs the same game on their own computer and screen. Practice mode fights three bots on a single machine. The carnival city includes a wrestling ring, textured buildings, and four animated wrestler builds. Combat adds three-hit combos, grab/slam, dash, block, dropkick, and elbow drop alongside loot, jump pads, and the shrinking ring. Host-authoritative Godot ENet on the local network; no account or Internet connection. Address entry is the join path in this slice (LAN discovery and online relay remain later). See [match rules and two-computer setup](sdk/examples/haymaker/README.md) and the [multiplayer design](docs/multiplayer.md).

## Play Sunbreak — single-player first-person shooter

```sh
python3 scripts/play.py --game sunbreak
```

Or select **Sunbreak** in the library. Fight three waves of robots across a colorful coastal outpost, keep ahead of the shrinking storm, collect shields/health, and deploy cover. Textured island scenery, wind-animated foliage, rigged armored enemies, navigation around cover, animated falls/hit reactions, and a detailed pulse rifle with spring recoil and magazine reloads run on the shared installed Godot. Uses Forward+ / Metal on macOS; add `--compatibility` for simpler lighting.

**WASD** moves, **right-button drag** looks/aims with the cursor free, **left click** fires, **right click** aims, **Space** jumps, **Shift** sprints, **R** reloads, **Q** builds cover, and **Esc** pauses. Standard gamepad: sticks move/look, RT/LT fire/aim, A/Cross jumps, X/Square reloads, Y/Triangle builds cover, and right-stick click levels the view. Press a controller button to select it; mouse capture is never automatic. F2 or the pause menu selects Performance / Balanced / Cinematic graphics; the default caps 3D rendering at 1080p on large displays. This is a single-player visual/motion pass toward AA quality, with original and credited CC0 art. See [match rules, controls, and verification](sdk/examples/sunbreak/README.md).

## Play Gauntlet — 1–16 player dungeon

```sh
python3 scripts/play_wii_native.py --game gauntlet
# Or gamepads / keyboard:
python3 scripts/play.py --game gauntlet
```

Gauntlet now has **three linked levels: The Ember Vault, The Sunken Archive, and The Crown Labyrinth**: four classes, ranged attacks, magic, monster generators, keys/doors, shared food and treasure, ally revival, and a team exit. It uses original levels inspired by arcade Gauntlet, with original scenery and CC0 character/material assets. Everyone plays on the same screen. The graphics pass adds textured, rigged heroes, blended walking and upper-body combat animation, alternating weapon swings, bow drawing, hit reactions, detailed equipment, a stocky Warrior, a bareheaded female Valkyrie with swept-back hair and braids, a shorter Elf, a robed wizard with a narrow felt hat and cloth that trails behind his movement, scanned stone materials, reflective metal, animated torchlight, low colored gates, a wider party-following camera, and a minimap. The camera keeps active and fallen heroes in frame without restricting movement; F3 toggles a full-dungeon view. Gauntlet selects Forward+ (native Metal on macOS); F2 toggles cinematic/performance lighting, and `--compatibility` on either direct play command selects the older renderer. Blender-authored armor and weapons now dress the existing animated heroes, and modeled wraiths, armored raiders and horned demons replace the primitive creatures with moving limbs and event-driven attacks. See [cast art and sources](sdk/examples/gauntlet/assets/cast/README.md). This is a stylized art upgrade; detailed facial animation and sustained crowd profiling remain future work. Each player escapes individually; once everyone is out, an animated portal screen introduces the next level. Level one introduces a Ruby gate/key and a Sapphire gate/key. The new maps grow into irregular rooms, winding corridors, optional dead ends and distant colored-key puzzles, with more enemies and generators. Every later-level gate spans its corridor, including the Sapphire gate at the third vault’s room entrance. Classes, colors and treasure carry forward with health/potion recovery. After the third vault, victory returns to the library.

In the lobby, **A / Cross / Wii 2** joins, then readies your hero after release; left/right or **X / Square / Wii 1** changes class and clears readiness. Once everyone is ready, press **A / Cross / Wii 2 again** to enter; **Menu / Options / Wii + / Home** also starts. Press **Down** for the lobby buttons, then left/right and confirm to open controls, toggle fullscreen, enter, or return to the library. Keyboard: **Enter** joins/readies, **Tab** or left/right chooses a hero, and **P** starts. You can also use each player's on-screen controls and the **Enter the vault** button. Each player can choose from **16 colors**: press **Up**, use left/right, and confirm to return to the hero card; keyboard **C** or clicking the color swatch cycles colors. Colors appear on clothing, shields, player numbers, floor markers, minimap dots, HUD bars, and magic rings. Class changes and returning to the lobby keep the color; changing color clears readiness.

In play, move with the stick / D-pad / WASD, release the menu confirmation, then hold **A / Cross / Wii 2 / Space** to attack, and use **X / Square / Wii 1** (keyboard **X**) for magic. Only joined players have compact health/magic entries; the minimap sits in the header outside the expanded dungeon view. **Esc / Menu / Options / Wii + / Home / P** opens the pause menu for camera, minimap, lighting, sound, controls, returning to the hero lobby, or exiting to the library. Leaving an active run asks you to confirm; returning to the lobby retains connected players.

Audio includes recorded class-specific hurt reactions (three takes per class), short softer enemy reactions, layered combat effects, a music loop for each vault, and recorded class-selection lines (“Ready!”, “Prepare yourself!”, “Go!”). The CC0 recordings come from Kenney (lobby), HaelDB and AuraVoice (hurt), with sources and credits included. All voices currently use the regular speakers: Wii speaker streaming was disabled after physical testing produced choppy audio and controller disconnects. Actual hero hits request a 120 ms controller rumble; passive drain and invulnerable contacts do not. The pause menu has independent **Sound effects**, **Music**, and **Controller rumble** switches. Native Wii rumble was not felt during physical testing; gamepad vibration still needs physical confirmation.

Each class now attacks with its equipped weapon: Warrior uses slow, heavy axe cleaves; Valkyrie uses quicker sword strikes; Elf fires modeled arrows; Wizard casts blue magic bolts. Melee has a short windup and cannot hit through walls or closed doors. Idle poses stay quiet, and entering/resuming requires releasing and pressing attack again. Potion damage lands when the expanding ring reaches each enemy or generator, once per target per blast. Real hits give a short flinch, soft sound, and restrained color flash. Enemy health bars appear only after damage, hold briefly, then fade; their length and green-to-red tint show remaining health.

Level-one combat routes and all three levels’ one-player/sixteen-player key-and-collision routes pass automated checks; later-level combat balance and physical multiplayer testing remain pending. See [Gauntlet controls, level guide, and verification](sdk/examples/gauntlet/README.md).

## Play Little World

```sh
python3 scripts/play.py
```

The script uses `couch doctor` to find a supported installed Godot, imports the project, and opens the 3D game. It does not download an engine, export templates, or art assets. Alternatively, open `sdk/project.godot` in the supported editor and press F6 on `examples/little_world/world.tscn`, or F5 for the project.

- Pair wireless Xbox or PlayStation controllers to the **computer**, then press **A / Cross** to join.
- Use the **left stick or D-pad** to move; **A / Cross** jumps. Hold **B / Circle** for 1.25 seconds to leave.
- Keyboard: **Enter** joins, **WASD / arrows** move, **Space** jumps, and **Backspace** leaves. The keyboard consumes one player slot.
- The game opens maximized with a 16:9 layout. **F11**, Xbox **Menu (☰)**, PlayStation **Options**, or the on-screen button toggles fullscreen; **F3** shows controller diagnostics. Fullscreen returns to the previous window mode.
- For a compatible Roku TV, use macOS AirPlay to share just the game window. See [TV display and window sharing](docs/tv-display.md).
- Disconnecting a controller removes its character and frees the slot immediately. Reconnect and press A / Cross once to join again; other players keep their characters.

For experimental Wii Remote/Remote Plus, Nunchuk, Classic/Classic Pro, and Wii U Pro profiles, run `python3 scripts/play.py --wii`. F3 includes per-device layout selection. Physical Wii pairing and play remain unverified; see [Wii setup and coverage](docs/wii-controllers.md).

The tested `Nintendo RVL-CNT-01` variant (`04e8:7021`) needs our native macOS reader. After closing the current game and standalone probe, use **`python3 scripts/play_wii_native.py`**. It builds the small reader with the existing Xcode compiler if needed, launches the reader and game together, and cleans up the reader on exit. One Wii Remote was physically verified for movement/jumping; other Wii variants and multiple native Remotes remain unverified. The Little World native path supports one Remote alongside the regular SDK player slots. Pocket Rally has a separate multi-Remote helper.

Wired DualShock 4 / DualSense / Xbox HID and Xbox Bluetooth pads join through Godot. Vendor-class Xbox 360 XID (`ff:5d:01`) and Xbox One GIP (`ff:47:d0`) USB pads need the host helper. **`python3 scripts/play.py`** starts it when such a pad is plugged in. Keep the **wired USB reader** window open. Press **A** to join. For an unknown USB pad, `python3 scripts/play_xpad_native.py --dump` prints a catalog row an agent can add. See [wired USB controllers](docs/wired-usb-controllers.md).


See [the game README](sdk/examples/little_world/README.md) and [wireless setup and hardware tests](docs/controller-test-matrix.md). The software supports sixteen slots. Physical wireless compatibility and simultaneous controller counts still need testing on the listed hardware.

## Play Cloudbound — Wii flying game

```sh
python3 scripts/play_wii_native.py --game cloudbound
```

A second **3D source game**, sharing the installed runtime. Hold the Wii Remote **sideways, D-pad left and 1/2 buttons right**. Press **2**, hold steady for one second, then turn left/right to bank. **Roll away from your body to dive; toward yourself to climb.** **B** boosts; **2** recenters. Fly through the golden rings.

The experimental native reader now requests acceleration as well as buttons for the observed `04e8:7021` variant. Live motion status, neutral calibration, and pause on stale input are built in. Sideways Wii flight, live acceleration, and factory calibration are physically confirmed on this Mac. Close the old standalone probe/game before starting this launcher.

For original Switch Joy-Con / Pro motion or keyboard/controller flight: `python3 scripts/play.py --game cloudbound`. Individual Joy-Cons are separate by default; add `--joycons paired` for a grip. Motion uses actual driver capabilities, with an explicit stick fallback. Switch hardware tests are pending; Switch 2 has profiles but needs a compatible driver not included in the pinned runtime. See [Switch coverage and setup](docs/switch-controllers.md). See [Cloudbound controls and verification](sdk/examples/cloudbound/README.md).

## Play Pocket Rally — split-screen driving

```sh
python3 scripts/play_wii_native.py --game pocket-rally
```

A separate **1–16 player 3D driving game**, with a chase camera for every car. Two players get side-by-side views; four get a 2×2 grid; sixteen get a 4×4 grid. Hold the Wii sideways, press **2**, and hold steady for one second. **Tilt steers, 2 accelerates, 1 brakes/reverses, Home rescues/recalibrates, and holding minus leaves.** Each controller owns its car and calibration. Disconnecting removes its car and view.

For keyboard or regular controllers: `python3 scripts/play.py --game pocket-rally`. Enter joins; WASD/arrows drive. Xbox A / PlayStation Cross joins and accelerates; left stick steers, X / Square brakes. F11 toggles fullscreen.

Pocket Rally uses a separate native helper with sixteen independent channels for the observed Wii variant. Physical multi-Remote and sixteen-player performance tests remain pending. Cloudbound's game and existing reader are preserved. See [Pocket Rally controls and verification](sdk/examples/pocket_rally/README.md).

## Play Super Mario Bros. — World 1-1

Choose its card in the library, or run `python3 scripts/play_wii_native.py --game world-1-1` on this Mac. For ordinary gamepads and keyboard: `python3 scripts/play.py --game world-1-1`.

The single-player recreation uses original NES sprite captures and the World 1-1 layout, with enemies, power-ups, hidden 1-up, the underground coin room, and the flagpole/castle finish. Wii: sideways D-pad moves, **2 jumps**, **1 runs/fires**; hold jump for height. Xbox/PlayStation: A/Cross jumps, B/Circle runs. Keyboard: arrows + Space/Shift. Esc returns to the library.

The gameplay is recreated, not frame-perfect NES emulation. Synthesized effects replace the original audio; music and fireworks are absent. See [controls, asset credits, tests, and fidelity limits](sdk/examples/world_1_1/README.md).

## Prerequisites

Use an existing current stable Rust toolchain with Cargo. The workspace uses Rust edition 2024. Builds fetch Rust crates as needed and compile a bundled SQLite library; no separate SQLite server or installation is necessary.

Development and test builds disable debug symbols and incremental compilation to save disk space. No build script downloads Godot or export templates. Once finished testing, `cargo clean` removes this checkout's generated build artifacts; a later Cargo command rebuilds them.

## Try the CLI

From the repository root:

```sh
cargo run --locked -p couch-cli -- doctor
cargo run --locked -p couch-cli -- validate tests/fixtures/package/release.json
cargo run --locked -p couch-cli -- --data-dir .local-demo install tests/fixtures/package/release.json --target macos-aarch64
cargo run --locked -p couch-cli -- --data-dir .local-demo library
```

The fixture contains three tiny text files with `.pck` filenames. **They are synthetic test data, not real Godot games.** This walkthrough only demonstrates metadata checks, file copying, and the SQLite library. It creates a small `.local-demo/` directory, ignored by Git.

The explicit target makes this content-only demo work on any development OS. Real imports default to the current computer's target. Provisional targets are `windows-x86_64`, `macos-x86_64`, and `macos-aarch64`; runtime/hardware support has not yet been verified.

`doctor` is read-only and does not create the data directory. `init` copies a template project and writes `creator-projects.json` under the data directory. `install` and `library` initialize their data directory and SQLite database. Without `--data-dir`, they use the OS's per-user application-data location. No Neon or other cloud connection is made. None of these commands install Godot or run a game.

### Commands

| Command | Behavior |
| --- | --- |
| `doctor [--require-godot] [--project <path>]` | Detect Godot and optionally inspect a project without launching Godot; `--require-godot` fails if no supported editor is found |
| `init <title> --parent <dir> [--template <path>]` | Copy a template into a new project folder and register it for Giga Couch |
| `validate <release.json>` | Validate metadata and all declared artifacts |
| `install <release.json> [--target <target>]` | Verify and import the selected target as unsigned local content |
| `library` | List installed releases; `*` marks the active release |

Global options: `--data-dir <path>`, `--godot <executable-or-app>`, `--godot-timeout-secs <1–120>`, `--json`, `--help`, and `--version`.

## Godot setup and SDK checks

The initial supported development engine is **Godot 4.7.2 stable, standard official build**. The shared policy is in [runtime_policy.json](sdk/addons/couchgames/runtime_policy.json).

```sh
cargo run --locked -p couch-cli -- doctor --require-godot
python3 scripts/test_sdk.py
```

Discovery checks `--godot` first, then `COUCH_GODOT`, then executables on `PATH` and common application locations (including `/Applications` and `~/Applications` on macOS). An explicit path is authoritative: if it fails, the tool reports the problem instead of silently choosing another installation. Auto-detection can skip an old/broken candidate and find a supported one.

If Godot is missing, unusable, or unsupported, the report includes the pinned official download page and setup instructions. `--require-godot` exits 1 with a `GODOT_NOT_READY` error while preserving the report in JSON `data`. Ordinary `doctor` still succeeds so package-only development can continue. Each executable probe has a 15-second timeout and bounded output; no shell command is constructed from the executable path. For a slow first launch, open Godot normally to complete OS checks, then retry; `--godot-timeout-secs 60` can allow more time.

The [SDK README](sdk/README.md) explains the editor status panel and `Platform.is_runtime_supported()`. The SDK checks the engine it is running inside; missing-engine detection necessarily happens in the Rust host before Godot starts. The SDK check script dogfoods the host's selected executable and never downloads an engine. Export templates are unnecessary for these checks.

Example automation:

```sh
cargo run --quiet --locked -p couch-cli -- --json validate tests/fixtures/package/release.json
```

Operational results use one stdout JSON envelope:

```json
{"ok":true,"data":{"game_id":"example.fixture","release_id":"release-001","artifacts_verified":3,"scope":"metadata_and_content_integrity","playability_verified":false,"signature_verified":false}}
```

Operational failures return `{"ok":false,"error":{"code":"...","message":"..."}}` and exit status 1. CLI usage errors/help are handled by Clap and use its standard text output (usage errors exit 2). Cargo itself may print build diagnostics to stderr; a built `couch` executable has no Cargo wrapper output.

### Import guarantees and limits

- Manifest JSON is capped at 64 KiB; each artifact is limited to 4 GiB in the provisional contract.
- Artifact paths are portable lowercase basenames. Traversal, absolute paths, symlinks, directories, and reserved Windows device names are rejected.
- Bytes are verified while copying to staging before changing the active library entry.
- Retrying an identical installed release does not create a duplicate or roll back a newer active release.
- Reusing a release ID with changed metadata fails. Make a new release ID for an update.
- A failed import leaves the previous active database entry unchanged. A retry can reuse matching orphaned content left between rename and database commit.
- Old releases and save directories are preserved. Save/load APIs and garbage collection are not implemented yet.
- The library directory is assumed to be controlled by the current user. These checks do not sandbox hostile processes running under that same account.
- Recovery from sudden power loss, proactive free-space checks, abandoned staging cleanup, signed packages, and automatic repair of missing installed content remain future work. `library` lists database state; it does not rehash every installed file.

## Repository

```text
apps/cli/                  couch executable
crates/manifests/          manifest model, semantic checks, streaming verification
crates/local-library/      SQLite persistence and staged local import
crates/runtime/            discovery, version checks, and setup reports for the host
sdk/                       GDScript addon, shared runtime policy, engine test harness
sdk/examples/little_world/ playable 3D source sample using the SDK
scripts/play.py             launch sample through platform runtime discovery
scripts/test_sdk.py         explicit SDK checks using our own Godot discovery
migrations/sqlite/         embedded, checksummed SQLx migrations
schemas/                   JSON Schema for creator/agent tooling
tests/fixtures/package/    tiny non-playable package fixture
.github/workflows/         manually dispatched Rust checks; no automatic triggers
```

Schema validation is useful for editor feedback. The Rust validator also enforces cross-field rules such as player-count ordering and unique artifact targets/filenames, and verifies actual content bytes.

## Checks

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Rust tests use temporary directories, synthetic content, and test executables. They do not install Godot, run games, need a database server, or access cloud services. The manually dispatched GitHub Actions workflow runs the same checks on Windows, macOS, and Linux; Linux is a development/test environment, not a declared V1 game target. `scripts/test_sdk.py` is a separate explicit check that runs GDScript in an already-installed supported engine.

The source sample has rendered and passed scene/physics checks on macOS ARM64. Shared-PCK runtime execution, other graphics/OS targets, physical controller hardware, and sandbox validation remain required before distribution is ready.

GitHub Actions runs only through **workflow_dispatch** (manual invocation). Pushes and pull requests do not start workflows.

## Blender art authoring

Editable models and props live in [`art/`](art/README.md), with GLB exports for Godot. The first Blender study includes a treasure chest, jade potion flask and ember brazier. Blender is a development tool; players do not need it.

Gauntlet's latest art pass adds detailed Blender props and beveled dungeon masonry, varied floor motifs, articulated enemy movement and an authored Warrior axe cleave with matching visible contact and damage. Temporary fog follows all players (seven clear tiles plus a three-tile fade), including fallen teammates; visibility depends only on distance, including through walls and doors, and the minimap follows the same rules. Previously visited rooms become hidden again after the party leaves. [Art and visibility details](docs/gauntlet-art.md#dungeon-polish-and-temporary-party-fog).
