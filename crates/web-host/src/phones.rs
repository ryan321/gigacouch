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

use crate::package::{DEFAULT_PHONE_LAYOUT, PHONE_LAYOUTS};
use crate::session::{Axis, RawDevice};
use crate::{State, text_response};
use qrcodegen::{QrCode, QrCodeEcc};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fmt::Write as _,
    hash::{BuildHasher, Hasher},
    net::{IpAddr, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
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
const MAX_MESSAGE: usize = 1024;
const CODE_LETTERS: &[u8] = b"ACDEFHJKMNPRTWXY3479";
const PAD_HTML: &str = include_str!("assets/pad.html");
const PAD_JS: &str = include_str!("assets/pad.js");
const PHONE_ICON: &[u8] = include_bytes!("assets/phone-icon.png");

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
}

impl PhoneHub {
    /// Opens the phone listener. The join code is kept in `code_file` so a
    /// phone that added the pad to its home screen can come back after the
    /// app restarts.
    pub(crate) fn bind(
        listen: PhoneListen,
        code_file: Option<&std::path::Path>,
    ) -> Option<(Arc<Self>, tiny_http::Server)> {
        let server = match listen {
            PhoneListen::Off => return None,
            PhoneListen::Loopback => tiny_http::Server::http("127.0.0.1:0").ok()?,
            PhoneListen::Network => tiny_http::Server::http(("0.0.0.0", PREFERRED_PORT))
                .or_else(|_| tiny_http::Server::http("0.0.0.0:0"))
                .ok()?,
        };
        let port = server.server_addr().to_ip()?.port();
        let hub = Self {
            code: code_file.map(kept_code).unwrap_or_else(new_code),
            port,
            network: listen == PhoneListen::Network,
            phones: Mutex::new(HashMap::new()),
            layout: Mutex::new(DEFAULT_PHONE_LAYOUT.to_string()),
            in_game: AtomicBool::new(false),
            shelf_request: AtomicBool::new(false),
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
            // Already on the shelf: a late request has nothing left to do.
            self.shelf_request.store(false, Ordering::SeqCst);
        }
        let layout = layout
            .filter(|name| PHONE_LAYOUTS.contains(name))
            .unwrap_or(DEFAULT_PHONE_LAYOUT);
        *self.layout.lock().expect("layout") = layout.to_string();
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
        let mut list: Vec<(&String, &Phone)> = phones.iter().collect();
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
                }
            })
            .collect()
    }

    /// What the shelf shows: the join address, its QR code, and each phone.
    pub(crate) fn status(&self, state: &State) -> Value {
        let url = self.join_url();
        let now = Instant::now();
        let mut rows: Vec<(String, String, bool, Instant)> = {
            let phones = self.phones.lock().expect("phones");
            phones
                .iter()
                .filter(|(_, phone)| now.saturating_duration_since(phone.seen) < GONE)
                .map(|(id, phone)| {
                    let quiet = now.saturating_duration_since(phone.seen) >= QUIET;
                    (id.clone(), phone.state.name.clone(), quiet, phone.joined)
                })
                .collect()
        };
        rows.sort_by_key(|row| row.3);
        let session = state.session.lock().expect("session");
        let phones: Vec<Value> = rows
            .into_iter()
            .map(|(id, name, quiet, _)| {
                json!({
                    "name": name,
                    "player": session.player_of(&device_id(&id)),
                    "quiet": quiet,
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
        state.name = name;
        state.x = clamp(state.x);
        state.y = clamp(state.y);
        state.lx = clamp(state.lx);
        state.ly = clamp(state.ly);
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

/// The phone listener. Runs until the host stops.
pub(crate) fn serve(server: tiny_http::Server, state: Arc<State>) {
    let Some(hub) = state.phones.clone() else {
        return;
    };
    let prefix = format!("/p/{}", hub.code);
    loop {
        if state.stop.load(Ordering::SeqCst) {
            break;
        }
        let Some(request) = server
            .recv_timeout(Duration::from_millis(200))
            .ok()
            .flatten()
        else {
            continue;
        };
        let url = request.url().to_string();
        let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
        let get = request.method().as_str() == "GET";
        if get && (path == prefix || path == format!("{prefix}/")) {
            let page = PAD_HTML.replace("{{CODE}}", &hub.code);
            let _ = request.respond(text_response(
                200,
                "text/html; charset=utf-8",
                page.into_bytes(),
            ));
        } else if get && path == format!("{prefix}/pad.js") {
            let _ = request.respond(text_response(
                200,
                "text/javascript; charset=utf-8",
                PAD_JS.as_bytes().to_vec(),
            ));
        } else if get && path == format!("{prefix}/manifest.webmanifest") {
            let _ = request.respond(text_response(
                200,
                "application/manifest+json",
                manifest(&hub.code).into_bytes(),
            ));
        } else if get && path == format!("{prefix}/icon.png") {
            let _ = request.respond(text_response(200, "image/png", PHONE_ICON.to_vec()));
        } else if get && path == format!("{prefix}/ws") {
            let id = query
                .split('&')
                .find_map(|pair| pair.strip_prefix("id="))
                .unwrap_or("")
                .to_string();
            accept(request, &id, Arc::clone(&hub), Arc::clone(&state));
        } else if path.starts_with("/p/") {
            let _ = request.respond(text_response(
                404,
                "text/html; charset=utf-8",
                EXPIRED_HTML.as_bytes().to_vec(),
            ));
        } else {
            let _ = request.respond(text_response(
                404,
                "text/plain; charset=utf-8",
                b"Scan the code on the Giga Couch screen.".to_vec(),
            ));
        }
    }
}

const EXPIRED_HTML: &str = "<!doctype html><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>Giga Couch</title><body style=\"font:18px system-ui;background:#101722;color:#f2f5f8;padding:32px\"><h1 style=\"font-size:24px\">This code has changed</h1><p style=\"color:#a0afbf\">Scan the code on the Giga Couch screen again.</p>";

fn valid_phone_id(id: &str) -> bool {
    (8..=64).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn accept(request: tiny_http::Request, id: &str, hub: Arc<PhoneHub>, state: Arc<State>) {
    let key = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Sec-WebSocket-Key"))
        .map(|header| header.value.as_str().to_string());
    let (Some(key), true) = (key, valid_phone_id(id)) else {
        let _ = request.respond(text_response(
            400,
            "application/json",
            br#"{"ok":false,"error":"not a phone connection","code":"BAD_PHONE"}"#.to_vec(),
        ));
        return;
    };
    let response = tiny_http::Response::empty(101).with_header(
        tiny_http::Header::from_bytes(
            &b"Sec-WebSocket-Accept"[..],
            derive_accept_key(key.as_bytes()).as_bytes(),
        )
        .expect("header"),
    );
    let stream = request.upgrade("websocket", response);
    let id = id.to_string();
    thread::spawn(move || run_phone(stream, &id, &hub, &state));
}

/// One phone's socket. Each message is the phone's whole pad state; each
/// reply says which player it is, sent when that changes.
fn run_phone(
    stream: Box<dyn tiny_http::ReadWrite + Send>,
    id: &str,
    hub: &PhoneHub,
    state: &State,
) {
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let mut socket = WebSocket::from_raw_socket(stream, Role::Server, Some(config));
    let device = device_id(id);
    let mut told: Option<String> = None;
    loop {
        if state.stop.load(Ordering::SeqCst) {
            break;
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                let Ok(update) = serde_json::from_str::<PhoneState>(text.as_str()) else {
                    continue;
                };
                if update.shelf && hub.in_game.load(Ordering::SeqCst) {
                    hub.shelf_request.store(true, Ordering::SeqCst);
                }
                hub.update(id, update);
                let player = state.session.lock().expect("session").player_of(&device);
                let reply = json!({
                    "player": player,
                    "layout": hub.layout(),
                    "game": hub.in_game.load(Ordering::SeqCst),
                })
                .to_string();
                if told.as_deref() != Some(reply.as_str()) {
                    if socket.send(Message::Text(reply.clone().into())).is_err() {
                        break;
                    }
                    told = Some(reply);
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(_) => {}
        }
    }
    hub.remove(id);
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
    fn phone_ids_are_checked() {
        assert!(valid_phone_id("abcDEF12_-"));
        assert!(!valid_phone_id("short"));
        assert!(!valid_phone_id("has space in it"));
        assert!(!valid_phone_id(&"x".repeat(65)));
    }
}
