# AI-Native Living Room Game Platform

> **V1 changed on October 8, 2026.** V1 is a website for uploading, browsing, and playing web games in Chrome. See [docs/v1.md](docs/v1.md). Where this document describes V1 differently, such as Godot first, an installed app, or phones as controllers, docs/v1.md wins. Those parts describe research and later stages.

The consumer platform is **Giga Couch** ([gigacouch.com](https://gigacouch.com)).

Creator delivery specification: [Giga Couch Game Development Kit](GDK.md), covering what developers get, how they run it and the remaining release work.

Player experience specification: [Giga Couch player app](docs/giga-couch-app.md), the controller-operated TV library, accounts and social experience, separate from Creator Hub.

## Product Vision

Build a gaming platform for a world where ordinary people can create real games with AI.

The platform allows someone to:

1. Use the AI tools of their choice to create a game.
2. Build the game using a real game engine.
3. Add our SDK to make the game work naturally on a TV with controllers.
4. Play the game themselves or with friends and family.
5. Share the game privately.
6. Publish good games publicly.
7. Eventually sell games through a marketplace.

The fundamental idea is:

> **AI can make the game. We make it a console game.**

The platform is not primarily an AI game generator and is not intended to replace existing game engines.

It provides the infrastructure that turns AI-assisted game development into a simple consumer gaming experience.

---

# The Opportunity

Historically, creating a video game required specialized skills in:

- programming
- game engines
- 3D modeling
- illustration
- animation
- music and sound
- level design
- UI design
- testing
- deployment

That meant games generally needed sufficiently large audiences to justify the cost and effort required to create them.

AI changes this equation.

Increasingly, one person can use AI agents and generative tools to create:

- gameplay code
- characters
- environments
- textures
- animations
- sound effects
- music
- dialogue
- menus
- levels
- game rules

As these tools improve, the number of potential game creators expands from professional developers to nearly anyone with an idea.

A child could create a game for three friends.

A family could create a game about themselves.

Someone could turn an inside joke into a party game.

An enthusiast could create a polished game and eventually sell it.

Games no longer necessarily need millions—or even thousands—of potential customers to justify being created.

This creates a need for infrastructure designed specifically for this new generation of creators.

---

# What We Are Building

The platform has two primary components:

## 1. Game Platform SDK

A Godot SDK that handles the difficult and repetitive work required to make games behave like living-room console games.

## 2. Game Platform App

A controller-first application running on the player's own PC or Mac, displayed on a television through a direct connection or screen sharing, where users can:

- launch games
- manage their library
- play games
- share games
- receive games from friends and family
- eventually discover and purchase community-created games

Together these create a simple workflow:

```text
Idea
 ↓
AI + Godot
 ↓
Game
 ↓
Platform SDK
 ↓
Publish
 ↓
TV
 ↓
Controllers
 ↓
Play
```

Games run locally on the player's own PC or Mac, using that computer's CPU and GPU. Installed games should support offline launch and local saves. Online services support distribution, sharing, and global platform data.

---

# Core Product Principle

## Do not build another game engine.

For the initial version, games are built with **Godot**.

Godot provides:

- rendering
- physics
- scripting
- animation
- shaders
- audio
- navigation
- 2D and 3D
- networking capabilities
- scene management
- asset pipelines
- editor tooling

Creators should retain access to the full capabilities of a real game engine.

We do not want to create a simplified Roblox-like environment that limits what games can eventually become.

Instead:

> **Godot is the engine. We are the platform.**

---

# Why Standardize on Godot Initially?

Supporting one engine gives us significant advantages.

Games can share a known runtime environment.

The SDK only needs to support one engine.

AI agents only need to understand one integration specification.

Controller behavior can be standardized.

Packaging becomes predictable.

Testing becomes predictable.

Security becomes more manageable.

Cross-platform distribution becomes easier.

Documentation becomes dramatically simpler.

The platform can eventually support other engines, but supporting everything from the beginning would unnecessarily increase complexity.

For V1:

> **Godot-first, not Godot-forever.**

---

# Bring Your Own AI

The platform should not depend on owning the AI creation process.

Creators should be able to use whichever tools work best for them.

Examples might include:

- ChatGPT
- Codex
- Claude
- Claude Code
- Gemini
- Cursor
- future autonomous development agents
- image generation tools
- 3D generation tools
- music generators
- animation generators
- Blender
- traditional human development

The platform benefits whenever these tools improve.

If a new AI launches next year that can create extraordinary Godot games, that improves our ecosystem rather than threatening it.

Our goal is to become the destination where those games are played and distributed.

---

# AI-Friendly by Design

Although the platform does not need to provide the AI itself, the SDK should be designed specifically so AI agents can use it extremely reliably.

A creator should be able to tell an AI:

> "Make this game compatible with the Platform SDK."

The AI should be able to read our documentation and perform the integration without specialized knowledge.

The SDK should therefore have:

- small APIs
- predictable naming
- extensive examples
- machine-readable documentation
- automated tests
- clear error messages
- an `AGENTS.md` or equivalent AI instruction file
- starter projects
- reference implementations

A prompt might eventually be as simple as:

> "Create a four-player Godot racing game. Use the Platform SDK so we can play it on the living-room TV."

---

# The Platform SDK

The SDK is one of the central products.

It removes the repetitive work that almost every couch game otherwise has to recreate.

## Controller Management

The SDK should handle:

- controller detection
- wireless Xbox controllers (Bluetooth-capable models)
- wireless PlayStation controllers (DualShock 4 and DualSense)
- the Wii controller family: Remote/Remote Plus, Nunchuk, Classic/Classic Pro, and Wii U Pro; track specialty accessories, GameCube adapters, and Wii U GamePad separately until their connection and input paths are implemented and tested
- Nintendo Switch Pro and Joy-Con controllers, including individual sideways Joy-Cons and paired grips; Switch 2 is a target pending driver/runtime and hardware qualification
- common PC controllers
- Bluetooth controllers
- USB controllers
- up to 16 controllers simultaneously, including mixed Xbox and PlayStation groups
- controller disconnect/reconnect
- player assignment
- "Press any button to join"
- controller remapping
- dead zones
- analog input
- rumble/haptics
- controller-specific button glyphs

Games should generally work with **actions**, rather than hardware-specific buttons.

For example:

```gdscript
Platform.input.action(player, "jump")
Platform.input.action(player, "interact")
Platform.input.axis(player, "move")
```

The game should not care whether the player is using:

- Xbox A
- PlayStation Cross
- an 8BitDo controller
- another compatible controller

The SDK translates physical hardware into standardized game actions.

Wii family support is a product target. The prototype has experimental basic-action profiles for the core controllers, with physical compatibility still pending. Motion, pointer input, and specialty accessories require additional APIs and hardware validation; see [Wii coverage](docs/wii-controllers.md).

---

# Standard Game Actions

The SDK should establish recommended semantic actions such as:

```text
move
look

jump
interact

primary_action
secondary_action

left_action
right_action

pause

menu_up
menu_down
menu_left
menu_right
menu_accept
menu_back
```

Games can add their own actions as needed.

The standard exists primarily to make common functionality easy.

It is not intended to restrict what games can do.

---

# Controller Glyphs

Games should be able to ask:

```gdscript
Platform.input.glyph(player, "jump")
```

The platform determines the connected controller and displays the correct visual prompt.

For example:

```text
Xbox → A

PlayStation → Cross

Other controller → appropriate south-face button
```

This creates a polished console-style experience without every game developer implementing the system separately.

---

# Local Multiplayer

Local multiplayer should be a first-class platform capability.

Support **1–16 local players** as a core product requirement. Games can choose a smaller maximum; the platform and SDK must not impose a four-player limit. Wireless Xbox and PlayStation controllers are required, paired to the player's computer. Sixteen player slots in software do not establish that every computer or wireless adapter can connect sixteen controllers: publish tested OS/controller/connection combinations and validate the full sixteen-controller session before advertising that hardware setup as supported.

The SDK should make this trivial:

```text
PLAYER 1
READY

PLAYER 2
Press any button

PLAYER 3
Press any button

PLAYER 4
Press any button
```

The SDK handles controller ownership.

Games deal with:

```text
Player 1
Player 2
Player 3
Player 4
```

rather than low-level hardware device IDs.

Local multiplayer may become one of the platform's major differentiators.

---

# TV-Friendly UI

Games on the platform should be designed to work naturally from a couch.

The SDK can provide:

- controller-driven menu navigation
- focus management
- TV-readable font recommendations
- safe screen margins
- standard confirmation dialogs
- pause menus
- settings menus
- player-join interfaces
- controller-disconnected interfaces
- standard back behavior
- quit-to-platform behavior

A creator should not need to reinvent these systems.

---

# Save and Restore

The SDK should provide standardized game save functionality.

For example:

```gdscript
Platform.save("campaign", data)

Platform.load("campaign")
```

Initially saves may simply be stored locally.

Later the same API could transparently support:

- cloud saves
- family profiles
- multiple users
- synchronization between machines

Games should not need to change simply because the platform gains cloud functionality later.

---

# Platform Lifecycle

The SDK should give games predictable lifecycle behavior.

Examples:

```text
launch
pause
resume
controller disconnected
controller connected
user changed
quit to platform
```

This allows the overall experience to behave more like a game console than a collection of unrelated PC applications.

---

# Game Packaging

V1 games should follow a defined Godot platform specification.

Possible initial requirements:

- supported Godot version
- GDScript
- no required keyboard/mouse interaction
- controller-compatible menus
- platform manifest
- standardized save behavior
- supported rendering targets
- no unauthorized native extensions
- known platform runtime compatibility

The preferred distribution model is Godot PCK/resource packages running against a shared, platform-managed Godot runtime installed on the player's computer.

The platform downloads the runtime once and reuses it for every compatible game. Individual game downloads contain their game content and manifest rather than another copy of the engine.

Players should only need to install the platform app. The app handles runtime installation automatically; players do not need to install the Godot editor or manage engine versions themselves.

The platform should first detect whether a supported Godot installation is already available and reuse a compatible installation where appropriate. If Godot is missing, unusable, or unsupported, show the required version and clear installation instructions. Do not silently replace a user's incompatible installation.

The host and SDK must share a supported-version policy. The host checks availability before launching Godot; the SDK checks the engine it is running inside and gives creators setup guidance when that engine is unsupported. Initial developer tooling can guide installation before automatic runtime management is implemented.

Each game runs in its own process using the shared runtime executable. Sharing an installed runtime does not mean loading every game into the launcher's process.

This packaging approach must be validated against the chosen runtime build and export settings before finalizing the package specification.

---

# Runtime Versioning

V1 should standardize on one tested Godot runtime so all V1 games share a single engine download.

The platform should not assume that every game will always run against the newest Godot version.

Instead:

```text
Platform Runtime 1
Platform Runtime 2
Platform Runtime 3
```

Games declare the runtime they target.

For example:

```json
{
  "runtime": "1",
  "sdk": "1.2",
  "players": {
    "min": 1,
    "max": 4
  }
}
```

This prevents future engine upgrades from breaking older games.

If future games require incompatible engine versions, the app can install additional runtimes automatically, once per required version. All games targeting the same runtime reuse that installation. The goal is one shared runtime wherever compatibility allows, with no manual version management for players.

Each platform runtime ID maps to a tested Godot build and supported export/rendering configuration.

---

# Global Data and Backend

Global platform data will use **PostgreSQL hosted on Neon (neon.tech)**. Neon is the platform's server-side database.

This includes:

- platform user profiles
- game and release metadata
- library ownership and access
- family and sharing relationships
- publishing and visibility state
- future discovery and marketplace records

The platform backend mediates access to Neon and enforces authorization. Database credentials remain on the server; desktop apps and games access global data through authenticated platform APIs.

Player computers will use **SQLite** for local platform data such as installed game metadata, library state, and local profile settings. Players do not need to install or manage a database server.

Game saves also remain on the player's computer, managed through the SDK. Installed games and local data remain available offline so ordinary play does not depend on a connection to Neon or the platform backend.

Game packages, screenshots, and other large assets belong in object storage, with their metadata and storage references in Neon. The object storage provider, authentication system, and API hosting remain separate implementation decisions.

---

# Starter Project

We should provide a standard Godot starter project containing:

```text
Platform SDK

Input setup

Player manager

Controller joining

Pause system

Save manager

TV-safe UI

Basic settings menu

Platform manifest

Publishing configuration
```

Someone—or their AI agent—can clone the project and immediately begin making the actual game.

---

# Player Experience

The consumer experience should feel like a console.

The player's PC or Mac displays the platform and games on the television through either:

- a direct display connection, such as HDMI
- screen sharing or mirroring through a compatible setup

The game runs on the player's computer in both cases; the TV displays its output. V1 can rely on existing screen-sharing solutions rather than requiring a platform-built streaming system.

The user opens the Platform app.

Everything works with a controller.

Possible home navigation:

```text
MAKE

MY GAMES

FRIENDS & FAMILY

DISCOVER
```

Initially, `MY GAMES` and `FRIENDS & FAMILY` may be more important than `DISCOVER`.

The platform should provide value even when there are relatively few public games.

---

# The Cold-Start Advantage

Traditional gaming platforms face a major chicken-and-egg problem.

Developers want players.

Players want games.

Our platform is unusual because **the player may also be the creator**.

A user can create a game solely because they personally want to play it.

Example:

> "Make a four-player game where our family are wizards defending our house from goblins."

That game does not need a marketplace.

It already has four players.

This creates the initial loop:

```text
Player
 ↓
creates game
 ↓
plays game
 ↓
shares with family
 ↓
family plays
 ↓
improves game
```

Public distribution is optional.

This dramatically reduces the normal platform cold-start problem.

---

# Personal Games

AI enables a category of games that historically could not economically exist.

Examples:

- a game about your family
- a game featuring your friends
- a game about your dog
- a birthday game
- a game based on an inside joke
- a game based on a child's imaginary world
- a game based on a family vacation
- a game for a school group
- a one-night party game
- a game for four specific friends

Historically, creating a game for eight people made no economic sense.

If AI reduces creation time dramatically, it can.

This leads to an important principle:

> **Games no longer need large audiences to justify existing.**

---

# Sharing

Games should move naturally through increasing levels of distribution.

```text
MY GAME
   ↓
PLAY LOCALLY
   ↓
FAMILY
   ↓
FRIENDS
   ↓
SHARE LINK
   ↓
PUBLIC
```

A creator should never be forced to become a public publisher.

A child's game can remain inside a family library forever.

---

# Family Library

A possible feature:

```text
THE FAMILY LIBRARY

Dad vs Kids Racing
Made by Dad

Dragon Basement
Made by Ben

Horse Adventure
Made by Lucy

Christmas Snowball Fight
Made together
```

Games may become personal artifacts in much the same way families currently keep:

- photographs
- videos
- drawings
- stories

Except these artifacts are playable.

---

# Public Marketplace

Once the creation and sharing experience works well, public publishing can create an entirely new incentive structure.

Creators can choose to release games:

- free
- paid
- eventually included in subscriptions
- potentially supported by optional additional content

Example prices might include:

```text
Free
$0.99
$1.99
$4.99
$9.99
```

The exact marketplace model can evolve.

The important concept is:

> **If someone creates something genuinely good, they should have a path to earning money from it.**

---

# Why the Marketplace Matters

AI will probably create an enormous increase in the number of games.

The difficult problem may eventually stop being:

> "How do we make enough games?"

and become:

> "Which of these millions of games are actually worth playing?"

The marketplace and discovery system become a human quality filter.

Strong games rise because people:

- play them
- finish them
- return to them
- recommend them
- purchase them
- rate them highly
- play them with friends

This rewards creators who use AI well instead of simply creators who generate the most content.

---

# Discovery

The marketplace should feel designed for someone sitting on a couch, not someone browsing a PC software catalog.

Possible categories:

```text
PLAY TOGETHER TONIGHT

2 PLAYER

4 PLAYER

8 PLAYER

16 PLAYER

FAMILY GAMES

PARTY GAMES

UNDER 20 MINUTES

LONG ADVENTURES

GREAT WITH KIDS

NEW FROM FRIENDS

TRENDING

CREATOR PICKS

UNDER $5
```

Player count and living-room context should be prominent metadata.

---

# Creator Economy

Eventually the platform could support several creator types.

## Game Creators

Make and sell complete games.

## Asset Creators

Create:

- characters
- environment packs
- models
- textures
- sound packs
- animation packs

## System Creators

Create reusable Godot systems such as:

- racing
- inventory
- quests
- combat
- dialogue
- procedural worlds
- vehicles

## Template Creators

Create high-quality starting projects designed for AI modification.

This could eventually produce an ecosystem where AI agents assemble games using high-quality human-created components.

---

# Remixing

Games could optionally support remixing.

A creator might publish:

```text
Raccoon Heist
```

Someone else could create:

```text
Christmas Raccoon Heist
```

or:

```text
Raccoon Heist: Space Station
```

If commercial remixing is supported, the platform could eventually automate attribution or revenue sharing.

This is a later capability, not a requirement for V1.

---

# Creator Ownership

One important principle should be:

> **Creators own real game projects.**

If AI creates a game, the result should be an ordinary Godot project:

```text
MyGame/
    project.godot
    scenes/
    scripts/
    models/
    textures/
    audio/
```

Creators should be able to:

- open it in Godot
- inspect the code
- modify it manually
- use Git
- use external tools
- back it up
- potentially export it elsewhere

The platform should not intentionally trap creators inside a proprietary editor.

This is a major philosophical difference from many traditional user-generated-game platforms.

---

# Platform Certification

Publishing should be easy without sacrificing consistency.

The platform can automatically test:

```text
✓ Game launches

✓ Controller works

✓ Main menu works without mouse

✓ Pause menu works

✓ Controller disconnect is handled

✓ Controller reconnect is handled

✓ UI works at TV distance

✓ Game can return to platform

✓ Save directory is valid

✓ Runtime version is valid

✓ Package contains no prohibited functionality
```

AI could help creators resolve failures automatically.

Example:

> Your game is almost ready to publish.
>
> Three issues were found:
>
> 1. Settings menu requires a mouse.
> 2. Player 2 is mapped to Player 1's controller.
> 3. Pause menu text is below the recommended TV size.
>
> Fix automatically?

This could make platform certification much less painful than traditional console certification.

---

# Security

Because user-created content is distributed to other machines, security must be designed into the platform from the beginning.

For early versions, restricting the supported Godot environment can substantially reduce risk.

Potential initial restrictions:

- GDScript only
- no arbitrary native executables
- no arbitrary operating-system commands
- limited filesystem access
- no unsanctioned native libraries
- signed packages
- automated scanning
- controlled runtime versions

Public publishing should receive more scrutiny than private/local games.

Possible trust levels:

```text
LOCAL
Only the creator uses it.

PRIVATE
Shared with specific accounts.

VERIFIED
Automated platform checks passed.

PUBLIC
Available through discovery.

MARKETPLACE
Eligible for commercial sale.
```

---

# V1

V1 should prove one thing extremely well:

> **Someone can create a Godot game and very easily turn it into a couch game that they can play and share.**

V1 does not need a giant public marketplace.

Core components:

### SDK

- standardized controller actions
- multiple-controller support
- controller identification
- player assignment
- button glyphs
- remapping
- disconnect/reconnect
- rumble
- controller menu navigation
- pause
- save/load
- quit-to-platform
- TV UI helpers

### Platform App

- controller-first interface
- local game library
- launch game
- install game
- update game
- family/private sharing

### Developer Tools

- Godot starter project
- AI-readable SDK documentation
- publishing CLI
- local validation
- game packaging
- automated controller testing

---

# V1.5

Add community publishing.

Possible additions:

- creator profiles
- public free games
- search
- ratings
- categories
- sharing links
- following creators
- basic recommendations
- reporting/moderation

---

# V2

Add the marketplace.

Creators can:

- price games
- receive payments
- view sales
- publish updates
- build audiences

Players can:

- purchase games
- review games
- follow creators
- receive recommendations
- maintain libraries

Platform revenue can come primarily from a percentage of transactions.

This aligns platform incentives with creator success.

---

# Longer-Term Platform Services

Possible later functionality includes:

- cloud saves
- achievements
- friends
- invitations
- online multiplayer services
- voice chat
- leaderboards
- family accounts
- parental controls
- subscriptions
- creator analytics
- asset marketplace
- DLC
- game remixing
- creator revenue sharing
- phone-as-controller support
- additional engine runtimes

None of these are necessary to validate V1.

---

# What We Should Not Build Initially

Avoid unnecessary scope.

Do not initially build:

- a new rendering engine
- a new physics engine
- a proprietary programming language
- a Roblox-style game editor
- our own foundation AI model
- our own image generator
- our own 3D generator
- proprietary game mechanics that every game must use
- custom console hardware
- PlayStation/Xbox/Nintendo deployment
- sophisticated online multiplayer infrastructure
- a huge marketplace before creation and sharing work

Use existing technology whenever possible.

Our value lies in connecting those pieces into a dramatically simpler experience.

---

# Competitive Position

The platform is not simply another Steam.

Steam assumes:

> professional or experienced developer → finished PC game → distribution

Our platform assumes:

> person with an idea → AI helps build real game → SDK makes it couch-ready → play/share/sell

It is also not Roblox.

Roblox generally means:

> create inside Roblox → run inside Roblox's game framework

Our approach is closer to:

> create a real Godot game → add lightweight platform services → distribute it through our gaming environment

The game engine remains powerful and broadly capable.

---

# Core Differentiators

## 1. AI-Native Creator Assumption

The platform assumes many creators will not be traditional programmers.

## 2. Bring Your Own AI

We benefit from advances across the entire AI industry.

## 3. Real Game Engine

Creators use Godot rather than a severely constrained proprietary game framework.

## 4. Controller First

Controllers and local multiplayer are platform primitives rather than afterthoughts.

## 5. Living Room First

The primary consumer experience is the TV.

## 6. Personal Games Matter

Games do not need public audiences.

## 7. Extremely Easy Sharing

Going from development machine to family TV should feel almost instantaneous.

## 8. Path From Hobby to Business

The same game someone creates for friends can eventually be sold publicly.

## 9. Creator Ownership

AI produces real projects creators can inspect and modify.

## 10. SDK Before Ecosystem

The SDK can provide value even before the marketplace has a large audience.

---

# Platform Flywheel

The desired long-term flywheel is:

```text
More players
     ↓
More people experiment with creating
     ↓
AI makes creation increasingly easy
     ↓
More games
     ↓
Some games become excellent
     ↓
Marketplace rewards those creators
     ↓
More serious creation
     ↓
Better games
     ↓
More players
```

But unlike most marketplaces, the system can provide meaningful value even before this flywheel becomes large.

A player can make a game for themselves.

That is the key cold-start advantage.

---

# Product Philosophy

The platform should consistently follow several principles.

## Hide infrastructure, not capability.

Make difficult things easy without restricting what sophisticated creators can eventually build.

## Let AI handle complexity.

Design APIs and tools so agents can reliably perform integration work.

## Standardize the boring parts.

Input, controller handling, saving, menus, packaging and distribution should not have to be reinvented.

## Leave creativity open.

Gameplay, art style, genre and mechanics belong to the creator.

## Make private creation as important as public publishing.

A game played by four family members can still be successful.

## Make good games valuable.

Marketplace economics should reward quality rather than sheer content volume.

## Build for the couch.

The platform should feel like a console even though it is running on ordinary computer hardware.

---

# Short Explanation

When explaining the idea quickly:

> **We're building a gaming platform for the AI era. People can use whatever AI tools they want to create real Godot games. Our SDK handles controllers, local multiplayer, TV menus, saving and other console-style functionality. Then they can play those games on their own PC or Mac, displayed on their TV through a direct connection or screen sharing, share them with friends and family, and eventually publish good games to a marketplace. Think Steam for a world where regular people can make games with AI—but designed around the TV and controllers from the beginning.**

---

# One-Sentence Vision

> **Anyone should be able to turn an idea into a real game, pick up a controller, and play it with the people they care about.**
