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
];

/// The layout a phone shows on the shelf and in games that name none.
pub const DEFAULT_PHONE_LAYOUT: &str = "stick-2";
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
    layout: String,
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
        let package = Self { raw };
        package.validate(package_dir)?;
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
            .map(|phone| phone.layout.as_str())
            .unwrap_or(DEFAULT_PHONE_LAYOUT)
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
        if let Some(phone) = &raw.phone
            && !PHONE_LAYOUTS.contains(&phone.layout.as_str())
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
