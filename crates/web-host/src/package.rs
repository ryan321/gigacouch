use crate::{Error, io_error};
use semver::Version;
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Component, Path},
};

/// Built-in phone controller layouts. The phone page draws each one; see
/// `assets/pad.js`. Every layout maps onto the same five buttons and two
/// sticks the host gives every player.
pub const PHONE_LAYOUTS: &[&str] = &[
    "stick-2",
    "dpad-2",
    "stick-4",
    "twin-stick",
    "one-button",
    "quiz-4",
    "racing",
    "paddle",
    "touchpad",
    "lanes-4",
    "two-choice",
];

/// The layout a phone shows on the shelf and in games that name none.
pub const DEFAULT_PHONE_LAYOUT: &str = "stick-2";

/// Sounds every phone can play without a file: the pad generates them. See
/// `STOCK_SOUNDS` in `assets/pad.js`. A game's own sounds cannot reuse these
/// names.
pub const STOCK_SOUNDS: &[&str] = &[
    "click", "tick", "ding", "success", "fail", "buzzer", "coin", "whoosh",
];

/// Limits for a game's own phone sounds. Phones download them all when the
/// game opens, so they stay small.
pub const MAX_SOUND_BYTES: u64 = 256 * 1024;
pub const MAX_SOUNDS_TOTAL_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_SOUNDS: usize = 32;
/// Formats every phone browser plays, iPhone included.
const SOUND_TYPES: &[(&str, &str)] = &[
    ("mp3", "audio/mpeg"),
    ("m4a", "audio/mp4"),
    ("wav", "audio/wav"),
];

/// One of a game's own phone sounds, checked and resolved to its file.
#[derive(Debug, Clone)]
pub struct GameSound {
    pub name: String,
    pub path: std::path::PathBuf,
    pub content_type: &'static str,
    pub extension: &'static str,
}
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Players {
    min: u16,
    /// Optional. Without it, everyone who joins gets to play.
    #[serde(default)]
    max: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Phone {
    #[serde(default)]
    layout: Option<String>,
    /// Sound name to a file inside web/, for example "sounds/ding.mp3".
    #[serde(default)]
    sounds: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackage {
    manifest_version: u32,
    game_id: String,
    title: String,
    version: String,
    runtime: String,
    entrypoint: String,
    gigacouch_api: String,
    players: Players,
    #[serde(default)]
    phone: Option<Phone>,
}

#[derive(Debug, Clone)]
pub struct WebPackage {
    raw: RawPackage,
    sounds: Vec<GameSound>,
}

impl WebPackage {
    pub fn read(package_dir: &Path) -> Result<Self, Error> {
        let path = package_dir.join("gigacouch.json");
        let file = File::open(&path).map_err(|source| io_error(&path, source))?;
        if !file
            .metadata()
            .map_err(|source| io_error(&path, source))?
            .is_file()
        {
            return Err(Error::Invalid(
                "gigacouch.json must be a regular file".into(),
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_MANIFEST_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|source| io_error(&path, source))?;
        if bytes.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(Error::Invalid("gigacouch.json exceeds 64 KiB".into()));
        }
        let raw: RawPackage = serde_json::from_slice(&bytes).map_err(|err| {
            Error::Invalid(format!("gigacouch.json is not a web-1 package: {err}"))
        })?;
        let mut package = Self {
            raw,
            sounds: Vec::new(),
        };
        package.validate(package_dir)?;
        package.sounds = check_sounds(&package.raw, package_dir)?;
        Ok(package)
    }

    pub fn game_id(&self) -> &str {
        &self.raw.game_id
    }

    pub fn title(&self) -> &str {
        &self.raw.title
    }

    /// The most players this game allows, or `None` for no limit.
    pub fn players_max(&self) -> Option<u16> {
        self.raw.players.max
    }

    /// The phone layout this game asks for, or the default.
    pub fn phone_layout(&self) -> &str {
        self.raw
            .phone
            .as_ref()
            .and_then(|phone| phone.layout.as_deref())
            .unwrap_or(DEFAULT_PHONE_LAYOUT)
    }

    /// The game's own phone sounds, already checked.
    pub fn phone_sounds(&self) -> &[GameSound] {
        &self.sounds
    }

    pub fn web_relative_entry(&self) -> &str {
        self.raw
            .entrypoint
            .strip_prefix("web/")
            .unwrap_or(self.raw.entrypoint.as_str())
    }

    fn validate(&self, package_dir: &Path) -> Result<(), Error> {
        let raw = &self.raw;
        if raw.manifest_version != 1 {
            return Err(Error::Invalid(
                "unsupported manifest_version; expected 1".into(),
            ));
        }
        if !safe_id(&raw.game_id) {
            return Err(Error::Invalid(
                "game_id must be a portable 1–64 character identifier".into(),
            ));
        }
        if raw.title.trim().is_empty()
            || raw.title.chars().count() > 80
            || raw.title.chars().any(char::is_control)
        {
            return Err(Error::Invalid(
                "title must contain 1–80 printable characters".into(),
            ));
        }
        if raw.version.len() > 64 || Version::parse(&raw.version).is_err() {
            return Err(Error::Invalid("version must be a semantic version".into()));
        }
        if raw.runtime != "web-1" {
            return Err(Error::Invalid("runtime must be web-1 for this host".into()));
        }
        if raw.gigacouch_api != "1" {
            return Err(Error::Invalid("gigacouch_api must be \"1\"".into()));
        }
        if raw.players.min == 0 || raw.players.max.is_some_and(|max| raw.players.min > max) {
            return Err(Error::Invalid(
                "players must satisfy 1 <= min, and min <= max when max is set".into(),
            ));
        }
        if let Some(layout) = raw.phone.as_ref().and_then(|phone| phone.layout.as_deref())
            && !PHONE_LAYOUTS.contains(&layout)
        {
            return Err(Error::Invalid(format!(
                "phone.layout must be one of {}",
                PHONE_LAYOUTS.join(", ")
            )));
        }
        validate_entrypoint(&raw.entrypoint)?;
        let entry = package_dir.join(&raw.entrypoint);
        if !entry.is_file() {
            return Err(Error::Invalid(format!(
                "entrypoint does not exist: {}",
                raw.entrypoint
            )));
        }
        Ok(())
    }
}

/// Checks a game's own phone sounds: names, formats, sizes, and that each
/// file sits inside web/.
fn check_sounds(raw: &RawPackage, package_dir: &Path) -> Result<Vec<GameSound>, Error> {
    let Some(phone) = &raw.phone else {
        return Ok(Vec::new());
    };
    if phone.sounds.len() > MAX_SOUNDS {
        return Err(Error::Invalid(format!(
            "phone.sounds may name at most {MAX_SOUNDS} sounds"
        )));
    }
    let web = package_dir.join("web");
    let mut total = 0_u64;
    let mut sounds = Vec::new();
    for (name, relative) in &phone.sounds {
        let good_name = !name.is_empty()
            && name.len() <= 32
            && !name.starts_with('-')
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if !good_name {
            return Err(Error::Invalid(format!(
                "phone sound name \"{name}\" must be 1–32 lowercase letters, digits, or dashes"
            )));
        }
        if STOCK_SOUNDS.contains(&name.as_str()) {
            return Err(Error::Invalid(format!(
                "phone sound \"{name}\" is a stock sound; give the game's own sound another name"
            )));
        }
        let path = Path::new(relative);
        let inside = !relative.is_empty()
            && !relative.contains('\\')
            && !relative.contains('\0')
            && path.components().all(|part| {
                matches!(part, Component::Normal(segment) if !segment.to_string_lossy().starts_with('.'))
            });
        if !inside {
            return Err(Error::Invalid(format!(
                "phone sound \"{name}\" must be a file path inside web/"
            )));
        }
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let Some((extension, content_type)) = SOUND_TYPES
            .iter()
            .find(|(known, _)| *known == extension)
            .copied()
        else {
            return Err(Error::Invalid(format!(
                "phone sound \"{name}\" must be .mp3, .m4a, or .wav"
            )));
        };
        let file = web.join(path);
        let size = std::fs::metadata(&file)
            .ok()
            .filter(|meta| meta.is_file())
            .map(|meta| meta.len())
            .ok_or_else(|| {
                Error::Invalid(format!("phone sound \"{name}\" is missing: web/{relative}"))
            })?;
        if size > MAX_SOUND_BYTES {
            return Err(Error::Invalid(format!(
                "phone sound \"{name}\" is over {} KiB",
                MAX_SOUND_BYTES / 1024
            )));
        }
        total += size;
        sounds.push(GameSound {
            name: name.clone(),
            path: file,
            content_type,
            extension,
        });
    }
    if total > MAX_SOUNDS_TOTAL_BYTES {
        return Err(Error::Invalid(format!(
            "phone sounds add up to more than {} MiB",
            MAX_SOUNDS_TOTAL_BYTES / (1024 * 1024)
        )));
    }
    Ok(sounds)
}

fn validate_entrypoint(entrypoint: &str) -> Result<(), Error> {
    if !entrypoint.starts_with("web/") || entrypoint.contains('\\') || entrypoint.contains('\0') {
        return Err(Error::Invalid(
            "entrypoint must be a path inside web/ ending in .html".into(),
        ));
    }
    let relative = Path::new(entrypoint);
    if relative.is_absolute() {
        return Err(Error::Invalid(
            "entrypoint must stay inside the package".into(),
        ));
    }
    let mut depth = 0_i32;
    for component in relative.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_string_lossy();
                if part.starts_with('.') {
                    return Err(Error::Invalid(
                        "entrypoint must not use dot path segments".into(),
                    ));
                }
                depth += 1;
            }
            _ => {
                return Err(Error::Invalid("entrypoint must stay inside web/".into()));
            }
        }
    }
    if depth < 2 || !entrypoint.ends_with(".html") {
        return Err(Error::Invalid(
            "entrypoint must be a path inside web/ ending in .html".into(),
        ));
    }
    Ok(())
}

fn safe_id(value: &str) -> bool {
    let alnum = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    !value.is_empty()
        && value.len() <= 64
        && alnum(value.as_bytes()[0])
        && alnum(value.as_bytes()[value.len() - 1])
        && value
            .bytes()
            .all(|byte| alnum(byte) || b"._-".contains(&byte))
}
