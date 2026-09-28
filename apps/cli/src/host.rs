//! Preview library host. Uses the installed Godot editor as the runtime; no downloads.

use anyhow::{Context, Result};
use couch_local_library::Library;
use couch_manifests::Target;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const SESSION_VARS: &[&str] = &[
    "COUCH_WII_NATIVE_STATE",
    "COUCH_WII_FLEET_DIR",
    "COUCH_XPAD_NATIVE_STATE",
    "COUCH_LIBRARY_SESSION",
    "COUCH_PLAYER_STARTUP",
    "COUCH_LIBRARY_CATALOG",
    "COUCH_SAVE_DIR",
    "COUCH_PROFILE",
];

pub async fn run(
    godot: &Path,
    root: &Path,
    sdk: &Path,
    data_dir: &Path,
    startup: Option<&Path>,
) -> Result<i32> {
    if let Some(path) = startup {
        write_json(path, &json!({ "phase": "checking" }))?;
        write_json(path, &json!({ "phase": "importing" }))?;
    }
    let import = Command::new(godot)
        .args(["--headless", "--editor", "--path"])
        .arg(sdk)
        .arg("--quit")
        .output()
        .context("could not start Godot to import the library")?;
    if !import.status.success() {
        anyhow::bail!(
            "Godot import failed: {}{}",
            String::from_utf8_lossy(&import.stdout),
            String::from_utf8_lossy(&import.stderr)
        );
    }
    let session_dir = std::env::temp_dir().join(format!("couch-library-{}", unix_now() as u64));
    fs::create_dir_all(&session_dir).context("could not create library session")?;
    eprintln!("Library session/logs: {}", session_dir.display());
    let mut host = Host::open(godot, root, sdk, data_dir, &session_dir).await?;
    if let Some(path) = startup {
        write_json(path, &json!({ "phase": "opening" }))?;
    }
    let mut ui = Command::new(godot)
        .arg("--path")
        .arg(sdk)
        .arg("res://launcher/library.tscn")
        .env("COUCH_LIBRARY_SESSION", &session_dir)
        .env("COUCH_LIBRARY_CATALOG", host.catalog_path.clone())
        .envs(startup.map(|path| ("COUCH_PLAYER_STARTUP", path.as_os_str())))
        .stdin(Stdio::null())
        .spawn()
        .context("could not open the library UI")?;
    let ui_pid = ui.id();
    let opened = Instant::now();
    let mut ready = startup.is_none();
    let mut foreground = 0;
    loop {
        if let Some(status) = ui.try_wait()? {
            host.cleanup();
            return Ok(status.code().unwrap_or(1));
        }
        if !ready && let Some(path) = startup {
            ready = startup_ready(path);
            if !ready && opened.elapsed() > Duration::from_secs(60) {
                host.cleanup();
                let _ = ui.kill();
                anyhow::bail!(
                    "The library did not finish drawing within 60 seconds. Please try again."
                );
            }
        }
        host.tick().await?;
        if let Some(path) = startup {
            let active = host.game.as_ref().map(|child| child.id()).unwrap_or(ui_pid);
            if active != foreground {
                write_json(
                    &path.with_file_name("foreground.json"),
                    &json!({ "pid": active, "library_pid": ui_pid }),
                )?;
                foreground = active;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

struct Host {
    godot: PathBuf,
    root: PathBuf,
    sdk: PathBuf,
    data_dir: PathBuf,
    session: PathBuf,
    catalog_path: PathBuf,
    games: Vec<Value>,
    state: Value,
    game: Option<Child>,
    helper: Option<Child>,
    log: Option<fs::File>,
}

impl Host {
    async fn open(
        godot: &Path,
        root: &Path,
        sdk: &Path,
        data_dir: &Path,
        session: &Path,
    ) -> Result<Self> {
        let player = load_player_state(data_dir);
        let mut host = Self {
            godot: godot.to_path_buf(),
            root: root.to_path_buf(),
            sdk: sdk.to_path_buf(),
            data_dir: data_dir.to_path_buf(),
            session: session.to_path_buf(),
            catalog_path: session.join("catalog.json"),
            games: Vec::new(),
            state: json!({
                "phase": "idle",
                "message": "Choose something to play.",
                "native_wii": cfg!(target_os = "macos"),
                "profile": player["profile"],
                "profiles": player["profiles"],
                "saves": data_dir.join("saves").display().to_string(),
                "catalog_revision": 0
            }),
            game: None,
            helper: None,
            log: None,
        };
        host.reload_catalog().await?;
        host.publish()?;
        Ok(host)
    }

    async fn reload_catalog(&mut self) -> Result<()> {
        self.games = shelf_catalog(&self.sdk, &self.data_dir).await?;
        self.state["catalog_revision"] =
            json!(self.state["catalog_revision"].as_u64().unwrap_or(0) + 1);
        write_json(&self.catalog_path, &Value::Array(self.games.clone()))
    }

    fn publish(&self) -> Result<()> {
        let mut state = self.state.clone();
        state["updated"] = json!(unix_now());
        write_json(&self.session.join("status.json"), &state)
    }

    fn cleanup(&mut self) {
        stop(&mut self.game);
        stop(&mut self.helper);
        self.log = None;
    }

    async fn tick(&mut self) -> Result<()> {
        if let Some(child) = &mut self.game
            && let Some(status) = child.try_wait()?
        {
            let ok = status.success();
            self.game = None;
            self.log = None;
            self.state["phase"] = json!(if ok { "idle" } else { "error" });
            self.state["message"] = json!(if ok {
                "Welcome back. Pick your next game."
            } else {
                "The game closed unexpectedly. You can try again."
            });
        }
        let request_path = self.session.join("request.json");
        if request_path.exists() {
            let result = self.handle_request(&request_path).await;
            let _ = fs::remove_file(&request_path);
            if let Err(error) = result {
                self.cleanup();
                self.state["phase"] = json!("error");
                self.state["message"] = json!(error.to_string());
            }
        }
        self.publish()
    }

    async fn handle_request(&mut self, path: &Path) -> Result<()> {
        let bytes = fs::read(path)?;
        if bytes.len() > 4096 {
            anyhow::bail!("Library request is too large.");
        }
        let request: Value = serde_json::from_slice(&bytes)?;
        self.state["request_id"] = json!(
            request
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or("")
        );
        match request.get("action").and_then(Value::as_str) {
            Some("launch") => self.launch(&request),
            Some("stop") => {
                self.cleanup();
                self.state["phase"] = json!("idle");
                self.state["message"] = json!("Back to your games.");
                Ok(())
            }
            Some("refresh") => {
                self.reload_catalog().await?;
                self.state["phase"] = json!("idle");
                self.state["message"] = json!("Library updated.");
                Ok(())
            }
            Some("profile") => {
                let name = request.get("profile").and_then(Value::as_str).unwrap_or("");
                let mut player = load_player_state(&self.data_dir);
                let profiles = player["profiles"].as_array().cloned().unwrap_or_default();
                if !profiles.iter().any(|value| value.as_str() == Some(name)) {
                    anyhow::bail!("Unknown player profile.");
                }
                player["profile"] = json!(name);
                save_player_state(&self.data_dir, &player)?;
                self.state["profile"] = json!(name);
                self.state["phase"] = json!("idle");
                self.state["message"] = json!(format!("Playing as {name}."));
                Ok(())
            }
            _ => anyhow::bail!("Unknown library action."),
        }
    }

    fn launch(&mut self, request: &Value) -> Result<()> {
        if self.game.is_some() {
            return Ok(());
        }
        let game_id = request
            .get("game")
            .and_then(Value::as_str)
            .context("That game or controller setup is unavailable.")?;
        let game = self
            .games
            .iter()
            .find(|row| row["id"].as_str() == Some(game_id))
            .cloned()
            .context("That game or controller setup is unavailable.")?;
        if game["missing"].as_bool() == Some(true) {
            anyhow::bail!(
                "This project folder is missing or unreadable. Open it again from Creator Hub."
            );
        }
        if game["playable"].as_bool() == Some(false) {
            anyhow::bail!(
                "{}",
                game["reason"]
                    .as_str()
                    .unwrap_or("This package isn't playable yet.")
            );
        }
        let profile = request
            .get("profile")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| {
                self.state["profile"]
                    .as_str()
                    .unwrap_or("family")
                    .to_string()
            });
        self.state["phase"] = json!("starting");
        self.state["game"] = json!(game_id);
        self.state["message"] = json!(format!(
            "Opening {}…",
            game["title"].as_str().unwrap_or("game")
        ));
        self.publish()?;
        let mode = request
            .get("input")
            .and_then(Value::as_str)
            .unwrap_or("standard");
        let joycons = request
            .get("joycons")
            .and_then(Value::as_str)
            .unwrap_or("separate");
        let log = fs::File::create(self.session.join("game.log"))?;
        let saves = save_dir(&self.data_dir, &profile, game_id)?;
        let mut command = Command::new(&self.godot);
        command.stdin(Stdio::null());
        command.stdout(log.try_clone()?);
        command.stderr(Stdio::from(log.try_clone()?));
        strip_session_vars(&mut command);
        command.env("COUCH_SAVE_DIR", &saves);
        command.env("COUCH_PROFILE", &profile);
        command.env(
            "SDL_JOYSTICK_HIDAPI_COMBINE_JOY_CONS",
            if joycons == "paired" { "1" } else { "0" },
        );
        command.env("SDL_JOYSTICK_HIDAPI_VERTICAL_JOY_CONS", "0");
        if mode == "sdl-wii" {
            command.env("SDL_JOYSTICK_HIDAPI_WII", "1");
        }
        let is_sample = game
            .get("project")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .is_none();
        if mode == "native-wii"
            && (game.get("native_wii").and_then(Value::as_str).is_some() || is_sample)
        {
            let fleet = game["native_wii"].as_str() == Some("fleet");
            let binary = self.root.join(if fleet {
                ".gigacouch/Wii Fleet.app/Contents/MacOS/CouchWiiReader"
            } else {
                ".gigacouch/Wii Reader.app/Contents/MacOS/CouchWiiReader"
            });
            if !binary.is_file() {
                anyhow::bail!(
                    "The Wii helper is not built yet. Choose Gamepads / keyboard, or run python3 scripts/library.py once to compile it."
                );
            }
            let helper_dir = self.session.join("wii");
            fs::create_dir_all(&helper_dir)?;
            let state = if fleet {
                helper_dir.clone()
            } else {
                helper_dir.join("state.json")
            };
            let mut helper = Command::new(&binary);
            helper.arg(&state).stdin(Stdio::null());
            helper.stdout(log.try_clone()?);
            helper.stderr(Stdio::from(log.try_clone()?));
            self.helper = Some(helper.spawn().context("Cannot start Wii helper")?);
            if fleet {
                command.env("COUCH_WII_FLEET_DIR", &state);
            } else {
                command.env("COUCH_WII_NATIVE_STATE", &state);
            }
        }
        attach_game_target(&mut command, &game, &self.sdk, &self.root)?;
        self.log = Some(log);
        self.game = Some(command.spawn().context("Cannot start engine")?);
        self.state["phase"] = json!("running");
        self.state["message"] = json!(format!(
            "{} is playing. Close its window to come back.",
            game["title"].as_str().unwrap_or("The game")
        ));
        Ok(())
    }
}

pub struct RunningGame {
    pub child: Child,
    pub title: String,
}

/// Start one Godot game in its own window. The caller keeps the shelf process alive.
pub fn spawn_listed_game(
    godot: &Path,
    root: &Path,
    sdk: &Path,
    data_dir: &Path,
    session: &Path,
    game: &Value,
    profile: &str,
) -> Result<RunningGame> {
    if game["missing"].as_bool() == Some(true) {
        anyhow::bail!(
            "This project folder is missing or unreadable. Open it again from Creator Hub."
        );
    }
    if game["playable"].as_bool() == Some(false) {
        anyhow::bail!(
            "{}",
            game["reason"]
                .as_str()
                .unwrap_or("This package isn't playable yet.")
        );
    }
    let game_id = game
        .get("id")
        .and_then(Value::as_str)
        .context("That game or controller setup is unavailable.")?;
    let title = game["title"].as_str().unwrap_or("The game").to_string();
    fs::create_dir_all(session)?;
    let log = fs::File::create(session.join("game.log"))?;
    let saves = save_dir(data_dir, profile, game_id)?;
    let mut command = Command::new(godot);
    command.stdin(Stdio::null());
    command.stdout(log.try_clone()?);
    command.stderr(Stdio::from(log));
    strip_session_vars(&mut command);
    command.env("COUCH_SAVE_DIR", &saves);
    command.env("COUCH_PROFILE", profile);
    command.env("SDL_JOYSTICK_HIDAPI_COMBINE_JOY_CONS", "0");
    command.env("SDL_JOYSTICK_HIDAPI_VERTICAL_JOY_CONS", "0");
    attach_game_target(&mut command, game, sdk, root)?;
    let child = command.spawn().context("Cannot start engine")?;
    Ok(RunningGame { child, title })
}

fn attach_game_target(command: &mut Command, game: &Value, sdk: &Path, root: &Path) -> Result<()> {
    if let Some(pck) = game["pck"].as_str().filter(|value| !value.is_empty()) {
        command.arg("--main-pack").arg(pck);
        if let Some(parent) = Path::new(pck).parent() {
            command.current_dir(parent);
        }
        return Ok(());
    }
    let project = game["project"].as_str().filter(|value| !value.is_empty());
    let path = project
        .map(PathBuf::from)
        .unwrap_or_else(|| sdk.to_path_buf());
    let scene = game["scene"]
        .as_str()
        .filter(|value| value.starts_with("res://"))
        .context("That game or controller setup is unavailable.")?;
    if !path.join("project.godot").is_file() {
        anyhow::bail!(
            "This project folder is missing or unreadable. Open it again from Creator Hub."
        );
    }
    command.arg("--path").arg(&path).arg(scene);
    command.current_dir(if project.is_some() {
        path
    } else {
        root.to_path_buf()
    });
    Ok(())
}

fn strip_session_vars(command: &mut Command) {
    for key in SESSION_VARS {
        command.env_remove(key);
    }
}

fn stop(child: &mut Option<Child>) {
    if let Some(process) = child.as_mut() {
        let _ = process.kill();
        let _ = process.wait();
    }
    *child = None;
}

fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs_f64())
        .unwrap_or(0.0)
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(serde_json::to_string(value)?.as_bytes())?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn startup_ready(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| {
            value
                .get("phase")
                .and_then(Value::as_str)
                .map(|phase| phase == "ready")
        })
        .unwrap_or(false)
}

fn load_player_state(data_dir: &Path) -> Value {
    let path = data_dir.join("player.json");
    if let Ok(text) = fs::read_to_string(path)
        && let Ok(value) = serde_json::from_str::<Value>(&text)
        && value.get("profile").and_then(Value::as_str).is_some()
    {
        return value;
    }
    json!({ "profile": "family", "profiles": ["family", "guest"] })
}

fn save_player_state(data_dir: &Path, value: &Value) -> Result<()> {
    write_json(&data_dir.join("player.json"), value)
}

fn save_dir(data_dir: &Path, profile: &str, game_id: &str) -> Result<PathBuf> {
    let clean = |value: &str| {
        value
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
            .collect::<String>()
    };
    let path = data_dir
        .join("saves")
        .join(if clean(profile).is_empty() {
            "family".into()
        } else {
            clean(profile)
        })
        .join(if clean(game_id).is_empty() {
            "game".into()
        } else {
            clean(game_id)
        });
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn pck_is_playable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if path.extension().and_then(|ext| ext.to_str()) != Some("pck") || metadata.len() < 64 {
        return false;
    }
    fs::read(path)
        .ok()
        .map(|bytes| bytes.starts_with(b"GDPC"))
        .unwrap_or(false)
}

pub async fn shelf_catalog(sdk: &Path, data_dir: &Path) -> Result<Vec<Value>> {
    // The packaged browser ships without the SDK samples, so a missing list
    // means no built-in Godot games rather than an empty shelf.
    let builtin: Vec<Value> = match fs::read_to_string(sdk.join("launcher/games.json")) {
        Ok(text) => serde_json::from_str(&text)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error.into()),
    };
    let mut games = Vec::new();
    let mut used = std::collections::HashSet::new();
    for mut game in builtin {
        if let Some(row) = game.as_object_mut() {
            row.entry("source").or_insert(json!("sample"));
            row.entry("playable").or_insert(json!(true));
            row.entry("missing").or_insert(json!(false));
        }
        if let Some(id) = game.get("id").and_then(Value::as_str) {
            used.insert(id.to_string());
        }
        games.push(game);
    }
    let registry_path = std::env::var_os("COUCH_CREATOR_PROJECTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("creator-projects.json"));
    if let Ok(text) = fs::read_to_string(registry_path)
        && let Ok(registry) = serde_json::from_str::<Value>(&text)
        && let Some(projects) = registry.get("projects").and_then(Value::as_array)
    {
        for row in projects {
            let project = PathBuf::from(row.get("path").and_then(Value::as_str).unwrap_or(""));
            let slug = row.get("id").and_then(Value::as_str).unwrap_or_else(|| {
                project
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("game")
            });
            let id = if slug.starts_with("local:") {
                slug.to_string()
            } else {
                format!("local:{slug}")
            };
            if used.contains(&id) {
                continue;
            }
            used.insert(id.clone());
            let present = project.join("project.godot").is_file();
            let title = row.get("title").and_then(Value::as_str).unwrap_or(slug);
            games.push(json!({
                    "id": id,
                    "title": title,
                    "players": "1–16 players",
                    "description": if present { "Your game." } else { "Folder missing. Open it again from Creator Hub." },
                    "scene": "res://examples/little_world/world.tscn",
                    "color": "8ce8be",
                    "project": project.display().to_string(),
                    "source": "creator",
                    "missing": !present,
                    "playable": present,
                    "reason": if present { "" } else { "This project folder moved or was removed." }
                }));
        }
    }
    if let Ok(library) = Library::open(data_dir).await {
        let target = Target::current()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "macos-aarch64".into());
        let listed = library.list().await;
        if let Ok(releases) = listed {
            for release in releases
                .into_iter()
                .filter(|row| row.active && row.target == target)
            {
                let id = format!("installed:{}", release.game_id);
                if used.contains(&id) {
                    continue;
                }
                used.insert(id.clone());
                let content = data_dir.join(&release.relative_path);
                let pck = fs::read_dir(&content).ok().and_then(|entries| {
                    entries
                        .flatten()
                        .map(|entry| entry.path())
                        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("pck"))
                });
                let playable = pck.as_deref().is_some_and(pck_is_playable);
                games.push(json!({
                    "id": id,
                    "title": release.title,
                    "players": "1–16 players",
                    "description": if playable { "Unsigned local install." } else { "Installed package record — not a playable Godot pack yet." },
                    "scene": "",
                    "color": "8ce8be",
                    "source": "installed",
                    "playable": playable,
                    "reason": if playable { "" } else { "This unsigned import has no playable Godot pack." },
                    "pck": pck.as_ref().map(|path| path.display().to_string()).unwrap_or_default(),
                    "content": content.display().to_string()
                }));
            }
        }
        library.close().await;
    }
    Ok(games)
}

#[cfg(test)]
mod launch_tests {
    use super::{RunningGame, spawn_listed_game};
    use serde_json::Value;
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

    #[test]
    fn spawn_listed_game_opens_the_scene_in_a_separate_process() {
        let dir = tempfile::tempdir().unwrap();
        let godot = dir.path().join("godot");
        fs::write(
            &godot,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$(dirname \"$0\")/args.txt\"\nprintf '%s' \"$COUCH_PROFILE\" > \"$(dirname \"$0\")/profile.txt\"\nprintf '%s' \"$COUCH_SAVE_DIR\" > \"$(dirname \"$0\")/save.txt\"\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&godot).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&godot, permissions).unwrap();
        let sdk = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sdk");
        let root = sdk.parent().unwrap().to_path_buf();
        let games: Vec<Value> =
            serde_json::from_str(&fs::read_to_string(sdk.join("launcher/games.json")).unwrap())
                .unwrap();
        let game = games
            .iter()
            .find(|row| row["id"] == "little-world")
            .unwrap();
        let RunningGame { mut child, title } = spawn_listed_game(
            &godot,
            &root,
            &sdk,
            &dir.path().join("data"),
            &dir.path().join("session"),
            game,
            "family",
        )
        .unwrap();
        assert_eq!(title, "Little World");
        assert!(child.wait().unwrap().success());
        let args = fs::read_to_string(dir.path().join("args.txt")).unwrap();
        assert!(args.contains("--path"), "{args}");
        assert!(
            args.contains("res://examples/little_world/world.tscn"),
            "{args}"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("profile.txt")).unwrap(),
            "family"
        );
        let save = fs::read_to_string(dir.path().join("save.txt")).unwrap();
        assert!(save.contains("family/little-world"), "{save}");
    }
}
