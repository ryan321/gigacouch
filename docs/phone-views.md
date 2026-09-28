# Phone Views

Every phone screen becomes a **view** built from **widgets**, described as data, and drawn by one runtime on the phone. A controller, a private screen, and a question are all views. Games reuse the built-in views, change them, or define their own, and every definition is checked by the same rules before it reaches a phone.

## Why

Phone controllers grew one feature at a time. Built-in layouts are a table in `pad.js`. Private screens, questions, and button labels are overlays with their own rules. A game's own layouts can use only a fixed set of controls. The list of layout names is repeated in the Rust host, the pad, the package schema, and the Controller Gallery. Every new interaction has meant changing four places, and the overlay rules have already produced one bug: the phone keyboard opening and closing in a loop.

Games will want interactions nobody has built yet. That needs a small set of general parts instead of more special cases.

## The parts

### Widget

The one building block. Each widget type has:

* **Properties**, checked by a schema: position, label, color, options.
* **A renderer** in the pad.
* **An output**, one of two kinds:
    * **Input**: a stick moving, a button held. It feeds the player's input, which games read every frame.
    * **Events**: a choice picked, text sent, strokes drawn. They arrive in order through `GigaCouch.phone.events()`.

Today's controls and screen parts are all widgets:

| Kind | Widgets |
| --- | --- |
| Input | stick, dpad, button, arrow, slider, touchpad |
| Events | canvas, palette, choices, text input |
| Display | text, image |

New widgets fit the same shape: a timer, a meter, a sortable list, a grid of cells, a gesture pad that reports swipes.

### View

A named arrangement of widgets, with a landscape and an optional portrait version. Positions are fractions of the phone's control area.

Views stack. A base controller sits underneath, and a view such as a private hand or a question sits on top. The base is never rebuilt while a view is on top, so typing, drawing, and choosing never fight the controller.

### Named actions

A game declares its inputs by name in `gigacouch.json`, such as `"jump": "button"` and `"throttle": "axis"`, and binds widgets to them. The standard names stay the defaults (south, east, west, north, start, move, look), so pads and today's games keep working. Each action can name a standard key or stick that stands in for it on pads and keyboards, and the host reports the game's actions for each player alongside the standard buttons.

### Data binding

A view is a template, and each player gets their own data. A game sends a view once with data, then small updates:

```javascript
GigaCouch.phone.show(2, "hand", { cards: ["sun", "moon", "star"] })
GigaCouch.phone.update(2, "hand", { "cards.1.disabled": true })
```

One definition serves every player's private hand, and updates stay small.

### Feedback on the phone

A widget can declare what a press feels like: `feedback: { sound: "coin", rumble: "tap", flash: true }`. The phone plays it at once, with no round trip to the computer. Game-driven sounds and rumble stay for moments the game decides.

### Where views come from

| Source | Where | Checked |
| --- | --- | --- |
| Built-in | JSON files shipped with the host | By tests |
| Package | `gigacouch.json`, or a `phone/` folder in the package | When the package is checked |
| Runtime | Sent by a running game | On every message |

All three use the same format and the same validator.

### One source of truth

Widget schemas and the built-in views live as JSON files. The Rust validation, the pad's renderer registry, the package schema, TypeScript types for game developers, the docs, and the galleries all read from them. A test fails when any of them drifts.

### Versioning

Each phone reports its pad version and the widgets it can draw. The host refuses or downgrades a view that phone cannot draw. Phones already reload themselves when the host's pad is newer.

## Games inventing new interactions

Three tiers, from safest to most flexible:

1. **Composition.** Most new interactions combine existing widgets, and that covers most party games.
2. **Parameterized widgets.** Richer widgets with options instead of new code: a canvas with draw, trace, or tap modes, or a gesture pad with chosen gestures. The platform adds them as games need them.
3. **Custom widgets, sandboxed.** A widget type that loads a script from the game's package into a sandboxed frame on the phone. It has no network access and no access to the pad or the host, and talks only through a narrow message API: it receives properties and sends events. This breaks today's rule that no game code runs on phones, so it is an explicit capability in the package. Phones show that the area is drawn by the game. It waits for package signing.

## How the code splits

| Part | Where | Job |
| --- | --- | --- |
| Widget schemas | `crates/web-host/src/assets/phone/widgets.json` | Every widget type, its properties, and its output |
| Built-in views | `crates/web-host/src/assets/phone/layouts.json` | The built-in controllers, with a title and a description |
| Validation | `crates/web-host/src/phones/views.rs` | Checks any view or widget against the schemas, for packages and running games |
| Messages | `crates/web-host/src/phones/protocol.rs` | Every message between host, phone, and game in one place |
| Phone runtime | `crates/web-host/src/assets/pad.js` | Layers, layout, sending, reconnecting |
| Widget renderers | A registry in the pad, one renderer per widget type | Draws a widget and reports its output |
| Game API | `GigaCouch.phone` in the bridge | `show`, `update`, `hide`, `events`, and today's calls |
| Godot SDK | `sdk/addons/couchgames` | The same messages, so Godot games get phones |
| Preview | A page on the host | Any view with sample data in a phone-sized frame, for building without a phone |

## Plan

1. **Schemas and registry.** Move today's controls into `widgets.json`, the built-in layouts into `layouts.json`, validation into `views.rs`, and the pad's controls into a registry. Nothing changes for players or games. The repeated lists go away.
2. **Views, layers, and data binding.** Private screens and questions become views on a layer above the controller. `show` takes a view name and data, and `update` changes parts of it. The Phone Lab moves onto them.
3. **Named actions and feedback on the phone.**
4. **Preview page and generated types** for creators.
5. **Sandboxed custom widgets**, once packages can be signed.

## Status

| Step | Status |
| --- | --- |
| 1. Schemas and registry | Done: `widgets.json`, `layouts.json`, `views.rs`, and the pad's widget registry. The host fills the pad in from the JSON, and tests fail if the pad, the package schema, or the host drift from it. |
| 2. Views, layers, and data binding | Done: panel views (`views.json` built in, `phone.views` in packages, or written out by a game), `{"bind": "path"}` into per-player data, `show`, `update`, and `hide`, one panel layer drawn in place on the phone. Today's `show(player, screen)` and `ask` are the built-in `screen` and `ask` views. The Phone Lab's hands and vote use the lab's own views. |
| 3. Named actions and feedback | Done: top-level `actions` in `gigacouch.json` with a `pad` fallback for pads and keyboards, layouts binding buttons and axes to them, `named` in each player's snapshot, and the bridge reading them through `action`, `held`, and `axis`. A button or arrow's `feedback` (sound, rumble preset, flash) plays on the phone at the press. The Phone Lab's Named actions station and its quiz and Rock, Paper, Scissors buttons use them. |
| 4. Preview page and types | Not started |
| 5. Sandboxed custom widgets | Not started |
