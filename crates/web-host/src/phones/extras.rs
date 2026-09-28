//! Phone features a game can opt into, on top of layouts, sounds, and rumble:
//! private screens, typed answers, button labels, events back to the game,
//! audience phones, links to Home profiles, photos, the game's own images and
//! layouts, and starting a game from a phone.
//!
//! Everything a game sends to a phone is checked here and drawn by the pad
//! from data. No game code runs on a phone.

use super::{PhoneHub, device_id};
use crate::State;
use crate::package::{GameImage, LAYOUT_KEYS, MAX_IMAGE_BYTES, PHONE_COLORS, PHONE_LAYOUTS};
use serde_json::{Map, Value, json};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hasher,
    path::{Path, PathBuf},
    sync::{Mutex, atomic::Ordering},
};

/// Events waiting for the game. The oldest go first when a game stops
/// reading them.
const MAX_EVENTS: usize = 2000;
/// A phone photo: a small JPEG the pad makes before sending.
pub(crate) const MAX_AVATAR_BYTES: usize = 200 * 1024;
const MAX_CHOICES: usize = 12;
const MAX_STROKE_POINTS: usize = 128;
/// Phone actions a game's choice may start. The pad does them itself.
const PHONE_ACTIONS: &[&str] = &["photo", "profile", "audience"];

/// Who a game's message goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    All,
    Player(u16),
    /// Phones watching as the audience: no player slot, but they can vote.
    Audience,
}

/// `2`, `"all"`, or `"audience"`.
pub(crate) fn parse_target(value: &Value) -> Result<Target, &'static str> {
    match value {
        Value::String(word) if word == "all" => Ok(Target::All),
        Value::String(word) if word == "audience" => Ok(Target::Audience),
        other => other
            .as_u64()
            .and_then(|number| u16::try_from(number).ok())
            .map(Target::Player)
            .ok_or("player must be a player number, \"all\", or \"audience\""),
    }
}

pub(crate) struct Extras {
    /// Where photos and profile links are kept: next to the Home profiles.
    data_dir: Option<PathBuf>,
    events: Mutex<VecDeque<Value>>,
    /// What the game last showed, asked, and labeled on each phone, so a
    /// phone that reconnects gets it back.
    screens: Mutex<HashMap<String, Value>>,
    asks: Mutex<HashMap<String, Value>>,
    labels: Mutex<HashMap<String, Value>>,
    /// Phone to the Home person it linked to: (id, name).
    profiles: Mutex<HashMap<String, (String, String)>>,
    /// Photos on disk, by token.
    avatars: Mutex<HashSet<String>>,
    /// A game a phone picked from the shelf, for Home to start.
    open_request: Mutex<Option<String>>,
    /// The open game's own images, each with its fingerprint file name.
    images: Mutex<Vec<(GameImage, String)>>,
    /// The open game's own layouts, already checked.
    layouts: Mutex<Map<String, Value>>,
}

impl Extras {
    pub(crate) fn load(data_dir: Option<PathBuf>) -> Self {
        let profiles = data_dir
            .as_ref()
            .and_then(|dir| std::fs::read_to_string(dir.join("phone-profiles.json")).ok())
            .and_then(|text| serde_json::from_str::<HashMap<String, (String, String)>>(&text).ok())
            .unwrap_or_default();
        let avatars = data_dir
            .as_ref()
            .and_then(|dir| std::fs::read_dir(dir.join("phone-avatars")).ok())
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|entry| {
                        let name = entry.file_name().to_string_lossy().to_string();
                        name.strip_suffix(".jpg")
                            .filter(|token| valid_token(token))
                            .map(str::to_string)
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            data_dir,
            events: Mutex::new(VecDeque::new()),
            screens: Mutex::new(HashMap::new()),
            asks: Mutex::new(HashMap::new()),
            labels: Mutex::new(HashMap::new()),
            profiles: Mutex::new(profiles),
            avatars: Mutex::new(avatars),
            open_request: Mutex::new(None),
            images: Mutex::new(Vec::new()),
            layouts: Mutex::new(Map::new()),
        }
    }
}

/// A stable, private stand-in for a phone's id: names its photo, and tells a
/// game two audience votes came from the same phone without revealing the id.
pub(crate) fn phone_token(id: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(b"gigacouch-phone:");
    hasher.write(id.as_bytes());
    format!("{:016x}", hasher.finish())
}

pub(crate) fn valid_token(token: &str) -> bool {
    token.len() == 16 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn clean_text(value: &Value, most: usize) -> Option<String> {
    let words: String = value
        .as_str()?
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n')
        .take(most)
        .collect();
    let words = words.trim().to_string();
    (!words.is_empty()).then_some(words)
}

fn clean_id(value: &Value) -> Option<String> {
    let id = value.as_str()?;
    (!id.is_empty()
        && id.len() <= 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
    .then(|| id.to_string())
}

impl PhoneHub {
    // ---- The open game's images and layouts -------------------------------

    /// The open game's own images and layouts. Phones hear the image list at
    /// once so they can fetch them before a screen shows one.
    pub(crate) fn set_game_extras(&self, images: &[GameImage], layouts: &Map<String, Value>) {
        let resolved: Vec<(GameImage, String)> = images
            .iter()
            .filter_map(|image| {
                let bytes = std::fs::read(&image.path).ok()?;
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                hasher.write(&bytes);
                Some((
                    image.clone(),
                    format!("{:016x}.{}", hasher.finish(), image.extension),
                ))
            })
            .collect();
        *self.extras.images.lock().expect("images") = resolved;
        *self.extras.layouts.lock().expect("layouts") = layouts.clone();
        self.push_all(&json!({ "images": self.image_urls() }));
    }

    /// Back on the shelf: no game's images, layouts, screens, questions, or
    /// labels apply, and every phone clears them.
    pub(crate) fn clear_game_extras(&self) {
        self.extras.images.lock().expect("images").clear();
        self.extras.layouts.lock().expect("layouts").clear();
        self.extras.screens.lock().expect("screens").clear();
        self.extras.asks.lock().expect("asks").clear();
        self.extras.labels.lock().expect("labels").clear();
        self.push_all(&json!({ "images": {}, "screen": null, "ask": null, "labels": null }));
    }

    pub(crate) fn image_urls(&self) -> Value {
        let images = self.extras.images.lock().expect("images");
        Value::Object(
            images
                .iter()
                .map(|(image, file)| {
                    (
                        image.name.clone(),
                        json!(format!("/p/{}/image/{file}", self.code)),
                    )
                })
                .collect(),
        )
    }

    /// One of the open game's images, by its fingerprint file name.
    pub(crate) fn image_file(&self, file: &str) -> Option<(PathBuf, &'static str)> {
        self.extras
            .images
            .lock()
            .expect("images")
            .iter()
            .find(|(_, name)| name == file)
            .map(|(image, _)| (image.path.clone(), image.content_type))
    }

    /// Whether a layout name is built in or one of the open game's.
    pub(crate) fn known_layout(&self, name: &str) -> bool {
        PHONE_LAYOUTS.contains(&name)
            || self
                .extras
                .layouts
                .lock()
                .expect("layouts")
                .contains_key(name)
    }

    pub(crate) fn extras_layout_names(&self) -> Vec<String> {
        self.extras
            .layouts
            .lock()
            .expect("layouts")
            .keys()
            .cloned()
            .collect()
    }

    /// The drawing data for one of the open game's layouts.
    pub(crate) fn layout_spec(&self, name: &str) -> Option<Value> {
        self.extras
            .layouts
            .lock()
            .expect("layouts")
            .get(name)
            .cloned()
    }

    // ---- Who hears a message ----------------------------------------------

    pub(crate) fn targets_for(&self, state: &State, target: Target) -> Vec<String> {
        let phones = self.phones.lock().expect("phones");
        let ids: Vec<(String, bool)> = phones
            .iter()
            .map(|(id, phone)| (id.clone(), phone.state.audience))
            .collect();
        drop(phones);
        match target {
            Target::All => ids.into_iter().map(|(id, _)| id).collect(),
            Target::Audience => ids
                .into_iter()
                .filter(|(_, audience)| *audience)
                .map(|(id, _)| id)
                .collect(),
            Target::Player(number) => {
                let session = state.session.lock().expect("session");
                ids.into_iter()
                    .filter(|(id, _)| session.player_of(&device_id(id)) == Some(number))
                    .map(|(id, _)| id)
                    .collect()
            }
        }
    }

    // ---- What a game sends to phones ----------------------------------------

    /// A private screen: text, an image, and choices a player can tap.
    /// `null` takes it away.
    pub(crate) fn show(
        &self,
        state: &State,
        target: Target,
        screen: &Value,
    ) -> Result<usize, String> {
        let clean = if screen.is_null() {
            Value::Null
        } else {
            self.clean_screen(screen)?
        };
        Ok(self.send_kept(state, target, "screen", &self.extras.screens, clean))
    }

    /// A question answered by typing on the phone. `null` takes it away.
    pub(crate) fn ask(&self, state: &State, target: Target, ask: &Value) -> Result<usize, String> {
        let clean = if ask.is_null() {
            Value::Null
        } else {
            let id = clean_id(&ask["id"])
                .ok_or("ask.id is 1–32 letters, digits, dashes, or underscores")?;
            let prompt = clean_text(&ask["prompt"], 120).ok_or("ask.prompt is 1–120 characters")?;
            let most = ask["max"].as_u64().unwrap_or(80).clamp(1, 500);
            json!({
                "id": id,
                "prompt": prompt,
                "placeholder": clean_text(&ask["placeholder"], 60),
                "max": most,
                "multiline": ask["multiline"].as_bool() == Some(true),
            })
        };
        Ok(self.send_kept(state, target, "ask", &self.extras.asks, clean))
    }

    /// New words on a player's buttons, such as the answers of a quiz.
    /// `null` puts the layout's own labels back.
    pub(crate) fn labels(
        &self,
        state: &State,
        target: Target,
        labels: &Value,
    ) -> Result<usize, String> {
        let clean = if labels.is_null() {
            Value::Null
        } else {
            let map = labels
                .as_object()
                .ok_or("labels is an object of button to words")?;
            let mut clean = Map::new();
            for (key, words) in map {
                if !LAYOUT_KEYS.contains(&key.as_str()) {
                    return Err("label keys are south, east, west, north, and start".into());
                }
                let words = clean_text(words, 24).ok_or("each label is 1–24 characters")?;
                clean.insert(key.clone(), json!(words));
            }
            Value::Object(clean)
        };
        Ok(self.send_kept(state, target, "labels", &self.extras.labels, clean))
    }

    /// Pushes a message and remembers it per phone, so a phone that comes
    /// back gets it again.
    fn send_kept(
        &self,
        state: &State,
        target: Target,
        field: &str,
        store: &Mutex<HashMap<String, Value>>,
        value: Value,
    ) -> usize {
        let ids = self.targets_for(state, target);
        {
            let mut kept = store.lock().expect("kept");
            for id in &ids {
                if value.is_null() {
                    kept.remove(id);
                } else {
                    kept.insert(id.clone(), value.clone());
                }
            }
        }
        let mut message = Map::new();
        message.insert(field.to_string(), value);
        self.push_to(&ids, &Value::Object(message))
    }

    fn clean_screen(&self, screen: &Value) -> Result<Value, String> {
        let mut clean = Map::new();
        if let Some(id) = clean_id(&screen["id"]) {
            clean.insert("id".into(), json!(id));
        }
        if let Some(title) = clean_text(&screen["title"], 80) {
            clean.insert("title".into(), json!(title));
        }
        if let Some(text) = clean_text(&screen["text"], 1000) {
            clean.insert("text".into(), json!(text));
        }
        if let Some(image) = screen["image"].as_str() {
            let images = self.image_urls();
            let url = images
                .get(image)
                .ok_or("screen.image must name one of the game's phone.images")?;
            clean.insert("image".into(), url.clone());
        }
        if let Some(choices) = screen["choices"].as_array() {
            if choices.len() > MAX_CHOICES {
                return Err(format!("a screen has at most {MAX_CHOICES} choices"));
            }
            let mut out = Vec::new();
            for choice in choices {
                let id = clean_id(&choice["id"]).ok_or(
                    "each choice needs an id of 1–32 letters, digits, dashes, or underscores",
                )?;
                let label = clean_text(&choice["label"], 60)
                    .ok_or("each choice needs a label of 1–60 characters")?;
                let mut item = Map::new();
                item.insert("id".into(), json!(id));
                item.insert("label".into(), json!(label));
                if let Some(detail) = clean_text(&choice["detail"], 120) {
                    item.insert("detail".into(), json!(detail));
                }
                if let Some(color) = choice["color"].as_str() {
                    if !PHONE_COLORS.contains(&color) {
                        return Err("a choice's color is mint, blue, amber, coral, or panel".into());
                    }
                    item.insert("color".into(), json!(color));
                }
                if let Some(image) = choice["image"].as_str() {
                    let images = self.image_urls();
                    let url = images
                        .get(image)
                        .ok_or("a choice's image must name one of the game's phone.images")?;
                    item.insert("image".into(), url.clone());
                }
                // A choice can start one of the phone's own actions, done by
                // the pad from the player's tap: open the camera for a photo,
                // pick a Home person, or join the audience.
                if let Some(action) = choice["action"].as_str() {
                    if !PHONE_ACTIONS.contains(&action) {
                        return Err("a choice's action is photo, profile, or audience".into());
                    }
                    item.insert("action".into(), json!(action));
                }
                out.push(Value::Object(item));
            }
            clean.insert("choices".into(), Value::Array(out));
        }
        if clean.is_empty() {
            return Err("a screen needs a title, text, an image, or choices".into());
        }
        Ok(Value::Object(clean))
    }

    /// What a reconnecting phone should get back: the game's screen, question,
    /// labels, images, and a custom layout's drawing data, plus its profile.
    pub(crate) fn hello_extras(&self, id: &str, hello: &mut Map<String, Value>) {
        hello.insert("images".into(), self.image_urls());
        let pick = |store: &Mutex<HashMap<String, Value>>| {
            store
                .lock()
                .expect("kept")
                .get(id)
                .cloned()
                .unwrap_or(Value::Null)
        };
        hello.insert("screen".into(), pick(&self.extras.screens));
        hello.insert("ask".into(), pick(&self.extras.asks));
        hello.insert("labels".into(), pick(&self.extras.labels));
        hello.insert("profile".into(), self.profile_json(id));
        hello.insert("photo".into(), json!(self.has_avatar(id)));
        if let Some(spec) = self.layout_spec(&self.layout()) {
            hello.insert("layout_spec".into(), spec);
        }
    }

    // ---- What phones send to the game ---------------------------------------

    /// Phone events since the last call, oldest first.
    pub(crate) fn take_events(&self) -> Vec<Value> {
        self.extras
            .events
            .lock()
            .expect("events")
            .drain(..)
            .collect()
    }

    /// A discrete message from a phone: an event for the game, a ping, a
    /// profile link, a request for Home's people or games, or a game to open.
    /// Returns the reply for that phone, if any.
    pub(crate) fn handle_message(&self, state: &State, id: &str, message: &Value) -> Option<Value> {
        match message["kind"].as_str()? {
            "event" => {
                if let Some(event) = self.clean_event(state, id, &message["event"]) {
                    let mut events = self.extras.events.lock().expect("events");
                    if events.len() >= MAX_EVENTS {
                        events.pop_front();
                    }
                    events.push_back(event);
                }
                None
            }
            "ping" => Some(json!({ "pong": message["t"] })),
            "profile" => {
                let people = crate::home::people(state);
                let wanted = message["id"].as_str();
                let mut links = self.extras.profiles.lock().expect("profiles");
                match wanted.and_then(|wanted| people.iter().find(|(person, _)| person == wanted)) {
                    Some((person, name)) => {
                        links.insert(id.to_string(), (person.clone(), name.clone()));
                    }
                    None => {
                        links.remove(id);
                    }
                }
                self.save_profiles(&links);
                drop(links);
                Some(json!({ "profile": self.profile_json(id) }))
            }
            "request" => match message["what"].as_str()? {
                "profiles" => Some(json!({
                    "profiles": crate::home::people(state)
                        .into_iter()
                        .map(|(person, name)| json!({"id": person, "name": name}))
                        .collect::<Vec<_>>(),
                })),
                "shelf" => Some(json!({ "shelf": crate::home::startable_games(state) })),
                _ => None,
            },
            "open" => {
                let wanted = message["id"].as_str().unwrap_or("");
                let known = crate::home::startable_games(state)
                    .iter()
                    .any(|game| game["id"].as_str() == Some(wanted));
                if !known || self.in_game.load(Ordering::SeqCst) {
                    return Some(json!({ "notice": "That game can't start from here right now." }));
                }
                *self.extras.open_request.lock().expect("open") = Some(wanted.to_string());
                Some(json!({ "notice": "Starting it on the TV…" }))
            }
            _ => None,
        }
    }

    fn clean_event(&self, state: &State, id: &str, event: &Value) -> Option<Value> {
        let mut clean = Map::new();
        match event["type"].as_str()? {
            "choice" => {
                clean.insert("type".into(), json!("choice"));
                clean.insert("choice".into(), json!(clean_id(&event["choice"])?));
                if let Some(screen) = clean_id(&event["screen"]) {
                    clean.insert("screen".into(), json!(screen));
                }
            }
            "text" => {
                let ask = self.extras.asks.lock().expect("asks").get(id).cloned()?;
                let most = ask["max"].as_u64().unwrap_or(80) as usize;
                clean.insert("type".into(), json!("text"));
                clean.insert("ask".into(), ask["id"].clone());
                clean.insert("text".into(), json!(clean_text(&event["text"], most)?));
            }
            "stroke" => {
                let points = event["points"].as_array()?;
                if points.is_empty() || points.len() > MAX_STROKE_POINTS {
                    return None;
                }
                let mut clean_points = Vec::new();
                for point in points {
                    let pair = point.as_array().filter(|pair| pair.len() == 2)?;
                    let x = pair[0].as_f64()?.clamp(0.0, 1.0);
                    let y = pair[1].as_f64()?.clamp(0.0, 1.0);
                    clean_points.push(json!([
                        (x * 10000.0).round() / 10000.0,
                        (y * 10000.0).round() / 10000.0
                    ]));
                }
                let color = event["color"]
                    .as_str()
                    .filter(|color| PHONE_COLORS.contains(color))
                    .unwrap_or("mint");
                clean.insert("type".into(), json!("stroke"));
                clean.insert(
                    "stroke".into(),
                    json!(event["stroke"].as_u64().unwrap_or(0) % 1_000_000),
                );
                clean.insert("points".into(), Value::Array(clean_points));
                clean.insert("color".into(), json!(color));
                clean.insert("end".into(), json!(event["end"].as_bool() == Some(true)));
            }
            "clear" => {
                clean.insert("type".into(), json!("clear"));
            }
            _ => return None,
        }
        let (name, audience) = {
            let phones = self.phones.lock().expect("phones");
            let phone = phones.get(id)?;
            (phone.state.name.clone(), phone.state.audience)
        };
        let player = state
            .session
            .lock()
            .expect("session")
            .player_of(&device_id(id));
        clean.insert("player".into(), json!(player));
        clean.insert("audience".into(), json!(audience));
        clean.insert("name".into(), json!(name));
        clean.insert("phone".into(), json!(phone_token(id)));
        Some(Value::Object(clean))
    }

    // ---- Home profiles ------------------------------------------------------

    pub(crate) fn profile_of(&self, id: &str) -> Option<(String, String)> {
        self.extras
            .profiles
            .lock()
            .expect("profiles")
            .get(id)
            .cloned()
    }

    fn profile_json(&self, id: &str) -> Value {
        self.profile_of(id)
            .map(|(person, name)| json!({"id": person, "name": name}))
            .unwrap_or(Value::Null)
    }

    fn save_profiles(&self, links: &HashMap<String, (String, String)>) {
        if let Some(dir) = &self.extras.data_dir
            && let Ok(text) = serde_json::to_string_pretty(links)
        {
            let file = dir.join("phone-profiles.json");
            let temporary = file.with_extension("json.tmp");
            if std::fs::write(&temporary, text).is_ok() {
                let _ = std::fs::rename(temporary, file);
            }
        }
    }

    // ---- Photos --------------------------------------------------------------

    fn avatar_dir(&self) -> Option<PathBuf> {
        self.extras
            .data_dir
            .as_ref()
            .map(|dir| dir.join("phone-avatars"))
    }

    pub(crate) fn has_avatar(&self, id: &str) -> bool {
        self.extras
            .avatars
            .lock()
            .expect("avatars")
            .contains(&phone_token(id))
    }

    /// Where a game loads a phone's photo, on the host's own origin.
    pub(crate) fn avatar_url(&self, id: &str) -> Option<String> {
        let token = phone_token(id);
        self.extras
            .avatars
            .lock()
            .expect("avatars")
            .contains(&token)
            .then(|| format!("/__gigacouch/v1/avatar/{token}.jpg"))
    }

    /// Saves a phone's photo: a JPEG the pad already shrank.
    pub(crate) fn save_avatar(&self, id: &str, bytes: &[u8]) -> Result<(), &'static str> {
        if bytes.len() > MAX_AVATAR_BYTES || !bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Err("a photo is a JPEG of at most 200 KiB");
        }
        let dir = self.avatar_dir().ok_or("this host keeps no photos")?;
        std::fs::create_dir_all(&dir).map_err(|_| "could not save the photo")?;
        let token = phone_token(id);
        let file = dir.join(format!("{token}.jpg"));
        let temporary = dir.join(format!("{token}.tmp"));
        std::fs::write(&temporary, bytes).map_err(|_| "could not save the photo")?;
        std::fs::rename(&temporary, &file).map_err(|_| "could not save the photo")?;
        self.extras.avatars.lock().expect("avatars").insert(token);
        Ok(())
    }

    /// A photo file by token, for the host's own origin.
    pub(crate) fn avatar_file(&self, token: &str) -> Option<PathBuf> {
        if !valid_token(token) || !self.extras.avatars.lock().expect("avatars").contains(token) {
            return None;
        }
        let file = self.avatar_dir()?.join(format!("{token}.jpg"));
        std::fs::metadata(&file)
            .ok()
            .filter(|meta| meta.is_file() && meta.len() <= MAX_AVATAR_BYTES as u64)
            .map(|_| file)
    }

    // ---- Starting a game from a phone ----------------------------------------

    /// A game a phone picked, for Home to start. Taken once.
    pub(crate) fn take_open_request(&self) -> Option<String> {
        self.extras.open_request.lock().expect("open").take()
    }
}

/// An image file of the open game, read for the phone listener.
pub(crate) fn read_image(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    (bytes.len() as u64 <= MAX_IMAGE_BYTES).then_some(bytes)
}
