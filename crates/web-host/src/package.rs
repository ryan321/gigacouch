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
    "draw",
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

/// One of a game's own phone files (a sound or an image), checked and
/// resolved to its file inside web/.
#[derive(Debug, Clone)]
pub struct GameFile {
    pub name: String,
    pub path: std::path::PathBuf,
    pub content_type: &'static str,
    pub extension: &'static str,
}
pub type GameSound = GameFile;
pub type GameImage = GameFile;

/// Limits for a game's own phone images, shown on private phone screens.
pub const MAX_IMAGE_BYTES: u64 = 512 * 1024;
const MAX_IMAGES_TOTAL_BYTES: u64 = 4 * 1024 * 1024;
const MAX_IMAGES: usize = 64;
/// Raster formats only: an SVG can carry script.
const IMAGE_TYPES: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("webp", "image/webp"),
];

/// Button keys a layout can use: the five every player has.
pub const LAYOUT_KEYS: &[&str] = &["south", "east", "west", "north", "start"];
/// Colors a layout or a phone screen can use, from the brand palette.
pub const PHONE_COLORS: &[&str] = &["mint", "blue", "amber", "coral", "panel"];
const MAX_CUSTOM_LAYOUTS: usize = 16;
const MAX_LAYOUT_CONTROLS: usize = 16;
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
    /// Image name to a file inside web/, for private phone screens.
    #[serde(default)]
    images: std::collections::BTreeMap<String, String>,
    /// The game's own controller layouts, by name. See `check_layout`.
    #[serde(default)]
    layouts: std::collections::BTreeMap<String, serde_json::Value>,
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
    images: Vec<GameImage>,
    layouts: serde_json::Map<String, serde_json::Value>,
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
            images: Vec::new(),
            layouts: serde_json::Map::new(),
        };
        package.layouts = check_layouts(&package.raw)?;
        package.validate(package_dir)?;
        package.sounds = check_sounds(&package.raw, package_dir)?;
        package.images = check_images(&package.raw, package_dir)?;
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

    /// The game's own phone images, already checked.
    pub fn phone_images(&self) -> &[GameImage] {
        &self.images
    }

    /// The game's own phone layouts, already checked, by name.
    pub fn phone_layouts(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.layouts
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
            && !self.layouts.contains_key(layout)
        {
            return Err(Error::Invalid(format!(
                "phone.layout must be one of {}, or one of the game's phone.layouts",
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
    check_files(
        &phone.sounds,
        package_dir,
        &FileRules {
            what: "sound",
            types: SOUND_TYPES,
            formats: ".mp3, .m4a, or .wav",
            max_each: MAX_SOUND_BYTES,
            max_total: MAX_SOUNDS_TOTAL_BYTES,
            max_count: MAX_SOUNDS,
            reserved: STOCK_SOUNDS,
        },
    )
}

/// Checks a game's own phone images the same way.
fn check_images(raw: &RawPackage, package_dir: &Path) -> Result<Vec<GameImage>, Error> {
    let Some(phone) = &raw.phone else {
        return Ok(Vec::new());
    };
    check_files(
        &phone.images,
        package_dir,
        &FileRules {
            what: "image",
            types: IMAGE_TYPES,
            formats: ".png, .jpg, or .webp",
            max_each: MAX_IMAGE_BYTES,
            max_total: MAX_IMAGES_TOTAL_BYTES,
            max_count: MAX_IMAGES,
            reserved: &[],
        },
    )
}

struct FileRules {
    what: &'static str,
    types: &'static [(&'static str, &'static str)],
    formats: &'static str,
    max_each: u64,
    max_total: u64,
    max_count: usize,
    reserved: &'static [&'static str],
}

fn check_files(
    files: &std::collections::BTreeMap<String, String>,
    package_dir: &Path,
    rules: &FileRules,
) -> Result<Vec<GameFile>, Error> {
    let what = rules.what;
    if files.len() > rules.max_count {
        return Err(Error::Invalid(format!(
            "phone.{what}s may name at most {} {what}s",
            rules.max_count
        )));
    }
    let web = package_dir.join("web");
    let mut total = 0_u64;
    let mut checked = Vec::new();
    for (name, relative) in files {
        if !good_phone_name(name) {
            return Err(Error::Invalid(format!(
                "phone {what} name \"{name}\" must be 1–32 lowercase letters, digits, or dashes"
            )));
        }
        if rules.reserved.contains(&name.as_str()) {
            return Err(Error::Invalid(format!(
                "phone {what} \"{name}\" is a stock {what}; give the game's own {what} another name"
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
                "phone {what} \"{name}\" must be a file path inside web/"
            )));
        }
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let Some((extension, content_type)) = rules
            .types
            .iter()
            .find(|(known, _)| *known == extension)
            .copied()
        else {
            return Err(Error::Invalid(format!(
                "phone {what} \"{name}\" must be {}",
                rules.formats
            )));
        };
        let file = web.join(path);
        let size = std::fs::metadata(&file)
            .ok()
            .filter(|meta| meta.is_file())
            .map(|meta| meta.len())
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "phone {what} \"{name}\" is missing: web/{relative}"
                ))
            })?;
        if size > rules.max_each {
            return Err(Error::Invalid(format!(
                "phone {what} \"{name}\" is over {} KiB",
                rules.max_each / 1024
            )));
        }
        total += size;
        checked.push(GameFile {
            name: name.clone(),
            path: file,
            content_type,
            extension,
        });
    }
    if total > rules.max_total {
        return Err(Error::Invalid(format!(
            "phone {what}s add up to more than {} MiB",
            rules.max_total / (1024 * 1024)
        )));
    }
    Ok(checked)
}

fn good_phone_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && !name.starts_with('-')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Checks a game's own controller layouts and returns them cleaned: only the
/// fields the pad draws, every number inside the screen, every button one of
/// the five keys. The pad draws layouts from this data alone.
fn check_layouts(raw: &RawPackage) -> Result<serde_json::Map<String, serde_json::Value>, Error> {
    let Some(phone) = &raw.phone else {
        return Ok(serde_json::Map::new());
    };
    if phone.layouts.len() > MAX_CUSTOM_LAYOUTS {
        return Err(Error::Invalid(format!(
            "phone.layouts may name at most {MAX_CUSTOM_LAYOUTS} layouts"
        )));
    }
    let mut clean = serde_json::Map::new();
    for (name, spec) in &phone.layouts {
        if !good_phone_name(name) || PHONE_LAYOUTS.contains(&name.as_str()) {
            return Err(Error::Invalid(format!(
                "phone layout \"{name}\" needs a lowercase name that is not a built-in layout"
            )));
        }
        clean.insert(name.clone(), check_layout(name, spec)?);
    }
    Ok(clean)
}

fn check_layout(name: &str, spec: &serde_json::Value) -> Result<serde_json::Value, Error> {
    use serde_json::{Value, json};
    let bad = |why: &str| Error::Invalid(format!("phone layout \"{name}\": {why}"));
    let fraction = |value: &Value, what: &str| -> Result<f64, Error> {
        value
            .as_f64()
            .filter(|number| (0.0..=1.0).contains(number))
            .ok_or_else(|| bad(&format!("{what} must be a number from 0 to 1")))
    };
    let text = |value: &Value, most: usize, what: &str| -> Result<Option<String>, Error> {
        match value {
            Value::Null => Ok(None),
            Value::String(words)
                if words.chars().count() <= most && !words.chars().any(char::is_control) =>
            {
                Ok(Some(words.clone()))
            }
            _ => Err(bad(&format!(
                "{what} must be text of at most {most} characters"
            ))),
        }
    };
    let rect = |value: &Value| -> Result<Value, Error> {
        let parts = value
            .as_array()
            .filter(|parts| parts.len() == 4)
            .ok_or_else(|| bad("rect must be [x, y, width, height]"))?;
        let numbers: Vec<f64> = parts
            .iter()
            .map(|part| fraction(part, "rect"))
            .collect::<Result<_, _>>()?;
        Ok(json!(numbers))
    };
    let controls = |list: &Value| -> Result<Value, Error> {
        let list = list
            .as_array()
            .ok_or_else(|| bad("landscape and portrait are lists of controls"))?;
        if list.is_empty() || list.len() > MAX_LAYOUT_CONTROLS {
            return Err(bad(&format!(
                "a layout has 1–{MAX_LAYOUT_CONTROLS} controls"
            )));
        }
        let mut out = Vec::new();
        for control in list {
            let kind = control["type"].as_str().unwrap_or("");
            let mut item = serde_json::Map::new();
            item.insert("type".into(), json!(kind));
            match kind {
                "stick" => {
                    let axis = control["axis"].as_str().unwrap_or("move");
                    if axis != "move" && axis != "look" {
                        return Err(bad("a stick's axis is move or look"));
                    }
                    item.insert("axis".into(), json!(axis));
                    item.insert("rect".into(), rect(&control["rect"])?);
                }
                "slider" | "touchpad" | "canvas" | "palette" => {
                    item.insert("rect".into(), rect(&control["rect"])?);
                }
                "dpad" | "button" | "arrow" => {
                    if kind == "button" {
                        let key = control["key"].as_str().unwrap_or("");
                        if !LAYOUT_KEYS.contains(&key) {
                            return Err(bad(
                                "a button's key is south, east, west, north, or start",
                            ));
                        }
                        item.insert("key".into(), json!(key));
                        if let Some(color) = control["color"].as_str() {
                            if !PHONE_COLORS.contains(&color) {
                                return Err(bad("color is mint, blue, amber, coral, or panel"));
                            }
                            item.insert("color".into(), json!(color));
                        }
                        if control["small"].as_bool() == Some(true) {
                            item.insert("small".into(), json!(true));
                        }
                    }
                    if kind == "arrow" {
                        let dir = control["dir"]
                            .as_i64()
                            .filter(|dir| *dir == -1 || *dir == 1)
                            .ok_or_else(|| bad("an arrow's dir is -1 or 1"))?;
                        item.insert("dir".into(), json!(dir));
                    }
                    if kind != "dpad" {
                        let label = text(&control["label"], 16, "label")?
                            .ok_or_else(|| bad("buttons and arrows need a label"))?;
                        item.insert("label".into(), json!(label));
                    }
                    if kind == "button" && !control["rect"].is_null() {
                        item.insert("rect".into(), rect(&control["rect"])?);
                    } else {
                        item.insert("x".into(), json!(fraction(&control["x"], "x")?));
                        item.insert("y".into(), json!(fraction(&control["y"], "y")?));
                        item.insert("size".into(), json!(fraction(&control["size"], "size")?));
                    }
                }
                _ => {
                    return Err(bad(
                        "each control's type is stick, dpad, button, arrow, slider, touchpad, canvas, or palette",
                    ));
                }
            }
            if let Some(hint) = text(&control["hint"], 60, "hint")? {
                item.insert("hint".into(), json!(hint));
            }
            out.push(Value::Object(item));
        }
        Ok(Value::Array(out))
    };
    let mut clean = serde_json::Map::new();
    clean.insert("landscape".into(), controls(&spec["landscape"])?);
    if !spec["portrait"].is_null() {
        clean.insert("portrait".into(), controls(&spec["portrait"])?);
    }
    Ok(Value::Object(clean))
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
