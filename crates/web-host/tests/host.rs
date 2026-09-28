use couch_web_host::{
    DEFAULT_PHONE_LAYOUT, DevicePost, Host, PHONE_LAYOUTS, PhoneListen, Session, WebPackage,
    exchange, raw_exchange,
};
use serde_json::json;
use std::path::PathBuf;
use std::{
    fs,
    time::{Duration, Instant},
};

fn package(dir: &std::path::Path, max_players: u8, entry: &str) {
    fs::create_dir_all(dir.join("web")).unwrap();
    fs::write(
        dir.join("web/index.html"),
        "<!DOCTYPE html><head><title>Blob</title></head><body><script src=\"game.js\"></script></body>",
    )
    .unwrap();
    fs::write(dir.join("web/game.js"), "/* sample bytes */").unwrap();
    fs::write(dir.join("secret.txt"), "nope").unwrap();
    fs::write(
        dir.join("gigacouch.json"),
        json!({
            "manifest_version": 1,
            "game_id": "gigacouch.blob-island",
            "title": "Blob Island",
            "version": "0.1.0",
            "runtime": "web-1",
            "entrypoint": entry,
            "gigacouch_api": "1",
            "players": { "min": 1, "max": max_players }
        })
        .to_string(),
    )
    .unwrap();
}

#[test]
fn package_rejects_paths_outside_web() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), 16, "web/index.html");
    fs::write(
        dir.path().join("gigacouch.json"),
        fs::read_to_string(dir.path().join("gigacouch.json"))
            .unwrap()
            .replace("web/index.html", "web/../secret.txt"),
    )
    .unwrap();
    let error = WebPackage::read(dir.path()).unwrap_err();
    assert_eq!(error.code(), "INVALID_WEB_PACKAGE");
    assert!(error.to_string().contains("web/"));
}

#[test]
fn host_serves_the_game_and_hides_the_package_root() {
    let dir = tempfile::tempdir().unwrap();
    let saves = tempfile::tempdir().unwrap();
    package(dir.path(), 16, "web/index.html");
    let host = Host::start(dir.path(), saves.path()).unwrap();
    let (status, html) = exchange(host.origin(), "GET", "/", None);
    assert_eq!(status, 200);
    assert!(html.contains("/__gigacouch/input.js"));
    assert!(html.contains("/__gigacouch/bridge.js"));
    assert!(html.find("/__gigacouch/input.js").unwrap() < html.find("game.js").unwrap());
    let (status, script) = exchange(host.origin(), "GET", "/__gigacouch/bridge.js", None);
    assert_eq!(status, 200);
    assert!(script.contains("window.GigaCouch"));
    let (status, _) = exchange(host.origin(), "GET", "/../secret.txt", None);
    assert_eq!(status, 404);
    let (status, _) = exchange(host.origin(), "GET", "/%2e%2e/secret.txt", None);
    assert_eq!(status, 404);
    let (status, game) = exchange(host.origin(), "GET", "/game.js", None);
    assert_eq!(status, 200);
    assert!(game.contains("sample bytes"));
}

#[test]
fn response_carries_a_restrictive_content_policy() {
    let dir = tempfile::tempdir().unwrap();
    let saves = tempfile::tempdir().unwrap();
    package(dir.path(), 4, "web/index.html");
    let host = Host::start(dir.path(), saves.path()).unwrap();
    let text = raw_exchange(host.origin(), "GET", "/", None).to_ascii_lowercase();
    assert!(
        text.contains("content-security-policy: default-src 'self'"),
        "{text}"
    );
    assert!(text.contains("x-content-type-options: nosniff"), "{text}");
}

#[test]
fn south_joins_then_jumps_once_per_press() {
    let mut session = Session::new(16);
    let now = Instant::now();
    session.apply(post("pad-a", true, false), now);
    let snap = session.snapshot();
    assert_eq!(snap.players.len(), 1);
    assert_eq!(snap.players[0].id, 1);
    assert!(!snap.players[0].edges.jump);
    session.apply(post("pad-a", true, false), now);
    assert!(!session.snapshot().players[0].edges.jump);
    session.apply(post("pad-a", false, false), now);
    session.apply(post("pad-a", true, false), now);
    assert!(session.snapshot().players[0].edges.jump);
    assert!(!session.snapshot().players[0].edges.jump);
}

#[test]
fn the_seventeenth_pad_does_not_get_a_slot() {
    let mut session = Session::new(16);
    let devices = (0..17)
        .map(|index| {
            json!({
                "id": format!("pad-{index}"),
                "kind": "pad",
                "name": "Pad",
                "family": "generic",
                "south": true,
                "analog": false,
                "move": { "x": 0, "y": 0 }
            })
        })
        .collect::<Vec<_>>();
    session.apply(
        serde_json::from_value(json!({ "devices": devices })).unwrap(),
        Instant::now(),
    );
    assert_eq!(session.snapshot().players.len(), 16);
}

#[test]
fn holding_east_leaves_and_disconnect_frees_the_slot() {
    let mut session = Session::new(16);
    let now = Instant::now();
    session.apply(post("pad-a", true, false), now);
    session.apply(post("pad-a", false, true), now);
    assert_eq!(session.snapshot().players.len(), 1);
    session.apply(
        post("pad-a", false, true),
        now + Duration::from_millis(1249),
    );
    assert_eq!(session.snapshot().players.len(), 1);
    session.apply(
        post("pad-a", false, true),
        now + Duration::from_millis(1250),
    );
    assert!(session.snapshot().players.is_empty());

    session.apply(post("pad-a", true, false), now);
    session.apply(DevicePost { devices: vec![] }, now);
    assert!(session.snapshot().players.is_empty());
}

#[test]
fn keyboard_space_joins_without_jumping_and_backspace_leaves() {
    let mut session = Session::new(16);
    let now = Instant::now();
    session.apply(keyboard(false, true, false), now);
    let snap = session.snapshot();
    assert_eq!(snap.players.len(), 1);
    assert!(!snap.players[0].edges.jump);
    assert_eq!(snap.players[0].glyphs.jump, "Space");
    session.apply(keyboard(false, false, false), now);
    session.apply(keyboard(false, true, false), now);
    assert!(session.snapshot().players[0].edges.jump);
    session.apply(keyboard(false, false, true), now);
    assert!(session.snapshot().players.is_empty());
}

#[test]
fn analog_dead_zone_matches_the_sdk_ramp() {
    let mut session = Session::new(16);
    let now = Instant::now();
    session.apply(post("pad-a", true, false), now);
    session.apply(analog("pad-a", 0.1, 0.0), now);
    let snap = session.snapshot();
    assert_eq!(snap.players[0].movement.x, 0.0);
    session.apply(analog("pad-a", 0.6, 0.0), now);
    let snap = session.snapshot();
    assert!((snap.players[0].movement.x - 0.5).abs() < 0.001);
}

#[test]
fn saves_are_atomic_json_and_reject_a_bad_slot() {
    let dir = tempfile::tempdir().unwrap();
    let saves = tempfile::tempdir().unwrap();
    package(dir.path(), 16, "web/index.html");
    let host = Host::start(dir.path(), saves.path()).unwrap();
    let (status, body) = exchange(
        host.origin(),
        "POST",
        "/__gigacouch/v1/save/write",
        Some(br#"{"slot":"campaign","data":{"lives":3}}"#),
    );
    assert_eq!(status, 200, "{body}");
    let (status, body) = exchange(
        host.origin(),
        "POST",
        "/__gigacouch/v1/save/read",
        Some(br#"{"slot":"campaign"}"#),
    );
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"lives\":3"));
    let (status, _) = exchange(
        host.origin(),
        "POST",
        "/__gigacouch/v1/save/read",
        Some(br#"{"slot":"missing"}"#),
    );
    assert_eq!(status, 404);
    let (status, _) = exchange(
        host.origin(),
        "POST",
        "/__gigacouch/v1/save/write",
        Some(br#"{"slot":"../nope","data":1}"#),
    );
    assert_eq!(status, 400);
    let (status, body) = exchange(host.origin(), "POST", "/__gigacouch/v1/quit", Some(b""));
    assert_eq!(status, 200, "{body}");
    let (status, body) = exchange(host.origin(), "GET", "/__gigacouch/v1/control", None);
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"quit\":true"));
}

#[test]
fn menu_hears_confirm_and_back_without_a_separate_join_step() {
    let mut session = Session::new(16);
    let now = Instant::now();
    session.apply(post("pad-a", true, false), now);
    let snap = session.snapshot();
    assert!(snap.menu.confirm);
    assert!(!snap.menu.back);
    session.apply(post("pad-a", false, true), now);
    let snap = session.snapshot();
    assert!(snap.menu.back);
    assert_eq!(snap.players.len(), 1);
}

#[test]
fn home_signs_in_a_local_player_and_serves_blob_island() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let blob = root.join("../../runtimes/web/examples/blob-island/web");
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("home-profiles.json");
    let host = Host::start_home(&home, &[("blob-island", blob.as_path())], &profiles).unwrap();
    let (status, html) = exchange(host.origin(), "GET", "/", None);
    assert_eq!(status, 200, "{html}");
    assert!(html.contains("home.js"));
    let (status, script) = exchange(host.origin(), "GET", "/home.js", None);
    assert_eq!(status, 200, "{script}");
    assert!(script.contains("Who's on the couch"));
    let (status, body) = exchange(host.origin(), "GET", "/__gigacouch/v1/home", None);
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("Blob Island"));
    assert!(body.contains("Family"));
    let (status, _) = exchange(
        host.origin(),
        "POST",
        "/__gigacouch/v1/home/add",
        Some(br#"{"name":"Ada"}"#),
    );
    assert_eq!(status, 200);
    let (status, body) = exchange(host.origin(), "GET", "/__gigacouch/v1/home", None);
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("Ada"));
    let (status, game) = exchange(host.origin(), "GET", "/play/blob-island/game.js", None);
    assert_eq!(status, 200, "{game}");
    assert!(game.contains("canvas"));
    let (status, _) = exchange(
        host.origin(),
        "GET",
        "/play/blob-island/../../catalog.json",
        None,
    );
    assert_eq!(status, 404);
}

#[test]
fn home_serves_the_brand_fonts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("home-profiles.json");
    let host = Host::start_home(&home, &[], &profiles).unwrap();
    let (status, css) = exchange(host.origin(), "GET", "/home.css", None);
    assert_eq!(status, 200, "{css}");
    assert!(css.contains("fonts/SpaceGrotesk.woff2"), "{css}");
    assert!(css.contains("fonts/Inter.woff2"), "{css}");
    let raw =
        raw_exchange(host.origin(), "GET", "/fonts/SpaceGrotesk.woff2", None).to_ascii_lowercase();
    assert!(raw.contains("content-type: font/woff2"), "{raw}");
    let (status, font) = exchange(host.origin(), "GET", "/fonts/Inter.woff2", None);
    assert_eq!(status, 200, "{font}");
    assert!(font.starts_with("wOF2"), "woff2 magic missing");
    let (status, _) = exchange(host.origin(), "GET", "/fonts/../catalog.json", None);
    assert_eq!(status, 404);
}

#[test]
fn home_asks_for_a_godot_window_and_keeps_the_reply() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("home-profiles.json");
    let (tx, rx) = std::sync::mpsc::channel();
    let games = serde_json::json!([{
        "id": "little-world",
        "title": "Little World",
        "runtime": "godot",
        "playable": true,
        "players": "1–16",
        "description": "A tiny island.",
        "color": "#a5cfa1"
    }]);
    let host = Host::start_home_with(
        &home,
        &[],
        &profiles,
        Some(games),
        Some(tx),
        None,
        PhoneListen::Off,
    )
    .unwrap();
    let origin = host.origin().to_string();
    std::thread::spawn(move || {
        let request = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(request.id, "little-world");
        assert_eq!(request.profile, "family");
        request.reply.send(Ok("Little World".into())).unwrap();
    });
    let (status, body) = exchange(
        &origin,
        "POST",
        "/__gigacouch/v1/home/play",
        Some(br#"{"id":"little-world"}"#),
    );
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("own window"), "{body}");
}

#[test]
fn home_downloads_a_signed_in_package_and_serves_it() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let source = root.join("../../runtimes/web/examples/blob-island");
    let dir = tempfile::tempdir().unwrap();
    let packages = dir.path().join("packages");
    couch_platform::publish_blob_island(&packages, &source).unwrap();
    let platform = couch_platform::open(dir.path(), &packages, "http://127.0.0.1:9").unwrap();
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let origin = format!("http://127.0.0.1:{port}");
    std::thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let response = couch_platform::handle_request(&platform, &mut request);
            let _ = request.respond(response);
        }
    });
    let started: serde_json::Value =
        serde_json::from_str(&post_raw(&origin, "/v1/device", "{}")).unwrap();
    let user_code = started["user_code"].as_str().unwrap();
    let device = started["device_code"].as_str().unwrap();
    post_raw(
        &origin,
        "/v1/link",
        &format!(
            "{{\"user_code\":\"{user_code}\",\"name\":\"Ada\",\"passphrase\":\"couch-night\"}}"
        ),
    );
    let approved: serde_json::Value = serde_json::from_str(&post_raw(
        &origin,
        "/v1/device/token",
        &format!("{{\"device_code\":\"{device}\"}}"),
    ))
    .unwrap();
    let token = approved["token"].as_str().unwrap();
    let account_path = dir.path().join("account.json");
    std::fs::write(
        &account_path,
        format!("{{\"token\":\"{token}\",\"name\":\"Ada\"}}"),
    )
    .unwrap();
    let host = Host::start_home_with(
        &home,
        &[],
        &dir.path().join("profiles.json"),
        None,
        None,
        Some(couch_web_host::AccountStore {
            base: origin,
            token_path: account_path,
            install_root: dir.path().join("installed"),
        }),
        PhoneListen::Off,
    )
    .unwrap();
    let (status, body) = exchange(
        host.origin(),
        "POST",
        "/__gigacouch/v1/library/download",
        Some(br#"{"id":"blob-island"}"#),
    );
    assert_eq!(status, 200, "{body}");
    let (status, home_body) = exchange(host.origin(), "GET", "/__gigacouch/v1/home", None);
    assert_eq!(status, 200, "{home_body}");
    let listed: serde_json::Value = serde_json::from_str(&home_body).unwrap();
    let cards = listed["games"].as_array().unwrap();
    let blob = cards
        .iter()
        .find(|game| game["id"] == "blob-island")
        .unwrap();
    assert_eq!(blob["place"], "here", "{blob}");
    assert_eq!(blob["installed"], true, "{blob}");
    let waiting = cards
        .iter()
        .find(|game| game["id"] == "little-world")
        .unwrap();
    assert_eq!(waiting["place"], "library", "{waiting}");
    let (status, game) = exchange(host.origin(), "GET", "/play/blob-island/game.js", None);
    assert_eq!(status, 200, "{game}");
    assert!(game.contains("canvas"), "{game}");
}

#[test]
fn downloaded_game_stays_playable_when_the_library_server_is_down() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let dir = tempfile::tempdir().unwrap();
    let install = dir.path().join("installed").join("blob-island");
    std::fs::create_dir_all(install.join("web")).unwrap();
    std::fs::write(install.join("gigacouch.json"), r#"{"title":"Blob Island"}"#).unwrap();
    std::fs::write(
        install.join("web/index.html"),
        "<!DOCTYPE html><head></head><body></body>",
    )
    .unwrap();
    std::fs::write(install.join("web/game.js"), "/* downloaded canvas */").unwrap();
    let account_path = dir.path().join("account.json");
    std::fs::write(&account_path, r#"{"token":"saved-token","name":"Ada"}"#).unwrap();
    let host = Host::start_home_with(
        &home,
        &[],
        &dir.path().join("profiles.json"),
        Some(serde_json::json!([])),
        None,
        Some(couch_web_host::AccountStore {
            base: "http://127.0.0.1:9".into(),
            token_path: account_path.clone(),
            install_root: dir.path().join("installed"),
        }),
        PhoneListen::Off,
    )
    .unwrap();
    let (status, body) = exchange(host.origin(), "GET", "/__gigacouch/v1/home", None);
    assert_eq!(status, 200, "{body}");
    let listed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(listed["account"]["server"], "down");
    assert_eq!(listed["account"]["name"], "Ada");
    let cards = listed["games"].as_array().unwrap();
    let blob = cards
        .iter()
        .find(|game| game["id"] == "blob-island")
        .unwrap();
    assert_eq!(blob["place"], "here", "{blob}");
    assert_eq!(blob["installed"], true, "{blob}");
    assert_eq!(blob["playable"], true, "{blob}");
    let (status, game) = exchange(host.origin(), "GET", "/play/blob-island/game.js", None);
    assert_eq!(status, 200, "{game}");
    assert!(game.contains("downloaded canvas"), "{game}");
    assert!(account_path.is_file());
}

fn post_raw(origin: &str, path: &str, body: &str) -> String {
    let (status, text) = exchange(origin, "POST", path, Some(body.as_bytes()));
    assert!(status == 200 || status == 400, "{status} {text}");
    text
}

fn post(id: &str, south: bool, east: bool) -> DevicePost {
    serde_json::from_value(json!({
        "devices": [{
            "id": id,
            "kind": "pad",
            "name": "Pad",
            "family": "xbox",
            "south": south,
            "east": east,
            "analog": false,
            "move": { "x": 0, "y": 0 }
        }]
    }))
    .unwrap()
}

fn analog(id: &str, x: f32, y: f32) -> DevicePost {
    serde_json::from_value(json!({
        "devices": [{
            "id": id,
            "kind": "pad",
            "name": "Pad",
            "family": "xbox",
            "south": false,
            "east": false,
            "analog": true,
            "move": { "x": x, "y": y }
        }]
    }))
    .unwrap()
}

fn keyboard(south: bool, jump: bool, leave: bool) -> DevicePost {
    serde_json::from_value(json!({
        "devices": [{
            "id": "keyboard",
            "kind": "keyboard",
            "name": "Keyboard",
            "family": "keyboard",
            "south": south,
            "jump": jump,
            "leave": leave,
            "analog": false,
            "move": { "x": 1, "y": 0 }
        }]
    }))
    .unwrap()
}

fn phone_home(dir: &std::path::Path) -> Host {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    Host::start_home_with(
        &home,
        &[],
        &dir.join("profiles.json"),
        Some(json!([])),
        None,
        None,
        PhoneListen::Loopback,
    )
    .unwrap()
}

fn phones(host: &Host) -> serde_json::Value {
    let (status, body) = exchange(host.origin(), "GET", "/__gigacouch/v1/phones", None);
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

fn post_devices(host: &Host) {
    let (status, body) = exchange(
        host.origin(),
        "POST",
        "/__gigacouch/v1/devices",
        Some(br#"{"devices":[]}"#),
    );
    assert_eq!(status, 200, "{body}");
}

/// The next message from the host that contains `needle`. The host sends a
/// hello on connect and pushes changes as they happen, so replies can queue.
fn next_with<S: std::io::Read + std::io::Write>(
    socket: &mut tungstenite::WebSocket<S>,
    needle: &str,
) -> String {
    for _ in 0..50 {
        let message = socket.read().expect("phone socket");
        if let Ok(text) = message.to_text()
            && text.contains(needle)
        {
            return text.to_string();
        }
    }
    panic!("no message containing {needle}");
}

fn wait_for(what: &str, mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !check() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn home_offers_a_phone_join_address_with_a_qr_code() {
    let dir = tempfile::tempdir().unwrap();
    let host = phone_home(dir.path());
    let status = phones(&host);
    assert_eq!(status["enabled"], true);
    let url = status["join_url"].as_str().unwrap();
    let port = host.phone_port().unwrap();
    assert!(
        url.starts_with(&format!("http://127.0.0.1:{port}/p/")),
        "{url}"
    );
    assert!(status["qr_svg"].as_str().unwrap().starts_with("<svg"));
    assert_eq!(status["phones"], json!([]));

    let phone = format!("http://127.0.0.1:{port}");
    let path = url.trim_start_matches(&phone).to_string();
    let (code, page) = exchange(&phone, "GET", &path, None);
    assert_eq!(code, 200);
    assert!(page.contains("Back to the shelf"));
    assert!(page.contains("Reload controller"));
    // The host's content policy blocks inline scripts, so the pad's script
    // must come from its own file on this origin.
    assert!(!page.contains("<script>"), "inline script would be blocked");
    assert!(page.contains(&format!("{path}/pad.js")));
    let (code, script) = exchange(&phone, "GET", &format!("{path}/pad.js"), None);
    assert_eq!(code, 200);
    assert!(script.contains("new WebSocket"));
    assert!(script.contains("Drag anywhere here to move"));
    assert!(
        !script.contains("{{PAD_VERSION}}"),
        "the host fills in the pad version"
    );
    let raw = raw_exchange(&phone, "GET", &path, None).to_ascii_lowercase();
    assert!(raw.contains("script-src 'self'"));
}

#[test]
fn the_phone_listener_serves_nothing_but_the_pad() {
    let dir = tempfile::tempdir().unwrap();
    let host = phone_home(dir.path());
    let phone = format!("http://127.0.0.1:{}", host.phone_port().unwrap());
    for path in [
        "/",
        "/home.js",
        "/p/WRONGCODE",
        "/p/WRONGCODE/ws?id=abcdefgh12",
        "/__gigacouch/v1/home",
        "/__gigacouch/v1/phones",
        "/__gigacouch/bridge.js",
    ] {
        let (status, _) = exchange(&phone, "GET", path, None);
        assert_eq!(status, 404, "{path} must not be served to the network");
    }
    let (status, _) = exchange(
        &phone,
        "POST",
        "/__gigacouch/v1/save/write",
        Some(br#"{"slot":"campaign","data":{}}"#),
    );
    assert_eq!(status, 404);
}

#[test]
fn a_phone_joins_as_a_player_and_leaves_when_it_disconnects() {
    use tungstenite::Message;

    let dir = tempfile::tempdir().unwrap();
    let host = phone_home(dir.path());
    let url = phones(&host)["join_url"].as_str().unwrap().to_string();
    let socket_url = format!("{}/ws?id=testphone01", url.replacen("http://", "ws://", 1));
    let (mut socket, _) = tungstenite::connect(socket_url.as_str()).unwrap();

    let send = |socket: &mut tungstenite::WebSocket<_>, south: bool| {
        let state = json!({"name": "Ada's phone", "south": south, "x": 0.5, "y": 0});
        socket
            .send(Message::Text(state.to_string().into()))
            .unwrap();
    };
    send(&mut socket, false);
    wait_for("the phone to show up", || {
        phones(&host)["phones"].as_array().unwrap().len() == 1
    });
    let listed = phones(&host);
    assert_eq!(listed["phones"][0]["name"], "Ada's phone");
    assert_eq!(listed["phones"][0]["player"], serde_json::Value::Null);

    // The page's next device post carries the phone; south joins.
    post_devices(&host);
    send(&mut socket, true);
    wait_for("the phone to join", || {
        post_devices(&host);
        phones(&host)["phones"][0]["player"] == 1
    });
    let (_, snapshot) = exchange(host.origin(), "GET", "/__gigacouch/v1/snapshot", None);
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(snapshot["players"][0]["name"], "Ada's phone");
    assert_eq!(snapshot["players"][0]["glyphs"]["jump"], "A");

    // The phone hears which player it is.
    send(&mut socket, false);
    wait_for("the player reply", || match socket.read() {
        Ok(Message::Text(text)) => text.as_str().contains("\"player\":1"),
        _ => false,
    });

    socket.close(None).unwrap();
    let _ = socket.read();
    wait_for("the phone to leave", || {
        phones(&host)["phones"].as_array().unwrap().is_empty()
    });
    post_devices(&host);
    let (_, snapshot) = exchange(host.origin(), "GET", "/__gigacouch/v1/snapshot", None);
    // A phone that drops keeps its spot for a while, marked away.
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(snapshot["players"][0]["away"], true, "{snapshot}");
    assert_eq!(snapshot["players"][0]["kind"], "phone");
}

#[test]
fn the_phone_page_draws_exactly_the_layouts_the_host_knows() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = fs::read_to_string(root.join("src/assets/pad.js")).unwrap();
    let table = &script
        [script.find("var LAYOUTS = {").unwrap()..script.find("var DEFAULT_LAYOUT").unwrap()];
    let drawn: Vec<&str> = table
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix('"')
                .and_then(|rest| rest.strip_suffix("\": {"))
        })
        .collect();
    assert_eq!(
        drawn, PHONE_LAYOUTS,
        "pad.js and PHONE_LAYOUTS must list the same layouts"
    );
    assert!(script.contains(&format!("var DEFAULT_LAYOUT = \"{DEFAULT_PHONE_LAYOUT}\"")));
}

#[test]
fn a_package_names_its_phone_layout() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), 4, "web/index.html");
    assert_eq!(
        WebPackage::read(dir.path()).unwrap().phone_layout(),
        "stick-2"
    );

    let manifest = dir.path().join("gigacouch.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    value["phone"] = json!({ "layout": "quiz-4" });
    fs::write(&manifest, value.to_string()).unwrap();
    assert_eq!(
        WebPackage::read(dir.path()).unwrap().phone_layout(),
        "quiz-4"
    );

    value["phone"] = json!({ "layout": "piano" });
    fs::write(&manifest, value.to_string()).unwrap();
    let error = WebPackage::read(dir.path()).unwrap_err().to_string();
    assert!(error.contains("phone.layout must be one of"), "{error}");
}

fn phone_device(extra: serde_json::Value) -> DevicePost {
    let mut device = json!({"id": "phone:abc", "kind": "phone", "name": "Ada", "family": "phone", "analog": true});
    for (key, value) in extra.as_object().unwrap() {
        device[key] = value.clone();
    }
    serde_json::from_value(json!({ "devices": [device] })).unwrap()
}

#[test]
fn extra_buttons_and_the_look_stick_reach_the_game() {
    let mut session = Session::new(4);
    let now = Instant::now();
    session.apply(phone_device(json!({"south": true})), now);
    session.apply(phone_device(json!({})), now);
    let _ = session.snapshot();
    session.apply(
        phone_device(json!({"west": true, "start": true, "look": {"x": 1.0, "y": 0.0}})),
        now,
    );
    let first = serde_json::to_value(session.snapshot()).unwrap();
    let player = &first["players"][0];
    assert_eq!(player["pressed"]["west"], true);
    assert_eq!(player["pressed"]["start"], true);
    assert_eq!(player["pressed"]["north"], false);
    assert_eq!(player["buttons"]["west"], true);
    assert!(player["look"]["x"].as_f64().unwrap() > 0.9);
    // A press is reported once; holding keeps it in `buttons` only.
    session.apply(phone_device(json!({"west": true})), now);
    let second = serde_json::to_value(session.snapshot()).unwrap();
    assert_eq!(second["players"][0]["pressed"]["west"], false);
    assert_eq!(second["players"][0]["buttons"]["west"], true);
}

#[test]
fn a_phone_leaves_with_leave_and_holding_east_does_not_drop_it() {
    let mut session = Session::new(4);
    let start = Instant::now();
    session.apply(phone_device(json!({"south": true})), start);
    session.apply(phone_device(json!({})), start);
    session.apply(phone_device(json!({"east": true})), start);
    session.apply(
        phone_device(json!({"east": true})),
        start + Duration::from_secs(3),
    );
    assert_eq!(
        session.player_of("phone:abc"),
        Some(1),
        "east is a game button on a phone"
    );
    session.apply(
        phone_device(json!({"leave": true})),
        start + Duration::from_secs(4),
    );
    assert_eq!(session.player_of("phone:abc"), None);
}

#[test]
fn phones_switch_to_the_open_games_layout_and_back() {
    use tungstenite::Message;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let dir = tempfile::tempdir().unwrap();
    let game = dir.path().join("quiz");
    package(&game, 8, "web/index.html");
    let manifest = game.join("gigacouch.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    value["phone"] = json!({ "layout": "quiz-4" });
    fs::write(&manifest, value.to_string()).unwrap();
    let web = game.join("web");
    let host = Host::start_home_with(
        &home,
        &[("quiz", web.as_path())],
        &dir.path().join("profiles.json"),
        Some(json!([])),
        None,
        None,
        PhoneListen::Loopback,
    )
    .unwrap();
    assert_eq!(phones(&host)["layout"], "stick-2");

    let url = phones(&host)["join_url"].as_str().unwrap().to_string();
    let socket_url = format!("{}/ws?id=layoutphone1", url.replacen("http://", "ws://", 1));
    let (mut socket, _) = tungstenite::connect(socket_url.as_str()).unwrap();
    let hello = json!({"name": "Ada"}).to_string();
    socket.send(Message::Text(hello.clone().into())).unwrap();
    next_with(&mut socket, "\"layout\":\"stick-2\"");

    let (status, _) = exchange(host.origin(), "GET", "/play/quiz/", None);
    assert_eq!(status, 200);
    assert_eq!(phones(&host)["layout"], "quiz-4");
    // A game's scripts and images do not change the layout.
    let (status, _) = exchange(host.origin(), "GET", "/play/quiz/game.js", None);
    assert_eq!(status, 200);
    assert_eq!(phones(&host)["layout"], "quiz-4");
    // The switch is pushed to the phone without it asking.
    next_with(&mut socket, "\"layout\":\"quiz-4\"");

    let (status, _) = exchange(host.origin(), "GET", "/", None);
    assert_eq!(status, 200);
    assert_eq!(phones(&host)["layout"], "stick-2");
}

#[test]
fn the_pad_installs_to_a_home_screen_and_keeps_its_address() {
    let dir = tempfile::tempdir().unwrap();
    let first_url = {
        let host = phone_home(dir.path());
        let url = phones(&host)["join_url"].as_str().unwrap().to_string();
        let phone = format!("http://127.0.0.1:{}", host.phone_port().unwrap());
        let path = url.trim_start_matches(&phone).to_string();
        let (status, page) = exchange(&phone, "GET", &path, None);
        assert_eq!(status, 200);
        assert!(page.contains(&format!("{path}/manifest.webmanifest")));
        assert!(page.contains(&format!("{path}/icon.png")));
        let (status, manifest) =
            exchange(&phone, "GET", &format!("{path}/manifest.webmanifest"), None);
        assert_eq!(status, 200);
        let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        assert_eq!(manifest["start_url"], path.as_str());
        let raw = raw_exchange(&phone, "GET", &format!("{path}/icon.png"), None);
        assert!(raw.to_ascii_lowercase().contains("content-type: image/png"));
        path
    };
    // A restarted Home keeps the join code, so a home-screen pad still works.
    let host = phone_home(dir.path());
    let url = phones(&host)["join_url"].as_str().unwrap().to_string();
    assert!(url.ends_with(&first_url), "{url} should keep {first_url}");
}

#[test]
fn a_phone_can_send_the_tv_back_to_the_shelf_once() {
    use tungstenite::Message;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let dir = tempfile::tempdir().unwrap();
    let game = dir.path().join("blob");
    package(&game, 8, "web/index.html");
    let web = game.join("web");
    let host = Host::start_home_with(
        &home,
        &[("blob", web.as_path())],
        &dir.path().join("profiles.json"),
        Some(json!([])),
        None,
        None,
        PhoneListen::Loopback,
    )
    .unwrap();
    let control = |host: &Host| -> serde_json::Value {
        let (_, body) = exchange(host.origin(), "GET", "/__gigacouch/v1/control", None);
        serde_json::from_str(&body).unwrap()
    };
    let url = phones(&host)["join_url"].as_str().unwrap().to_string();
    let socket_url = format!("{}/ws?id=menuphone01", url.replacen("http://", "ws://", 1));
    let (mut socket, _) = tungstenite::connect(socket_url.as_str()).unwrap();
    let ask = json!({"name": "Ada", "shelf": true}).to_string();

    // On the shelf there is no game to leave.
    socket.send(Message::Text(ask.clone().into())).unwrap();
    next_with(&mut socket, "\"player\"");
    assert_eq!(control(&host)["shelf"], false);

    let (status, _) = exchange(host.origin(), "GET", "/play/blob/", None);
    assert_eq!(status, 200);
    next_with(&mut socket, "\"game\":true");
    socket.send(Message::Text(ask.into())).unwrap();
    wait_for("the shelf request", || control(&host)["shelf"] == true);
    assert_eq!(control(&host)["shelf"], false, "reported once");
    assert_eq!(control(&host)["quit"], false, "the app keeps running");
}

fn pad_device(index: usize, south: bool) -> serde_json::Value {
    json!({"id": format!("phone:p{index:03}"), "kind": "phone", "name": format!("Phone {index}"), "family": "phone", "south": south, "analog": true})
}

#[test]
fn without_a_game_limit_everyone_who_joins_plays() {
    let mut session = Session::unlimited();
    let now = Instant::now();
    let pressed: Vec<_> = (0..50).map(|index| pad_device(index, true)).collect();
    session.apply(
        serde_json::from_value(json!({ "devices": pressed })).unwrap(),
        now,
    );
    let snapshot = serde_json::to_value(session.snapshot()).unwrap();
    let players = snapshot["players"].as_array().unwrap();
    assert_eq!(players.len(), 50);
    assert_eq!(players[49]["id"], 50);

    // A game with room for four keeps the first four; the rest can rejoin
    // once the shelf lifts the limit.
    session.set_limit(Some(4));
    assert_eq!(session.player_of("phone:p003"), Some(4));
    assert_eq!(session.player_of("phone:p004"), None);
    let released: Vec<_> = (0..50).map(|index| pad_device(index, false)).collect();
    session.apply(
        serde_json::from_value(json!({ "devices": released })).unwrap(),
        now,
    );
    let again: Vec<_> = (0..50).map(|index| pad_device(index, index >= 4)).collect();
    session.apply(
        serde_json::from_value(json!({ "devices": again.clone() })).unwrap(),
        now,
    );
    assert_eq!(session.player_of("phone:p010"), None, "the game is full");
    session.set_limit(None);
    session.apply(
        serde_json::from_value(json!({ "devices": released })).unwrap(),
        now,
    );
    session.apply(
        serde_json::from_value(json!({ "devices": again })).unwrap(),
        now,
    );
    assert!(
        session.player_of("phone:p010").is_some(),
        "room again on the shelf"
    );
}

#[test]
fn a_package_may_leave_out_the_player_maximum_or_go_past_sixteen() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), 4, "web/index.html");
    let manifest = dir.path().join("gigacouch.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    value["players"] = json!({ "min": 3 });
    fs::write(&manifest, value.to_string()).unwrap();
    assert_eq!(WebPackage::read(dir.path()).unwrap().players_max(), None);
    value["players"] = json!({ "min": 3, "max": 48 });
    fs::write(&manifest, value.to_string()).unwrap();
    assert_eq!(
        WebPackage::read(dir.path()).unwrap().players_max(),
        Some(48)
    );
    value["players"] = json!({ "min": 5, "max": 4 });
    fs::write(&manifest, value.to_string()).unwrap();
    assert!(WebPackage::read(dir.path()).is_err());
}

#[test]
fn forty_phones_join_home_as_forty_players() {
    use tungstenite::Message;

    let dir = tempfile::tempdir().unwrap();
    let host = phone_home(dir.path());
    let url = phones(&host)["join_url"].as_str().unwrap().to_string();
    let base = url.replacen("http://", "ws://", 1);
    let mut sockets: Vec<_> = (0..40)
        .map(|index| {
            let (socket, _) =
                tungstenite::connect(format!("{base}/ws?id=crowd{index:04}")).unwrap();
            socket
        })
        .collect();
    for (index, socket) in sockets.iter_mut().enumerate() {
        let state = json!({"name": format!("Guest {index}"), "south": true});
        socket
            .send(Message::Text(state.to_string().into()))
            .unwrap();
    }
    wait_for("forty phones to join", || {
        post_devices(&host);
        let status = phones(&host);
        let list = status["phones"].as_array().unwrap();
        list.len() == 40 && list.iter().all(|phone| phone["player"].is_u64())
    });
    let status = phones(&host);
    let mut numbers: Vec<u64> = status["phones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|phone| phone["player"].as_u64().unwrap())
        .collect();
    numbers.sort_unstable();
    assert_eq!(numbers, (1..=40).collect::<Vec<u64>>());
}

#[test]
fn home_raises_the_open_file_limit_for_many_phones() {
    couch_web_host::raise_open_file_limit();
    #[cfg(unix)]
    {
        let mut limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: reads into a struct on this stack frame.
        assert_eq!(
            unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
            0
        );
        assert!(
            limit.rlim_cur >= limit.rlim_max.min(10240),
            "soft limit {} should reach the hard limit or 10240",
            limit.rlim_cur
        );
    }
}

#[test]
fn the_sample_games_are_valid_packages() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/web/examples");
    let blob = WebPackage::read(&root.join("blob-island")).unwrap();
    assert_eq!(blob.players_max(), Some(16));
    let gallery = WebPackage::read(&root.join("controller-gallery")).unwrap();
    assert_eq!(
        gallery.players_max(),
        None,
        "the gallery has no player limit"
    );
}

#[test]
fn a_game_switches_the_phone_layout_while_it_runs() {
    use tungstenite::Message;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let gallery = root.join("../../runtimes/web/examples/controller-gallery/web");
    let dir = tempfile::tempdir().unwrap();
    let host = Host::start_home_with(
        &home,
        &[("controller-gallery", gallery.as_path())],
        &dir.path().join("profiles.json"),
        Some(json!([])),
        None,
        None,
        PhoneListen::Loopback,
    )
    .unwrap();
    let set = |layout: &str| {
        exchange(
            host.origin(),
            "POST",
            "/__gigacouch/v1/phone/layout",
            Some(json!({ "layout": layout }).to_string().as_bytes()),
        )
    };
    // On the shelf the layout stays the default.
    assert_eq!(set("racing").0, 400);

    let (status, page) = exchange(host.origin(), "GET", "/play/controller-gallery/", None);
    assert_eq!(status, 200);
    assert!(page.contains("gallery.js"));
    let (_, info) = exchange(host.origin(), "GET", "/__gigacouch/v1/phone/layout", None);
    let info: serde_json::Value = serde_json::from_str(&info).unwrap();
    assert_eq!(info["layout"], "stick-2");
    assert_eq!(
        info["layouts"].as_array().unwrap().len(),
        PHONE_LAYOUTS.len()
    );

    assert_eq!(set("racing").0, 200);
    assert_eq!(set("theremin").0, 400);
    assert_eq!(phones(&host)["layout"], "racing");

    let url = phones(&host)["join_url"].as_str().unwrap().to_string();
    let socket_url = format!("{}/ws?id=galleryphone", url.replacen("http://", "ws://", 1));
    let (mut socket, _) = tungstenite::connect(socket_url.as_str()).unwrap();
    socket
        .send(Message::Text(json!({"name": "Ada"}).to_string().into()))
        .unwrap();
    let reply: serde_json::Value =
        serde_json::from_str(&next_with(&mut socket, "\"pad\"")).unwrap();
    assert_eq!(reply["layout"], "racing");
    // The reply carries the pad version the page was served with, so an
    // older pad knows to reload.
    let phone = format!("http://127.0.0.1:{}", host.phone_port().unwrap());
    let path = url.trim_start_matches(&phone);
    let (_, script) = exchange(&phone, "GET", &format!("{path}/pad.js"), None);
    let version = reply["pad"].as_str().unwrap();
    assert!(script.contains(&format!("var PAD_VERSION = \"{version}\"")));
}

#[test]
fn slider_and_touchpad_positions_skip_the_dead_zone() {
    let mut session = Session::new(4);
    let now = Instant::now();
    let device = |x: f64, lx: f64, south: bool| -> DevicePost {
        serde_json::from_value(json!({"devices": [{
            "id": "phone:slide", "kind": "phone", "name": "Ada", "family": "phone",
            "south": south, "analog": true, "absolute": true,
            "move": {"x": x, "y": 0.0}, "look": {"x": lx, "y": 1.0}
        }]}))
        .unwrap()
    };
    session.apply(device(0.0, 0.0, true), now);
    session.apply(device(0.1, 1.0, false), now);
    let snapshot = serde_json::to_value(session.snapshot()).unwrap();
    let player = &snapshot["players"][0];
    // A stick would read 0.1 as rest; a slider keeps it.
    assert!((player["move"]["x"].as_f64().unwrap() - 0.1).abs() < 1e-6);
    // A touchpad corner stays a corner instead of being pulled onto a circle.
    assert_eq!(player["look"]["x"], 1.0);
    assert_eq!(player["look"]["y"], 1.0);
}

#[test]
fn the_phone_page_plays_exactly_the_stock_sounds_the_host_knows() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = fs::read_to_string(root.join("src/assets/pad.js")).unwrap();
    let table = &script
        [script.find("var STOCK_SOUNDS = {").unwrap()..script.find("function playSound").unwrap()];
    let names: Vec<&str> = table
        .lines()
        .filter_map(|line| line.trim().split_once(": function").map(|(name, _)| name))
        .collect();
    assert_eq!(names, couch_web_host::STOCK_SOUNDS);
}

fn with_sounds(dir: &std::path::Path, sounds: serde_json::Value) -> Result<WebPackage, String> {
    let manifest = dir.join("gigacouch.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    value["phone"] = json!({ "sounds": sounds });
    fs::write(&manifest, value.to_string()).unwrap();
    WebPackage::read(dir).map_err(|error| error.to_string())
}

#[test]
fn a_package_checks_its_own_phone_sounds() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), 4, "web/index.html");
    fs::create_dir_all(dir.path().join("web/sounds")).unwrap();
    fs::write(dir.path().join("web/sounds/honk.wav"), b"RIFF-sample-bytes").unwrap();
    fs::write(dir.path().join("web/sounds/song.ogg"), b"OggS").unwrap();
    fs::write(
        dir.path().join("web/sounds/huge.mp3"),
        vec![0_u8; 300 * 1024],
    )
    .unwrap();
    fs::write(dir.path().join("outside.mp3"), b"ID3").unwrap();

    let package = with_sounds(dir.path(), json!({ "honk": "sounds/honk.wav" })).unwrap();
    assert_eq!(
        package.phone_layout(),
        "stick-2",
        "sounds alone keep the default layout"
    );
    assert_eq!(package.phone_sounds().len(), 1);
    assert_eq!(package.phone_sounds()[0].content_type, "audio/wav");

    for (sounds, expected) in [
        (json!({ "song": "sounds/song.ogg" }), ".mp3, .m4a, or .wav"),
        (json!({ "gone": "sounds/gone.mp3" }), "is missing"),
        (json!({ "ding": "sounds/honk.wav" }), "is a stock sound"),
        (json!({ "sneaky": "../outside.mp3" }), "inside web/"),
        (json!({ "huge": "sounds/huge.mp3" }), "KiB"),
        (json!({ "Bad Name": "sounds/honk.wav" }), "lowercase"),
    ] {
        let error = with_sounds(dir.path(), sounds.clone()).unwrap_err();
        assert!(error.contains(expected), "{sounds}: {error}");
    }
}

#[test]
fn the_gallery_ships_its_own_sounds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/web/examples");
    let gallery = WebPackage::read(&root.join("controller-gallery")).unwrap();
    assert_eq!(gallery.phone_sounds().len(), 9);
}

fn gallery_home(dir: &std::path::Path) -> Host {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let gallery = root.join("../../runtimes/web/examples/controller-gallery/web");
    Host::start_home_with(
        &home,
        &[("controller-gallery", gallery.as_path())],
        &dir.join("profiles.json"),
        Some(json!([])),
        None,
        None,
        PhoneListen::Loopback,
    )
    .unwrap()
}

#[test]
fn a_game_s_sounds_reach_phones_when_it_opens_and_leave_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let host = gallery_home(dir.path());
    let url = phones(&host)["join_url"].as_str().unwrap().to_string();
    let phone = format!("http://127.0.0.1:{}", host.phone_port().unwrap());
    let (mut socket, _) = tungstenite::connect(format!(
        "{}/ws?id=soundphone1",
        url.replacen("http://", "ws://", 1)
    ))
    .unwrap();
    next_with(&mut socket, "\"sounds\":{}");

    let (status, _) = exchange(host.origin(), "GET", "/play/controller-gallery/", None);
    assert_eq!(status, 200);
    let pushed: serde_json::Value =
        serde_json::from_str(&next_with(&mut socket, "\"boing\"")).unwrap();
    let boing = pushed["sounds"]["boing"].as_str().unwrap().to_string();
    assert!(boing.ends_with(".wav"));
    assert_eq!(pushed["sounds"].as_object().unwrap().len(), 9);

    let raw = raw_exchange(&phone, "GET", &boing, None);
    let lower = raw.to_ascii_lowercase();
    assert!(
        lower.starts_with("http/1.1 200"),
        "{}",
        &raw[..raw.len().min(120)]
    );
    assert!(lower.contains("content-type: audio/wav"));
    assert!(lower.contains("immutable"), "sound files are cacheable");
    let other = format!("{}/0000000000000000.wav", boing.rsplit_once('/').unwrap().0);
    let (status, _) = exchange(&phone, "GET", &other, None);
    assert_eq!(status, 404, "only declared sounds are served");

    let (status, _) = exchange(host.origin(), "GET", "/", None);
    assert_eq!(status, 200);
    next_with(&mut socket, "\"sounds\":{}");
    let (status, _) = exchange(&phone, "GET", &boing, None);
    assert_eq!(status, 404, "the shelf serves no game sounds");
}

#[test]
fn a_game_plays_a_sound_on_one_player_s_phone_or_all_of_them() {
    use tungstenite::Message;

    let dir = tempfile::tempdir().unwrap();
    let host = gallery_home(dir.path());
    let base = phones(&host)["join_url"]
        .as_str()
        .unwrap()
        .replacen("http://", "ws://", 1);
    let (mut ada, _) = tungstenite::connect(format!("{base}/ws?id=adaphone01")).unwrap();
    let (mut bob, _) = tungstenite::connect(format!("{base}/ws?id=bobphone01")).unwrap();
    ada.send(Message::Text(
        json!({"name": "Ada", "south": true}).to_string().into(),
    ))
    .unwrap();
    bob.send(Message::Text(json!({"name": "Bob"}).to_string().into()))
        .unwrap();
    wait_for("Ada to join", || {
        post_devices(&host);
        phones(&host)["phones"]
            .as_array()
            .unwrap()
            .iter()
            .any(|phone| phone["player"] == 1)
    });
    let (status, _) = exchange(host.origin(), "GET", "/play/controller-gallery/", None);
    assert_eq!(status, 200);
    let play = |player: serde_json::Value, name: &str| {
        exchange(
            host.origin(),
            "POST",
            "/__gigacouch/v1/phone/sound",
            Some(
                json!({"player": player, "name": name})
                    .to_string()
                    .as_bytes(),
            ),
        )
    };

    // Player 1 is Ada: only her phone hears it, straight away.
    let started = Instant::now();
    let (status, body) = play(json!(1), "boing");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"phones\":1"));
    next_with(&mut ada, "\"sound\":\"boing\"");
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "pushed, not waiting for the phone"
    );

    let (status, body) = play(json!("all"), "ding");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"phones\":2"));
    next_with(&mut ada, "\"sound\":\"ding\"");
    let heard = next_with(&mut bob, "\"sound\"");
    assert!(
        heard.contains("\"ding\""),
        "Bob hears the stock sound, not Ada's boing: {heard}"
    );

    assert_eq!(play(json!("all"), "piano").0, 400);
    assert_eq!(play(json!("everyone"), "ding").0, 400);
}

#[test]
fn a_game_rumbles_one_player_s_phone_and_sees_who_can_vibrate() {
    use tungstenite::Message;

    let dir = tempfile::tempdir().unwrap();
    let host = gallery_home(dir.path());
    let base = phones(&host)["join_url"]
        .as_str()
        .unwrap()
        .replacen("http://", "ws://", 1);
    let (mut android, _) = tungstenite::connect(format!("{base}/ws?id=androidphone")).unwrap();
    let (mut iphone, _) = tungstenite::connect(format!("{base}/ws?id=iphonephone1")).unwrap();
    android
        .send(Message::Text(
            json!({"name": "Ada", "south": true, "rumble": true})
                .to_string()
                .into(),
        ))
        .unwrap();
    iphone
        .send(Message::Text(
            json!({"name": "Bob", "rumble": false}).to_string().into(),
        ))
        .unwrap();
    wait_for("Ada to join", || {
        post_devices(&host);
        phones(&host)["phones"]
            .as_array()
            .unwrap()
            .iter()
            .any(|phone| phone["player"] == 1)
    });
    let listed = phones(&host);
    let ada = listed["phones"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Ada")
        .unwrap()
        .clone();
    let bob = listed["phones"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Bob")
        .unwrap()
        .clone();
    assert_eq!(ada["rumble"], true);
    assert_eq!(bob["rumble"], false);

    let rumble = |player: serde_json::Value, pattern: serde_json::Value| {
        exchange(
            host.origin(),
            "POST",
            "/__gigacouch/v1/phone/rumble",
            Some(
                json!({"player": player, "pattern": pattern})
                    .to_string()
                    .as_bytes(),
            ),
        )
    };
    let (status, body) = rumble(json!(1), json!("hit"));
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"phones\":1"));
    next_with(&mut android, "\"rumble\":[90]");

    let (status, body) = rumble(json!("all"), json!([30, 40, 30]));
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"phones\":2"));
    next_with(&mut iphone, "\"rumble\":[30,40,30]");

    assert_eq!(rumble(json!(1), json!("earthquake")).0, 400);
    assert_eq!(rumble(json!(1), json!(9000)).0, 400);
}

// ---- Phone features games opt into -------------------------------------------

fn lab_home(dir: &std::path::Path) -> Host {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = root.join("../../runtimes/web/home");
    let lab = root.join("../../runtimes/web/examples/phone-lab/web");
    Host::start_home_with(
        &home,
        &[("phone-lab", lab.as_path())],
        &dir.join("profiles.json"),
        Some(json!([])),
        None,
        None,
        PhoneListen::Loopback,
    )
    .unwrap()
}

fn phone_socket(
    host: &Host,
    id: &str,
) -> tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>> {
    let base = phones(host)["join_url"]
        .as_str()
        .unwrap()
        .replacen("http://", "ws://", 1);
    tungstenite::connect(format!("{base}/ws?id={id}"))
        .unwrap()
        .0
}

fn say<S: std::io::Read + std::io::Write>(
    socket: &mut tungstenite::WebSocket<S>,
    value: serde_json::Value,
) {
    socket
        .send(tungstenite::Message::Text(value.to_string().into()))
        .unwrap();
}

fn post_json(host: &Host, path: &str, body: serde_json::Value) -> (u16, String) {
    exchange(
        host.origin(),
        "POST",
        path,
        Some(body.to_string().as_bytes()),
    )
}

fn join_player<S: std::io::Read + std::io::Write>(
    host: &Host,
    socket: &mut tungstenite::WebSocket<S>,
    name: &str,
) {
    say(socket, json!({"name": name, "south": true}));
    wait_for("the phone to join", || {
        post_devices(host);
        phones(host)["phones"]
            .as_array()
            .unwrap()
            .iter()
            .any(|phone| phone["name"] == name && phone["player"].is_u64())
    });
    say(socket, json!({"name": name}));
}

fn snapshot_json(host: &Host) -> serde_json::Value {
    let (_, body) = exchange(host.origin(), "GET", "/__gigacouch/v1/snapshot", None);
    serde_json::from_str(&body).unwrap()
}

#[test]
fn the_phone_lab_ships_its_own_layouts_and_images() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/web/examples");
    let lab = WebPackage::read(&root.join("phone-lab")).unwrap();
    assert_eq!(lab.phone_layout(), "lab-pad");
    assert_eq!(lab.phone_layouts().len(), 4);
    assert_eq!(lab.phone_images().len(), 5);
    assert_eq!(lab.players_max(), None);
}

#[test]
fn a_package_checks_its_own_layouts() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), 4, "web/index.html");
    let manifest = dir.path().join("gigacouch.json");
    let write = |phone: serde_json::Value| -> Result<WebPackage, String> {
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
        value["phone"] = phone;
        fs::write(&manifest, value.to_string()).unwrap();
        WebPackage::read(dir.path()).map_err(|error| error.to_string())
    };
    let good = json!({"landscape": [{"type": "button", "key": "south", "label": "Go", "rect": [0, 0, 1, 1], "sneaky": "<script>"}]});
    let package = write(json!({"layout": "big-go", "layouts": {"big-go": good}})).unwrap();
    assert_eq!(package.phone_layout(), "big-go");
    let cleaned = &package.phone_layouts()["big-go"]["landscape"][0];
    assert!(
        cleaned.get("sneaky").is_none(),
        "unknown fields are dropped: {cleaned}"
    );

    for (layouts, expected) in [
        (json!({"stick-2": good}), "not a built-in layout"),
        (
            json!({"bad": {"landscape": [{"type": "button", "key": "select", "label": "?", "x": 0.5, "y": 0.5, "size": 0.3}]}}),
            "south, east, west, north, or start",
        ),
        (
            json!({"bad": {"landscape": [{"type": "iframe", "rect": [0, 0, 1, 1]}]}}),
            "each control's type",
        ),
        (
            json!({"bad": {"landscape": [{"type": "stick", "rect": [0, 0, 2, 1]}]}}),
            "from 0 to 1",
        ),
        (json!({"bad": {"landscape": []}}), "1–16 controls"),
    ] {
        let error = write(json!({"layouts": layouts})).unwrap_err();
        assert!(error.contains(expected), "{layouts}: {error}");
    }
    let error = write(json!({"layout": "missing"})).unwrap_err();
    assert!(error.contains("phone.layout must be one of"), "{error}");
}

#[test]
fn opening_the_lab_sends_its_own_layout_and_images_to_phones() {
    let dir = tempfile::tempdir().unwrap();
    let host = lab_home(dir.path());
    let mut socket = phone_socket(&host, "labphone001");
    next_with(&mut socket, "\"layout\":\"stick-2\"");
    let (status, _) = exchange(host.origin(), "GET", "/play/phone-lab/", None);
    assert_eq!(status, 200);
    let images: serde_json::Value =
        serde_json::from_str(&next_with(&mut socket, "\"images\":{\"comet\"")).unwrap();
    let sun = images["images"]["sun"].as_str().unwrap().to_string();
    let pushed: serde_json::Value =
        serde_json::from_str(&next_with(&mut socket, "\"layout\":\"lab-pad\"")).unwrap();
    assert_eq!(pushed["layout_spec"]["landscape"][0]["type"], "stick");

    let phone = format!("http://127.0.0.1:{}", host.phone_port().unwrap());
    let raw = raw_exchange(&phone, "GET", &sun, None).to_ascii_lowercase();
    assert!(raw.starts_with("http/1.1 200") && raw.contains("content-type: image/png"));
    let path = phones(&host)["join_url"]
        .as_str()
        .unwrap()
        .trim_start_matches(&phone)
        .to_string();
    let raw = raw_exchange(&phone, "GET", &format!("{path}/awake.mp4"), None).to_ascii_lowercase();
    assert!(raw.starts_with("http/1.1 200") && raw.contains("content-type: video/mp4"));

    // A game can switch to its own layout while running, and back.
    let (status, _) = post_json(
        &host,
        "/__gigacouch/v1/phone/layout",
        json!({"layout": "lab-rps"}),
    );
    assert_eq!(status, 200);
    let pushed: serde_json::Value =
        serde_json::from_str(&next_with(&mut socket, "\"layout\":\"lab-rps\"")).unwrap();
    assert_eq!(pushed["layout_spec"]["landscape"][0]["label"], "Rock");
}

#[test]
fn a_private_screen_reaches_one_player_and_its_choice_comes_back() {
    let dir = tempfile::tempdir().unwrap();
    let host = lab_home(dir.path());
    let mut ada = phone_socket(&host, "adaphone001");
    let mut bob = phone_socket(&host, "bobphone001");
    join_player(&host, &mut ada, "Ada");
    say(&mut bob, json!({"name": "Bob", "audience": true}));
    let (status, _) = exchange(host.origin(), "GET", "/play/phone-lab/", None);
    assert_eq!(status, 200);

    let screen = json!({"id": "hand", "title": "Your hand", "choices": [{"id": "sun-0", "label": "Sun", "image": "sun"}]});
    let (status, body) = post_json(
        &host,
        "/__gigacouch/v1/phone/screen",
        json!({"player": 1, "screen": screen}),
    );
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"phones\":1"));
    let shown: serde_json::Value =
        serde_json::from_str(&next_with(&mut ada, "\"screen\":{")).unwrap();
    assert_eq!(shown["screen"]["title"], "Your hand");
    assert!(
        shown["screen"]["choices"][0]["image"]
            .as_str()
            .unwrap()
            .contains("/image/")
    );

    say(
        &mut ada,
        json!({"kind": "event", "event": {"type": "choice", "choice": "sun-0", "screen": "hand"}}),
    );
    wait_for("the choice", || {
        let events = snapshot_json(&host)["phone_events"].clone();
        events.as_array().unwrap().iter().any(|event| {
            event["choice"] == "sun-0" && event["player"] == 1 && event["name"] == "Ada"
        })
    });

    // The audience phone takes no player spot but hears audience screens.
    let listed = phones(&host);
    let bob_row = listed["phones"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Bob")
        .unwrap()
        .clone();
    assert_eq!(bob_row["audience"], true);
    assert_eq!(bob_row["player"], serde_json::Value::Null);
    // A choice can start one of the phone's own actions.
    let photo = json!({"player": 1, "screen": {"choices": [{"id": "snap", "label": "Take a photo", "action": "photo"}]}});
    assert_eq!(
        post_json(&host, "/__gigacouch/v1/phone/screen", photo).0,
        200
    );
    next_with(&mut ada, "\"action\":\"photo\"");
    let (status, body) = post_json(
        &host,
        "/__gigacouch/v1/phone/screen",
        json!({"player": "audience", "screen": {"title": "Vote!"}}),
    );
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"phones\":1"));
    next_with(&mut bob, "\"Vote!\"");

    for bad in [
        json!({"player": 1, "screen": {"image": "not-declared"}}),
        json!({"player": 1, "screen": {}}),
        json!({"player": 1, "screen": {"choices": [{"id": "has space", "label": "x"}]}}),
        json!({"player": "everyone", "screen": {"title": "x"}}),
        json!({"player": 1, "screen": {"choices": [{"id": "go", "label": "Go", "action": "open-browser"}]}}),
    ] {
        assert_eq!(
            post_json(&host, "/__gigacouch/v1/phone/screen", bad.clone()).0,
            400,
            "{bad}"
        );
    }
}

#[test]
fn a_typed_answer_and_button_labels() {
    let dir = tempfile::tempdir().unwrap();
    let host = lab_home(dir.path());
    let mut ada = phone_socket(&host, "adaphone002");
    join_player(&host, &mut ada, "Ada");
    exchange(host.origin(), "GET", "/play/phone-lab/", None);

    // Text without a question is ignored.
    say(
        &mut ada,
        json!({"kind": "event", "event": {"type": "text", "text": "unasked"}}),
    );
    let (status, _) = post_json(
        &host,
        "/__gigacouch/v1/phone/ask",
        json!({"player": 1, "ask": {"id": "caption", "prompt": "Caption?", "max": 10}}),
    );
    assert_eq!(status, 200);
    next_with(&mut ada, "\"prompt\":\"Caption?\"");
    say(
        &mut ada,
        json!({"kind": "event", "event": {"type": "text", "text": "A very long caption indeed"}}),
    );
    let mut texts = Vec::new();
    wait_for("the typed answer", || {
        texts.extend(
            snapshot_json(&host)["phone_events"]
                .as_array()
                .unwrap()
                .clone(),
        );
        texts.iter().any(|event| event["type"] == "text")
    });
    let answers: Vec<&serde_json::Value> = texts
        .iter()
        .filter(|event| event["type"] == "text")
        .collect();
    assert_eq!(answers.len(), 1, "the unasked text was dropped: {texts:?}");
    assert_eq!(
        answers[0]["text"], "A very lon",
        "cut to the question's max"
    );
    assert_eq!(answers[0]["ask"], "caption");

    let (status, _) = post_json(
        &host,
        "/__gigacouch/v1/phone/labels",
        json!({"player": "all", "labels": {"south": "Paris", "east": "Rome"}}),
    );
    assert_eq!(status, 200);
    next_with(&mut ada, "\"Paris\"");
    assert_eq!(
        post_json(
            &host,
            "/__gigacouch/v1/phone/labels",
            json!({"player": 1, "labels": {"select": "x"}})
        )
        .0,
        400
    );
}

#[test]
fn drawing_strokes_are_cleaned_on_the_way_to_the_game() {
    let dir = tempfile::tempdir().unwrap();
    let host = lab_home(dir.path());
    let mut ada = phone_socket(&host, "adaphone003");
    join_player(&host, &mut ada, "Ada");
    say(
        &mut ada,
        json!({"kind": "event", "event": {"type": "stroke", "stroke": 3, "points": [[0.25, 0.5], [2.0, -1.0]], "color": "javascript:", "end": true}}),
    );
    say(
        &mut ada,
        json!({"kind": "event", "event": {"type": "stroke", "points": vec![[0.5, 0.5]; 200]}}),
    );
    let mut events = Vec::new();
    wait_for("the stroke", || {
        events.extend(
            snapshot_json(&host)["phone_events"]
                .as_array()
                .unwrap()
                .clone(),
        );
        !events.is_empty()
    });
    std::thread::sleep(Duration::from_millis(100));
    events.extend(
        snapshot_json(&host)["phone_events"]
            .as_array()
            .unwrap()
            .clone(),
    );
    assert_eq!(
        events.len(),
        1,
        "a stroke over 128 points is dropped: {events:?}"
    );
    assert_eq!(events[0]["points"], json!([[0.25, 0.5], [1.0, 0.0]]));
    assert_eq!(events[0]["color"], "mint");
    assert_eq!(events[0]["end"], true);
}

#[test]
fn a_dropped_phone_keeps_its_player_number_and_comes_back_into_it() {
    let mut session = Session::unlimited();
    let start = Instant::now();
    let ada = |south: bool| phone_device(json!({"south": south}));
    session.apply(ada(true), start);
    session.apply(ada(false), start);
    assert_eq!(session.player_of("phone:abc"), Some(1));
    // Someone else joins while Ada's phone is away.
    let bob = json!({"id": "phone:bob", "kind": "phone", "name": "Bob", "family": "phone", "analog": true, "south": true});
    session.apply(
        serde_json::from_value(json!({"devices": [bob.clone()]})).unwrap(),
        start,
    );
    let snapshot = serde_json::to_value(session.snapshot()).unwrap();
    let away: Vec<_> = snapshot["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].clone(), p["away"].clone()))
        .collect();
    assert_eq!(
        away,
        vec![(json!(1), json!(true)), (json!(2), json!(false))],
        "Bob takes 2; 1 waits for Ada"
    );
    // Ada comes back without pressing anything.
    let mut both: DevicePost = serde_json::from_value(json!({"devices": [bob.clone()]})).unwrap();
    both.devices.extend(ada(false).devices);
    session.apply(both, start + Duration::from_secs(30));
    assert_eq!(session.player_of("phone:abc"), Some(1));
    // After the rejoin window the spot is freed.
    session.apply(
        serde_json::from_value(json!({"devices": [bob.clone()]})).unwrap(),
        start + Duration::from_secs(40),
    );
    session.apply(
        serde_json::from_value(json!({"devices": [bob]})).unwrap(),
        start + Duration::from_secs(40) + couch_web_host::REJOIN_WINDOW,
    );
    let snapshot = serde_json::to_value(session.snapshot()).unwrap();
    assert_eq!(snapshot["players"].as_array().unwrap().len(), 1);
}

#[test]
fn a_phone_links_to_a_home_person_takes_a_photo_and_starts_a_game() {
    let dir = tempfile::tempdir().unwrap();
    let host = lab_home(dir.path());
    let (status, _) = post_json(&host, "/__gigacouch/v1/home/add", json!({"name": "Ada"}));
    assert_eq!(status, 200);
    let mut phone = phone_socket(&host, "adaphone004");
    join_player(&host, &mut phone, "Phone");

    say(&mut phone, json!({"kind": "request", "what": "profiles"}));
    let people = next_with(&mut phone, "\"profiles\"");
    assert!(people.contains("\"Ada\""), "{people}");
    say(&mut phone, json!({"kind": "profile", "id": "ada"}));
    next_with(&mut phone, "\"profile\":{\"id\":\"ada\"");
    say(&mut phone, json!({"name": "Phone"}));
    wait_for("the linked name", || {
        post_devices(&host);
        let players = snapshot_json(&host)["players"].clone();
        players[0]["name"] == "Ada" && players[0]["profile"] == "ada"
    });
    assert!(
        dir.path().join("phone-profiles.json").is_file(),
        "links are kept"
    );

    // A photo: a JPEG, saved and served on the host's own origin.
    let phone_origin = format!("http://127.0.0.1:{}", host.phone_port().unwrap());
    let path = phones(&host)["join_url"]
        .as_str()
        .unwrap()
        .trim_start_matches(&phone_origin)
        .to_string();
    let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0];
    jpeg.extend_from_slice(b"photo-bytes");
    let (status, _) = exchange(
        &phone_origin,
        "POST",
        &format!("{path}/avatar?id=adaphone004"),
        Some(&jpeg),
    );
    assert_eq!(status, 200);
    let (status, _) = exchange(
        &phone_origin,
        "POST",
        &format!("{path}/avatar?id=adaphone004"),
        Some(b"not a jpeg"),
    );
    assert_eq!(status, 400);
    let mut avatar = String::new();
    wait_for("the photo on the player", || {
        post_devices(&host);
        avatar = snapshot_json(&host)["players"][0]["avatar"]
            .as_str()
            .unwrap_or("")
            .to_string();
        !avatar.is_empty()
    });
    let raw = raw_exchange(host.origin(), "GET", &avatar, None).to_ascii_lowercase();
    assert!(
        raw.contains(" 200 ok") && raw.contains("content-type: image/jpeg"),
        "{avatar}: {}",
        &raw[..raw.len().min(300)]
    );
    assert_eq!(
        exchange(
            host.origin(),
            "GET",
            "/__gigacouch/v1/avatar/0123456789abcdef.jpg",
            None
        )
        .0,
        404
    );

    // A ping is answered at once.
    say(&mut phone, json!({"kind": "ping", "t": 1234}));
    next_with(&mut phone, "\"pong\":1234");

    // Starting a game from the phone: Home picks the request up once.
    say(&mut phone, json!({"kind": "request", "what": "shelf"}));
    next_with(&mut phone, "\"shelf\"");
    say(&mut phone, json!({"kind": "open", "id": "not-a-game"}));
    next_with(&mut phone, "\"notice\"");
    let (_, remote) = exchange(host.origin(), "GET", "/__gigacouch/v1/home/remote", None);
    assert!(remote.contains("\"open\":null"), "{remote}");
}
