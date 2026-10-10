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
