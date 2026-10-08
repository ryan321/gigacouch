# GigaCouch Runtime Architecture

> **V1 changed on October 8, 2026.** V1 is a website for uploading, browsing, and playing web games in Chrome. See [docs/v1.md](v1.md). Where this document describes V1 differently, such as Godot first, an installed app, or phones as controllers, docs/v1.md wins. Those parts describe research and later stages.

**Status:** Proposed architecture for implementation  
**Audience:** AI coding agents, engineers, and technical contributors  
**Updated:** September 22, 2026

---

## 1. Purpose

GigaCouch is a controller-first gaming platform for AI-created games played on a TV through a user's own computer.

The platform should support two first-class execution models:

1. **GigaCouch Web Runtime** — the default, sandboxed, engine-independent runtime.
2. **GigaCouch Godot Runtime** — a managed native runtime for Godot games that need higher performance or deeper engine capabilities.

The platform should not require every game to be created with one engine.

The guiding principle is:

> **GigaCouch has two first-class runtimes: Web for lightweight, agent-native games and Godot for full native game-engine capability.**

Many GigaCouch creators are expected to create games almost entirely through AI agents. The creator chooses the game they want; the agent should choose the simplest appropriate implementation.

A creator should not need to understand or select a game engine in order to make a game.

---

# 2. Product-Level Architecture

```text
                         GIGACOUCH
                             │
                  Controller-first launcher
                             │
                     GigaCouch Host
                     (native process)
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
     Platform APIs      Local services      Online services
          │                  │                  │
          │             controllers         accounts
          │             saves               friends
          │             LAN                 sharing
          │             profiles            publishing
          │             lifecycle           marketplace
          │                  │                  │
          └──────────────────┼──────────────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
        GigaCouch Web Runtime      GigaCouch Godot Runtime
                │                         │
        sandboxed web games          native Godot games
                │                         │
       Three.js / Babylon.js            Godot PCK/project
       Phaser / PixiJS                   managed runtime
       JS / WASM / WebGPU
       Unity Web / Godot Web
       other web-capable engines
```

The native GigaCouch Host owns privileged functionality.

Games should not directly own accounts, filesystem access, controller identity, marketplace credentials, or unrestricted operating-system access.

---

# 3. Core Architectural Principles

## 3.1 GigaCouch is the platform, not the game engine

GigaCouch should not build:

- a rendering engine
- a physics engine
- a proprietary game editor
- a proprietary programming language
- a required AI creation system

Creators should be able to use real game engines and the AI tools of their choice.

---

## 3.2 Web and Godot are both first-class runtimes

Do not treat the Web Runtime as merely a compatibility layer for games exported from traditional engines.

A major expected GigaCouch workflow is:

```text
Creator describes a game
        ↓
AI coding agent
        ↓
Three.js / Babylon.js / Phaser / PixiJS /
JavaScript / TypeScript / WebAssembly
        ↓
GigaCouch Web Runtime
        ↓
Play
```

For many party games, board games, arena games, 2D games, lightweight 3D games, personal games, and prototypes, creating the game directly with web technologies may be the fastest and simplest route.

The Web Runtime provides:

- direct AI-agent creation without a traditional game engine
- strong sandboxing
- easy packaging
- predictable distribution
- offline installation
- cross-platform compatibility
- simple certification
- support for JavaScript, TypeScript, WebAssembly, WebGL, WebGPU, and compatible libraries/frameworks
- compatibility with web exports from engines such as Unity and Godot when useful

Godot is equally first-class when the game benefits from a full game engine.

---

## 3.3 The creator chooses the game; the agent chooses the runtime

Ordinary creators should not be asked to choose between Web, Godot, Unity, Unreal, or other implementation technologies before creating a game.

The expected creator interaction is:

```text
"What do you want to make?"
        ↓
creator describes the game
        ↓
AI agent evaluates requirements
        ↓
Web or Godot
        ↓
build / test / play
```

The agent should generally prefer **Web** when:

- the game can be implemented cleanly with browser technologies
- fast creation and iteration are especially valuable
- the game is 2D or moderate 3D
- gameplay is relatively self-contained
- browser graphics/CPU capabilities are sufficient
- a traditional engine would add unnecessary project complexity

The agent should generally prefer **Godot Native** when:

- advanced physics, navigation, animation, or scene tooling are useful
- the game has substantial 3D complexity
- native rendering or CPU performance matters
- the project benefits from Godot's scene/node model
- the game is expected to grow significantly
- required functionality is unavailable or impractical in the Web Runtime

These are guidance, not rigid genre rules. Agents should choose the simplest runtime that meets the game's actual requirements.

---

## 3.4 Keep Godot as the recommended native path

Godot remains strategically important because:

- it is open source
- it has no revenue-based licensing requirement
- AI agents can modify normal project files directly
- GDScript is easy for AI agents to generate and understand
- a managed shared runtime can reduce game package size
- it provides substantially more native capability than the Web Runtime
- creators retain normal Godot projects outside GigaCouch

GigaCouch should optimize deeply for Godot without requiring it.

---

## 3.5 Platform capabilities live above runtimes

Do not implement separate product concepts for Web and Godot.

There should be one conceptual GigaCouch API with multiple bindings.

Example:

```text
GigaCouch.players
GigaCouch.input
GigaCouch.save
GigaCouch.network
GigaCouch.lifecycle
GigaCouch.achievements
```

Bindings may include:

```text
JavaScript / TypeScript  -> Web Runtime
GDScript                 -> Godot Runtime
```

Future bindings may include C#, C++, or other languages.

The semantics should remain as consistent as practical.

---

# 4. Native GigaCouch Host

The GigaCouch Host is the trusted native application installed on the player's computer.

The initial recommendation remains:

```text
Rust + Tokio
```

The host owns:

- launcher lifecycle
- game process/runtime lifecycle
- local database writes
- installed game metadata
- downloads and updates
- runtime management
- game package verification
- controller discovery
- player/controller assignment
- local saves
- LAN services
- online API authentication
- credential storage
- platform permissions
- crash recovery
- logs and diagnostics
- communication with running games

Games communicate with the host through restricted runtime-specific bridges.

---

# 5. GigaCouch Web Runtime

## 5.1 Definition

The GigaCouch Web Runtime is a dedicated game browser environment embedded in or launched by the GigaCouch application.

It should not behave like a normal user-facing browser.

The player should not see:

- address bars
- browser tabs
- bookmarks
- ordinary browser navigation
- arbitrary website access
- browser downloads UI

The user experience should feel like a console.

---

## 5.2 Runtime engine

The preferred investigation path is a **bundled Chromium-based runtime**.

Possible implementation technologies include:

- Chromium Embedded Framework
- Electron for early prototypes
- another embeddable Chromium distribution

Do not permanently depend on the user's installed Chrome, Firefox, Safari, or Edge version.

The goal is a versioned runtime such as:

```text
GigaCouch Web Runtime 1
GigaCouch Web Runtime 2
```

Each runtime ID should map to a known:

- Chromium build
- WebAssembly feature set
- WebGL/WebGPU support level
- JavaScript environment
- GigaCouch Web API version
- sandbox configuration
- supported OS/architecture matrix

Games declare the runtime they require.

---

## 5.3 Supported game sources

The Web Runtime should be engine-independent.

Potential sources include:

```text
Three.js
Babylon.js
Phaser
PixiJS
PlayCanvas
custom JavaScript / TypeScript
custom WebAssembly
WebGL / WebGPU applications
Unity Web build
Godot Web export
future browser-capable engines
```

Direct AI-authored web games are a primary use case, not an edge case.

An AI agent may create a complete GigaCouch game directly as HTML, JavaScript/TypeScript, assets, shaders, and WebAssembly without using Godot, Unity, Unreal, or another traditional game engine.

The platform should care about the resulting runtime package and GigaCouch API compatibility, not which tool generated it.

---

## 5.4 Web package contract

A minimal package may look like:

```text
MyGame.giga/
    gigacouch.json
    web/
        index.html
        game.wasm
        game.js
        game.data
        assets/
```

Actual filenames inside `web/` are engine-specific.

GigaCouch only requires a valid entry point and manifest.

Example conceptual manifest:

```json
{
  "manifest_version": 1,
  "game_id": "creator.example-game",
  "version": "0.1.0",
  "runtime": "web-1",
  "entrypoint": "web/index.html",
  "gigacouch_api": "1",
  "players": {
    "min": 1,
    "max": 16
  }
}
```

The finalized manifest must additionally include package hashes, sizes, permissions, supported OS requirements, and signed release metadata.

---

## 5.5 Local operation

Web games do not require an Internet-hosted website.

A downloaded game should run entirely from local storage.

Conceptually:

```text
GigaCouch Host
      ↓
local game origin/server
      ↓
embedded Chromium
      ↓
installed game files
```

Do not rely on `file://` behavior.

The host should expose each installed game through a controlled local origin or equivalent embedded resource mechanism.

Installed games should remain playable offline unless the game itself explicitly requires online services.

---

# 6. Web-to-Native Bridge

The Web Runtime must not expose unrestricted native functionality.

Instead, GigaCouch injects or exposes a small, versioned bridge.

Conceptual JavaScript API:

```javascript
GigaCouch.players.list()

GigaCouch.input.action(playerId, "jump")
GigaCouch.input.axis(playerId, "move")

await GigaCouch.save.write("campaign", data)
await GigaCouch.save.read("campaign")

GigaCouch.lifecycle.quit()

GigaCouch.network.createLobby(...)
GigaCouch.network.send(...)

GigaCouch.achievements.unlock("winner")
```

The bridge forwards authorized requests to the native host.

```text
Web game
   ↓
GigaCouch JS bridge
   ↓
restricted IPC
   ↓
Rust host
   ↓
native operating-system capability
```

The web game must never receive general native-process access.

---

# 7. Controller Architecture

Controller support is a platform primitive.

The preferred design is:

```text
Physical controllers
        ↓
Native GigaCouch input layer
        ↓
Player assignment
        ↓
Semantic actions
        ↓
Game runtime
```

Games should work with players and actions rather than raw hardware IDs.

Conceptual API:

```text
input.action(player_id, "jump")
input.action(player_id, "primary_action")
input.axis(player_id, "move")
input.glyph(player_id, "jump")
```

The host should eventually support:

- Xbox controllers
- PlayStation controllers
- 8BitDo and common PC controllers
- USB
- Bluetooth
- connect/disconnect
- reconnect/reclaim
- player assignment
- dead zones
- remapping
- analog input
- controller-specific glyphs
- haptics where supported
- large local player counts

The architecture should not assume four players.

The platform should be designed so **1–16 local players** is a normal supported range where the game itself supports it.

Game rendering cost is separate from controller count.

---

# 8. 16-Player Local Multiplayer

Sixteen local players should be supported as a platform capability.

Examples that are technically reasonable:

- trivia
- party games
- board games
- shared-camera arena games
- 2D games
- top-down games
- turn-based games
- social deduction games
- lightweight shared-world 3D

The platform should not promise that every game can render 16 simultaneous split-screen 3D cameras.

Player count and rendering workload are separate constraints.

The GigaCouch runtime should impose no arbitrary four-player ceiling.

---

# 9. LAN and Networking

## 9.1 Browser limitation

Web games cannot be expected to open arbitrary native TCP/UDP sockets.

Do not make game creators solve this themselves.

---

## 9.2 GigaCouch networking service

The native host should provide LAN and later online networking services.

Conceptually:

```text
Web or Godot game
        ↓
GigaCouch networking API
        ↓
Native host
        ↓
LAN / Internet transport
```

Possible native transports may include:

- UDP
- QUIC
- TCP
- WebRTC where useful
- platform relay services
- local discovery protocols

Exact transports are implementation decisions.

The public API should hide them where possible.

Potential API:

```text
network.create_session(max_players)
network.find_sessions()
network.join_session(id)
network.send(peer, channel, payload)
network.broadcast(channel, payload)
```

Later APIs may include replication or higher-level multiplayer primitives.

---

# 10. Godot Native Runtime

The native Godot runtime remains a first-class GigaCouch target.

Conceptually:

```text
Godot project
      ↓
GigaCouch Godot SDK
      ↓
packaged game content
      ↓
managed GigaCouch Godot Runtime
      ↓
native process
```

Compatible games should reuse a shared installed Godot runtime when feasible.

Example:

```text
GigaCouch/
    runtimes/
        godot-1/
        web-1/
    games/
        ...
```

A Godot runtime ID identifies a tested combination of:

- exact Godot build
- renderer policy
- export settings
- supported OS/CPU architectures
- GigaCouch SDK compatibility

---

# 11. Why Native Godot Exists

AI agents should choose native Godot when the game requires or clearly benefits from capabilities such as:

- higher CPU performance
- higher-end rendering
- complex physics
- large worlds
- engine features unavailable in Web export
- native Godot extensions approved by GigaCouch
- functionality unsupported by browser APIs
- workloads that exceed practical WebAssembly/browser limits

The Web Runtime is the lightweight, highly portable, agent-native path.

Godot Native is the full-engine performance/capability path.

Neither runtime is a fallback. Both are core GigaCouch targets.

---

# 12. Godot Dual-Target Workflow

A Godot project may support both runtimes.

```text
                    Godot project
                         │
              ┌──────────┴──────────┐
              │                     │
          Web export           Native package
              │                     │
     GigaCouch Web Runtime   GigaCouch Godot Runtime
```

A creator may begin with Web and move to Native if needed.

The project should not need to change engines simply because its performance requirements increase.

---

# 13. Unity Strategy

Unity is not required for the core GigaCouch creator workflow.

Most creators are expected to use AI agents that either create Web games directly or create Godot games. Existing Unity projects can still enter through the Web Runtime.

The initial Unity path should be:

```text
Unity project
     ↓
Unity Web build
     ↓
GigaCouch Web SDK
     ↓
GigaCouch Web Runtime
```

This gives GigaCouch Unity compatibility without requiring the platform to manage arbitrary native Unity executables initially.

Native Unity support may be revisited later if demand justifies the security, packaging, certification, and runtime complexity.

---

# 14. Unreal Strategy

Do not make Unreal a V1 runtime requirement.

Modern Unreal does not currently provide an equivalent mainstream official Web export path suitable for treating Unreal like Unity Web.

High-end Unreal support would therefore most likely require a future native runtime model.

Possible future architecture:

```text
Unreal native game
       ↓
GigaCouch native-game contract
       ↓
platform APIs / certification
```

This is deferred.

---

# 15. Shared Platform API

The GigaCouch API should be defined independently of any specific engine.

Initial modules:

```text
GigaCouch.input
GigaCouch.players
GigaCouch.ui
GigaCouch.lifecycle
GigaCouch.save
GigaCouch.network
GigaCouch.platform
```

Later modules may include:

```text
GigaCouch.accounts
GigaCouch.friends
GigaCouch.invites
GigaCouch.achievements
GigaCouch.leaderboards
GigaCouch.cloud
GigaCouch.voice
GigaCouch.commerce
```

Every public API should have:

- explicit versioning
- machine-readable schema/documentation
- stable error codes
- clear permission model
- examples for AI agents
- conformance tests
- small, predictable method names

---

# 16. AI-Agent Requirements

GigaCouch is explicitly designed for creators using AI coding agents.

The expected creator often cares about the game idea, not the underlying technology. AI agents should be capable of deciding whether Web or Godot is the better implementation target.

Therefore:

1. APIs must be easy to infer from documentation.
2. Documentation must contain complete working examples.
3. Error messages should explain how to correct the problem.
4. CLI commands should support structured output.
5. Schemas should be machine-readable.
6. Starter projects should include an `AGENTS.md`.
7. SDKs should avoid unnecessary abstraction and hidden magic.
8. Runtime differences should be documented clearly.
9. Validation failures should identify exact files, scenes, settings, or APIs involved.
10. An agent should be able to make an existing compatible project GigaCouch-ready without specialized platform knowledge.
11. Agent documentation must include runtime-selection guidance.
12. Agents should be able to create a Web game directly without requiring a traditional engine project.
13. Agents should be able to create a normal Godot project when the game benefits from a full engine.
14. Agents must not force creators to understand runtime choices unless a meaningful tradeoff requires user input.

Likely creator instructions should eventually be sufficient:

> "Make this game compatible with GigaCouch."

or simply:

> "Make a 10-player game where everyone is a blob trying to knock each other off an island."

---

# 17. Security Model

## 17.1 Web Runtime

Web should be the default marketplace runtime partly because browser sandboxing naturally restricts:

- arbitrary filesystem access
- arbitrary native process execution
- arbitrary DLL/native-library loading
- raw native sockets
- unrestricted OS APIs

GigaCouch should expose only approved capabilities through its bridge.

Web games should run with:

- no Node.js/native runtime access
- isolated per-game storage
- isolated origins where practical
- restrictive navigation policy
- restrictive permissions
- bounded memory/storage policies where practical
- no access to GigaCouch credentials
- no arbitrary host IPC

---

## 17.2 Native Godot

Native Godot code is more powerful and therefore requires stronger certification.

Signing alone is not a security boundary.

Native distribution requires a proven isolation strategy or appropriately restricted trust model.

Potential platform-specific sandboxing remains an investigation item.

Until robust native isolation is proven, public marketplace policies may differ between:

```text
LOCAL
PRIVATE
WEB VERIFIED
NATIVE VERIFIED
PUBLIC
MARKETPLACE
```

Exact names are not decided.

---

# 18. Saves

Games should not own platform database access.

Use a platform save API.

```text
Game
  ↓
GigaCouch.save
  ↓
Host
  ↓
per-profile / per-game save storage
```

Requirements:

- local-first
- offline-capable
- atomic writes where possible
- versioned payloads
- recoverable prior version
- preserved across updates
- preserved across uninstall by default
- future-compatible with cloud synchronization

Web and Godot games should see conceptually identical save behavior.

---

# 19. Local Data

SQLite remains a good local platform datastore.

The host is the only process that directly owns the GigaCouch library database.

Possible tables include:

```text
local_profiles
installed_games
installed_releases
installed_runtimes
download_jobs
library_cache
controller_preferences
settings
```

Game saves should remain separate from platform metadata.

---

# 20. Online Backend

The existing backend direction remains valid:

```text
Rust API
PostgreSQL / Neon
object storage
managed identity provider
```

Online services support:

- accounts
- families
- sharing
- game metadata
- release metadata
- publishing
- authorization
- discovery
- marketplace
- future cloud services

Installed local games should not require constant backend connectivity.

---

# 21. Runtime Selection

A release manifest chooses its runtime.

Examples:

```json
{
  "runtime": "web-1"
}
```

or:

```json
{
  "runtime": "godot-1"
}
```

Future runtimes might include:

```text
web-2
godot-2
native-unity-1
native-unreal-1
```

Do not add a runtime merely because an engine exists.

Add a runtime only when it provides meaningful capabilities that cannot be handled cleanly through existing targets.

---

# 22. Runtime Versioning

Runtime releases are immutable.

A game targets an exact compatibility runtime ID, not a floating `latest`.

The platform may install multiple runtime versions side by side.

```text
runtimes/
    web-1/
    web-2/
    godot-1/
    godot-2/
```

Games sharing the same runtime reuse it.

Runtime updates must support:

- side-by-side installation
- compatibility testing
- deliberate activation
- rollback
- security patch policy
- cleanup only after no installed game depends on a runtime

---

# 23. Launcher

The launcher must be fully controller-operable.

It should handle:

- library browsing
- game launch
- downloads
- updates
- profile selection
- controller joining
- settings
- error dialogs
- invitations
- future marketplace browsing

The launcher may itself use Web technology, Godot UI, or another appropriate UI stack.

Its implementation technology is less important than the controller-first behavior.

Do not make the game runtime depend on the launcher UI technology.

---

# 24. Performance Policy

Do not define Web as "low quality."

Web should support serious games where the browser environment is sufficient.

Target categories include:

- 2D games
- 3D party games
- racing
- arena games
- strategy
- board games
- platformers
- social games
- moderate 3D adventures
- many 1–16 player couch games

Native Godot should be available when the game's needs exceed Web.

Resolution and rendering complexity should be treated separately.

A 4K display does not require a game to render internally at native 4K.

Dynamic or fixed internal render resolutions may be used and upscaled for TV output.

---

# 25. Certification

Publishing should include automated runtime-specific checks.

Common checks:

```text
game launches
manifest valid
runtime supported
controller-only flow works
main menu reachable
pause works
quit-to-platform works
save API works
package paths valid
TV-safe UI requirements pass
declared player count valid
```

Web-specific checks may include:

```text
no forbidden navigation
no unsupported browser APIs
no prohibited permissions
no Node/native escape
offline package completeness
resource loading succeeds from local origin
```

Godot-native checks may include:

```text
approved runtime target
approved extensions
restricted OS functionality
process/network policy
sandbox compatibility
```

Certification output must be easy for AI agents to understand and fix.

---

# 26. Repository Direction

A possible revised monorepo structure:

```text
PRODUCT.md
ARCHITECTURE.md
TECH_STACK.md

Cargo.toml

apps/
    desktop-host/
    launcher/
    api/
    worker/
    cli/

runtimes/
    web/
        bridge/
        harness/
    godot/
        runner/

sdk/
    web/
    godot/

crates/
    protocol/
    manifests/
    installer/
    api-client/
    runtime-manager/

schemas/
    manifest.schema.json
    protocol.schema.json

examples/
    web-demo/
    godot-demo/
    sixteen-player-input-demo/

tests/
    runtime/
    integration/
    fixtures/

docs/
    agents/
    sdk/
    runtimes/
    security/
```

Exact structure may change.

---

# 27. Recommended V1 Sequence

## Milestone 0A — Web feasibility

Have an AI coding agent create a tiny controller-based Web game directly with a lightweight web stack such as Three.js, Babylon.js, Phaser, or plain JavaScript, then run it inside a bundled/embedded Chromium environment.

This milestone must prove that a creator does not need Godot, Unity, or Unreal to create a real GigaCouch game.

Prove:

- Windows
- macOS
- hardware-accelerated rendering
- gamepad input
- fullscreen TV output
- local/offline loading
- JS ↔ Rust messaging
- game exit back to launcher
- sandbox restrictions

---

## Milestone 0B — Godot native feasibility

Retain the existing native Godot prototype goal.

Prove:

- one managed Godot runtime
- multiple independent games
- PCK/project launch
- controller input
- saves
- crash recovery
- return to launcher

---

## Milestone 1 — Shared platform APIs

Implement the common conceptual API:

```text
players
input
lifecycle
save
```

Provide:

- Web binding
- Godot binding

Build equivalent sample games using both runtimes.

---

## Milestone 2 — Local platform

Implement:

- library
- runtime manager
- install/uninstall
- local profiles
- SQLite
- downloads
- update recovery
- controller-only launcher

---

## Milestone 3 — LAN

Add host-managed LAN discovery/session APIs.

Build at least one game that demonstrates multiple GigaCouch computers communicating without requiring the game to open raw sockets.

---

## Milestone 4 — Private distribution

Implement:

- authentication
- backend API
- private object storage
- publishing
- validation
- signed releases
- invitations
- authorized downloads

Web distribution should be the first public/private remote distribution path unless native sandboxing is already proven.

---

# 28. Decisions That Are Currently Made

Treat these as current architecture decisions unless explicitly changed:

1. GigaCouch runs on the user's own PC or Mac and displays on a TV.
2. GigaCouch is controller-first.
3. Local multiplayer is a first-class capability.
4. The architecture should support up to 16 local players where the game supports it.
5. GigaCouch should not build its own game engine.
6. AI creation tools remain external / bring-your-own-AI.
7. Web and Godot Native are both first-class runtime targets.
8. Direct AI-authored Web games using technologies such as Three.js, Babylon.js, Phaser, JavaScript/TypeScript, WebAssembly, WebGL, and WebGPU are a primary GigaCouch creation path.
9. GigaCouch should use a dedicated controlled browser runtime rather than depend on the user's browser.
10. Godot remains the recommended full native engine.
11. Ordinary creators should not need to choose an engine/runtime; AI agents should normally choose Web or Godot based on the game's requirements.
12. Unity initially enters through Unity Web export as a compatibility path, not as a required creator stack.
13. Unreal native support is deferred.
14. Privileged functionality belongs in the native GigaCouch Host.
15. Games use restricted platform APIs rather than direct access to platform credentials/databases.
16. Installed games should support offline play when the game itself does not require networking.
17. Runtime versions are explicit and immutable.
18. Creators retain real game projects and should not be intentionally locked into GigaCouch.

---

# 29. Decisions Still Requiring Prototypes

Do not treat the following as final until tested:

- CEF vs Electron vs another Chromium embedding strategy
- exact Chromium version/update policy
- whether launcher UI is Web, Godot, or another stack
- exact Rust ↔ Web IPC mechanism
- exact Rust ↔ Godot IPC mechanism
- maximum practical controller count by OS/device stack
- haptics behavior across controller types
- local-origin strategy for offline Web games
- WebGPU support policy
- WebAssembly threads/SIMD policy
- per-game browser process model
- browser memory limits
- Godot native sandbox design
- LAN transport implementation
- online multiplayer architecture
- native Unity support
- native Unreal support
- marketplace trust levels
- exact packaging extension and file layout

Agents must not silently convert these investigation items into fixed architecture decisions.

---

# 30. Non-Goals for V1

Do not initially build:

- a custom rendering engine
- a custom physics engine
- custom console hardware
- a proprietary AI model
- a proprietary game editor
- PlayStation deployment
- Nintendo deployment
- native Unreal support
- native Unity runtime management
- MMO infrastructure
- sophisticated matchmaking
- giant public marketplace
- cloud game streaming
- arbitrary native plugin support
- cross-platform native sandboxing before Web distribution works

---

# 31. Short Architecture Summary

For agents needing the shortest possible interpretation:

> **GigaCouch is a native desktop gaming platform with two first-class game runtimes. The Web Runtime is a controlled Chromium-based environment intended both for direct AI-authored games built with technologies such as Three.js, Babylon.js, Phaser, JavaScript/TypeScript, WebAssembly, WebGL, and WebGPU, and for compatible Web exports from engines such as Unity or Godot. The Godot Runtime is a managed native environment for games that benefit from a full engine, deeper tooling, or greater native performance. A trusted Rust host owns controllers, player assignment, saves, LAN networking, accounts, installation, updates, and other privileged services. Creators describe the game they want; AI agents should normally choose the simplest suitable runtime. Both runtimes access the same conceptual GigaCouch platform API through runtime-specific bindings.**

---

# 32. Agent Instruction

When implementing GigaCouch:

1. Preserve Web and Godot as two first-class runtimes.
2. Treat direct AI-authored Web games as a primary creation path, not merely an engine-export compatibility feature.
3. Let AI agents choose the simplest suitable runtime based on game requirements; do not force ordinary creators to make engine decisions.
4. Put privileged operations in the native host.
5. Keep platform APIs engine-independent.
6. Do not make Web games depend on arbitrary browser features outside the declared runtime.
7. Do not expose unrestricted native APIs to Web content.
8. Do not make Godot a requirement for all creators.
9. Do not add another native engine runtime without a demonstrated need.
10. Prefer simple, versioned, machine-readable interfaces.
11. Maintain offline local play.
12. Optimize every user-facing flow for a TV and controller.
13. Treat 1–16 local players as a platform-level input requirement, not as a promise that every game can render 16 views.
14. Clearly distinguish current decisions from unproven implementation hypotheses.

