# Giga Couch brand assets

Source artwork, fonts, and the shared design tokens for every surface. Do not put USB or game code here.

## Artwork

| File | Use |
| --- | --- |
| `mark.png` | App icon and dark UI (Game Player splash, Setup rail, library, Creator Hub). Transparent mint couch. |
| `logo.png` | Wordmark for light surfaces. Navy type plus mark. |

`scripts/branding.py` builds `AppIcon.icns` from `mark.png` during `build_player.py` and `build_gdk.py`. The Game Browser (`build_browser.py`) uses a macOS tile instead: the mark is redrawn in exact mint and navy, which drops the faint white specks around the arm gaps, and placed on a rounded rail-gradient tile. That step needs Pillow. Godot projects keep a copy of the mark next to their scenes so `res://` loads work.

## Fonts (`brand/fonts/`)

All surfaces use the same two fonts, with system fonts as fallback. Both are SIL Open Font License; the license texts ship next to the files.

| Font | Role | Files |
| --- | --- | --- |
| Space Grotesk | Display: headings, wordmark, PIN/code entry | `SpaceGrotesk.ttf` (Godot/AppKit), `SpaceGrotesk.woff2` (web) |
| Inter | UI text: body, buttons, labels | `Inter.ttf`, `Inter.woff2` |

Code stays on the system mono font.

## Design tokens

Every surface (Fly site, Web Home, GDK Setup, player splash, Godot launcher, Creator Hub, GDK docs page) consumes these same values. When you change a token, change it in every surface's token block; do not let copies drift.

**Palette**

| Token | Value |
| --- | --- |
| `bg` | `#101722` |
| `panel` | `#192331` / `#1a2736` |
| `line` | `#2d3b4b` |
| `ink` | `#f2f5f8` |
| `muted` | `#a0afbf` |
| `mint` | `#8ce8be` |
| `blue` | `#9abef7` |
| `amber` | `#f3c77d` |
| `footer` | `#142b28` |
| rail gradient | `#213d3c → #16272d → #111d2a` |

**Elevation & glow**

- Focus/hover halo: mint at 18–28% alpha.
- Panels and cards: soft drop shadow, roughly `0 8px 24px rgba(0,0,0,0.35)`.
- Panels get a subtle top-light gradient instead of a flat fill; cover art gets a top-light gradient, an edge vignette, and a bottom scrim behind titles.

**Motion**

| Name | Value | Use |
| --- | --- | --- |
| fast | 120ms ease-out | hover, press |
| focus | 180ms cubic-bezier(.2,.8,.2,1) | focus moves, scale 1.03–1.05 |
| page | 300ms | fade/slide-in on load |

Godot: Tweens with `TRANS_CUBIC` / `EASE_OUT`. Always honor Reduce Motion / `prefers-reduced-motion`.
