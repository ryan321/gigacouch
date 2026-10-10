// End-to-end checks against a running site, over HTTP, the way a browser uses it.
//
//   node scripts/smoke.mjs          run every check with a throwaway account
//   node scripts/smoke.mjs --seed   add a demo account and the Star Scramble sample
//
// SITE and GAMES set the two origins (default http://localhost:3000 and
// http://127.0.0.1:3000). The site must already be running.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { zipSync, strToU8 } from "fflate";

const SITE = (process.env.SITE ?? "http://localhost:3000").replace(/\/$/, "");
const GAMES = (process.env.GAMES ?? "http://127.0.0.1:3000").replace(/\/$/, "");
const here = path.dirname(fileURLToPath(import.meta.url));
const samples = path.join(here, "..", "samples");

/** A browser stand-in: keeps cookies and sends the site's Origin on changes. */
function browser() {
  const jar = new Map();
  async function request(url, options = {}) {
    const headers = new Headers(options.headers);
    if (jar.size) headers.set("cookie", [...jar].map(([k, v]) => `${k}=${v}`).join("; "));
    if (options.method && options.method !== "GET" && !options.noOrigin) headers.set("origin", SITE);
    const response = await fetch(url.startsWith("http") ? url : SITE + url, { ...options, headers, redirect: "manual" });
    for (const cookie of response.headers.getSetCookie()) {
      const [pair] = cookie.split(";");
      const [name, ...rest] = pair.split("=");
      const value = rest.join("=");
      if (!value || /max-age=0|expires=thu, 01 jan 1970/i.test(cookie)) jar.delete(name);
      else jar.set(name, value);
    }
    return response;
  }
  return {
    jar,
    get: (url) => request(url),
    async form(url, fields, method = "POST", extra = {}) {
      const body = new FormData();
      for (const [key, value] of Object.entries(fields)) {
        for (const item of Array.isArray(value) ? value : [value]) body.append(key, item);
      }
      const response = await request(url, { method, body, ...extra });
      const text = await response.text();
      let json = null;
      try { json = JSON.parse(text); } catch { /* not JSON */ }
      return { status: response.status, json, text };
    },
  };
}

function sampleFiles(prefix = "") {
  const dir = path.join(samples, "star-scramble");
  const files = {};
  for (const name of fs.readdirSync(dir)) files[prefix + name] = new Uint8Array(fs.readFileSync(path.join(dir, name)));
  return files;
}

const zipFile = (files, name = "game.zip") => new File([zipSync(files)], name, { type: "application/zip" });
const cover = () => new File([fs.readFileSync(path.join(samples, "star-scramble-cover.png"))], "cover.png", { type: "image/png" });

const starScramble = {
  title: "Star Scramble",
  description:
    "Grab more stars than everyone else before the clock runs out.\n\nPress any button on a controller to join, or Enter on the keyboard. Up to four controllers and a keyboard can play at once. Move with the left stick, the d-pad, WASD, or the arrow keys.",
  tags: "party, arcade",
  playersMin: "1",
  playersMax: "4",
  gamepad: "on",
  keyboard: "on",
  visibility: "public",
};

async function seed() {
  const demo = browser();
  const account = { email: "demo@gigacouch.test", handle: "demo", displayName: "Giga Couch Demo", password: "couch-demo-password", oldEnough: "on" };
  let result = await demo.form("/api/auth/signup", account);
  if (!result.json?.ok) {
    result = await demo.form("/api/auth/signin", { login: account.handle, password: account.password });
    if (!result.json?.ok) throw new Error(`Couldn't sign in as the demo account: ${result.text}`);
  }
  const profile = await demo.get("/u/demo");
  if ((await profile.text()).includes("Star Scramble")) {
    console.log("The demo account already has Star Scramble.");
  } else {
    result = await demo.form("/api/games", { ...starScramble, build: zipFile(sampleFiles()), cover: cover() });
    if (!result.json?.ok) throw new Error(`Upload failed: ${result.text}`);
    console.log(`Uploaded Star Scramble: ${SITE}/games/${result.json.slug}`);
  }
  console.log(`Demo account: handle "demo", password "${account.password}".`);
}

async function smoke() {
  const results = [];
  const check = (name, ok, detail = "") => {
    results.push({ name, ok });
    console.log(`${ok ? "ok  " : "FAIL"} ${name}${!ok && detail ? `\n     ${detail}` : ""}`);
  };
  const me = browser();
  const stranger = browser();
  const handle = `smoke_${Date.now().toString(36)}`.slice(0, 20);
  const password = "smoke-test-password";

  const home = await me.get("/");
  check("Home page loads", home.status === 200 && (await home.text()).includes("Web games you play with a controller."));

  let r = await me.form("/api/auth/signup", { email: `${handle}@gigacouch.test`, handle, displayName: "Smoke Test", password, oldEnough: "on" });
  check("Sign up creates an account and signs in", r.json?.ok === true && me.jar.has("gc_session"), r.text);
  r = await stranger.form("/api/auth/signup", { email: `other-${handle}@gigacouch.test`, handle, password, oldEnough: "on" });
  check("A taken handle is refused", r.status === 400 && /taken/.test(r.json?.errors?.handle ?? ""), r.text);
  r = await stranger.form("/api/auth/signup", { email: "nope", handle: "Bad Handle", password: "short" });
  check("Sign up explains every problem at once", ["email", "handle", "password", "oldEnough"].every((key) => r.json?.errors?.[key]), r.text);

  r = await me.form("/api/games", { ...starScramble, build: zipFile(sampleFiles()) }, "POST", { noOrigin: true });
  check("Changes without the site's Origin are refused", r.status === 403, r.text);
  r = await stranger.form("/api/games", { ...starScramble, build: zipFile(sampleFiles()) });
  check("Uploading needs an account", r.status === 401, r.text);

  r = await me.form("/api/games", { ...starScramble, build: zipFile({ "game.js": strToU8("x") }) });
  check("A build without index.html is refused", r.status === 400 && /index\.html/.test(r.json?.errors?.build ?? ""), r.text);
  r = await me.form("/api/games", {
    ...starScramble,
    build: zipFile({ "index.html": strToU8("<script>const GODOT_THREADS_ENABLED = true;</script>") }),
  });
  check("A threaded Godot export is refused with how to fix it", /Thread Support/.test(r.json?.errors?.build ?? ""), r.text);
  r = await me.form("/api/games", { ...starScramble, build: zipFile({ "index.html": strToU8("hi"), "tool.exe": strToU8("MZ") }) });
  check("Files a web game doesn't use are refused", /tool\.exe/.test(r.json?.errors?.build ?? ""), r.text);
  r = await me.form("/api/games", { ...starScramble, build: new File([strToU8("not a zip")], "game.zip") });
  check("A file that isn't a zip is refused", /isn't a zip/.test(r.json?.errors?.build ?? ""), r.text);
  r = await me.form("/api/games", { ...starScramble, title: "", playersMin: "3", playersMax: "2", gamepad: "", keyboard: "", build: zipFile(sampleFiles()) });
  check("Missing details are all reported", ["title", "players", "controls"].every((key) => r.json?.errors?.[key]), r.text);

  // Zipping the folder instead of its contents is common; it should still work.
  r = await me.form("/api/games", { ...starScramble, build: zipFile(sampleFiles("star-scramble/")), cover: cover() });
  check("Uploading a game works, even zipped inside a folder", r.json?.ok === true, r.text);
  const slug = r.json?.slug;
  const gameId = r.json?.id;

  const browse = await (await stranger.get("/games")).text();
  check("The game shows up in browsing", browse.includes(`/games/${slug}`));
  const search = await (await stranger.get("/games?q=scramble&players=4&control=gamepad")).text();
  check("Search and filters find it", search.includes(`/games/${slug}`));
  const wrongFilter = await (await stranger.get("/games?control=mouse&q=scramble")).text();
  check("Filters leave it out when it doesn't match", !wrongFilter.includes(`/games/${slug}`));

  const page = await stranger.get(`/games/${slug}`);
  const pageText = await page.text();
  check("The game page loads with a Play button", page.status === 200 && pageText.includes(`/play/${slug}`));
  const play = await (await stranger.get(`/play/${slug}`)).text();
  const build = play.match(/\/g\/([0-9a-f]{32})\/index\.html/)?.[1];
  check("The play page points at the games origin", Boolean(build) && play.includes(`${GAMES}/g/${build}/index.html`));

  const index = await fetch(`${GAMES}/g/${build}/index.html`);
  check(
    "Game files are served on the games origin",
    index.status === 200 && index.headers.get("content-type")?.startsWith("text/html") && /frame-ancestors/.test(index.headers.get("content-security-policy") ?? ""),
    `${index.status} ${index.headers.get("content-type")}`,
  );
  const script = await fetch(`${GAMES}/g/${build}/game.js`);
  check("Scripts get a JavaScript type and long caching", script.headers.get("content-type")?.startsWith("text/javascript") && /immutable/.test(script.headers.get("cache-control") ?? ""));
  const onSite = await fetch(`${SITE}/g/${build}/index.html`);
  check("Game files are never served on the site's origin", onSite.status === 404, String(onSite.status));
  const manifestResponse = await fetch(`${GAMES}/g/${build}/index.html?couch=manifest`);
  const manifest = await manifestResponse.json();
  check("Download manifest lists the uploaded files and size", manifestResponse.ok && manifest.files.some(file => file.path === "index.html") && manifest.bytes > 0);
  const privateManifest = await fetch(`${SITE}/g/${build}/index.html?couch=manifest`);
  check("Download helpers stay off the account origin", privateManifest.status === 404);
  const worker = await fetch(`${GAMES}/g/${build}/index.html?couch=worker`);
  check("Download worker is JavaScript and is not cached as a game asset", worker.ok && worker.headers.get("content-type").startsWith("text/javascript") && worker.headers.get("cache-control") === "no-store");
  const escape = await fetch(`${GAMES}/g/${build}/..%2f..%2fsite.sqlite`);
  check("Paths can't leave a build's folder", escape.status === 404, String(escape.status));
  const pageOnGames = await fetch(`${GAMES}/games`, { redirect: "manual" });
  check("Site pages on the games origin send people back to the site", [307, 308].includes(pageOnGames.status) && pageOnGames.headers.get("location")?.startsWith(SITE), String(pageOnGames.status));

  r = await stranger.form(`/api/games/${gameId}/plays`, {});
  check("Starting the game counts a play", r.json?.ok === true, r.text);
  check("The play count shows on the game page", (await (await stranger.get(`/games/${slug}`)).text()).includes("1 time"));

  r = await stranger.form(`/api/games/${gameId}`, { ...starScramble, title: "Taken over" });
  check("Only the owner can change a game", r.status === 401 || r.status === 404, r.text);
  r = await me.form(`/api/games/${gameId}`, { ...starScramble, visibility: "draft" });
  check("The owner can change a game's details", r.json?.ok === true, r.text);
  check("A draft leaves browsing", !(await (await stranger.get("/games")).text()).includes(`/games/${slug}`));
  check("A draft's page is hidden from others", (await stranger.get(`/games/${slug}`)).status === 404);
  check("A draft's page still opens for its owner", (await me.get(`/games/${slug}`)).status === 200);
  r = await me.form(`/api/games/${gameId}`, { ...starScramble, visibility: "unlisted" });
  const unlisted = await stranger.get(`/games/${slug}`);
  check("An unlisted game opens by link but stays out of browsing", unlisted.status === 200 && !(await (await stranger.get("/games")).text()).includes(`/games/${slug}`));

  r = await me.form(`/api/games/${gameId}/builds`, { build: zipFile(sampleFiles()) });
  const newPlay = await (await me.get(`/play/${slug}`)).text();
  const newBuild = newPlay.match(/\/g\/([0-9a-f]{32})\/index\.html/)?.[1];
  check("A new build replaces the live one", r.json?.ok === true && newBuild && newBuild !== build, r.text);

  r = await me.form("/api/profile", { displayName: "Smoke Tester", bio: "I check things." });
  const profile = await (await stranger.get(`/u/${handle}`)).text();
  check("Profiles can be edited and show the person's games", r.json?.ok === true && profile.includes("Smoke Tester") && profile.includes("I check things."), r.text);

  r = await me.form("/api/auth/signout", {});
  const upload = await me.get("/upload");
  check("Signing out works, and Upload asks you to sign in", r.json?.ok === true && upload.status === 307 && upload.headers.get("location")?.includes("/signin"), `${upload.status}`);
  r = await me.form("/api/auth/signin", { login: handle, password: "wrong password" });
  check("A wrong password is refused", r.status === 400, r.text);
  r = await me.form("/api/auth/signin", { login: handle, password });
  check("Signing in with a handle works", r.json?.ok === true, r.text);

  r = await me.form(`/api/games/${gameId}`, {}, "DELETE");
  check("The owner can delete a game", r.json?.ok === true && (await me.get(`/games/${slug}`)).status === 404, r.text);
  const gone = await fetch(`${GAMES}/g/${newBuild}/index.html`);
  check("A deleted game's files are gone", gone.status === 404);

  const failed = results.filter((result) => !result.ok).length;
  console.log(`\n${results.length - failed} of ${results.length} checks passed.`);
  process.exit(failed ? 1 : 0);
}

if (process.argv.includes("--seed")) await seed();
else await smoke();
