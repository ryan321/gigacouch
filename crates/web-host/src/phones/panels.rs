//! Panel views: the layer above a phone's controller.
//!
//! A view is a list of items (text, image, choices, text input) whose values
//! are written in the view or bound to its data with `{"bind": "path"}`.
//! Views come from `assets/phone/views.json` (built in), from a game's
//! `phone.views`, or from a running game, and all go through `check_view`.
//!
//! A game shows a view with data and changes that data with updates. The
//! host fills the view in from the data, checks the finished panel with the
//! widget rules, and sends the panel. A phone only ever draws checked
//! content. See docs/phone-views.md.

use super::{PhoneHub, Target};
use crate::State;
use serde_json::{Map, Value, json};

const VIEWS_JSON: &str = include_str!("../assets/phone/views.json");
const MAX_ITEMS: usize = 12;
const MAX_CHOICES: usize = 12;
const MAX_DATA_BYTES: usize = 16 * 1024;
/// Phone actions a choice may start. The pad does them itself.
const PHONE_ACTIONS: &[&str] = &["photo", "profile", "audience"];
/// The built-in views; a game's own views cannot reuse these names.
pub(crate) const BUILT_IN_VIEWS: &[&str] = &["screen", "ask"];

/// What one phone is showing: which view, its data, and the checked panel.
#[derive(Clone)]
pub(crate) struct Shown {
    name: String,
    spec: Value,
    data: Value,
    pub(crate) panel: Value,
}

fn built_in(name: &str) -> Option<Value> {
    let views: Value = serde_json::from_str(VIEWS_JSON).ok()?;
    views.get(name).cloned()
}

fn widget_rule(kind: &str) -> Option<Value> {
    let widgets: Value = serde_json::from_str(super::views::widgets_json()).ok()?;
    widgets["widgets"].get(kind).cloned()
}

fn good_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 64
        && path.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        })
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

/// Checks a view: its items, their types, and every value written in it.
/// Bound values are checked when the view is filled in with data.
pub(crate) fn check_view(name: &str, spec: &Value) -> Result<Value, String> {
    let bad = |why: String| format!("phone view \"{name}\": {why}");
    let items = spec["items"]
        .as_array()
        .ok_or_else(|| bad("a view is { \"items\": [...] }".into()))?;
    if items.is_empty() || items.len() > MAX_ITEMS {
        return Err(bad(format!("a view has 1–{MAX_ITEMS} items")));
    }
    let mut clean = Vec::new();
    for item in items {
        let kind = item["type"].as_str().unwrap_or("");
        let rule = widget_rule(kind)
            .filter(|rule| rule["layout"] == false)
            .ok_or_else(|| bad("each item's type is text, image, choices, or text-input".into()))?;
        let mut out = Map::new();
        out.insert("type".into(), json!(kind));
        for (prop, how) in rule["props"].as_object().into_iter().flatten() {
            let value = &item[prop];
            if value.is_null() {
                if how["required"] == true {
                    return Err(bad(format!("{prop} is required for {kind}")));
                }
                continue;
            }
            if let Some(path) = value.get("bind") {
                let path = path.as_str().filter(|path| good_path(path));
                if how["bindable"] != true || path.is_none() {
                    return Err(bad(format!(
                        "{prop} can't be bound, or its bind path is not a.b.c"
                    )));
                }
                out.insert(prop.clone(), json!({ "bind": path }));
                continue;
            }
            out.insert(prop.clone(), value.clone());
        }
        clean.push(Value::Object(out));
    }
    Ok(json!({ "items": clean }))
}

/// A value at a dotted path, with numbers for list places: "cards.1.label".
fn lookup<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(data, |at, part| match at {
        Value::Array(list) => list.get(part.parse::<usize>().ok()?),
        Value::Object(map) => map.get(part),
        _ => None,
    })
}

/// Sets a value at a dotted path, creating objects along the way.
fn set_path(data: &mut Value, path: &str, value: Value) -> Result<(), String> {
    if !good_path(path) {
        return Err(format!("\"{path}\" is not a path like cards.1.disabled"));
    }
    let parts: Vec<&str> = path.split('.').collect();
    let mut at = data;
    for (index, part) in parts.iter().enumerate() {
        let last = index == parts.len() - 1;
        if !at.is_object() && !at.is_array() {
            *at = json!({});
        }
        at = match at {
            Value::Array(list) => {
                let place: usize = part
                    .parse()
                    .map_err(|_| format!("\"{path}\": {part} is not a list place"))?;
                if place >= list.len() {
                    return Err(format!("\"{path}\": there is no place {place}"));
                }
                &mut list[place]
            }
            Value::Object(map) => map.entry(part.to_string()).or_insert(Value::Null),
            _ => unreachable!(),
        };
        if last {
            *at = value;
            return Ok(());
        }
    }
    Ok(())
}

impl PhoneHub {
    /// The view spec for a name: built in, then the open game's own.
    fn find_view(&self, name: &str) -> Option<Value> {
        built_in(name).or_else(|| self.extras.views.lock().expect("views").get(name).cloned())
    }

    /// Fills a view in from data and checks every finished value.
    fn fill(&self, name: &str, spec: &Value, data: &Value) -> Result<Value, String> {
        let mut items = Vec::new();
        for item in spec["items"].as_array().into_iter().flatten() {
            let kind = item["type"].as_str().unwrap_or("");
            let rule = widget_rule(kind).unwrap_or_default();
            let mut out = Map::new();
            out.insert("type".into(), json!(kind));
            let mut complete = true;
            for (prop, how) in rule["props"].as_object().into_iter().flatten() {
                let written = &item[prop];
                let value = match written.get("bind").and_then(Value::as_str) {
                    Some(path) => lookup(data, path).cloned().unwrap_or(Value::Null),
                    None => written.clone(),
                };
                let value = if value.is_null() {
                    how["default"].clone()
                } else {
                    value
                };
                if value.is_null() {
                    // Data left out: an optional value is skipped, and an
                    // item missing a required one is left off the panel.
                    if how["required"] == true {
                        complete = false;
                    }
                    continue;
                }
                let clean = self.clean_value(prop, how, &value)?;
                if let Some(clean) = clean {
                    out.insert(prop.clone(), clean);
                } else if how["required"] == true {
                    complete = false;
                }
            }
            if complete {
                items.push(Value::Object(out));
            }
        }
        if items.is_empty() {
            return Err(format!("view \"{name}\" shows nothing with this data"));
        }
        let id = clean_id(&data["id"]).unwrap_or_else(|| name.to_string());
        Ok(json!({ "view": name, "id": id, "items": items }))
    }

    /// One finished value, checked by its rule. None: empty, so left out.
    fn clean_value(&self, prop: &str, how: &Value, value: &Value) -> Result<Option<Value>, String> {
        if let Some(most) = how["text"].as_u64() {
            return Ok(clean_text(value, most as usize).map(Value::from));
        }
        if how["id"] == true {
            return clean_id(value)
                .map(|id| Some(json!(id)))
                .ok_or_else(|| format!("{prop} is 1–32 letters, digits, dashes, or underscores"));
        }
        if let Some(range) = how["number"].as_array() {
            let (low, high) = (
                range[0].as_u64().unwrap_or(0),
                range[1].as_u64().unwrap_or(u64::MAX),
            );
            return Ok(Some(json!(value.as_u64().unwrap_or(low).clamp(low, high))));
        }
        if how["bool"] == true {
            return Ok((value.as_bool() == Some(true)).then_some(json!(true)));
        }
        if let Some(allowed) = how["enum"].as_array() {
            return if allowed.contains(value) {
                Ok(Some(value.clone()))
            } else {
                Err(format!("{prop} is not one of the allowed values"))
            };
        }
        if how["image"] == true {
            let name = value.as_str().unwrap_or("");
            let images = self.image_urls();
            return images
                .get(name)
                .map(|url| Some(url.clone()))
                .ok_or_else(|| format!("image \"{name}\" is not one of the game's phone.images"));
        }
        if how["choices"] == true {
            return self.clean_choices(value).map(Some);
        }
        Ok(None)
    }

    fn clean_choices(&self, value: &Value) -> Result<Value, String> {
        let choices = value.as_array().ok_or("choices is a list")?;
        if choices.len() > MAX_CHOICES {
            return Err(format!("a view has at most {MAX_CHOICES} choices"));
        }
        let mut out = Vec::new();
        for choice in choices {
            let id = clean_id(&choice["id"])
                .ok_or("each choice needs an id of 1–32 letters, digits, dashes, or underscores")?;
            let label = clean_text(&choice["label"], 60)
                .ok_or("each choice needs a label of 1–60 characters")?;
            let mut item = Map::new();
            item.insert("id".into(), json!(id));
            item.insert("label".into(), json!(label));
            if let Some(detail) = clean_text(&choice["detail"], 120) {
                item.insert("detail".into(), json!(detail));
            }
            if let Some(color) = choice["color"].as_str() {
                if !super::views::phone_colors().contains(&color) {
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
            // A choice can start one of the phone's own actions, done by the
            // pad from the player's tap: open the camera for a photo, pick a
            // Home person, or join the audience.
            if let Some(action) = choice["action"].as_str() {
                if !PHONE_ACTIONS.contains(&action) {
                    return Err("a choice's action is photo, profile, or audience".into());
                }
                item.insert("action".into(), json!(action));
            }
            for flag in ["disabled", "picked"] {
                if choice[flag] == true {
                    item.insert(flag.into(), json!(true));
                }
            }
            out.push(Value::Object(item));
        }
        Ok(Value::Array(out))
    }

    /// Shows a view on phones: a view name (built in or the game's own) or a
    /// view written out, with data. `null` takes the panel away.
    pub(crate) fn show_view(
        &self,
        state: &State,
        target: Target,
        view: &Value,
        data: &Value,
    ) -> Result<usize, String> {
        let ids = self.targets_for(state, target);
        if view.is_null() {
            let mut panels = self.extras.panels.lock().expect("panels");
            for id in &ids {
                panels.remove(id);
            }
            drop(panels);
            return Ok(self.push_to(&ids, &json!({ "panel": null })));
        }
        if data.to_string().len() > MAX_DATA_BYTES {
            return Err(format!(
                "a view's data is at most {} KiB",
                MAX_DATA_BYTES / 1024
            ));
        }
        let (name, spec) = match view {
            Value::String(name) => {
                let spec = self
                    .find_view(name)
                    .ok_or_else(|| format!("there is no phone view \"{name}\""))?;
                (name.clone(), check_view(name, &spec)?)
            }
            Value::Object(_) => {
                let name = clean_id(&view["id"]).unwrap_or_else(|| "view".into());
                (name.clone(), check_view(&name, view)?)
            }
            _ => return Err("view is a view name, a view, or null".into()),
        };
        let data = if data.is_null() {
            json!({})
        } else {
            data.clone()
        };
        let panel = self.fill(&name, &spec, &data)?;
        {
            let mut panels = self.extras.panels.lock().expect("panels");
            for id in &ids {
                panels.insert(
                    id.clone(),
                    Shown {
                        name: name.clone(),
                        spec: spec.clone(),
                        data: data.clone(),
                        panel: panel.clone(),
                    },
                );
            }
        }
        Ok(self.push_to(&ids, &json!({ "panel": panel })))
    }

    /// Changes part of the data behind the view each phone shows, and sends
    /// the refreshed panel: `{"cards.1.disabled": true, "text": "Played"}`.
    /// With `view` set, only phones showing that view change.
    pub(crate) fn update_view(
        &self,
        state: &State,
        target: Target,
        view: Option<&str>,
        set: &Value,
    ) -> Result<usize, String> {
        let changes = set.as_object().ok_or("set is an object of path to value")?;
        let ids = self.targets_for(state, target);
        let mut refreshed = Vec::new();
        {
            let panels = self.extras.panels.lock().expect("panels");
            for id in &ids {
                let Some(shown) = panels.get(id) else {
                    continue;
                };
                if view.is_some_and(|view| view != shown.name) {
                    continue;
                }
                let mut data = shown.data.clone();
                for (path, value) in changes {
                    set_path(&mut data, path, value.clone())?;
                }
                if data.to_string().len() > MAX_DATA_BYTES {
                    return Err(format!(
                        "a view's data is at most {} KiB",
                        MAX_DATA_BYTES / 1024
                    ));
                }
                let panel = self.fill(&shown.name, &shown.spec, &data)?;
                refreshed.push((
                    id.clone(),
                    Shown {
                        name: shown.name.clone(),
                        spec: shown.spec.clone(),
                        data,
                        panel,
                    },
                ));
            }
        }
        let mut panels = self.extras.panels.lock().expect("panels");
        let mut sent = 0;
        for (id, shown) in refreshed {
            let message = json!({ "panel": shown.panel });
            panels.insert(id.clone(), shown);
            sent += self.push_to(std::slice::from_ref(&id), &message);
        }
        Ok(sent)
    }

    /// The panel a phone shows now, for a phone that reconnects.
    pub(crate) fn panel_of(&self, id: &str) -> Value {
        self.extras
            .panels
            .lock()
            .expect("panels")
            .get(id)
            .map(|shown| shown.panel.clone())
            .unwrap_or(Value::Null)
    }

    /// The question in a phone's panel, if it has one: (id, most characters).
    pub(crate) fn question_of(&self, id: &str) -> Option<(String, usize)> {
        let panel = self.panel_of(id);
        panel["items"]
            .as_array()?
            .iter()
            .find(|item| item["type"] == "text-input")
            .map(|item| {
                (
                    item["id"].as_str().unwrap_or("").to_string(),
                    item["max"].as_u64().unwrap_or(80) as usize,
                )
            })
    }
}

/// Checks a game's own views from its package.
pub(crate) fn check_game_views(
    views: &std::collections::BTreeMap<String, Value>,
) -> Result<Map<String, Value>, String> {
    let mut clean = Map::new();
    for (name, spec) in views {
        let good = !name.is_empty()
            && name.len() <= 32
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if !good || BUILT_IN_VIEWS.contains(&name.as_str()) {
            return Err(format!(
                "phone view \"{name}\" needs a lowercase name that is not a built-in view ({})",
                BUILT_IN_VIEWS.join(", ")
            ));
        }
        clean.insert(name.clone(), check_view(name, spec)?);
    }
    Ok(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_views_pass_the_rules() {
        for name in BUILT_IN_VIEWS {
            check_view(name, &built_in(name).unwrap()).unwrap();
        }
    }

    #[test]
    fn paths_read_and_write_lists_and_objects() {
        let mut data = json!({"cards": [{"id": "a"}, {"id": "b"}]});
        assert_eq!(lookup(&data, "cards.1.id"), Some(&json!("b")));
        set_path(&mut data, "cards.0.disabled", json!(true)).unwrap();
        set_path(&mut data, "title", json!("Hi")).unwrap();
        assert_eq!(data["cards"][0]["disabled"], true);
        assert_eq!(data["title"], "Hi");
        assert!(set_path(&mut data, "cards.9.id", json!("x")).is_err());
        assert!(set_path(&mut data, "a..b", json!(1)).is_err());
    }

    #[test]
    fn a_view_rejects_unknown_items_and_bad_binds() {
        assert!(check_view("v", &json!({"items": [{"type": "stick"}]})).is_err());
        assert!(
            check_view(
                "v",
                &json!({"items": [{"type": "text", "text": {"bind": "a b"}}]})
            )
            .is_err()
        );
        assert!(
            check_view(
                "v",
                &json!({"items": [{"type": "text", "style": {"bind": "s"}, "text": "x"}]})
            )
            .is_err()
        );
        assert!(check_view("v", &json!({"items": []})).is_err());
        let ok = check_view(
            "v",
            &json!({"items": [{"type": "text", "text": {"bind": "a.b"}, "extra": 1}]}),
        )
        .unwrap();
        assert!(ok["items"][0].get("extra").is_none());
    }
}
