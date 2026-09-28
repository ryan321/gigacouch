use crate::{
    PlayRequest, Reply, State,
    account::{AccountStore, Link},
    serve_rooted, text_response,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub(crate) struct Mount {
    pub id: String,
    pub web_root: PathBuf,
}

pub(crate) struct Shelf {
    root: PathBuf,
    mounts: Vec<Mount>,
    book: Mutex<Book>,
    path: PathBuf,
    games: Option<serde_json::Value>,
    play_tx: Option<std::sync::mpsc::Sender<PlayRequest>>,
    account: Option<Mutex<Link>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Person {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Book {
    active: String,
    profiles: Vec<Person>,
}

impl Shelf {
    pub(crate) fn open(
        root: &Path,
        mounts: Vec<Mount>,
        path: &Path,
        games: Option<serde_json::Value>,
        play_tx: Option<std::sync::mpsc::Sender<PlayRequest>>,
        account: Option<AccountStore>,
    ) -> Result<Self, crate::Error> {
        if !root.join("index.html").is_file() {
            return Err(crate::Error::Invalid("home is missing index.html".into()));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| crate::Error::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let book = load_book(path);
        Ok(Self {
            root: root.to_path_buf(),
            mounts,
            book: Mutex::new(book),
            path: path.to_path_buf(),
            games,
            play_tx,
            account: account.map(|store| Mutex::new(Link::from_store(store))),
        })
    }
}

pub(crate) fn route(state: &State, method: &str, path: &str, body: &[u8]) -> Option<Reply> {
    let shelf = state.home.as_ref()?;
    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => {
            // Back on the shelf: phones return to the default layout, and
            // everyone who joins gets a slot again.
            if let Some(hub) = &state.phones {
                hub.set_layout(None);
            }
            let mut session = state.session.lock().expect("session");
            session.set_limit(None);
            session.set_actions(Vec::new());
            Some(serve_rooted(&shelf.root, "index.html"))
        }
        ("GET", "/home.js") => Some(serve_rooted(&shelf.root, "home.js")),
        ("GET", "/home.css") => Some(serve_rooted(&shelf.root, "home.css")),
        ("GET", fonts) if fonts.starts_with("/fonts/") => {
            Some(serve_rooted(&shelf.root, fonts.trim_start_matches('/')))
        }
        ("GET", "/__gigacouch/v1/home") => Some(text_response(
            200,
            "application/json",
            home_payload(shelf).into_bytes(),
        )),
        ("POST", "/__gigacouch/v1/home/use") => Some(use_profile(shelf, body)),
        ("POST", "/__gigacouch/v1/home/add") => Some(add_profile(shelf, body)),
        ("POST", "/__gigacouch/v1/home/play") => Some(play_game(shelf, body)),
        ("POST", "/__gigacouch/v1/account/begin") => Some(begin_account(shelf)),
        ("POST", "/__gigacouch/v1/library/download") => Some(download_game(shelf, body)),
        ("GET", play) if play.starts_with("/play/") => {
            let (reply, web_root) = serve_play(shelf, play);
            if let Some(web_root) = web_root {
                // A game page opened: its phone layout, and its player limit
                // if it sets one.
                let package = package_for(&web_root);
                if let Some(hub) = &state.phones {
                    // Images and layouts first: the layout may be one of them.
                    match package.as_ref() {
                        Some(package) => hub.set_game_extras(
                            package.phone_images(),
                            package.phone_layouts(),
                            package.phone_views(),
                        ),
                        None => hub.set_game_extras(
                            &[],
                            &serde_json::Map::new(),
                            &serde_json::Map::new(),
                        ),
                    }
                    hub.set_layout(Some(
                        package
                            .as_ref()
                            .map(|package| package.phone_layout())
                            .unwrap_or(crate::DEFAULT_PHONE_LAYOUT),
                    ));
                    hub.set_sounds(
                        package
                            .as_ref()
                            .map(|package| package.phone_sounds())
                            .unwrap_or(&[]),
                    );
                }
                let limit = package
                    .as_ref()
                    .and_then(|package| package.players_max())
                    .map(usize::from);
                let mut session = state.session.lock().expect("session");
                session.set_limit(limit);
                session.set_actions(
                    package
                        .as_ref()
                        .map(|package| package.actions().to_vec())
                        .unwrap_or_default(),
                );
            }
            Some(reply)
        }
        _ => None,
    }
}

fn home_payload(shelf: &Shelf) -> String {
    let book = shelf.book.lock().expect("profiles");
    let local = local_games(shelf);
    let (account, games) = match &shelf.account {
        Some(link) => {
            let mut link = link.lock().expect("account");
            let account = link.status();
            let games = if account["signed_in"] == true && account["server"] != "down" {
                match link.library() {
                    Ok(remote) => merge_library(&local, &remote, &link),
                    Err(_) => offline_shelf(&local, &link),
                }
            } else {
                offline_shelf(&local, &link)
            };
            (account, games)
        }
        None => (
            json!({"signed_in": false, "server": "off", "name": "", "user_code": "", "verification_uri": ""}),
            tag_here(&local),
        ),
    };
    json!({
        "active": book.active,
        "profiles": book.profiles,
        "games": games,
        "account": account,
    })
    .to_string()
}

/// Home's people, for a phone's "I'm…" menu: (id, name).
pub(crate) fn people(state: &State) -> Vec<(String, String)> {
    let Some(shelf) = state.home.as_ref() else {
        return Vec::new();
    };
    let book = shelf.book.lock().expect("profiles");
    book.profiles
        .iter()
        .map(|person| (person.id.clone(), person.name.clone()))
        .collect()
}

/// Games a phone can start from the shelf: the ones on this computer that
/// play, as id, title, and color.
pub(crate) fn startable_games(state: &State) -> Vec<serde_json::Value> {
    let Some(shelf) = state.home.as_ref() else {
        return Vec::new();
    };
    let payload: serde_json::Value = serde_json::from_str(&home_payload(shelf)).unwrap_or_default();
    payload["games"]
        .as_array()
        .map(|games| {
            games
                .iter()
                .filter(|game| {
                    game["place"] != "library"
                        && game["action"] != "download"
                        && game["playable"] != false
                })
                .map(|game| json!({"id": game["id"], "title": game["title"], "color": game["color"]}))
                .collect()
        })
        .unwrap_or_default()
}

fn local_games(shelf: &Shelf) -> serde_json::Value {
    shelf.games.clone().unwrap_or_else(|| {
        let text =
            fs::read_to_string(shelf.root.join("catalog.json")).unwrap_or_else(|_| "[]".into());
        serde_json::from_str(&text).unwrap_or(json!([]))
    })
}

fn offline_shelf(local: &serde_json::Value, link: &crate::account::Link) -> serde_json::Value {
    let mut games = tag_here(local);
    let Some(rows) = games.as_array_mut() else {
        return json!(link.downloaded());
    };
    for download in link.downloaded() {
        let id = download["id"].as_str().unwrap_or("").to_string();
        if let Some(existing) = rows
            .iter_mut()
            .find(|row| row["id"].as_str() == Some(id.as_str()))
        {
            if let Some(object) = existing.as_object_mut() {
                object.insert("installed".into(), json!(true));
                object.insert("place".into(), json!("here"));
                object.insert("playable".into(), json!(true));
                object.insert("action".into(), json!("play"));
            }
        } else if !id.is_empty() {
            rows.push(download);
        }
    }
    games
}

fn tag_here(games: &serde_json::Value) -> serde_json::Value {
    let Some(rows) = games.as_array() else {
        return json!([]);
    };
    serde_json::Value::Array(
        rows.iter()
            .map(|game| {
                let mut card = game.clone();
                if let Some(object) = card.as_object_mut() {
                    object.insert("place".into(), json!("here"));
                    object.entry("action").or_insert(json!("play"));
                    object.insert("installed".into(), json!(false));
                }
                card
            })
            .collect(),
    )
}

fn merge_library(
    local: &serde_json::Value,
    remote: &[serde_json::Value],
    link: &crate::account::Link,
) -> serde_json::Value {
    let mut seen = std::collections::HashSet::new();
    let mut cards = Vec::new();
    for game in remote {
        let id = game["id"].as_str().unwrap_or("");
        seen.insert(id.to_string());
        let local_card = local
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["id"].as_str() == Some(id)));
        let installed = link.installed(id);
        let downloadable = game["downloadable"].as_bool().unwrap_or(false);
        let local_playable = local_card
            .and_then(|row| row["playable"].as_bool())
            .unwrap_or(false);
        let on_this_mac = if downloadable {
            installed
        } else {
            local_playable
        };
        let action = if on_this_mac {
            "play"
        } else if downloadable {
            "download"
        } else {
            "unavailable"
        };
        cards.push(json!({
            "id": id,
            "title": game["title"],
            "description": game["description"],
            "players": game["players"],
            "runtime": game["runtime"],
            "color": game["color"],
            "playable": on_this_mac,
            "action": action,
            "installed": installed,
            "place": if on_this_mac { "here" } else { "library" },
            "reason": local_card.and_then(|row| row["reason"].as_str()).unwrap_or("")
        }));
    }
    if let Some(rows) = local.as_array() {
        for game in rows {
            let id = game["id"].as_str().unwrap_or("");
            if seen.contains(id) || !game["playable"].as_bool().unwrap_or(false) {
                continue;
            }
            let mut card = game.clone();
            if let Some(object) = card.as_object_mut() {
                object.insert("place".into(), json!("here"));
                object.insert("action".into(), json!("play"));
                object.insert("installed".into(), json!(false));
            }
            cards.push(card);
        }
    }
    serde_json::Value::Array(cards)
}

fn begin_account(shelf: &Shelf) -> Reply {
    let Some(link) = &shelf.account else {
        return text_response(
            503,
            "application/json",
            br#"{"ok":false,"error":"this Home has no library server","code":"PLATFORM_OFF"}"#
                .to_vec(),
        );
    };
    match link.lock().expect("account").begin() {
        Ok(value) => text_response(200, "application/json", value.to_string().into_bytes()),
        Err(error) => text_response(
            503,
            "application/json",
            json!({"ok": false, "error": error, "code": "PLATFORM_OFF"})
                .to_string()
                .into_bytes(),
        ),
    }
}

fn download_game(shelf: &Shelf, body: &[u8]) -> Reply {
    let Some(link) = &shelf.account else {
        return text_response(
            503,
            "application/json",
            br#"{"ok":false,"error":"this Home has no library server","code":"PLATFORM_OFF"}"#
                .to_vec(),
        );
    };
    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return text_response(
                400,
                "application/json",
                br#"{"ok":false,"error":"download must be JSON","code":"BAD_DOWNLOAD"}"#.to_vec(),
            );
        }
    };
    let id = value.get("id").and_then(|id| id.as_str()).unwrap_or("");
    match link.lock().expect("account").download(id) {
        Ok(()) => text_response(200, "application/json", br#"{"ok":true}"#.to_vec()),
        Err(error) => text_response(
            400,
            "application/json",
            json!({"ok": false, "error": error, "code": "DOWNLOAD_FAILED"})
                .to_string()
                .into_bytes(),
        ),
    }
}

fn play_game(shelf: &Shelf, body: &[u8]) -> Reply {
    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return text_response(
                400,
                "application/json",
                br#"{"ok":false,"error":"play request must be JSON","code":"BAD_PLAY"}"#.to_vec(),
            );
        }
    };
    let id = value.get("id").and_then(|id| id.as_str()).unwrap_or("");
    let games = shelf.games.clone().unwrap_or_else(|| {
        let text =
            fs::read_to_string(shelf.root.join("catalog.json")).unwrap_or_else(|_| "[]".into());
        serde_json::from_str(&text).unwrap_or(json!([]))
    });
    let Some(game) = games
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["id"].as_str() == Some(id)))
        .cloned()
    else {
        return text_response(
            404,
            "application/json",
            br#"{"ok":false,"error":"that game is not on this shelf","code":"GAME_NOT_FOUND"}"#
                .to_vec(),
        );
    };
    if game["runtime"].as_str() == Some("web-1") {
        return text_response(
            200,
            "application/json",
            br#"{"ok":true,"runtime":"web-1"}"#.to_vec(),
        );
    }
    if game["playable"].as_bool() == Some(false) {
        let reason = game["reason"]
            .as_str()
            .unwrap_or("This game cannot be opened.");
        return text_response(
            400,
            "application/json",
            json!({"ok": false, "error": reason, "code": "GAME_NOT_PLAYABLE"})
                .to_string()
                .into_bytes(),
        );
    }
    let Some(tx) = &shelf.play_tx else {
        return text_response(
            503,
            "application/json",
            br#"{"ok":false,"error":"this Home cannot start Godot","code":"GODOT_UNAVAILABLE"}"#
                .to_vec(),
        );
    };
    let profile = shelf.book.lock().expect("profiles").active.clone();
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    if tx
        .send(PlayRequest {
            id: id.to_string(),
            profile,
            reply: reply_tx,
        })
        .is_err()
    {
        return text_response(
            503,
            "application/json",
            br#"{"ok":false,"error":"Home is no longer watching for games","code":"GODOT_UNAVAILABLE"}"#
                .to_vec(),
        );
    }
    match reply_rx.recv_timeout(std::time::Duration::from_secs(8)) {
        Ok(Ok(title)) => text_response(
            200,
            "application/json",
            json!({
                "ok": true,
                "runtime": "godot",
                "title": title,
                "message": format!("{title} is open in its own window. Close that window to come back.")
            })
            .to_string()
            .into_bytes(),
        ),
        Ok(Err(error)) => text_response(
            400,
            "application/json",
            json!({"ok": false, "error": error, "code": "GODOT_LAUNCH"}).to_string().into_bytes(),
        ),
        Err(_) => text_response(
            504,
            "application/json",
            br#"{"ok":false,"error":"Godot did not start.","code":"GODOT_TIMEOUT"}"#.to_vec(),
        ),
    }
}

fn use_profile(shelf: &Shelf, body: &[u8]) -> Reply {
    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return text_response(
                400,
                "application/json",
                br#"{"ok":false,"error":"profile choice must be JSON","code":"BAD_PROFILE"}"#
                    .to_vec(),
            );
        }
    };
    let id = value.get("id").and_then(|id| id.as_str()).unwrap_or("");
    let mut book = shelf.book.lock().expect("profiles");
    if !book.profiles.iter().any(|person| person.id == id) {
        return text_response(
            404,
            "application/json",
            br#"{"ok":false,"error":"that player is not on this Mac","code":"PROFILE_NOT_FOUND"}"#
                .to_vec(),
        );
    }
    book.active = id.to_string();
    save_book(&shelf.path, &book);
    text_response(200, "application/json", br#"{"ok":true}"#.to_vec())
}

fn add_profile(shelf: &Shelf, body: &[u8]) -> Reply {
    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return text_response(
                400,
                "application/json",
                br#"{"ok":false,"error":"new player must be JSON","code":"BAD_PROFILE"}"#.to_vec(),
            );
        }
    };
    let name = value
        .get("name")
        .and_then(|name| name.as_str())
        .unwrap_or("")
        .trim();
    if !valid_name(name) {
        return text_response(
            400,
            "application/json",
            br#"{"ok":false,"error":"use 1-24 letters, numbers, or spaces","code":"BAD_PROFILE"}"#
                .to_vec(),
        );
    }
    let mut book = shelf.book.lock().expect("profiles");
    if book.profiles.len() >= 8 {
        return text_response(
            400,
            "application/json",
            br#"{"ok":false,"error":"this Mac already has 8 players","code":"PROFILE_LIMIT"}"#
                .to_vec(),
        );
    }
    let id = slug(name);
    if book.profiles.iter().any(|person| person.id == id) {
        book.active = id;
    } else {
        book.profiles.push(Person {
            id: id.clone(),
            name: name.to_string(),
        });
        book.active = id;
    }
    save_book(&shelf.path, &book);
    text_response(200, "application/json", br#"{"ok":true}"#.to_vec())
}

/// A game's package. The package file sits next to the game's web/ folder;
/// a missing or invalid one means the defaults: stick-2 and no player limit.
fn package_for(web_root: &Path) -> Option<crate::WebPackage> {
    web_root
        .parent()
        .and_then(|dir| crate::WebPackage::read(dir).ok())
}

/// Serves a file of a game. Also returns the game's web root when the file is
/// one of its pages, so the caller can switch phones to the game's layout.
fn serve_play(shelf: &Shelf, path: &str) -> (Reply, Option<PathBuf>) {
    let rest = path.trim_start_matches("/play/");
    let (id, relative) = rest.split_once('/').unwrap_or((rest, ""));
    let page = relative.is_empty() || relative.ends_with(".html");
    if let Some(link) = &shelf.account {
        let installed = link
            .lock()
            .expect("account")
            .install_root
            .join(id)
            .join("web");
        if installed.is_dir() {
            let relative = if relative.is_empty() {
                "index.html"
            } else {
                relative
            };
            let reply = serve_rooted(&installed, relative);
            return (reply, page.then_some(installed));
        }
    }
    let Some(mount) = shelf.mounts.iter().find(|mount| mount.id == id) else {
        return (
            text_response(404, "text/plain; charset=utf-8", b"not found".to_vec()),
            None,
        );
    };
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    (
        serve_rooted(&mount.web_root, relative),
        page.then(|| mount.web_root.clone()),
    )
}

fn load_book(path: &Path) -> Book {
    let fallback = Book {
        active: "family".into(),
        profiles: vec![
            Person {
                id: "family".into(),
                name: "Family".into(),
            },
            Person {
                id: "guest".into(),
                name: "Guest".into(),
            },
        ],
    };
    let Ok(bytes) = fs::read(path) else {
        return fallback;
    };
    serde_json::from_slice(&bytes).unwrap_or(fallback)
}

fn save_book(path: &Path, book: &Book) {
    let Ok(bytes) = serde_json::to_vec_pretty(book) else {
        return;
    };
    let temporary = path.with_extension("json.tmp");
    if fs::write(&temporary, bytes).is_ok() {
        let _ = fs::rename(temporary, path);
    }
}

fn valid_name(name: &str) -> bool {
    let chars: Vec<char> = name.chars().collect();
    (1..=24).contains(&chars.len())
        && chars
            .iter()
            .all(|ch| ch.is_ascii_alphanumeric() || *ch == ' ' || *ch == '-')
        && !name.starts_with(' ')
        && !name.ends_with(' ')
}

fn slug(name: &str) -> String {
    let mut id = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch.to_ascii_lowercase());
        } else if matches!(ch, ' ' | '-') && !id.ends_with('-') {
            id.push('-');
        }
    }
    id.trim_matches('-').to_string()
}
