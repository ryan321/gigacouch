# Giga Couch site

The v1 website described in [docs/v1.md](../../docs/v1.md): accounts, profiles, uploading web games, browsing, and playing them in Chrome with a controller. It's a Next.js and React app. Locally it keeps everything in SQLite and files on disk.

## Run it

Node 22.13 or later is needed, for its built-in SQLite. From this folder:

```
pnpm install
pnpm dev
```

Open **http://localhost:3000**. Use `localhost`, not `127.0.0.1`: games are served from `127.0.0.1`, as explained below.

To get a demo account and a sample game, run this while the site is running:

```
pnpm seed
```

That adds the account `demo`, password `couch-demo-password`, and uploads Star Scramble, a small game for up to four controllers and a keyboard.

From the repo root, `pnpm site` starts the dev server and `pnpm site:seed` seeds it.

For a production build, run `pnpm build`, then `pnpm start`.

## Fly playtest deployment

The port-3000 site is deployed at **https://gigacouch-platform.fly.dev**. The existing Fly machine and volume are
reused; the historical Rust platform data remains under `/data`, while this site uses `/data/site`.

From this folder, deploy with the installed Fly CLI:

```sh
fly deploy --remote-only --ha=false
cd game-origin
fly deploy --remote-only --ha=false
```

The remote Docker build installs the locked dependencies and creates Next's standalone server. No local runtime
or toolchain is installed. Deploy from `apps/site`, since the root Dockerfile and Fly config still describe the
R&D platform. Use a single site machine while SQLite and build files live on its volume.

`gigacouch-games.fly.dev` is a small gateway that only routes game-file requests. Fly's replay header preserves
the games hostname while sending the request to the site machine, so both origins use the same build files and
new uploads work immediately. Other game-origin paths redirect to the site; encoded traversal is refused before
nginx normalizes paths. Site cookies are secure and host-only. This is a shared games origin for the playtest;
per-game domains, object storage and Neon remain future work.

For the **first migration only**, `scripts/snapshot-for-fly.py --out <new-external-staging-folder>
--credentials <private-local-file>` backs up local SQLite consistently and archives the current game builds and
media. It removes sessions and unused smoke accounts, rotates the deployed demo password, and preserves other
accounts. It never changes local data. Historical build files stay local; subsequent uploaded versions are retained
on Fly normally. The initial transfer used a temporary Docker image containing the archive and an import script:
it validated SQLite and all three entry points, refused to overwrite live accounts or games, and retained the
previous data directory before switching to the snapshot. The regular site image was then restored with
`fly deploy --image <original-site-image> --ha=false`. Keep migration images and archives private. Never import
a local snapshot over an established live site: that would
discard newer accounts, uploads and plays. The demo credential file is private, outside the repository and archive.

Run the same checks on Fly with:

```sh
SITE=https://gigacouch-platform.fly.dev GAMES=https://gigacouch-games.fly.dev pnpm smoke
```

## Where things are kept

Everything lives in `data/`, which git ignores. Delete it to start over.

| Path | What |
| --- | --- |
| `data/site.sqlite` | Accounts, sessions, games, and builds |
| `data/builds/<build id>/` | Each uploaded build, unpacked |
| `data/media/` | Avatars, covers, and screenshots |

Set `DATA_DIR` to keep data somewhere else.

## Two origins

Uploaded games are other people's code. A game served from the site's own origin could act as whoever is signed in. So games are served from a different origin, and the site's sign-in cookie never reaches them.

| Setting | Default | What |
| --- | --- | --- |
| `SITE_ORIGIN` | `http://localhost:3000` | Where people use the site |
| `GAMES_ORIGIN` | `http://127.0.0.1:3000` | Where game files are served |
| `PORT` | `3000` | Used for both defaults |

Locally, Chrome treats `localhost` and `127.0.0.1` as different sites. The same server answers both, but:

- **Game files** under `/g/` are only served on the games origin.
- **Site pages** opened on the games origin send people back to the site.
- **Changes** such as uploads and sign-ins are refused unless they come from the site's own pages.

The play page shows the game in a sandboxed frame from the games origin. The frame can use pads, full screen, sound, and mouse lock. In production, `GAMES_ORIGIN` becomes a separate domain.

## Controllers on the play page

The play screen starts with **Download**. Clicking it downloads the uploaded build into browser storage and
shows progress; the button becomes **Play** only after every file is saved. The game does not execute during
the download. Play enters the game and full screen, and counts the play. A controller's bottom face button
does the same two steps. Failed or storage-limited downloads offer Retry; completed files are reused.

Downloads continue across site navigation. The upper-right Downloads panel shows the active transfer and queue;
`/downloads` lists saved builds, size, saved date, Pause/Resume and Remove. Closing or reloading the tab pauses
unfinished downloads, preserving complete files for Resume. New versions download separately; saved versions
remain available until removed. The saved player stays in the shared layout so other queued downloads continue.

The public offline shell (`/offline.html`) is cached on the account origin without caching account pages or API
responses. When the server cannot be reached, opening the site shows that library. Each game's own worker and
cache stay on the games origin, including the small helper needed to check/remove saved files offline. Saved
files are checked on reopening; missing files turn into resumable downloads rather than a false Ready state.
Only downloaded, self-contained game builds can run completely offline: external services still need a network.
Browser eviction or clearing site data can remove downloads. Closing the browser does not continue downloading.

The library and menus adapt to phone portrait, landscape, and tablet sizes, with touch-sized controls. Games
still supply their own touch controls and graphics settings; this pass does not create separate mobile builds.

`scripts/check-library.cjs` runs with an already-installed Electron. `ISOLATED_TEST=1` starts the built standalone
site on port 3101 using the existing Node 22, shuts that server down, and opens a new window offline. Set
`GAME_SLUG=spooky-game-browser-version` for the complete Spooky runtime, or `TEST_QUEUE=1` with Star Scramble
for queue ordering. The checks exercise navigation during download, pause/reload/resume, cold offline launch,
cache eviction, recovery, removal, and phone/tablet layouts. The test profile stays on the external drive. Local
cold-offline checks passed for Star Scramble and Spooky Game; real phone hardware and controllers remain untested.
`WORKER_UPGRADE=1 ISOLATED_TEST=1` also checks migration from the previous download worker and loader. The
downloader explicitly updates and waits for the new worker before declaring the saved game ready offline.
For a live deployed site, use `SITE=<origin> ONLINE_ONLY=1` to check navigation and responsive layouts; Electron's
network emulation alone does not reliably cut off service-worker requests, so it is not our cold-offline proof.
The Fly deployment passed that browser check and all 40 HTTP smoke checks. Downloads made before the library
was introduced need one click of Download to add their listing and offline helpers; existing game files are reused.

The play page reads pads itself, alongside the game. Chrome gives pad data to every visible frame, whichever one has focus.

- **Starting:** the bottom face button (A on Xbox) starts the game.
- **The system menu:** holding Select and Start together for 0.8 seconds opens it, and so does Home where Chrome reports it. It has Resume, Restart game, Full screen, and Leave game.
- **A hint:** holding just one of the two buttons shows a hint to hold both. It also shows the page is seeing the press.
- **Button names:** the ready screen and the menu name the buttons for the pad in use, so Xbox, PlayStation, and Switch players see their own labels.

`src/components/pads.ts` has the button numbers, the names, and the press tracking. `src/components/Player.tsx` has the ready screen and the menu.

## Checks

```
pnpm typecheck
pnpm smoke
```

`pnpm smoke` needs the site running. It drives the site over HTTP the way a browser would. It covers:

- **Accounts:** sign-up and sign-in rules.
- **Uploads:** the build checks.
- **Pages and visibility:** browsing, search, filters, drafts, and unlisted games.
- **Game files:** they're served only on the games origin.
- **Game changes:** play counts, new builds, editing, and deleting.

It doesn't check real controllers. Try those by hand on the home page's controller panel and in Star Scramble.

## Code

| Path | What |
| --- | --- |
| `src/app/(site)/` | The pages with the site header: home, games, a game's page, edit, profiles, settings, upload, sign up, and sign in |
| `src/app/play/[slug]/` | The full-screen play page |
| `src/app/api/` | Form endpoints for accounts, profiles, games, builds, and play counts |
| `src/app/g/` | Game files, served on the games origin |
| `src/app/media/` | Uploaded images |
| `src/lib/` | Server code: database, sign-in, build checks, game queries |
| `src/components/` | Page parts and forms. `PadPanel` is the live controller on the home page. `Player` runs the play page. |
| `samples/star-scramble/` | The sample game the seed uploads |
| `scripts/smoke.mjs` | The end-to-end checks and the seed |

Database changes go at the end of the list in `src/lib/db.ts`. Don't edit one that has already run.

## Not built yet

- **Account recovery:** email sign-in and password reset need an email service.
- **Rollback:** old builds are kept, but there's no button to roll back to one yet.
- **Safety:** reporting, admin tools, and the legal pages.
- **Production hosting:** per-game domains, object storage, and Neon instead of SQLite.
