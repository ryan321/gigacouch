# Repository guidance

## Development constraints

- Do not download or install Godot, export templates, or engine binaries on this computer unless the user explicitly asks to do so. Disk space is limited.
- The user has authorized installing the standard Godot 4.7.2 editor for development and dogfooding. Export templates and additional engine versions are not included in that authorization.
- Local builds, tests, and CLI commands must not implicitly install a runtime or toolchain.
- Keep the low-disk Rust build profiles in `Cargo.toml`. Use synthetic byte fixtures for engine-independent tests and clearly label them as non-playable.
- Do not claim game execution, controller behavior, runtime compatibility, or sandboxing is verified by package-integrity tests.

## Architecture

- V1 is the website in `docs/v1.md`: accounts, profiles, uploading web games, browsing, and playing in plain Chrome with the controllers Chrome supports. It lives in `apps/site`, a Next.js and React app that uses SQLite locally; `apps/site/README.md` explains it. Everything else in the repo is R&D and prototyping; keep it, and treat `docs/v1.md` as the scope when docs disagree.
- Read `docs/v1.md` and the V1 checklist at the top of `IMPLEMENTATION_PLAN.md` first. `PRODUCT.md`, `TECH_STACK.md`, and `docs/GIGACOUCH_ARCHITECTURE_v2.md` describe the R&D direction and later stages.
- The R&D game browser is the web-1 shell in `runtimes/web/`: a Rust loopback origin plus the Electron kiosk. `python3 runtimes/web/fetch_shell.py` is the only supported way to download that shell, and it writes to `/Volumes/External`. Do not resume the partial CEF/Chromium compile under `/Volumes/External/projects/gigacouch-chromium` unless asked. Do not add a new native Godot runtime; the existing Godot prototype stays.
- On this Mac a Chromium source tree, if one is used, stays at `/Volumes/External/projects/gigacouch-chromium`. Do not check Chromium out onto the internal disk.
- Games execute on player-owned computers. SQLite is local; Neon PostgreSQL is accessible through the platform backend only.
- Reuse one installed runtime for compatible games. Keep game processes separate from the launcher and privileged host operations.
- Keep package identity and validation in `crates/manifests`, and local persistence/install behavior in `crates/local-library`.
- Treat unsigned imports as local developer content, not approved distribution artifacts.

## Verification

- For `apps/site` changes, run `pnpm typecheck` and `pnpm build` there, then `pnpm smoke` against a running site. Smoke checks don't verify real controllers.
- Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings` for relevant Rust changes.
- For SDK changes, run `python3 scripts/test_sdk.py` against an already-installed supported engine. It uses our own doctor command and never installs an engine.
- Keep the Rust host and SDK on the shared policy in `sdk/addons/couchgames/runtime_policy.json`; add compatibility cases to `sdk/tests/runtime_versions.json` when changing it.
- Update the implementation checklist and README when behavior changes; distinguish configured CI from checks actually run.
- Database migrations are embedded in `crates/local-library/src/migrations.rs`. Add new migrations to that list and do not rewrite migrations already released to users.
