use anyhow::{Context, Result, anyhow};
use couch_web_host::{Host, PlayRequest, WebPackage};
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::mpsc,
    thread,
    time::Duration,
};

/// How Home is presented. `Launch` starts the Electron shell from this
/// checkout. `ServeOnly` is used by the packaged app: the app's own Electron
/// process already exists and reads the origin from our JSON line, and it
/// closes our stdin when it quits.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HomeShell {
    Launch { windowed: bool },
    ServeOnly,
}

pub async fn home(
    shell_mode: HomeShell,
    json: bool,
    data_dir: &Path,
    godot_override: Option<PathBuf>,
    godot_timeout: Duration,
    platform: &str,
) -> Result<()> {
    let root = home_root().context(
        "could not find runtimes/web/home; set GIGACOUCH_ROOT or run from this checkout",
    )?;
    let home_dir = root.join("runtimes/web/home");
    let sdk = root.join("sdk");
    let blob = root.join("runtimes/web/examples/blob-island/web");
    let profiles = data_dir.join("home-profiles.json");
    let session = data_dir.join("home-session");
    let launch = match shell_mode {
        HomeShell::Launch { windowed } => {
            let shell = shell_dir()
                .context("could not find runtimes/web/shell/main.js from this checkout")?;
            let electron = electron_binary().ok_or_else(|| {
                anyhow!(
                    "the web-1 browser shell is not installed. Run python3 runtimes/web/fetch_shell.py on this Mac. The couch CLI does not download it"
                )
            })?;
            Some((shell, electron, windowed))
        }
        HomeShell::ServeOnly => None,
    };
    let mut discovery = couch_runtime::Discovery::system(godot_override);
    discovery.probe_timeout = godot_timeout;
    let report = discovery.check().await;
    let godot = if report.supported {
        report.executable
    } else {
        None
    };
    let catalog = crate::host::shelf_catalog(&sdk, data_dir)
        .await
        .unwrap_or_default();
    let (play_tx, play_rx) = mpsc::channel();
    let host = Host::start_home_with(
        &home_dir,
        &[("blob-island", blob.as_path())],
        &profiles,
        Some(home_cards(&catalog)),
        Some(play_tx),
        Some(couch_web_host::AccountStore {
            base: platform.trim_end_matches('/').to_string(),
            token_path: data_dir.join("account.json"),
            install_root: data_dir.join("installed"),
        }),
    )?;
    announce(
        &host,
        "Home",
        launch.as_ref().map(|(_, electron, _)| electron.as_path()),
        json,
    )?;
    let mut child = match &launch {
        Some((shell, electron, windowed)) => Some(
            Command::new(electron)
                .arg(shell)
                .env("GIGACOUCH_ORIGIN", host.origin())
                .env("GIGACOUCH_TITLE", "Giga Couch")
                .env("GIGACOUCH_WINDOWED", if *windowed { "1" } else { "0" })
                .spawn()
                .with_context(|| format!("could not start {}", electron.display()))?,
        ),
        None => None,
    };
    let parent_gone = child.is_none().then(watch_stdin_close);
    let mut running: Option<Child> = None;
    let status = loop {
        if let Some(shell) = child.as_mut() {
            if let Some(status) = shell.try_wait()? {
                break Some(status);
            }
        } else if host.game_quit()
            || parent_gone
                .as_ref()
                .is_some_and(|gone| gone.load(std::sync::atomic::Ordering::SeqCst))
        {
            break None;
        }
        if let Some(game) = running.as_mut()
            && game.try_wait()?.is_some()
        {
            running = None;
        }
        if let Ok(request) = play_rx.try_recv() {
            answer_play(
                &request,
                &LaunchPaths {
                    godot: &godot,
                    root: &root,
                    sdk: &sdk,
                    data_dir,
                    session: &session,
                },
                &catalog,
                &mut running,
            );
        }
        thread::sleep(Duration::from_millis(50));
    };
    if let Some(mut game) = running {
        let _ = game.kill();
        let _ = game.wait();
    }
    if let Some(status) = status
        && !status.success()
    {
        anyhow::bail!("browser shell exited with {status}");
    }
    Ok(())
}

/// Sets the flag once stdin reaches end of file. The packaged app holds our
/// stdin open for its whole life, so this also catches a crashed shell.
fn watch_stdin_close() -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    let gone = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&gone);
    thread::spawn(move || {
        let _ = io::copy(&mut io::stdin().lock(), &mut io::sink());
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    gone
}

/// The packaged app sets GIGACOUCH_ROOT to its bundled Home files. A
/// checkout build finds them by walking up to runtimes/web/shell.
fn home_root() -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os("GIGACOUCH_ROOT") {
        let root = PathBuf::from(raw);
        return root.join("runtimes/web/home").is_dir().then_some(root);
    }
    checkout_root()
}

struct LaunchPaths<'a> {
    godot: &'a Option<PathBuf>,
    root: &'a Path,
    sdk: &'a Path,
    data_dir: &'a Path,
    session: &'a Path,
}

fn answer_play(
    request: &PlayRequest,
    paths: &LaunchPaths<'_>,
    catalog: &[Value],
    running: &mut Option<Child>,
) {
    if running.is_some() {
        let _ = request
            .reply
            .send(Err("Close the open game before starting another.".into()));
        return;
    }
    let Some(godot) = paths.godot else {
        let _ = request.reply.send(Err(
            "A supported Godot is not selected. Home does not download it.".into(),
        ));
        return;
    };
    let Some(game) = catalog
        .iter()
        .find(|row| row["id"].as_str() == Some(request.id.as_str()))
    else {
        let _ = request
            .reply
            .send(Err("That game is not on this Mac.".into()));
        return;
    };
    match crate::host::spawn_listed_game(
        godot,
        paths.root,
        paths.sdk,
        paths.data_dir,
        paths.session,
        game,
        &request.profile,
    ) {
        Ok(started) => {
            let _ = request.reply.send(Ok(started.title));
            *running = Some(started.child);
        }
        Err(error) => {
            let _ = request.reply.send(Err(format!("{error:#}")));
        }
    }
}

fn home_cards(catalog: &[Value]) -> Value {
    let mut cards = vec![json!({
        "id": "blob-island",
        "title": "Blob Island",
        "players": "1–16 on this couch",
        "description": "A small island and a crowd of blobs. This one plays in the browser.",
        "runtime": "web-1",
        "playable": true,
        "color": "#8ce8be"
    })];
    for game in catalog {
        let raw = game["color"].as_str().unwrap_or("8ce8be");
        let color = if raw.starts_with('#') {
            raw.to_string()
        } else {
            format!("#{raw}")
        };
        let playable =
            game["playable"].as_bool().unwrap_or(true) && game["missing"].as_bool() != Some(true);
        cards.push(json!({
            "id": game["id"],
            "title": game["title"],
            "players": game["players"],
            "description": game["description"],
            "runtime": "godot",
            "playable": playable,
            "reason": game["reason"].as_str().unwrap_or(""),
            "color": color
        }));
    }
    Value::Array(cards)
}

pub fn serve(
    package_dir: &Path,
    save_dir: Option<&Path>,
    json: bool,
    data_dir: &Path,
) -> Result<()> {
    let package = WebPackage::read(package_dir)?;
    let saves = save_path(save_dir, data_dir, package.game_id());
    let host = Host::start(package_dir, &saves)?;
    announce(&host, package.title(), None, json)?;
    host.wait_for_game_quit();
    Ok(())
}

pub fn run(
    package_dir: &Path,
    save_dir: Option<&Path>,
    windowed: bool,
    json: bool,
    data_dir: &Path,
) -> Result<()> {
    let package = WebPackage::read(package_dir)?;
    let saves = save_path(save_dir, data_dir, package.game_id());
    let shell =
        shell_dir().context("could not find runtimes/web/shell/main.js from this checkout")?;
    let electron = electron_binary().ok_or_else(|| {
        anyhow!(
            "the web-1 browser shell is not installed. Run python3 runtimes/web/fetch_shell.py on this Mac. The couch CLI does not download it"
        )
    })?;
    let mut host = Host::start(package_dir, &saves)?;
    announce(&host, package.title(), Some(&electron), json)?;
    let mut child = Command::new(&electron)
        .arg(&shell)
        .env("GIGACOUCH_ORIGIN", host.origin())
        .env("GIGACOUCH_TITLE", package.title())
        .env("GIGACOUCH_WINDOWED", if windowed { "1" } else { "0" })
        .spawn()
        .with_context(|| format!("could not start {}", electron.display()))?;
    loop {
        if host.game_quit() {
            let _ = child.kill();
            break;
        }
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                host.shutdown();
                anyhow::bail!("browser shell exited with {status}");
            }
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    host.shutdown();
    let _ = child.wait();
    Ok(())
}

fn announce(host: &Host, title: &str, electron: Option<&Path>, json: bool) -> Result<()> {
    let shell = electron
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "not launched".into());
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "origin": host.origin(),
                "title": title,
                "runtime": "web-1",
                "shell": shell,
            })
        );
    } else {
        println!(
            "Giga Couch web-1\n{title}\n{}\nShell: {shell}",
            host.origin()
        );
    }
    io::stdout().flush()?;
    Ok(())
}

fn save_path(save_dir: Option<&Path>, data_dir: &Path, game_id: &str) -> PathBuf {
    save_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| data_dir.join("saves").join("guest").join(game_id))
}

pub fn checkout_root() -> Option<PathBuf> {
    shell_dir().and_then(|dir| dir.parent()?.parent()?.parent().map(Path::to_path_buf))
}

fn shell_dir() -> Option<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        starts.push(parent.to_path_buf());
    }
    for start in starts {
        let mut cursor = Some(start.as_path());
        while let Some(dir) = cursor {
            let main = dir.join("runtimes/web/shell/main.js");
            if main.is_file() {
                return Some(dir.join("runtimes/web/shell"));
            }
            cursor = dir.parent();
        }
    }
    None
}

fn electron_binary() -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os("GIGACOUCH_ELECTRON") {
        let path = PathBuf::from(raw);
        if path.is_file() {
            return Some(path);
        }
        let nested = path.join("Contents/MacOS/Electron");
        if nested.is_file() {
            return Some(nested);
        }
    }
    let bundled = PathBuf::from(
        "/Volumes/External/projects/gigacouch-electron/Electron.app/Contents/MacOS/Electron",
    );
    bundled.is_file().then_some(bundled)
}
