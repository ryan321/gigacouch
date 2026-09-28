use crate::{Error, io_error};
use semver::Version;
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Component, Path},
};

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

const MAX_CUSTOM_LAYOUTS: usize = 16;
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
    /// The game's own panel views, by name. See `phones/panels.rs`.
    #[serde(default)]
    views: std::collections::BTreeMap<String, serde_json::Value>,
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
    /// Named actions: "jump": "button", or { "type": "axis", "pad": "move" }.
    #[serde(default)]
    actions: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct WebPackage {
    raw: RawPackage,
    sounds: Vec<GameSound>,
    images: Vec<GameImage>,
    layouts: serde_json::Map<String, serde_json::Value>,
    views: serde_json::Map<String, serde_json::Value>,
    actions: Vec<crate::session::ActionDef>,
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
            views: serde_json::Map::new(),
            actions: Vec::new(),
        };
        package.actions = check_actions(&package.raw)?;
        package.layouts = check_layouts(&package.raw, &package.actions)?;
        if let Some(phone) = &package.raw.phone {
            package.views =
                crate::phones::check_game_views(&phone.views).map_err(Error::Invalid)?;
        }
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

    /// The game's named actions, already checked.
    pub fn actions(&self) -> &[crate::session::ActionDef] {
        &self.actions
    }

    /// The game's own phone views, already checked, by name.
    pub fn phone_views(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.views
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
            && !crate::phones::views::phone_layouts().contains(&layout)
            && !self.layouts.contains_key(layout)
        {
            return Err(Error::Invalid(format!(
                "phone.layout must be one of {}, or one of the game's phone.layouts",
                crate::phones::views::phone_layouts().join(", ")
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
/// Checks a game's named actions: short names that are not standard keys or
/// sticks, a button or an axis each, and a pad fallback of the same kind.
fn check_actions(raw: &RawPackage) -> Result<Vec<crate::session::ActionDef>, Error> {
    const STANDARD: &[&str] = &[
        "south",
        "east",
        "west",
        "north",
        "start",
        "move",
        "look",
        "jump",
        "leave",
        "primary_action",
        "secondary_action",
    ];
    if raw.actions.len() > 16 {
        return Err(Error::Invalid("actions may name at most 16 actions".into()));
    }
    let mut out = Vec::new();
    for (name, spec) in &raw.actions {
        if !good_phone_name(name) || STANDARD.contains(&name.as_str()) {
            return Err(Error::Invalid(format!(
                "action \"{name}\" needs a lowercase name that is not a standard key or stick"
            )));
        }
        let (kind, pad) = match spec {
            serde_json::Value::String(kind) => (kind.as_str(), None),
            serde_json::Value::Object(map) => (
                map.get("type").and_then(|kind| kind.as_str()).unwrap_or(""),
                map.get("pad").and_then(|pad| pad.as_str()),
            ),
            _ => ("", None),
        };
        let axis = match kind {
            "button" => false,
            "axis" => true,
            _ => {
                return Err(Error::Invalid(format!(
                    "action \"{name}\" is a \"button\" or an \"axis\""
                )));
            }
        };
        if let Some(pad) = pad {
            let fits = if axis {
                ["move", "look"].contains(&pad)
            } else {
                crate::phones::views::layout_keys().contains(&pad)
            };
            if !fits {
                return Err(Error::Invalid(format!(
                    "action \"{name}\": pad is {} for a {kind}",
                    if axis {
                        "move or look"
                    } else {
                        "south, east, west, north, or start"
                    }
                )));
            }
        }
        out.push(crate::session::ActionDef {
            name: name.clone(),
            axis,
            pad: pad.map(str::to_string),
        });
    }
    Ok(out)
}

fn check_layouts(
    raw: &RawPackage,
    actions: &[crate::session::ActionDef],
) -> Result<serde_json::Map<String, serde_json::Value>, Error> {
    let Some(phone) = &raw.phone else {
        return Ok(serde_json::Map::new());
    };
    let context = crate::phones::views::LayoutContext {
        buttons: actions
            .iter()
            .filter(|def| !def.axis)
            .map(|def| def.name.clone())
            .collect(),
        axes: actions
            .iter()
            .filter(|def| def.axis)
            .map(|def| def.name.clone())
            .collect(),
        sounds: phone.sounds.keys().cloned().collect(),
    };
    if phone.layouts.len() > MAX_CUSTOM_LAYOUTS {
        return Err(Error::Invalid(format!(
            "phone.layouts may name at most {MAX_CUSTOM_LAYOUTS} layouts"
        )));
    }
    let mut clean = serde_json::Map::new();
    for (name, spec) in &phone.layouts {
        if !good_phone_name(name) || crate::phones::views::phone_layouts().contains(&name.as_str())
        {
            return Err(Error::Invalid(format!(
                "phone layout \"{name}\" needs a lowercase name that is not a built-in layout"
            )));
        }
        clean.insert(
            name.clone(),
            crate::phones::views::check_layout(name, spec, &context).map_err(Error::Invalid)?,
        );
    }
    Ok(clean)
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
