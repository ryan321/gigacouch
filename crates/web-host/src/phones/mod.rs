//! Phones as controllers.
//!
//! A second listener serves a touch pad page and one WebSocket per phone. It
//! answers only paths under `/p/<code>/`, where the code is new each time the
//! host starts and is carried by the QR code on the shelf. Saves, games,
//! Home, and accounts stay on the loopback origin.
//!
//! A phone sends its whole pad state on every change and at least twice a
//! second. The loopback origin adds live phones to the device list the page
//! posts, so the session treats a phone like any other pad: south joins and
//! jumps, and holding east leaves.

use crate::State;
mod extras;
mod panels;
pub(crate) mod views;
pub(crate) use extras::{MAX_AVATAR_BYTES, Target, parse_target};
pub(crate) use panels::check_game_views;

use crate::package::{DEFAULT_PHONE_LAYOUT, GameSound, MAX_SOUND_BYTES, STOCK_SOUNDS};
use crate::session::{Axis, RawDevice};
use qrcodegen::{QrCode, QrCodeEcc};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fmt::Write as _,
    hash::{BuildHasher, Hasher},
    io::{Read, Write},
    net::{IpAddr, TcpListener, TcpStream, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant, SystemTime},
};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket, handshake::derive_accept_key, protocol::Role};

/// Where phones connect. A fixed port keeps the address the same between
/// launches; another program on it moves us to a free port.
pub const PREFERRED_PORT: u16 = 8790;
/// After this long without a message, a phone's buttons and stick read as released.
const QUIET: Duration = Duration::from_millis(1500);
/// After this long the phone is dropped and its player slot is freed.
const GONE: Duration = Duration::from_secs(8);
/// A phone message: pad state, or an event such as a batch of drawing points.
const MAX_MESSAGE: usize = 8 * 1024;
const CODE_LETTERS: &[u8] = b"ACDEFHJKMNPRTWXY3479";
const PAD_HTML: &str = include_str!("../assets/pad.html");
const PAD_JS: &str = include_str!("../assets/pad.js");
const PHONE_ICON: &[u8] = include_bytes!("../assets/phone-icon.png");
/// A two-second black clip with a silent track. Playing it on a loop keeps a
/// phone's screen awake over plain http, where the Wake Lock API is off.
const AWAKE_MP4: &[u8] = include_bytes!("../assets/awake.mp4");

/// A fingerprint of the pad page this host serves. A phone still running an
/// older pad (the host restarted after an update, and the phone reconnected
/// without reloading) sees a different value and reloads itself.
fn pad_version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write(PAD_HTML.as_bytes());
        hasher.write(PAD_JS.as_bytes());
        hasher.write(views::layouts_json().as_bytes());
        hasher.write(views::widgets_json().as_bytes());
        format!("{:016x}", hasher.finish())
    })
}

/// The pad script as phones get it: with its version, the built-in layouts,
/// and the widget rules filled in.
fn pad_script() -> String {
    PAD_JS
        .replace("{{PAD_VERSION}}", pad_version())
        .replace("/*LAYOUTS*/{}", views::layouts_json().trim())
        .replace("/*WIDGETS*/{ widgets: {} }", views::widgets_json().trim())
}

/// Named vibration patterns a game can ask for: on and off lengths in
/// milliseconds, starting with on.
pub const RUMBLE_PRESETS: &[(&str, &[u32])] = &[
    ("tap", &[15]),
    ("bump", &[40]),
    ("hit", &[90]),
    ("long", &[400]),
    ("double", &[40, 70, 40]),
    ("heartbeat", &[60, 120, 60, 400]),
];
const MAX_RUMBLE_STEPS: usize = 20;
const MAX_RUMBLE_STEP_MS: u32 = 2000;
const MAX_RUMBLE_TOTAL_MS: u32 = 5000;

/// A rumble request: a preset name, one length, or on/off lengths.
pub(crate) fn rumble_pattern(value: &Value) -> Result<Vec<u32>, &'static str> {
    let pattern: Vec<u32> = match value {
        Value::String(name) => RUMBLE_PRESETS
            .iter()
            .find(|(preset, _)| preset == name)
            .map(|(_, steps)| steps.to_vec())
            .ok_or("unknown rumble preset")?,
        Value::Number(_) => vec![
            value
                .as_u64()
                .ok_or("rumble lengths are whole milliseconds")?
                .min(u64::from(u32::MAX)) as u32,
        ],
        Value::Array(steps) => steps
            .iter()
            .map(|step| step.as_u64().map(|ms| ms.min(u64::from(u32::MAX)) as u32))
            .collect::<Option<Vec<u32>>>()
            .ok_or("rumble lengths are whole milliseconds")?,
        _ => return Err("pattern must be a preset name, a length, or a list of lengths"),
    };
    if pattern.is_empty()
        || pattern.len() > MAX_RUMBLE_STEPS
        || pattern.iter().any(|&ms| ms == 0 || ms > MAX_RUMBLE_STEP_MS)
        || pattern.iter().sum::<u32>() > MAX_RUMBLE_TOTAL_MS
    {
        return Err("rumble patterns have 1–20 steps of 1–2000 ms, 5 seconds at most in all");
    }
    Ok(pattern)
}

/// Where the phone listener binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoneListen {
    /// No phone listener.
    Off,
    /// Loopback only, on a free port. For tests: no firewall prompt.
    Loopback,
    /// Every interface, so phones on the same Wi-Fi can reach it.
    Network,
}

pub(crate) struct PhoneHub {
    code: String,
    port: u16,
    network: bool,
    phones: Mutex<HashMap<String, Phone>>,
    /// The layout phones show now: the open game's, or the default.
    layout: Mutex<String>,
    /// Whether a game page is open, so phones offer "Back to the shelf".
    in_game: AtomicBool,
    /// Set when a phone asks to go back to the shelf; the shell takes it.
    shelf_request: AtomicBool,
    /// A queue into each connected phone's socket, so the host can send at
    /// any moment instead of waiting for the phone to speak.
    links: Mutex<HashMap<String, (u64, mpsc::Sender<String>)>>,
    next_link: AtomicU64,
    /// The open game's own phone sounds, each with the file name phones
    /// fetch it by: a fingerprint of its bytes, so phones can cache it.
    sounds: Mutex<Vec<(GameSound, String)>>,
    /// Screens, questions, labels, events, profiles, photos, and the open
    /// game's own images and layouts. See `extras.rs`.
    extras: extras::Extras,
}

struct Phone {
    state: PhoneState,
    seen: Instant,
    joined: Instant,
}

/// One message from a phone: its whole pad state.
#[derive(Deserialize, Default, Clone)]
struct PhoneState {
    #[serde(default)]
    name: String,
    #[serde(default)]
    south: bool,
    #[serde(default)]
    east: bool,
    #[serde(default)]
    west: bool,
    #[serde(default)]
    north: bool,
    #[serde(default)]
    start: bool,
    /// The platform Leave control, on every layout.
    #[serde(default)]
    leave: bool,
    /// The phone menu's "Back to the shelf": ends the open game for everyone.
    #[serde(default)]
    shelf: bool,
    #[serde(default)]
    x: f32,
    #[serde(default)]
    y: f32,
    /// The d-pad sends whole steps; a stick sends analog values.
    #[serde(default)]
    digital: bool,
    #[serde(default)]
    lx: f32,
    #[serde(default)]
    ly: f32,
    /// The slider and touchpad send positions, not stick tilt.
    #[serde(default)]
    absolute: bool,
    /// Whether this phone can vibrate. Android phones can; iPhones cannot.
    #[serde(default)]
    rumble: bool,
    /// Watching as the audience: gets screens and votes, takes no slot.
    #[serde(default)]
    audience: bool,
    /// The phone's own measure of its round trip to the host, in ms.
    #[serde(default)]
    rtt: u32,
    /// Named actions and axes the game's layout drives. The session keeps
    /// only the ones the open game declares.
    #[serde(default)]
    actions: std::collections::BTreeMap<String, bool>,
    #[serde(default)]
    axes: std::collections::BTreeMap<String, crate::session::NamedAxis>,
}

impl PhoneHub {
    /// Opens the phone listener. The join code is kept in `code_file` so a
    /// phone that added the pad to its home screen can come back after the
    /// app restarts.
    pub(crate) fn bind(
        listen: PhoneListen,
        code_file: Option<&std::path::Path>,
    ) -> Option<(Arc<Self>, TcpListener)> {
        let server = match listen {
            PhoneListen::Off => return None,
            PhoneListen::Loopback => TcpListener::bind("127.0.0.1:0").ok()?,
            PhoneListen::Network => TcpListener::bind(("0.0.0.0", PREFERRED_PORT))
                .or_else(|_| TcpListener::bind("0.0.0.0:0"))
                .ok()?,
        };
        // Accept without blocking so the loop can notice the host stopping.
        server.set_nonblocking(true).ok()?;
        let port = server.local_addr().ok()?.port();
        let hub = Self {
            code: code_file.map(kept_code).unwrap_or_else(new_code),
            port,
            network: listen == PhoneListen::Network,
            phones: Mutex::new(HashMap::new()),
            layout: Mutex::new(DEFAULT_PHONE_LAYOUT.to_string()),
            in_game: AtomicBool::new(false),
            shelf_request: AtomicBool::new(false),
            links: Mutex::new(HashMap::new()),
            next_link: AtomicU64::new(1),
            sounds: Mutex::new(Vec::new()),
            extras: extras::Extras::load(
                code_file
                    .and_then(std::path::Path::parent)
                    .map(std::path::Path::to_path_buf),
            ),
        };
        Some((Arc::new(hub), server))
    }

    /// The address a phone opens. Looked up each time, because the computer
    /// can change Wi-Fi networks while the shelf is open.
    fn join_url(&self) -> Option<String> {
        let host = if self.network {
            lan_address()?.to_string()
        } else {
            "127.0.0.1".to_string()
        };
        Some(format!("http://{host}:{}/p/{}", self.port, self.code))
    }

    /// Switch every phone to a layout: a game's when it opens, the default
    /// on the shelf. Unknown names fall back to the default.
    pub(crate) fn set_layout(&self, layout: Option<&str>) {
        self.in_game.store(layout.is_some(), Ordering::SeqCst);
        if layout.is_none() {
            // Already on the shelf: a late request has nothing left to do,
            // and no game's sounds, images, screens, or layouts apply.
            self.shelf_request.store(false, Ordering::SeqCst);
            self.set_sounds(&[]);
            self.clear_game_extras();
        }
        let layout = layout
            .filter(|name| self.known_layout(name))
            .unwrap_or(DEFAULT_PHONE_LAYOUT);
        *self.layout.lock().expect("layout") = layout.to_string();
        self.push_layout();
    }

    fn push_layout(&self) {
        let layout = self.layout();
        self.push_all(&json!({
            "layout_spec": self.layout_spec(&layout),
            "layout": layout,
            "game": self.in_game.load(Ordering::SeqCst),
        }));
    }

    /// The open game's own sounds. Phones are told at once so they can
    /// download and decode them before the game plays one.
    pub(crate) fn set_sounds(&self, sounds: &[GameSound]) {
        let resolved: Vec<(GameSound, String)> = sounds
            .iter()
            .filter_map(|sound| {
                let bytes = std::fs::read(&sound.path).ok()?;
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                hasher.write(&bytes);
                let file = format!("{:016x}.{}", hasher.finish(), sound.extension);
                Some((sound.clone(), file))
            })
            .collect();
        *self.sounds.lock().expect("sounds") = resolved;
        self.push_all(&json!({ "sounds": self.sound_urls() }));
    }

    /// The open game's sounds, by name, as addresses on the phone listener.
    fn sound_urls(&self) -> Value {
        let sounds = self.sounds.lock().expect("sounds");
        let map: serde_json::Map<String, Value> = sounds
            .iter()
            .map(|(sound, file)| {
                (
                    sound.name.clone(),
                    json!(format!("/p/{}/sound/{file}", self.code)),
                )
            })
            .collect();
        Value::Object(map)
    }

    /// Plays a stock sound or one of the open game's sounds on the phones of
    /// one player, or on every phone. Returns how many phones were told.
    pub(crate) fn play(
        &self,
        state: &State,
        target: Target,
        name: &str,
    ) -> Result<usize, &'static str> {
        let known = STOCK_SOUNDS.contains(&name)
            || self
                .sounds
                .lock()
                .expect("sounds")
                .iter()
                .any(|(sound, _)| sound.name == name);
        if !known {
            return Err("not a stock sound or one of this game's phone sounds");
        }
        Ok(self.push_to(&self.targets_for(state, target), &json!({ "sound": name })))
    }

    /// Vibrates the phones of one player, or every phone. Phones that cannot
    /// vibrate (iPhones) flash their edges instead. Returns how many phones
    /// were told.
    pub(crate) fn rumble(&self, state: &State, target: Target, pattern: &[u32]) -> usize {
        self.push_to(
            &self.targets_for(state, target),
            &json!({ "rumble": pattern }),
        )
    }

    fn push_to(&self, ids: &[String], message: &Value) -> usize {
        let text = message.to_string();
        let links = self.links.lock().expect("links");
        ids.iter()
            .filter_map(|id| links.get(id))
            .filter(|(_, sender)| sender.send(text.clone()).is_ok())
            .count()
    }

    fn push_all(&self, message: &Value) {
        let text = message.to_string();
        for (_, sender) in self.links.lock().expect("links").values() {
            let _ = sender.send(text.clone());
        }
    }

    /// A game changing the phone layout while it runs. Only while a game is
    /// open: the shelf always shows the default. Returns false for an
    /// unknown layout or when no game is open.
    pub(crate) fn game_sets_layout(&self, layout: &str) -> bool {
        if !self.known_layout(layout) || !self.in_game.load(Ordering::SeqCst) {
            return false;
        }
        *self.layout.lock().expect("layout") = layout.to_string();
        self.push_layout();
        true
    }

    /// The current layout and every built-in one, for the page bridge.
    pub(crate) fn layout_info(&self) -> Value {
        let sounds: Vec<String> = self
            .sounds
            .lock()
            .expect("sounds")
            .iter()
            .map(|(sound, _)| sound.name.clone())
            .collect();
        json!({
            "layout": self.layout(),
            "layouts": views::phone_layouts()
                .iter()
                .map(|name| (*name).to_string())
                .chain(self.extras_layout_names())
                .collect::<Vec<_>>(),
            "game": self.in_game.load(Ordering::SeqCst),
            "details": views::layout_details(),
            "stock_sounds": STOCK_SOUNDS,
            "rumble_presets": RUMBLE_PRESETS.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            "sounds": sounds,
        })
    }

    /// Takes a pending "back to the shelf" request, once.
    pub(crate) fn take_shelf_request(&self) -> bool {
        self.shelf_request.swap(false, Ordering::SeqCst)
    }

    fn layout(&self) -> String {
        self.layout.lock().expect("layout").clone()
    }

    /// Live phones as pads for the session.
    pub(crate) fn devices(&self) -> Vec<RawDevice> {
        let now = Instant::now();
        let mut phones = self.phones.lock().expect("phones");
        phones.retain(|_, phone| now.saturating_duration_since(phone.seen) < GONE);
        // Audience phones watch and vote; they never take a player slot.
        let mut list: Vec<(&String, &Phone)> = phones
            .iter()
            .filter(|(_, phone)| !phone.state.audience)
            .collect();
        list.sort_by_key(|(_, phone)| phone.joined);
        list.into_iter()
            .map(|(id, phone)| {
                // A quiet phone reads as released so nothing stays held.
                let live = now.saturating_duration_since(phone.seen) < QUIET;
                let pad = if live {
                    phone.state.clone()
                } else {
                    PhoneState {
                        name: phone.state.name.clone(),
                        ..PhoneState::default()
                    }
                };
                RawDevice {
                    id: device_id(id),
                    kind: "phone".into(),
                    name: pad.name,
                    family: "phone".into(),
                    south: pad.south,
                    east: pad.east,
                    west: pad.west,
                    north: pad.north,
                    start: pad.start,
                    jump: false,
                    leave: pad.leave,
                    analog: !pad.digital,
                    movement: Axis { x: pad.x, y: pad.y },
                    look: Axis {
                        x: pad.lx,
                        y: pad.ly,
                    },
                    absolute: pad.absolute,
                    actions: pad.actions,
                    axes: pad.axes,
                    avatar: self.avatar_url(id),
                    profile: self.profile_of(id).map(|(person, _)| person),
                }
            })
            .collect()
    }

    /// What the shelf shows: the join address, its QR code, and each phone.
    pub(crate) fn status(&self, state: &State) -> Value {
        let url = self.join_url();
        let now = Instant::now();
        let mut rows: Vec<(String, PhoneState, bool, Instant)> = {
            let phones = self.phones.lock().expect("phones");
            phones
                .iter()
                .filter(|(_, phone)| now.saturating_duration_since(phone.seen) < GONE)
                .map(|(id, phone)| {
                    let quiet = now.saturating_duration_since(phone.seen) >= QUIET;
                    (id.clone(), phone.state.clone(), quiet, phone.joined)
                })
                .collect()
        };
        rows.sort_by_key(|row| row.3);
        let players: Vec<Option<u16>> = {
            let session = state.session.lock().expect("session");
            rows.iter()
                .map(|(id, _, _, _)| session.player_of(&device_id(id)))
                .collect()
        };
        let phones: Vec<Value> = rows
            .into_iter()
            .zip(players)
            .map(|((id, phone, quiet, _), player)| {
                json!({
                    "name": phone.name,
                    "player": player,
                    "quiet": quiet,
                    "rumble": phone.rumble,
                    "audience": phone.audience,
                    "rtt": (phone.rtt > 0).then_some(phone.rtt),
                    "profile": self.profile_of(&id).map(|(person, _)| person),
                    "avatar": self.avatar_url(&id),
                    "phone": extras::phone_token(&id),
                })
            })
            .collect();
        json!({
            "enabled": true,
            "layout": self.layout(),
            "join_url": url,
            "qr_svg": url.as_deref().and_then(qr_svg),
            "phones": phones,
        })
    }

    fn update(&self, id: &str, mut state: PhoneState) {
        let now = Instant::now();
        let clamp = |value: f32| {
            if value.is_finite() {
                value.clamp(-1.0, 1.0)
            } else {
                0.0
            }
        };
        let name: String = state
            .name
            .chars()
            .filter(|ch| !ch.is_control())
            .take(24)
            .collect();
        let name = if name.trim().is_empty() {
            "Phone".to_string()
        } else {
            name.trim().to_string()
        };
        // A phone linked to a Home person plays under that person's name.
        state.name = self
            .profile_of(id)
            .map(|(_, person)| person)
            .unwrap_or(name);
        state.x = clamp(state.x);
        state.y = clamp(state.y);
        state.lx = clamp(state.lx);
        state.ly = clamp(state.ly);
        // Named actions: short names, a handful at most, values in range.
        let good = |name: &String| {
            name.len() <= 32
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        };
        state.actions.retain(|name, _| good(name));
        while state.actions.len() > 16 {
            state.actions.pop_last();
        }
        state.axes.retain(|name, _| good(name));
        while state.axes.len() > 16 {
            state.axes.pop_last();
        }
        for axis in state.axes.values_mut() {
            axis.x = clamp(axis.x);
            axis.y = clamp(axis.y);
        }
        let mut phones = self.phones.lock().expect("phones");
        let joined = phones.get(id).map(|phone| phone.joined).unwrap_or(now);
        phones.insert(
            id.to_string(),
            Phone {
                state,
                seen: now,
                joined,
            },
        );
    }

    fn remove(&self, id: &str) {
        self.phones.lock().expect("phones").remove(id);
    }
}

fn device_id(phone: &str) -> String {
    format!("phone:{phone}")
}

/// The phone listener. Runs until the host stops. Each connection gets its
/// own thread: a page request is answered and closed; a phone's WebSocket
/// stays open for as long as the phone is connected.
pub(crate) fn serve(server: TcpListener, state: Arc<State>) {
    let Some(hub) = state.phones.clone() else {
        return;
    };
    while !state.stop.load(Ordering::SeqCst) {
        match server.accept() {
            Ok((stream, _)) => {
                let hub = Arc::clone(&hub);
                let state = Arc::clone(&state);
                thread::spawn(move || handle(stream, &hub, &state));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => thread::sleep(Duration::from_millis(25)),
        }
    }
}

const MAX_REQUEST_HEAD: usize = 8 * 1024;
const PAGE_CACHE: &str = "no-store";
const SOUND_CACHE: &str = "public, max-age=31536000, immutable";
const CONTENT_POLICY: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob:; connect-src 'self'; worker-src 'self' blob:; font-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'";

/// Reads one request head and answers it. Only GET is served.
fn handle(mut stream: TcpStream, hub: &Arc<PhoneHub>, state: &Arc<State>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut head = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(read) => head.extend_from_slice(&chunk[..read]),
        }
        if head.len() > MAX_REQUEST_HEAD {
            respond(
                &mut stream,
                400,
                "text/plain; charset=utf-8",
                b"request too large",
                PAGE_CACHE,
            );
            return;
        }
    }
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut request = httparse::Request::new(&mut headers);
    if !matches!(request.parse(&head), Ok(httparse::Status::Complete(_))) {
        respond(
            &mut stream,
            400,
            "text/plain; charset=utf-8",
            b"bad request",
            PAGE_CACHE,
        );
        return;
    }
    let method = request.method.unwrap_or("");
    let url = request.path.unwrap_or("/").to_string();
    let header = |name: &str| {
        request
            .headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .and_then(|header| std::str::from_utf8(header.value).ok())
            .map(str::to_string)
    };
    let websocket_key = header("Sec-WebSocket-Key");
    let content_length: usize = header("Content-Length")
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0);
    let body_start = head
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
        .unwrap_or(head.len());
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
    let prefix = format!("/p/{}", hub.code);
    let get = method == "GET";
    if get && (path == prefix || path == format!("{prefix}/")) {
        let page = PAD_HTML.replace("{{CODE}}", &hub.code);
        respond(
            &mut stream,
            200,
            "text/html; charset=utf-8",
            page.as_bytes(),
            PAGE_CACHE,
        );
    } else if get && path == format!("{prefix}/pad.js") {
        let script = pad_script();
        respond(
            &mut stream,
            200,
            "text/javascript; charset=utf-8",
            script.as_bytes(),
            PAGE_CACHE,
        );
    } else if get && path == format!("{prefix}/manifest.webmanifest") {
        respond(
            &mut stream,
            200,
            "application/manifest+json",
            manifest(&hub.code).as_bytes(),
            PAGE_CACHE,
        );
    } else if get && path == format!("{prefix}/awake.mp4") {
        respond(&mut stream, 200, "video/mp4", AWAKE_MP4, SOUND_CACHE);
    } else if get && let Some(file) = path.strip_prefix(&format!("{prefix}/image/")) {
        match hub
            .image_file(file)
            .and_then(|(path, kind)| extras::read_image(&path).map(|bytes| (bytes, kind)))
        {
            Some((bytes, kind)) => respond(&mut stream, 200, kind, &bytes, SOUND_CACHE),
            None => respond(
                &mut stream,
                404,
                "text/plain; charset=utf-8",
                b"not found",
                PAGE_CACHE,
            ),
        }
    } else if method == "POST" && path == format!("{prefix}/avatar") {
        // A phone's photo: a small JPEG the pad already made.
        let id = query
            .split('&')
            .find_map(|pair| pair.strip_prefix("id="))
            .unwrap_or("");
        if !valid_phone_id(id) || content_length == 0 || content_length > MAX_AVATAR_BYTES {
            respond(
                &mut stream,
                400,
                "application/json",
                br#"{"ok":false,"error":"a photo is a JPEG of at most 200 KiB"}"#,
                PAGE_CACHE,
            );
            return;
        }
        let mut body = head[body_start.min(head.len())..].to_vec();
        while body.len() < content_length {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(read) => body.extend_from_slice(&chunk[..read]),
            }
        }
        body.truncate(content_length);
        match hub.save_avatar(id, &body) {
            Ok(()) => respond(
                &mut stream,
                200,
                "application/json",
                br#"{"ok":true}"#,
                PAGE_CACHE,
            ),
            Err(error) => respond(
                &mut stream,
                400,
                "application/json",
                json!({"ok": false, "error": error}).to_string().as_bytes(),
                PAGE_CACHE,
            ),
        }
    } else if get && path == format!("{prefix}/icon.png") {
        respond(&mut stream, 200, "image/png", PHONE_ICON, PAGE_CACHE);
    } else if get && let Some(file) = path.strip_prefix(&format!("{prefix}/sound/")) {
        serve_sound(&mut stream, hub, file);
    } else if get && path == format!("{prefix}/ws") {
        let id = query
            .split('&')
            .find_map(|pair| pair.strip_prefix("id="))
            .unwrap_or("")
            .to_string();
        match websocket_key {
            Some(key) if valid_phone_id(&id) => {
                let answer = format!(
                    "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
                    derive_accept_key(key.as_bytes())
                );
                if stream.write_all(answer.as_bytes()).is_ok() {
                    run_phone(stream, &id, hub, state);
                }
            }
            _ => respond(
                &mut stream,
                400,
                "application/json",
                br#"{"ok":false,"error":"not a phone connection","code":"BAD_PHONE"}"#,
                PAGE_CACHE,
            ),
        }
    } else if path.starts_with("/p/") {
        respond(
            &mut stream,
            404,
            "text/html; charset=utf-8",
            EXPIRED_HTML.as_bytes(),
            PAGE_CACHE,
        );
    } else {
        respond(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"Scan the code on the Giga Couch screen.",
            PAGE_CACHE,
        );
    }
}

/// One of the open game's declared sounds, found by its fingerprint name.
/// Nothing else in the game's folder is reachable from the network.
fn serve_sound(stream: &mut TcpStream, hub: &PhoneHub, file: &str) {
    let found = hub
        .sounds
        .lock()
        .expect("sounds")
        .iter()
        .find(|(_, name)| name == file)
        .map(|(sound, _)| sound.clone());
    let bytes = found.as_ref().and_then(|sound| {
        let bytes = std::fs::read(&sound.path).ok()?;
        (bytes.len() as u64 <= MAX_SOUND_BYTES).then_some(bytes)
    });
    match (found, bytes) {
        (Some(sound), Some(bytes)) => respond(stream, 200, sound.content_type, &bytes, SOUND_CACHE),
        _ => respond(
            stream,
            404,
            "text/plain; charset=utf-8",
            b"not found",
            PAGE_CACHE,
        ),
    }
}

/// Writes a whole response and closes: pages are small, and each phone
/// keeps only its WebSocket open.
fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8], cache: &str) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        _ => "Not Found",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: {cache}\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: {CONTENT_POLICY}\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

const EXPIRED_HTML: &str = "<!doctype html><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>Giga Couch</title><body style=\"font:18px system-ui;background:#101722;color:#f2f5f8;padding:32px\"><h1 style=\"font-size:24px\">This code has changed</h1><p style=\"color:#a0afbf\">Scan the code on the Giga Couch screen again.</p>";

fn valid_phone_id(id: &str) -> bool {
    (8..=64).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// How long a phone's socket waits for the phone before checking whether the
/// host has something to send it. This bounds the delay of a pushed sound.
const PUSH_POLL: Duration = Duration::from_millis(10);

/// One phone's socket. The phone sends its whole pad state; the host sends
/// its player number when that changes, and anything queued for it (layout
/// changes, sounds) as soon as it is queued.
fn run_phone(stream: TcpStream, id: &str, hub: &PhoneHub, state: &State) {
    let _ = stream.set_read_timeout(Some(PUSH_POLL));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let mut socket = WebSocket::from_raw_socket(stream, Role::Server, Some(config));
    let device = device_id(id);
    let (sender, queue) = mpsc::channel::<String>();
    let link = hub.next_link.fetch_add(1, Ordering::SeqCst);
    hub.links
        .lock()
        .expect("links")
        .insert(id.to_string(), (link, sender));
    // A new phone hears the current layout and the open game's sounds
    // straight away, so it can preload them.
    let mut hello = serde_json::Map::new();
    hello.insert("layout".into(), json!(hub.layout()));
    hello.insert("game".into(), json!(hub.in_game.load(Ordering::SeqCst)));
    hello.insert("pad".into(), json!(pad_version()));
    hello.insert("sounds".into(), hub.sound_urls());
    hub.hello_extras(id, &mut hello);
    let mut open = socket
        .send(Message::Text(Value::Object(hello).to_string().into()))
        .is_ok();
    let mut told: Option<String> = None;
    while open && !state.stop.load(Ordering::SeqCst) {
        match socket.read() {
            Ok(Message::Text(text)) => {
                // Discrete messages (events, pings, profile links, requests)
                // carry a kind; everything else is the pad's whole state.
                let parsed: Value = serde_json::from_str(text.as_str()).unwrap_or_default();
                if parsed["kind"].is_string() {
                    if let Some(reply) = hub.handle_message(state, id, &parsed) {
                        open = socket.send(Message::Text(reply.to_string().into())).is_ok();
                    }
                } else if let Ok(update) = serde_json::from_value::<PhoneState>(parsed) {
                    if update.shelf && hub.in_game.load(Ordering::SeqCst) {
                        hub.shelf_request.store(true, Ordering::SeqCst);
                    }
                    hub.update(id, update);
                    let player = state.session.lock().expect("session").player_of(&device);
                    let reply = json!({
                        "player": player,
                        "layout": hub.layout(),
                        "game": hub.in_game.load(Ordering::SeqCst),
                        "pad": pad_version(),
                    })
                    .to_string();
                    if told.as_deref() != Some(reply.as_str()) {
                        open = socket.send(Message::Text(reply.clone().into())).is_ok();
                        told = Some(reply);
                    }
                }
            }
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Ok(Message::Close(_)) | Err(_) => open = false,
            Ok(_) => {}
        }
        while open && let Ok(message) = queue.try_recv() {
            open = socket.send(Message::Text(message.into())).is_ok();
        }
    }
    // A reconnect may already have replaced this link; only drop our own.
    let mut links = hub.links.lock().expect("links");
    if links.get(id).is_some_and(|(current, _)| *current == link) {
        links.remove(id);
        drop(links);
        hub.remove(id);
    }
}

/// A dark-on-white QR code as inline SVG.
fn qr_svg(text: &str) -> Option<String> {
    let qr = QrCode::encode_text(text, QrCodeEcc::Medium).ok()?;
    let border = 2;
    let total = qr.size() + border * 2;
    let mut path = String::new();
    for y in 0..qr.size() {
        for x in 0..qr.size() {
            if qr.get_module(x, y) {
                let _ = write!(path, "M{},{}h1v1h-1z", x + border, y + border);
            }
        }
    }
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {total} {total}\" shape-rendering=\"crispEdges\" role=\"img\" aria-label=\"QR code to join with a phone\"><rect width=\"{total}\" height=\"{total}\" fill=\"#ffffff\"/><path d=\"{path}\" fill=\"#101722\"/></svg>"
    ))
}

/// This computer's address on the local network: the interface the default
/// route uses. Connecting a UDP socket sends nothing.
fn lan_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

/// Lets a phone add the pad to its home screen and open it without the
/// browser's toolbars. The start address carries the kept join code.
fn manifest(code: &str) -> String {
    json!({
        "name": "Giga Couch pad",
        "short_name": "Giga Couch",
        "start_url": format!("/p/{code}"),
        "scope": format!("/p/{code}"),
        "display": "fullscreen",
        "background_color": "#101722",
        "theme_color": "#101722",
        "icons": [{
            "src": format!("/p/{code}/icon.png"),
            "sizes": "512x512",
            "type": "image/png",
        }],
    })
    .to_string()
}

fn valid_code(code: &str) -> bool {
    code.len() == 8 && code.bytes().all(|byte| CODE_LETTERS.contains(&byte))
}

/// The saved join code, or a new one that is then saved.
fn kept_code(file: &std::path::Path) -> String {
    if let Ok(text) = std::fs::read_to_string(file) {
        let code = text.trim();
        if valid_code(code) {
            return code.to_string();
        }
    }
    let code = new_code();
    let _ = std::fs::write(file, &code);
    code
}

/// A short code that is hard to guess and easy to read aloud: no 0/O, 1/I, 5/S.
fn new_code() -> String {
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|time| time.as_nanos())
            .unwrap_or(0),
    );
    hasher.write_u32(std::process::id());
    let mut seed = hasher.finish();
    let base = CODE_LETTERS.len() as u64;
    (0..8)
        .map(|_| {
            let letter = CODE_LETTERS[(seed % base) as usize] as char;
            seed /= base;
            letter
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_use_the_readable_letters() {
        let code = new_code();
        assert!(valid_code(&code), "{code}");
    }

    #[test]
    fn the_join_code_is_kept_between_launches() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("phone-code");
        let first = kept_code(&file);
        assert_eq!(kept_code(&file), first);
        std::fs::write(&file, "not a code").unwrap();
        let replaced = kept_code(&file);
        assert!(valid_code(&replaced));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), replaced);
    }

    #[test]
    fn the_manifest_opens_the_pad_full_screen() {
        let value: Value = serde_json::from_str(&manifest("ACDEFHJK")).unwrap();
        assert_eq!(value["start_url"], "/p/ACDEFHJK");
        assert_eq!(value["display"], "fullscreen");
        assert_eq!(value["icons"][0]["src"], "/p/ACDEFHJK/icon.png");
    }

    #[test]
    fn qr_svg_is_a_square_image() {
        let svg = qr_svg("http://192.168.1.20:8790/p/ACDEFHJK").unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("viewBox=\"0 0 "));
    }

    #[test]
    fn rumble_patterns_are_presets_lengths_or_steps() {
        assert_eq!(rumble_pattern(&json!("double")).unwrap(), vec![40, 70, 40]);
        assert_eq!(rumble_pattern(&json!(120)).unwrap(), vec![120]);
        assert_eq!(
            rumble_pattern(&json!([50, 50, 200])).unwrap(),
            vec![50, 50, 200]
        );
        assert!(rumble_pattern(&json!("earthquake")).is_err());
        assert!(rumble_pattern(&json!(0)).is_err());
        assert!(
            rumble_pattern(&json!(2500)).is_err(),
            "one step is at most 2 s"
        );
        assert!(
            rumble_pattern(&json!([2000, 10, 2000, 10, 2000])).is_err(),
            "5 s in all"
        );
        assert!(
            rumble_pattern(&json!(vec![10; 21])).is_err(),
            "at most 20 steps"
        );
        assert!(rumble_pattern(&json!([10, -5])).is_err());
        assert!(rumble_pattern(&json!({"on": 10})).is_err());
    }

    #[test]
    fn phone_ids_are_checked() {
        assert!(valid_phone_id("abcDEF12_-"));
        assert!(!valid_phone_id("short"));
        assert!(!valid_phone_id("has space in it"));
        assert!(!valid_phone_id(&"x".repeat(65)));
    }
}
