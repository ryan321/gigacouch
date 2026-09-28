//! Phone views: the widget rules and the built-in layouts, read from
//! `assets/phone/widgets.json` and `assets/phone/layouts.json`, and the one
//! validator every layout goes through: built in, from a package, or sent by
//! a running game. See docs/phone-views.md.

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::{Map, Value, json};
use std::sync::OnceLock;

const WIDGETS_JSON: &str = include_str!("../assets/phone/widgets.json");
const LAYOUTS_JSON: &str = include_str!("../assets/phone/layouts.json");
const MAX_LAYOUT_CONTROLS: usize = 16;

/// A JSON object that keeps its keys in file order: the order layouts and
/// widgets are listed in is the order people see them in.
struct Ordered(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Ordered;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Ordered, A::Error> {
                let mut entries = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    entries.push((key, value));
                }
                Ok(Ordered(entries))
            }
        }
        deserializer.deserialize_map(Entries)
    }
}

struct Rules {
    widgets: Vec<(String, Value)>,
    keys: Vec<&'static str>,
    colors: Vec<&'static str>,
    layouts: Vec<(String, Value)>,
    layout_names: Vec<&'static str>,
}

fn leak(words: &str) -> &'static str {
    Box::leak(words.to_string().into_boxed_str())
}

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| {
        let widgets_file: Value = serde_json::from_str(WIDGETS_JSON).expect("widgets.json");
        #[derive(serde::Deserialize)]
        struct WidgetsOnly {
            widgets: Ordered,
        }
        let widgets = serde_json::from_str::<WidgetsOnly>(WIDGETS_JSON)
            .expect("widgets.json widgets")
            .widgets
            .0;
        let words = |field: &str| -> Vec<&'static str> {
            widgets_file[field]
                .as_array()
                .expect("widgets.json lists")
                .iter()
                .map(|word| leak(word.as_str().expect("a word")))
                .collect()
        };
        let layouts = serde_json::from_str::<Ordered>(LAYOUTS_JSON)
            .expect("layouts.json")
            .0;
        let layout_names = layouts.iter().map(|(name, _)| leak(name)).collect();
        Rules {
            widgets,
            keys: words("keys"),
            colors: words("colors"),
            layouts,
            layout_names,
        }
    })
}

/// The built-in layouts, in the order they are listed.
pub fn phone_layouts() -> &'static [&'static str] {
    &rules().layout_names
}

/// The five keys every player has, which buttons bind to.
pub fn layout_keys() -> &'static [&'static str] {
    &rules().keys
}

/// Colors a widget or a screen choice can use.
pub fn phone_colors() -> &'static [&'static str] {
    &rules().colors
}

/// The widget rules as served to the pad.
pub(crate) fn widgets_json() -> &'static str {
    WIDGETS_JSON
}

/// The built-in layouts as served to the pad.
pub(crate) fn layouts_json() -> &'static str {
    LAYOUTS_JSON
}

/// Title and description of each built-in layout, for games and galleries.
pub(crate) fn layout_details() -> Value {
    Value::Array(
        rules()
            .layouts
            .iter()
            .map(|(name, layout)| json!({"name": name, "title": layout["title"], "about": layout["about"]}))
            .collect(),
    )
}

fn widget(kind: &str) -> Option<&'static Value> {
    rules()
        .widgets
        .iter()
        .find(|(name, _)| name == kind)
        .map(|(_, rule)| rule)
}

/// Widgets a layout can use today, in the order they are listed.
fn layout_widgets() -> Vec<&'static str> {
    rules()
        .widgets
        .iter()
        .filter(|(_, rule)| rule["layout"] != false)
        .map(|(name, _)| name.as_str())
        .collect()
}

/// "a, b, or c".
fn one_of(words: &[String]) -> String {
    match words.len() {
        0 => String::new(),
        1 => words[0].clone(),
        2 => format!("{} or {}", words[0], words[1]),
        _ => format!(
            "{}, or {}",
            words[..words.len() - 1].join(", "),
            words[words.len() - 1]
        ),
    }
}

/// Checks a layout and returns it cleaned: only the fields the pad draws,
/// every number inside the screen, every value one the rules allow.
pub(crate) fn check_layout(
    name: &str,
    spec: &Value,
    context: &LayoutContext,
) -> Result<Value, String> {
    let mut clean = Map::new();
    clean.insert(
        "landscape".into(),
        check_controls(name, &spec["landscape"], context)?,
    );
    if !spec["portrait"].is_null() {
        clean.insert(
            "portrait".into(),
            check_controls(name, &spec["portrait"], context)?,
        );
    }
    Ok(Value::Object(clean))
}

/// What a layout may refer to besides the standard keys and sticks: the
/// game's declared actions and the sounds it can play.
#[derive(Default)]
pub(crate) struct LayoutContext {
    pub buttons: Vec<String>,
    pub axes: Vec<String>,
    /// The game's own sound names; the stock sounds always count.
    pub sounds: Vec<String>,
}

fn check_controls(name: &str, list: &Value, context: &LayoutContext) -> Result<Value, String> {
    let bad = |why: String| format!("phone layout \"{name}\": {why}");
    let list = list
        .as_array()
        .ok_or_else(|| bad("landscape and portrait are lists of controls".into()))?;
    if list.is_empty() || list.len() > MAX_LAYOUT_CONTROLS {
        return Err(bad(format!(
            "a layout has 1–{MAX_LAYOUT_CONTROLS} controls"
        )));
    }
    list.iter()
        .map(|control| check_control(control, context).map_err(bad))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

fn fraction(value: &Value, what: &str) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|number| (0.0..=1.0).contains(number))
        .ok_or_else(|| format!("{what} must be a number from 0 to 1"))
}

/// A press's feedback, played on the phone with no round trip: a stock or
/// game sound, a rumble preset (sent to the phone as its pattern), a flash.
fn check_feedback(value: &Value, context: &LayoutContext) -> Result<Value, String> {
    let mut clean = Map::new();
    if let Some(sound) = value["sound"].as_str() {
        if !crate::package::STOCK_SOUNDS.contains(&sound)
            && !context.sounds.iter().any(|name| name == sound)
        {
            return Err(format!(
                "feedback sound \"{sound}\" is not a stock sound or one of the game's phone.sounds"
            ));
        }
        clean.insert("sound".into(), json!(sound));
    }
    if let Some(preset) = value["rumble"].as_str() {
        let pattern = super::RUMBLE_PRESETS
            .iter()
            .find(|(name, _)| *name == preset)
            .map(|(_, steps)| steps.to_vec())
            .ok_or_else(|| format!("feedback rumble \"{preset}\" is not a rumble preset"))?;
        clean.insert("rumble".into(), json!(pattern));
    }
    if value["flash"] == true {
        clean.insert("flash".into(), json!(true));
    }
    if clean.is_empty() {
        return Err("feedback has a sound, a rumble, or flash".into());
    }
    Ok(Value::Object(clean))
}

fn check_control(control: &Value, context: &LayoutContext) -> Result<Value, String> {
    let kind = control["type"].as_str().unwrap_or("");
    let rule = widget(kind)
        .filter(|rule| rule["layout"] != false)
        .ok_or_else(|| {
            let names: Vec<String> = layout_widgets()
                .iter()
                .map(|name| name.to_string())
                .collect();
            format!("each control's type is {}", one_of(&names))
        })?;
    let mut clean = Map::new();
    clean.insert("type".into(), json!(kind));

    // Where it sits: a rect, a point with a size, or either.
    let place = rule["place"].as_str().unwrap_or("rect");
    let use_rect = place == "rect" || (place == "point-or-rect" && !control["rect"].is_null());
    if use_rect {
        let parts = control["rect"]
            .as_array()
            .filter(|parts| parts.len() == 4)
            .ok_or("rect must be [x, y, width, height]")?;
        let numbers: Vec<f64> = parts
            .iter()
            .map(|part| fraction(part, "rect"))
            .collect::<Result<_, _>>()?;
        clean.insert("rect".into(), json!(numbers));
    } else {
        for field in ["x", "y", "size"] {
            clean.insert(field.into(), json!(fraction(&control[field], field)?));
        }
    }

    // Its properties, as the widget's rules describe them. Anything else is
    // dropped: the pad draws from this data alone.
    if let Some(props) = rule["props"].as_object() {
        for (prop, how) in props {
            let value = &control[prop];
            if value.is_null() {
                if how["required"] == true {
                    return Err(format!("{prop} is required for a {kind}"));
                }
                if !how["default"].is_null() {
                    clean.insert(prop.clone(), how["default"].clone());
                }
                continue;
            }
            if let Some(kind_of) = how["action"].as_str() {
                // A standard key or stick, or one of the game's actions.
                let (standard, declared): (Vec<String>, &Vec<String>) = if kind_of == "axis" {
                    (vec!["move".into(), "look".into()], &context.axes)
                } else {
                    (
                        layout_keys().iter().map(|key| key.to_string()).collect(),
                        &context.buttons,
                    )
                };
                let word = value.as_str().unwrap_or("");
                if !standard
                    .iter()
                    .chain(declared.iter())
                    .any(|name| name == word)
                {
                    return Err(format!(
                        "{prop} is {}, or one of the game's {kind_of} actions",
                        one_of(&standard)
                    ));
                }
                clean.insert(prop.clone(), json!(word));
            } else if how["feedback"] == true {
                clean.insert(prop.clone(), check_feedback(value, context)?);
            } else if let Some(allowed) = how["enum"].as_array() {
                if !allowed.contains(value) {
                    let words: Vec<String> = allowed
                        .iter()
                        .map(|word| {
                            word.as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| word.to_string())
                        })
                        .collect();
                    return Err(format!("{prop} is one of {}", one_of(&words)));
                }
                clean.insert(prop.clone(), value.clone());
            } else if let Some(most) = how["text"].as_u64() {
                let words = value
                    .as_str()
                    .filter(|words| {
                        words.chars().count() <= most as usize
                            && !words.chars().any(char::is_control)
                    })
                    .ok_or_else(|| format!("{prop} must be text of at most {most} characters"))?;
                clean.insert(prop.clone(), json!(words));
            } else if how["bool"] == true && value.as_bool() == Some(true) {
                clean.insert(prop.clone(), json!(true));
            }
        }
    }
    Ok(Value::Object(clean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_layout_passes_the_widget_rules() {
        for (name, layout) in &rules().layouts {
            let clean = check_layout(name, layout, &LayoutContext::default())
                .unwrap_or_else(|error| panic!("{error}"));
            assert_eq!(
                clean["landscape"].as_array().unwrap().len(),
                layout["landscape"].as_array().unwrap().len()
            );
            assert!(
                layout["title"].is_string() && layout["about"].is_string(),
                "{name} needs a title and about"
            );
        }
    }

    #[test]
    fn names_keep_their_file_order() {
        assert_eq!(phone_layouts()[0], "stick-2");
        assert_eq!(layout_keys(), ["south", "east", "west", "north", "start"]);
        assert_eq!(layout_widgets()[0], "stick");
    }

    #[test]
    fn one_of_reads_like_a_sentence() {
        let words = |list: &[&str]| list.iter().map(|word| word.to_string()).collect::<Vec<_>>();
        assert_eq!(one_of(&words(&["a"])), "a");
        assert_eq!(one_of(&words(&["a", "b"])), "a or b");
        assert_eq!(one_of(&words(&["a", "b", "c"])), "a, b, or c");
    }
}
