//! Local origin and platform bridge for a web-1 game package.
//!
//! The host owns player slots, saves, and quit. The page receives semantic
//! input. Package checks do not certify sandboxing or that a game runs.

mod account;
mod home;
mod package;
mod phones;
mod session;

use std::{
    io::{self, Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub use account::AccountStore;
pub use package::{DEFAULT_PHONE_LAYOUT, PHONE_LAYOUTS, WebPackage};
pub use phones::{PREFERRED_PORT as PHONE_PORT, PhoneListen};
pub use session::{Buttons, DevicePost, Session, Snapshot};

/// A Home request to start a native game. The caller spawns the process and replies.
pub struct PlayRequest {
    pub id: String,
    pub profile: String,
    pub reply: std::sync::mpsc::Sender<Result<String, String>>,
}

const BRIDGE_JS: &str = include_str!("assets/bridge.js");
const INPUT_JS: &str = include_str!("assets/input.js");
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_HTML_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("could not access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{0}")]
    Host(String),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "INVALID_WEB_PACKAGE",
            Self::Io { .. } => "WEB_PACKAGE_IO",
            Self::Host(_) => "WEB_HOST",
        }
    }
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

struct State {
    package: Option<WebPackage>,
    web_root: PathBuf,
    save_dir: PathBuf,
    session: Mutex<Session>,
    game_quit: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    home: Option<home::Shelf>,
    phones: Option<Arc<phones::PhoneHub>>,
}

/// Lets many phones connect at once. Every phone holds one open connection,
/// and macOS starts an app at 256 open files. This raises the soft limit to
/// the hard limit (capped at 10240, the most macOS accepts). A failure keeps
/// the old limit; Home still works with fewer phones.
pub fn raise_open_file_limit() {
    #[cfg(unix)]
    {
        let mut limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: getrlimit and setrlimit only read and write the struct we
        // pass, which lives on this stack frame for the whole call.
        unsafe {
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) != 0 {
                return;
            }
            let wanted = if cfg!(target_os = "macos") {
                limit.rlim_max.min(10240)
            } else {
                limit.rlim_max
            };
            if wanted > limit.rlim_cur {
                limit.rlim_cur = wanted;
                libc::setrlimit(libc::RLIMIT_NOFILE, &limit);
            }
        }
    }
}

pub struct Host {
    origin: String,
    addr: String,
    game_quit: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    phone_thread: Option<JoinHandle<()>>,
    phone_port: Option<u16>,
}

impl Host {
    pub fn start(package_dir: &Path, save_dir: &Path) -> Result<Self, Error> {
        let package = WebPackage::read(package_dir)?;
        let web_root = package_dir.join("web");
        if !web_root.is_dir() {
            return Err(Error::Invalid(
                "web package is missing its web/ directory".into(),
            ));
        }
        std::fs::create_dir_all(save_dir).map_err(|source| Error::Io {
            path: save_dir.to_path_buf(),
            source,
        })?;
        let listener = tiny_http::Server::http("127.0.0.1:0")
            .map_err(|err| Error::Host(format!("could not bind the local origin: {err}")))?;
        let port = listener
            .server_addr()
            .to_ip()
            .ok_or_else(|| Error::Host("local origin did not bind an IP address".into()))?
            .port();
        let origin = format!("http://127.0.0.1:{port}");
        let max_players = package.players_max();
        let game_quit = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::new(State {
            package: Some(package),
            web_root,
            save_dir: save_dir.to_path_buf(),
            session: Mutex::new(match max_players {
                Some(max) => Session::new(max),
                None => Session::unlimited(),
            }),
            game_quit: Arc::clone(&game_quit),
            stop: Arc::clone(&stop),
            home: None,
            phones: None,
        });
        let thread = thread::spawn(move || serve_loop(listener, state));
        Ok(Self {
            origin,
            addr: format!("127.0.0.1:{port}"),
            game_quit,
            stop,
            thread: Some(thread),
            phone_thread: None,
            phone_port: None,
        })
    }

    pub fn start_home_with(
        home_dir: &Path,
        mounts: &[(&str, &Path)],
        profiles_path: &Path,
        games: Option<serde_json::Value>,
        play: Option<std::sync::mpsc::Sender<PlayRequest>>,
        account: Option<AccountStore>,
        phone_listen: PhoneListen,
    ) -> Result<Self, Error> {
        let mounts = mounts
            .iter()
            .map(|(id, root)| home::Mount {
                id: (*id).to_string(),
                web_root: (*root).to_path_buf(),
            })
            .collect();
        let shelf = home::Shelf::open(home_dir, mounts, profiles_path, games, play, account)?;
        // A phone listener that cannot bind leaves Home working without phones.
        let code_file = profiles_path.with_file_name("phone-code");
        let (hub, phone_server) = match phones::PhoneHub::bind(phone_listen, Some(&code_file)) {
            Some((hub, server)) => (Some(hub), Some(server)),
            None => (None, None),
        };
        let phone_port = phone_server
            .as_ref()
            .and_then(|server| server.server_addr().to_ip())
            .map(|addr| addr.port());
        let listener = tiny_http::Server::http("127.0.0.1:0")
            .map_err(|err| Error::Host(format!("could not bind the local origin: {err}")))?;
        let port = listener
            .server_addr()
            .to_ip()
            .ok_or_else(|| Error::Host("local origin did not bind an IP address".into()))?
            .port();
        let origin = format!("http://127.0.0.1:{port}");
        let game_quit = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::new(State {
            package: None,
            web_root: home_dir.to_path_buf(),
            save_dir: profiles_path.parent().unwrap_or(home_dir).to_path_buf(),
            session: Mutex::new(Session::unlimited()),
            game_quit: Arc::clone(&game_quit),
            stop: Arc::clone(&stop),
            home: Some(shelf),
            phones: hub,
        });
        let phone_thread = phone_server.map(|server| {
            let state = Arc::clone(&state);
            thread::spawn(move || phones::serve(server, state))
        });
        let thread = thread::spawn(move || serve_loop(listener, state));
        Ok(Self {
            origin,
            addr: format!("127.0.0.1:{port}"),
            game_quit,
            stop,
            thread: Some(thread),
            phone_thread,
            phone_port,
        })
    }

    pub fn start_home(
        home_dir: &Path,
        mounts: &[(&str, &Path)],
        profiles_path: &Path,
    ) -> Result<Self, Error> {
        Self::start_home_with(
            home_dir,
            mounts,
            profiles_path,
            None,
            None,
            None,
            PhoneListen::Off,
        )
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// The port phones connect to, when Home listens for them.
    pub fn phone_port(&self) -> Option<u16> {
        self.phone_port
    }

    pub fn game_quit(&self) -> bool {
        self.game_quit.load(Ordering::SeqCst)
    }

    pub fn wait_for_game_quit(&self) {
        while !self.game_quit() {
            thread::sleep(Duration::from_millis(200));
        }
    }

    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(&self.addr);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.phone_thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.shutdown();
    }
}

type Reply = tiny_http::Response<std::io::Cursor<Vec<u8>>>;

fn serve_loop(server: tiny_http::Server, state: Arc<State>) {
    loop {
        if state.stop.load(Ordering::SeqCst) {
            break;
        }
        let Some(mut request) = server
            .recv_timeout(Duration::from_millis(200))
            .ok()
            .flatten()
        else {
            continue;
        };
        if state.stop.load(Ordering::SeqCst) {
            break;
        }
        let method = request.method().clone();
        let url = request.url().to_string();
        let length = request.body_length().unwrap_or(0);
        let response = if length as u64 > session::MAX_REQUEST_BYTES {
            text_response(
                413,
                "application/json",
                br#"{"ok":false,"error":"request is too large","code":"REQUEST_TOO_LARGE"}"#
                    .to_vec(),
            )
        } else {
            let mut body = vec![0; length];
            let read = if length == 0 {
                Ok(())
            } else {
                request.as_reader().read_exact(&mut body)
            };
            if read.is_err() {
                text_response(400, "application/json", br#"{"ok":false,"error":"could not read the request body","code":"BAD_REQUEST"}"#.to_vec())
            } else {
                dispatch(&state, method.as_str(), &url, &body)
            }
        };
        let _ = request.respond(response);
    }
}

fn dispatch(state: &State, method: &str, url: &str, body: &[u8]) -> Reply {
    let path = url.split('?').next().unwrap_or("/");
    if let Some(reply) = home::route(state, method, path, body) {
        return reply;
    }
    match (method, path) {
        ("GET", "/__gigacouch/bridge.js") => text_response(200, "text/javascript; charset=utf-8", BRIDGE_JS.as_bytes().to_vec()),
        ("GET", "/__gigacouch/input.js") => text_response(200, "text/javascript; charset=utf-8", INPUT_JS.as_bytes().to_vec()),
        ("GET", "/__gigacouch/v1/snapshot") => {
            let snapshot = state.session.lock().expect("session").snapshot();
            text_response(200, "application/json", serde_json::to_vec(&snapshot).unwrap_or_default())
        }
        ("GET", "/__gigacouch/v1/control") => {
            let quit = state.game_quit.load(Ordering::SeqCst);
            // A phone asked to leave the game for the shelf; reported once.
            let shelf = state
                .phones
                .as_ref()
                .is_some_and(|hub| hub.take_shelf_request());
            text_response(
                200,
                "application/json",
                format!("{{\"quit\":{quit},\"shelf\":{shelf}}}").into_bytes(),
            )
        }
        ("GET", "/__gigacouch/v1/phones") => {
            let status = match &state.phones {
                Some(hub) => hub.status(state),
                None => serde_json::json!({ "enabled": false, "phones": [] }),
            };
            text_response(200, "application/json", status.to_string().into_bytes())
        }
        ("POST", "/__gigacouch/v1/devices") => match serde_json::from_slice::<DevicePost>(body) {
            Ok(mut post) => {
                if let Some(hub) = &state.phones {
                    post.devices.extend(hub.devices());
                }
                state.session.lock().expect("session").apply(post, std::time::Instant::now());
                text_response(200, "application/json", br#"{"ok":true}"#.to_vec())
            }
            Err(_) => text_response(400, "application/json", br#"{"ok":false,"error":"devices must be a JSON object with a devices array","code":"BAD_DEVICES"}"#.to_vec()),
        },
        ("POST", "/__gigacouch/v1/quit") => {
            state.game_quit.store(true, Ordering::SeqCst);
            text_response(200, "application/json", br#"{"ok":true}"#.to_vec())
        }
        ("POST", "/__gigacouch/v1/save/read") => save_read(state, body),
        ("POST", "/__gigacouch/v1/save/write") => save_write(state, body),
        ("GET", _) => serve_file(state, path),
        _ => text_response(405, "application/json", br#"{"ok":false,"error":"method not allowed","code":"METHOD_NOT_ALLOWED"}"#.to_vec()),
    }
}

fn save_read(state: &State, body: &[u8]) -> Reply {
    let slot = match slot_from(body) {
        Ok(slot) => slot,
        Err(response) => return response,
    };
    let path = state.save_dir.join(format!("{slot}.json"));
    match std::fs::read(&path) {
        Ok(bytes) => {
            let data: serde_json::Value =
                serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
            text_response(
                200,
                "application/json",
                serde_json::json!({"ok": true, "data": data})
                    .to_string()
                    .into_bytes(),
            )
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => text_response(
            404,
            "application/json",
            br#"{"ok":false,"error":"not found","code":"SAVE_NOT_FOUND"}"#.to_vec(),
        ),
        Err(_) => text_response(
            500,
            "application/json",
            br#"{"ok":false,"error":"could not read the save","code":"SAVE_IO"}"#.to_vec(),
        ),
    }
}

fn save_write(state: &State, body: &[u8]) -> Reply {
    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return text_response(
                400,
                "application/json",
                br#"{"ok":false,"error":"save body must be JSON","code":"BAD_SAVE"}"#.to_vec(),
            );
        }
    };
    let slot = match value.get("slot").and_then(|slot| slot.as_str()) {
        Some(slot) if session::valid_slot(slot) => slot.to_string(),
        _ => {
            return text_response(400, "application/json", br#"{"ok":false,"error":"slot must be 1-32 characters of letters, digits, underscore, or hyphen","code":"BAD_SLOT"}"#.to_vec());
        }
    };
    let Some(data) = value.get("data") else {
        return text_response(
            400,
            "application/json",
            br#"{"ok":false,"error":"save write requires data","code":"BAD_SAVE"}"#.to_vec(),
        );
    };
    let bytes = serde_json::to_vec(data).unwrap_or_default();
    if bytes.len() > session::MAX_SAVE_BYTES {
        return text_response(
            413,
            "application/json",
            br#"{"ok":false,"error":"save data exceeds 256 KiB","code":"SAVE_TOO_LARGE"}"#.to_vec(),
        );
    }
    let path = state.save_dir.join(format!("{slot}.json"));
    let temporary = state.save_dir.join(format!("{slot}.json.tmp"));
    let write = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .and_then(|mut file| {
            file.write_all(&bytes)?;
            file.sync_all()
        });
    if write.is_err() {
        return text_response(
            500,
            "application/json",
            br#"{"ok":false,"error":"could not write the save","code":"SAVE_IO"}"#.to_vec(),
        );
    }
    if std::fs::rename(&temporary, &path).is_err() {
        return text_response(
            500,
            "application/json",
            br#"{"ok":false,"error":"could not commit the save","code":"SAVE_IO"}"#.to_vec(),
        );
    }
    text_response(200, "application/json", br#"{"ok":true}"#.to_vec())
}

fn slot_from(body: &[u8]) -> Result<String, Reply> {
    let value: serde_json::Value = serde_json::from_slice(body).map_err(|_| {
        text_response(
            400,
            "application/json",
            br#"{"ok":false,"error":"save body must be JSON","code":"BAD_SAVE"}"#.to_vec(),
        )
    })?;
    match value.get("slot").and_then(|slot| slot.as_str()) {
        Some(slot) if session::valid_slot(slot) => Ok(slot.to_string()),
        _ => Err(text_response(400, "application/json", br#"{"ok":false,"error":"slot must be 1-32 characters of letters, digits, underscore, or hyphen","code":"BAD_SLOT"}"#.to_vec())),
    }
}

fn serve_file(state: &State, url_path: &str) -> Reply {
    let relative = if url_path == "/" {
        state
            .package
            .as_ref()
            .map(|package| package.web_relative_entry())
            .unwrap_or("index.html")
    } else {
        url_path.trim_start_matches('/')
    };
    serve_rooted(&state.web_root, relative)
}

pub(crate) fn serve_rooted(root: &Path, relative: &str) -> Reply {
    let Some(path) = safe_web_file(root, relative) else {
        return text_response(404, "text/plain; charset=utf-8", b"not found".to_vec());
    };
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => return text_response(404, "text/plain; charset=utf-8", b"not found".to_vec()),
    };
    if metadata.len() > MAX_FILE_BYTES {
        return text_response(
            413,
            "text/plain; charset=utf-8",
            b"file is too large".to_vec(),
        );
    }
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(_) => return text_response(404, "text/plain; charset=utf-8", b"not found".to_vec()),
    };
    let canonical = match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => return text_response(404, "text/plain; charset=utf-8", b"not found".to_vec()),
    };
    if !canonical.starts_with(&root) {
        return text_response(404, "text/plain; charset=utf-8", b"not found".to_vec());
    }
    let bytes = match std::fs::read(&canonical) {
        Ok(bytes) => bytes,
        Err(_) => return text_response(404, "text/plain; charset=utf-8", b"not found".to_vec()),
    };
    let content_type = content_type(&canonical);
    if content_type.starts_with("text/html") {
        if bytes.len() as u64 > MAX_HTML_BYTES {
            return text_response(
                413,
                "text/plain; charset=utf-8",
                b"html entry is too large".to_vec(),
            );
        }
        let html = String::from_utf8_lossy(&bytes);
        return text_response(
            200,
            content_type,
            inject_platform(html.as_ref()).into_bytes(),
        );
    }
    text_response(200, content_type, bytes)
}

fn inject_platform(html: &str) -> String {
    const TAGS: &str = "<script src=\"/__gigacouch/input.js\"></script>\n<script src=\"/__gigacouch/bridge.js\"></script>\n";
    if html.contains("/__gigacouch/input.js") && html.contains("/__gigacouch/bridge.js") {
        return html.to_string();
    }
    let lower = html.to_ascii_lowercase();
    if let Some(start) = lower.find("<head")
        && let Some(end) = html[start..].find('>')
    {
        let at = start + end + 1;
        let mut injected = String::with_capacity(html.len() + TAGS.len());
        injected.push_str(&html[..at]);
        injected.push('\n');
        injected.push_str(TAGS);
        injected.push_str(&html[at..]);
        return injected;
    }
    format!("{TAGS}{html}")
}

fn safe_web_file(web_root: &Path, relative: &str) -> Option<PathBuf> {
    if relative.is_empty() || relative.contains('\\') || relative.contains('\0') {
        return None;
    }
    let decoded = percent_decode(relative)?;
    if decoded.contains('\0') {
        return None;
    }
    let path = Path::new(&decoded);
    if path.is_absolute() {
        return None;
    }
    for component in path.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return None;
        }
    }
    Some(web_root.join(path))
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let high = hex_value(bytes[index + 1])?;
                let low = hex_value(bytes[index + 2])?;
                out.push((high << 4) | low);
                index += 3;
            }
            b'%' => return None,
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "glb" => "model/gltf-binary",
        "txt" | "map" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(crate) fn text_response(status: u16, content_type: &str, body: Vec<u8>) -> Reply {
    use tiny_http::{Header, Response, StatusCode};
    let mut response = Response::from_data(body).with_status_code(StatusCode(status));
    for (name, value) in [
        ("Content-Type", content_type),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        (
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob:; connect-src 'self'; worker-src 'self' blob:; font-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
        ),
        ("Referrer-Policy", "no-referrer"),
    ] {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response = response.with_header(header);
        }
    }
    response
}

/// One HTTP round trip against a running host. Test helper, not a browser.
pub fn exchange(origin: &str, method: &str, path: &str, body: Option<&[u8]>) -> (u16, String) {
    let raw = raw_exchange(origin, method, path, body);
    let (head, payload) = raw.split_once("\r\n\r\n").unwrap_or((raw.as_str(), ""));
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    (status, payload.to_string())
}

pub fn raw_exchange(origin: &str, method: &str, path: &str, body: Option<&[u8]>) -> String {
    let origin = origin.to_string();
    let method = method.to_string();
    let path = path.to_string();
    let body = body.map(|bytes| bytes.to_vec());
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(read_response(&origin, &method, &path, body.as_deref()));
    });
    receiver
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_else(|_| "HTTP/1.0 599 timeout\r\n\r\n".into())
}

fn read_response(origin: &str, method: &str, path: &str, body: Option<&[u8]>) -> String {
    let addr = origin.trim_start_matches("http://");
    let mut stream = TcpStream::connect(addr).expect("connect");
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let length = body.map(|bytes| bytes.len()).unwrap_or(0);
    let mut request = format!(
        "{method} {path} HTTP/1.0\r\nHost: {addr}\r\nConnection: close\r\nContent-Length: {length}\r\n"
    );
    if body.is_some() {
        request.push_str("Content-Type: application/json\r\n");
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).expect("write");
    if let Some(body) = body {
        stream.write_all(body).expect("body");
    }
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                if response_complete(&bytes) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn response_complete(bytes: &[u8]) -> bool {
    let Some(split) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&bytes[..split]);
    let length = head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    });
    match length {
        Some(length) => bytes.len() >= split + 4 + length,
        None => false,
    }
}
