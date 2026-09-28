use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};

pub const MAX_REQUEST_BYTES: u64 = 300 * 1024;
pub const MAX_SAVE_BYTES: usize = 256 * 1024;
const EAST_HOLD: Duration = Duration::from_millis(1250);
const DEAD_ZONE: f32 = 0.2;
/// How long a phone that drops keeps its player slot. Coming back within it
/// returns the same player number, without pressing A again.
pub const REJOIN_WINDOW: Duration = Duration::from_secs(120);
// A sanity bound on one page's device post, not a player limit: phones are
// added by the host, and players are limited only by the game.
const MAX_DEVICES: usize = 4096;

#[derive(Debug, Deserialize)]
pub struct DevicePost {
    #[serde(default)]
    pub devices: Vec<RawDevice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawDevice {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub south: bool,
    #[serde(default)]
    pub east: bool,
    #[serde(default)]
    pub jump: bool,
    #[serde(default)]
    pub leave: bool,
    #[serde(default)]
    pub west: bool,
    #[serde(default)]
    pub north: bool,
    #[serde(default)]
    pub start: bool,
    #[serde(default)]
    pub analog: bool,
    #[serde(default, rename = "move")]
    pub movement: Axis,
    /// A second stick: the right stick on a pad, or a phone's look stick.
    #[serde(default)]
    pub look: Axis,
    /// Positions, not stick tilt: a phone's slider or touchpad. Passed
    /// through as-is, with no dead zone and no rounding to a circle.
    #[serde(default)]
    pub absolute: bool,
    /// Named actions a phone's layout drives, by name: held or not. Only
    /// the actions the open game declares count.
    #[serde(default)]
    pub actions: std::collections::BTreeMap<String, bool>,
    /// Named axes a phone's layout drives, by name.
    #[serde(default)]
    pub axes: std::collections::BTreeMap<String, NamedAxis>,
    /// Set by the host for phones: the player's photo and Home profile.
    #[serde(default, skip)]
    pub avatar: Option<String>,
    #[serde(default, skip)]
    pub profile: Option<String>,
}

/// One named axis from a phone: x and y from -1 to 1, and whether they are
/// positions (a slider or touchpad) rather than stick tilt.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct NamedAxis {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub absolute: bool,
}

/// An action a game declares in gigacouch.json: a button or an axis, and the
/// standard key or stick a pad uses for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionDef {
    pub name: String,
    pub axis: bool,
    /// "south" … "start" for a button, "move" or "look" for an axis.
    pub pad: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Axis {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub players: Vec<SnapshotPlayer>,
    pub menu: MenuSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct MenuSnapshot {
    #[serde(rename = "move")]
    pub movement: Move,
    pub confirm: bool,
    pub back: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotPlayer {
    pub id: u16,
    pub name: String,
    #[serde(rename = "move")]
    pub movement: Move,
    pub look: Move,
    pub edges: Edges,
    /// Buttons held right now.
    pub buttons: Buttons,
    /// Buttons pressed since the last snapshot, each reported once.
    pub pressed: Buttons,
    pub glyphs: Glyphs,
    /// "pad", "phone", or "keyboard".
    pub kind: String,
    /// The pad's own id, so the page can rumble it. Pads only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pad: Option<String>,
    /// The player's photo, an address on this host. Phones that took one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// The Home profile a phone linked to, such as "ada".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// A phone that dropped and still holds its slot.
    pub away: bool,
    /// The open game's named actions: a button's { held, pressed } or an
    /// axis's { x, y }.
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub named: serde_json::Map<String, serde_json::Value>,
}

/// The five buttons every layout and pad maps onto: the four face buttons by
/// position, and Start.
#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
pub struct Buttons {
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north: bool,
    pub start: bool,
}

impl Buttons {
    fn merge(&mut self, other: Buttons) {
        self.south |= other.south;
        self.east |= other.east;
        self.west |= other.west;
        self.north |= other.north;
        self.start |= other.start;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Move {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Edges {
    pub jump: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Glyphs {
    pub jump: String,
    pub leave: String,
    pub r#move: String,
}

struct Tracked {
    id: String,
    south: bool,
    east: bool,
    west: bool,
    north: bool,
    start: bool,
    jump: bool,
    leave: bool,
    east_since: Option<Instant>,
    slot: Option<usize>,
    /// Named buttons held at the last update, for presses.
    named: std::collections::BTreeMap<String, bool>,
}

/// A named action's state in a player's slot.
#[derive(Clone, Copy, Default)]
struct NamedState {
    held: bool,
    pressed: bool,
    x: f32,
    y: f32,
}

struct Slot {
    device_id: String,
    name: String,
    family: String,
    move_x: f32,
    move_y: f32,
    look_x: f32,
    look_y: f32,
    jump: bool,
    held: Buttons,
    pressed: Buttons,
    kind: String,
    avatar: Option<String>,
    profile: Option<String>,
    /// When a phone holding this slot dropped. The slot waits for it.
    away_since: Option<Instant>,
    named: std::collections::BTreeMap<String, NamedState>,
}

pub struct Session {
    tracked: Vec<Tracked>,
    /// Player slots. They grow as people join; `limit` caps them when the
    /// game asks for a maximum.
    slots: Vec<Option<Slot>>,
    limit: Option<usize>,
    /// The open game's named actions.
    actions: Vec<ActionDef>,
    menu_x: f32,
    menu_y: f32,
    menu_confirm: bool,
    menu_back: bool,
}

impl Session {
    /// A session for at most `max_players` players.
    pub fn new(max_players: u16) -> Self {
        Self::with_limit(Some(max_players as usize))
    }

    /// A session with no player limit: everyone who joins gets a slot.
    pub fn unlimited() -> Self {
        Self::with_limit(None)
    }

    fn with_limit(limit: Option<usize>) -> Self {
        Self {
            tracked: Vec::new(),
            slots: Vec::new(),
            limit,
            actions: Vec::new(),
            menu_x: 0.0,
            menu_y: 0.0,
            menu_confirm: false,
            menu_back: false,
        }
    }

    /// The open game's named actions; empty on the shelf.
    pub fn set_actions(&mut self, actions: Vec<ActionDef>) {
        self.actions = actions;
        for slot in self.slots.iter_mut().flatten() {
            slot.named.clear();
        }
        for tracked in &mut self.tracked {
            tracked.named.clear();
        }
    }

    /// Change the player limit, for example when a game with a maximum opens
    /// from the shelf. Players past a new limit lose their slot and can join
    /// again when there is room.
    pub fn set_limit(&mut self, limit: Option<usize>) {
        self.limit = limit;
        if let Some(limit) = limit {
            let dropped: Vec<String> = self
                .slots
                .iter()
                .skip(limit)
                .flatten()
                .map(|slot| slot.device_id.clone())
                .collect();
            for id in dropped {
                self.release_device(&id);
            }
            self.slots.truncate(limit);
        }
    }

    pub fn apply(&mut self, post: DevicePost, now: Instant) {
        let mut devices = Vec::new();
        for device in post.devices {
            if devices.len() == MAX_DEVICES {
                break;
            }
            if !valid_device_id(&device.id)
                || devices.iter().any(|seen: &RawDevice| seen.id == device.id)
            {
                continue;
            }
            devices.push(device);
        }
        // A phone that dropped too long ago gives its slot up.
        for slot in &mut self.slots {
            if slot
                .as_ref()
                .and_then(|state| state.away_since)
                .is_some_and(|since| now.saturating_duration_since(since) >= REJOIN_WINDOW)
            {
                *slot = None;
            }
        }
        let live: Vec<String> = devices.iter().map(|device| device.id.clone()).collect();
        let gone: Vec<(String, Option<usize>)> = self
            .tracked
            .iter()
            .filter(|tracked| !live.iter().any(|id| id == &tracked.id))
            .map(|tracked| (tracked.id.clone(), tracked.slot))
            .collect();
        for (id, slot) in gone {
            match slot {
                // A phone keeps its slot for a while, marked away.
                Some(index) if id.starts_with("phone:") => {
                    if let Some(state) = self.slots.get_mut(index).and_then(Option::as_mut) {
                        state.away_since = Some(now);
                        state.held = Buttons::default();
                        state.move_x = 0.0;
                        state.move_y = 0.0;
                        state.look_x = 0.0;
                        state.look_y = 0.0;
                    }
                }
                _ => self.release_device(&id),
            }
            self.tracked.retain(|tracked| tracked.id != id);
        }
        self.menu_x = 0.0;
        self.menu_y = 0.0;
        for device in devices {
            self.apply_one(device, now);
        }
    }

    pub fn snapshot(&mut self) -> Snapshot {
        let axis_names: Vec<String> = self
            .actions
            .iter()
            .filter(|def| def.axis)
            .map(|def| def.name.clone())
            .collect();
        let mut players = Vec::new();
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let Some(slot) = slot else {
                continue;
            };
            let jump = slot.jump;
            slot.jump = false;
            let pressed = std::mem::take(&mut slot.pressed);
            players.push(SnapshotPlayer {
                id: u16::try_from(index + 1).unwrap_or(u16::MAX),
                name: slot.name.clone(),
                kind: slot.kind.clone(),
                pad: (slot.kind == "pad").then(|| {
                    slot.device_id
                        .strip_prefix("pad:")
                        .unwrap_or(&slot.device_id)
                        .to_string()
                }),
                avatar: slot.avatar.clone(),
                profile: slot.profile.clone(),
                away: slot.away_since.is_some(),
                // Held and axes stay; a press is reported once.
                named: slot
                    .named
                    .iter_mut()
                    .map(|(name, state)| {
                        let pressed = std::mem::take(&mut state.pressed);
                        let state = NamedState { pressed, ..*state };
                        let name = name.clone();
                        let value = if axis_names.contains(&name) {
                            json!({ "x": state.x, "y": state.y })
                        } else {
                            json!({ "held": state.held, "pressed": state.pressed })
                        };
                        (name, value)
                    })
                    .collect(),
                movement: Move {
                    x: slot.move_x,
                    y: slot.move_y,
                },
                look: Move {
                    x: slot.look_x,
                    y: slot.look_y,
                },
                edges: Edges { jump },
                buttons: slot.held,
                pressed,
                glyphs: glyphs(&slot.family),
            });
        }
        let menu = MenuSnapshot {
            movement: Move {
                x: self.menu_x,
                y: self.menu_y,
            },
            confirm: self.menu_confirm,
            back: self.menu_back,
        };
        self.menu_confirm = false;
        self.menu_back = false;
        Snapshot { players, menu }
    }

    fn apply_one(&mut self, device: RawDevice, now: Instant) {
        let keyboard = device.kind == "keyboard";
        // A phone leaves with its platform Leave control, never by holding
        // east: on some layouts east is an ordinary game button.
        let phone = device.kind == "phone";
        let family = if keyboard {
            "keyboard".to_string()
        } else if matches!(
            device.family.as_str(),
            "xbox" | "playstation" | "phone" | "generic"
        ) {
            device.family.clone()
        } else {
            "generic".to_string()
        };
        let name = clean_name(if device.name.trim().is_empty() {
            if keyboard { "Keyboard" } else { "Controller" }
        } else {
            device.name.trim()
        });
        let (move_x, move_y) = if device.absolute {
            absolute_move(device.movement.x, device.movement.y)
        } else if device.analog {
            analog_move(device.movement.x, device.movement.y)
        } else {
            digital_move(device.movement.x, device.movement.y)
        };
        let (look_x, look_y) = if device.absolute {
            absolute_move(device.look.x, device.look.y)
        } else {
            analog_move(device.look.x, device.look.y)
        };
        let position = self
            .tracked
            .iter()
            .position(|tracked| tracked.id == device.id);
        if position.is_none() {
            // A phone coming back within the rejoin window takes its old
            // slot again, without pressing A.
            let returning = self.slots.iter().position(|slot| {
                slot.as_ref()
                    .is_some_and(|state| state.device_id == device.id && state.away_since.is_some())
            });
            if let Some(index) = returning
                && let Some(state) = self.slots[index].as_mut()
            {
                state.away_since = None;
            }
            // A returning phone starts from what it holds now, so coming back
            // never fires a press. A new device starts released: Chrome shows
            // a pad only once a button is down, and that press must join.
            let back = returning.is_some();
            self.tracked.push(Tracked {
                id: device.id.clone(),
                south: back && device.south,
                east: back && device.east,
                west: back && device.west,
                north: back && device.north,
                start: back && device.start,
                jump: back && device.jump,
                leave: back && device.leave,
                east_since: None,
                slot: returning,
                named: std::collections::BTreeMap::new(),
            });
        }
        let position = self
            .tracked
            .iter()
            .position(|tracked| tracked.id == device.id)
            .expect("tracked");
        let south_rise = device.south && !self.tracked[position].south;
        let east_rise = device.east && !self.tracked[position].east;
        let jump_rise = device.jump && !self.tracked[position].jump;
        let leave_rise = device.leave && !self.tracked[position].leave;
        let held = Buttons {
            south: device.south || (keyboard && device.jump),
            east: device.east,
            west: device.west,
            north: device.north,
            start: device.start,
        };
        let rises = Buttons {
            south: south_rise || (keyboard && jump_rise),
            east: east_rise,
            west: device.west && !self.tracked[position].west,
            north: device.north && !self.tracked[position].north,
            start: device.start && !self.tracked[position].start,
        };
        if south_rise || jump_rise {
            self.menu_confirm = true;
        }
        if leave_rise || (!keyboard && east_rise) {
            self.menu_back = true;
        }
        if move_x != 0.0 || move_y != 0.0 {
            self.menu_x = move_x;
            self.menu_y = move_y;
        }
        let mut slot = self.tracked[position].slot;
        if slot.is_none() {
            let join = if keyboard {
                south_rise || jump_rise
            } else {
                south_rise
            };
            if join {
                slot = self.allocate(&device, &name, &family);
            }
        } else if let Some(slot_index) = slot {
            let state = self.slots[slot_index].as_mut().expect("slot");
            state.name = name.clone();
            state.family = family.clone();
            state.avatar = device.avatar.clone();
            state.profile = device.profile.clone();
            state.move_x = move_x;
            state.move_y = move_y;
            state.look_x = look_x;
            state.look_y = look_y;
            state.held = held;
            state.pressed.merge(rises);
            // The open game's named actions: a phone's own value, or the
            // standard key or stick a pad uses for it.
            for def in &self.actions {
                let entry = state.named.entry(def.name.clone()).or_default();
                if def.axis {
                    let (x, y) = match device.axes.get(&def.name) {
                        Some(axis) if axis.absolute => absolute_move(axis.x, axis.y),
                        Some(axis) => analog_move(axis.x, axis.y),
                        None => match def.pad.as_deref() {
                            Some("move") => (move_x, move_y),
                            Some("look") => (look_x, look_y),
                            _ => (0.0, 0.0),
                        },
                    };
                    entry.x = x;
                    entry.y = y;
                } else {
                    let now = device.actions.get(&def.name).copied().unwrap_or(
                        match def.pad.as_deref() {
                            Some("south") => held.south,
                            Some("east") => held.east,
                            Some("west") => held.west,
                            Some("north") => held.north,
                            Some("start") => held.start,
                            _ => false,
                        },
                    );
                    let before = self.tracked[position]
                        .named
                        .get(&def.name)
                        .copied()
                        .unwrap_or(false);
                    entry.held = now;
                    entry.pressed |= now && !before;
                }
            }
            if keyboard {
                if jump_rise {
                    state.jump = true;
                }
                if leave_rise {
                    slot = None;
                }
            } else if phone {
                if south_rise {
                    state.jump = true;
                }
                if leave_rise {
                    slot = None;
                }
            } else {
                if south_rise {
                    state.jump = true;
                }
                if device.east {
                    let started = self.tracked[position].east_since.get_or_insert(now);
                    if now.saturating_duration_since(*started) >= EAST_HOLD {
                        slot = None;
                    }
                }
            }
        }
        if !device.east {
            self.tracked[position].east_since = None;
        }
        if slot.is_none() {
            if self.tracked[position].slot.is_some() {
                self.release_device(&device.id);
            }
        } else if let Some(slot_index) = slot {
            if let Some(state) = self.slots[slot_index].as_mut() {
                state.move_x = move_x;
                state.move_y = move_y;
            }
            self.tracked[position].slot = Some(slot_index);
        }
        self.tracked[position].south = device.south;
        self.tracked[position].east = device.east;
        self.tracked[position].west = device.west;
        self.tracked[position].north = device.north;
        self.tracked[position].start = device.start;
        let named_now: std::collections::BTreeMap<String, bool> = self
            .actions
            .iter()
            .filter(|def| !def.axis)
            .map(|def| {
                let slot_state = self.tracked[position]
                    .slot
                    .and_then(|index| self.slots.get(index))
                    .and_then(Option::as_ref)
                    .and_then(|slot| slot.named.get(&def.name))
                    .map(|named| named.held);
                (def.name.clone(), slot_state.unwrap_or(false))
            })
            .collect();
        self.tracked[position].named = named_now;
        self.tracked[position].jump = device.jump;
        self.tracked[position].leave = device.leave;
        if slot.is_none() {
            self.tracked[position].slot = None;
            self.tracked[position].east_since = None;
        }
    }

    /// The 1-based player number a device holds, if it has joined.
    pub fn player_of(&self, device_id: &str) -> Option<u16> {
        self.tracked
            .iter()
            .find(|tracked| tracked.id == device_id)
            .and_then(|tracked| tracked.slot)
            .and_then(|slot| u16::try_from(slot + 1).ok())
    }

    /// The first free slot, or a new one when the game's limit allows.
    fn allocate(&mut self, device: &RawDevice, name: &str, family: &str) -> Option<usize> {
        let device_id = device.id.as_str();
        let kind = match device.kind.as_str() {
            "keyboard" => "keyboard",
            "phone" => "phone",
            _ => "pad",
        };
        let index = match self.slots.iter().position(|slot| slot.is_none()) {
            Some(index) => index,
            None if self.limit.is_none_or(|limit| self.slots.len() < limit) => {
                self.slots.push(None);
                self.slots.len() - 1
            }
            None => return None,
        };
        self.slots[index] = Some(Slot {
            device_id: device_id.to_string(),
            name: name.to_string(),
            family: family.to_string(),
            move_x: 0.0,
            move_y: 0.0,
            look_x: 0.0,
            look_y: 0.0,
            jump: false,
            held: Buttons::default(),
            pressed: Buttons::default(),
            kind: kind.to_string(),
            avatar: device.avatar.clone(),
            profile: device.profile.clone(),
            away_since: None,
            named: std::collections::BTreeMap::new(),
        });
        let slot = index;
        if let Some(tracked) = self
            .tracked
            .iter_mut()
            .find(|tracked| tracked.id == device_id)
        {
            tracked.slot = Some(slot);
        }
        Some(slot)
    }

    fn release_device(&mut self, device_id: &str) {
        for slot in &mut self.slots {
            if slot
                .as_ref()
                .is_some_and(|state| state.device_id == device_id)
            {
                *slot = None;
            }
        }
        if let Some(tracked) = self
            .tracked
            .iter_mut()
            .find(|tracked| tracked.id == device_id)
        {
            tracked.slot = None;
            tracked.east_since = None;
        }
    }
}

pub fn valid_slot(slot: &str) -> bool {
    (1..=32).contains(&slot.len())
        && slot
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn valid_device_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 160 && !id.chars().any(char::is_control)
}

fn clean_name(name: &str) -> String {
    name.chars()
        .filter(|ch| !ch.is_control())
        .take(48)
        .collect()
}

fn glyphs(family: &str) -> Glyphs {
    match family {
        "keyboard" => Glyphs {
            jump: "Space".into(),
            leave: "Backspace".into(),
            r#move: "WASD".into(),
        },
        "playstation" => Glyphs {
            jump: "Cross".into(),
            leave: "Circle".into(),
            r#move: "Stick".into(),
        },
        "xbox" | "phone" => Glyphs {
            jump: "A".into(),
            leave: "B".into(),
            r#move: "Stick".into(),
        },
        _ => Glyphs {
            jump: "South".into(),
            leave: "East".into(),
            r#move: "Stick".into(),
        },
    }
}

fn analog_move(x: f32, y: f32) -> (f32, f32) {
    if !x.is_finite() || !y.is_finite() {
        return (0.0, 0.0);
    }
    let x = x.clamp(-1.0, 1.0);
    let y = y.clamp(-1.0, 1.0);
    let strength = x.hypot(y);
    if strength <= DEAD_ZONE {
        return (0.0, 0.0);
    }
    let scaled = ((strength - DEAD_ZONE) / (1.0 - DEAD_ZONE)).clamp(0.0, 1.0);
    let unit = scaled / strength;
    (x * unit, y * unit)
}

fn absolute_move(x: f32, y: f32) -> (f32, f32) {
    if !x.is_finite() || !y.is_finite() {
        return (0.0, 0.0);
    }
    (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0))
}

fn digital_move(x: f32, y: f32) -> (f32, f32) {
    if !x.is_finite() || !y.is_finite() {
        return (0.0, 0.0);
    }
    let x = x.clamp(-1.0, 1.0);
    let y = y.clamp(-1.0, 1.0);
    let strength = x.hypot(y);
    if strength <= f32::EPSILON {
        return (0.0, 0.0);
    }
    if strength > 1.0 {
        return (x / strength, y / strength);
    }
    (x, y)
}
