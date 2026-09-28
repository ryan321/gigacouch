# Phone Controller Ideas

Every idea here except tilt is now built, as features games opt into. Tilt waits on HTTPS. The Phone Lab on the shelf has a station for each one.

## Status

| Idea | Status | Try it |
| --- | --- | --- |
| Private screens | Built: text, the game's images, and choices | Phone Lab: Private hands, Audience vote |
| Typing on the phone | Built, and used by Home's "Spell the name" | Phone Lab: Captions |
| Phone as the Home remote | Built: Start a game in the phone menu | Phone menu on the shelf |
| Game-defined layouts and labels | Built: `phone.layouts` and `setLabels` | Phone Lab: Answers on the buttons, Custom layout |
| Keep the phone awake | Built with a silent looping clip; not yet tried on real phones | Phone menu: Keep the screen on |
| Rejoin as the same player | Built: two minutes, no button press to return | Phone Lab: Connection and rejoin |
| Link a phone to a Home profile | Built: the phone plays under that person's name | Phone menu: Who's playing |
| Connection quality | Built: signal bars on the phone, `rtt` for games | Phone Lab: Connection and rejoin |
| Mirrored layout | Built | Phone menu: Left-handed controls |
| Drawing | Built: the `draw` layout and stroke events | Phone Lab: Drawing |
| Photo avatars | Built; not yet tried with a real phone camera | Phone menu: Take a photo, Phone Lab: Photos and names |
| Tilt and motion | Not built: needs HTTPS | — |
| Audience mode | Built | Phone menu: Watch as the audience |
| Pad rumble | Built; not yet tried with a real pad | Phone Lab: Rumble |

How games use each feature is in [runtimes/web/README.md](../runtimes/web/README.md). The rest of this doc is the original reasoning.

## Where phones stand

Phone controllers already work in the Giga Couch Game Browser. A phone on the same Wi-Fi scans the QR code on the shelf, opens a touch pad, and joins as a player. There is no limit on phones or players unless a game sets one.

* 11 built-in layouts, from a stick with two buttons to a racing wheel, a touchpad, and four rhythm lanes. A game picks one in `gigacouch.json` and can switch while it runs.
* Games play stock sounds or their own sounds on one player's phone or every phone.
* Android phones vibrate on request. iPhones flash the pad's edges instead.
* Every phone has a menu: back to the shelf, leave the game, change name, and reload the controller.
* The host pushes messages to phones as soon as it has them. Every idea below builds on that channel.

The details are in [runtimes/web/README.md](../runtimes/web/README.md). The Controller Gallery on the shelf shows every layout, sound, and rumble.

## Top three, as first proposed

1. **Private screens on the phone.** They open a whole genre of games that nothing else on the couch can do.
2. **Keep awake, plus rejoin as the same player.** Together they fix the most common real-world frustration: a phone dropping out mid-game.
3. **Typing on the phone.** It makes Home and many party games far easier, and it reuses the channel that already exists.

## Biggest impact

These change what kinds of games work on the couch. Effort is a rough estimate against the current code.

| Idea | What players get | Why it matters | Effort |
| --- | --- | --- | --- |
| Private screens | A game sends content to one player's phone only: a hand of cards, a secret role, a word to draw, the quiz answers | Opens hidden-information and Jackbox-style games. A TV alone can't do this | Large |
| Typing on the phone | Answers, captions, and names typed with the phone's own keyboard | Replaces spelling a name letter by letter with a pad on Home, and unlocks writing games | Medium |
| Phone as the Home remote | Browse the shelf and start a game from the phone | Nobody needs a pad to get started | Medium |
| Game-defined layouts and labels | A game describes its own buttons and labels them, such as the actual quiz answers | Games stop being limited to the built-in layouts | Medium |

Private screens stay safe by sending text and images from the game's package, laid out by the pad. No game code runs on phones.

## Smoother play

These fix what goes wrong in a real living room.

| Idea | What it fixes | How | Effort |
| --- | --- | --- | --- |
| Keep the phone awake | Phones dim and lock mid-game and drop the player | The Wake Lock API needs HTTPS. A silent looping video keeps the screen on over plain http on iPhone and Android | Small |
| Rejoin as the same player | A phone quiet for more than 8 seconds loses its player slot | The host remembers each phone and hands back the same player number and saves when it returns | Medium |
| Link a phone to a Home profile | Saves and names don't follow a person between sessions | The phone remembers "I'm Ada", so Ada's saves and name come with it | Medium |
| Connection quality | A lagging player can't tell whether it's their Wi-Fi | A small signal indicator in the pad's header, from round-trip time | Small |
| Mirrored layout | Left-handed players get the stick on the wrong side | A toggle in the phone menu that swaps sides | Small |

## New kinds of input

| Idea | What it enables | Needs | Effort |
| --- | --- | --- | --- |
| Drawing | A canvas layout that sends finger strokes, for Pictionary-style games | Nothing new | Medium |
| Photo avatars | A selfie on each player's TV card and in games | A file picker with the camera option, which works over plain http | Medium |
| Tilt and motion | Steering by tilting the phone | HTTPS: iPhone and Android block motion sensors on plain http pages | Large |
| Audience mode | Extra phones vote or react without taking a player slot | Nothing new | Medium |
| Pad rumble | `GigaCouch.phone.rumble` also buzzes Xbox and PlayStation pads | Chrome's gamepad vibration | Small |

## Constraints

* **HTTPS.** The phone pad is plain http on the home network, because there is no normal way to get a certificate for a home IP address. Tilt, the Wake Lock API, and some camera features need HTTPS. A relay through gigacouch.com, already on the checklist, would provide it and also reach phones on networks that keep devices apart.
* **iPhone limits.** iPhone web pages cannot vibrate, and sound starts only after the first tap. Real haptics on iPhone would need a native app.
* **Game content on phones.** Anything a game shows on a phone comes from its package, is checked by the host, and is drawn by the pad. Phones never run game code, and the phone listener serves only what the open game declared.
* **Real phones.** The host is tested with simulated phones, up to 40 at once. How many real phones one home router keeps responsive is still unmeasured; the estimate is 30 to 60 busy phones.
