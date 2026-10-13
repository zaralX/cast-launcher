use std::collections::{BTreeMap, HashSet};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{CommandError, CommandResult};

pub const BUILT_IN: [&str; 8] = [
    "depth", "coal", "oled", "light", "classic", "nether", "end", "moss",
];

pub const SYSTEM: &str = "system";

pub const DEFAULT_DARK: &str = "depth";

pub const DEFAULT_LIGHT: &str = "light";

pub const TOKENS: [&str; 19] = [
    "frame",
    "sheet",
    "panel",
    "control",
    "hover",
    "raise",
    "line",
    "line-strong",
    "card-line",
    "sheet-edge",
    "fg",
    "fg-muted",
    "fg-faint",
    "accent",
    "on-accent",
    "success",
    "warning",
    "danger",
    "on-danger",
];

pub const MAX_THEMES: usize = 64;

pub const MAX_NAME: usize = 40;

const SHARE_PREFIX: &str = "cast-theme:";

const SHARE_VERSION: u64 = 1;

const CUSTOM_PREFIX: &str = "custom-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomTheme {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub classic: bool,
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct Shared {
    v: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    classic: bool,
    #[serde(default)]
    colors: BTreeMap<String, String>,
}

pub fn is_built_in(value: &str) -> bool {
    BUILT_IN.contains(&value)
}

impl CustomTheme {
    pub fn normalized(self) -> Option<Self> {
        let id = self.id.trim().to_ascii_lowercase();

        valid_id(&id).then(|| Self { id, ..self }.cleaned())
    }

    fn cleaned(self) -> Self {
        let colors = self
            .colors
            .into_iter()
            .filter(|(token, _)| TOKENS.contains(&token.as_str()))
            .filter_map(|(token, value)| color(&value).map(|value| (token, value)))
            .collect();

        Self {
            id: self.id,
            name: self.name.trim().chars().take(MAX_NAME).collect(),
            classic: self.classic,
            colors,
        }
    }
}

fn valid_id(id: &str) -> bool {
    id.strip_prefix(CUSTOM_PREFIX).is_some_and(|rest| {
        !rest.is_empty()
            && rest.len() <= 64
            && rest
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    })
}

fn color(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    let hex = value.strip_prefix('#')?;

    let valid = matches!(hex.len(), 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    valid.then_some(value)
}

pub fn normalize(themes: Vec<CustomTheme>) -> Vec<CustomTheme> {
    let mut seen = HashSet::new();

    themes
        .into_iter()
        .filter_map(CustomTheme::normalized)
        .filter(|theme| seen.insert(theme.id.clone()))
        .take(MAX_THEMES)
        .collect()
}

pub fn choice(value: &str, themes: &[CustomTheme], allow_system: bool, fallback: &str) -> String {
    let value = value.trim();
    let known = is_built_in(value)
        || (allow_system && value == SYSTEM)
        || themes.iter().any(|theme| theme.id == value);

    match known {
        true => value.to_string(),
        false => fallback.to_string(),
    }
}

pub fn public_name(value: &str) -> &str {
    match is_built_in(value) || value == SYSTEM {
        true => value,
        false => "custom",
    }
}

pub fn new_id() -> String {
    format!("{CUSTOM_PREFIX}{}", uuid::Uuid::new_v4().simple())
}

pub fn encode(theme: CustomTheme) -> String {
    let theme = theme.cleaned();
    let shared = Shared {
        v: SHARE_VERSION,
        name: theme.name,
        classic: theme.classic,
        colors: theme.colors,
    };
    let json = serde_json::to_vec(&shared).expect("a theme of strings and a map always serializes");

    format!("{SHARE_PREFIX}{}", URL_SAFE_NO_PAD.encode(json))
}

pub fn decode(code: &str) -> CommandResult<CustomTheme> {
    let code = code.trim();
    let invalid = || CommandError::invalid_input("error.reason.theme.code");

    let json = match code.starts_with('{') {
        true => code.as_bytes().to_vec(),
        false => {
            let body: String = code
                .strip_prefix(SHARE_PREFIX)
                .ok_or_else(invalid)?
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();

            URL_SAFE_NO_PAD
                .decode(body.trim_end_matches('='))
                .map_err(|e| invalid().with_details(e.to_string()))?
        }
    };

    let value: Value =
        serde_json::from_slice(&json).map_err(|e| invalid().with_details(e.to_string()))?;

    let version = value.get("v").and_then(Value::as_u64).ok_or_else(invalid)?;
    if version > SHARE_VERSION {
        return Err(CommandError::unsupported("error.reason.theme.newer"));
    }

    let shared: Shared =
        serde_json::from_value(value).map_err(|e| invalid().with_details(e.to_string()))?;

    let theme = CustomTheme {
        id: new_id(),
        name: shared.name,
        classic: shared.classic,
        colors: shared.colors,
    }
    .normalized()
    .expect("new_id always makes a valid id");

    match theme.colors.is_empty() {
        true => Err(invalid()),
        false => Ok(theme),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    fn theme(id: &str) -> CustomTheme {
        CustomTheme {
            id: id.into(),
            name: "Ночь".into(),
            classic: false,
            colors: BTreeMap::from([
                ("panel".into(), "#1B2330".into()),
                ("line".into(), "#96b4dc1a".into()),
            ]),
        }
    }

    #[test]
    fn normalizing_keeps_known_tokens_with_valid_colors() {
        let mut raw = theme("custom-a1");
        raw.colors.insert("panel".into(), " #1B2330 ".into());
        raw.colors.insert("frame".into(), "red".into());
        raw.colors.insert("sheet".into(), "#12345".into());
        raw.colors.insert("shadow".into(), "#000000".into());

        let theme = raw.normalized().unwrap();

        assert_eq!(
            theme.colors,
            BTreeMap::from([
                ("line".to_string(), "#96b4dc1a".to_string()),
                ("panel".to_string(), "#1b2330".to_string()),
            ])
        );
    }

    #[test]
    fn normalizing_clips_the_name() {
        let mut raw = theme("custom-a1");
        raw.name = format!("  {}  ", "я".repeat(60));

        let theme = raw.normalized().unwrap();

        assert_eq!(theme.name.chars().count(), MAX_NAME);
    }

    #[test]
    fn ids_must_be_custom_and_unique() {
        let themes = normalize(vec![
            theme("custom-a1"),
            theme("CUSTOM-A1"),
            theme("depth"),
            theme("system"),
            theme("custom-"),
            theme("custom-bad id"),
            theme("custom-b2"),
        ]);

        let ids: Vec<&str> = themes.iter().map(|theme| theme.id.as_str()).collect();
        assert_eq!(ids, ["custom-a1", "custom-b2"]);
    }

    #[test]
    fn a_choice_falls_back_when_its_theme_is_gone() {
        let themes = vec![theme("custom-a1")];

        assert_eq!(
            choice("custom-a1", &themes, false, DEFAULT_DARK),
            "custom-a1"
        );
        assert_eq!(choice("light", &themes, false, DEFAULT_DARK), "light");
        assert_eq!(
            choice("custom-zz", &themes, false, DEFAULT_LIGHT),
            DEFAULT_LIGHT
        );
        assert_eq!(choice("system", &themes, true, DEFAULT_DARK), SYSTEM);
        assert_eq!(choice("system", &themes, false, DEFAULT_DARK), DEFAULT_DARK);
    }

    #[test]
    fn telemetry_sees_custom_themes_only_as_custom() {
        assert_eq!(public_name("moss"), "moss");
        assert_eq!(public_name(SYSTEM), SYSTEM);
        assert_eq!(public_name("custom-a1"), "custom");
    }

    #[test]
    fn a_shared_code_round_trips_with_a_new_id() {
        let original = theme("custom-a1").normalized().unwrap();
        let code = encode(original.clone());

        assert!(code.starts_with(SHARE_PREFIX));

        let decoded = decode(&format!("  {}\n{}  ", &code[..20], &code[20..])).unwrap();

        assert_ne!(decoded.id, original.id);
        assert!(valid_id(&decoded.id));
        assert_eq!(decoded.name, original.name);
        assert_eq!(decoded.classic, original.classic);
        assert_eq!(decoded.colors, original.colors);
    }

    #[test]
    fn plain_json_is_accepted_too() {
        let decoded = decode(
            r##"{ "v": 1, "name": "Мох", "classic": true, "colors": { "accent": "#f3a641" } }"##,
        )
        .unwrap();

        assert_eq!(decoded.name, "Мох");
        assert!(decoded.classic);
        assert_eq!(decoded.colors["accent"], "#f3a641");
    }

    #[test]
    fn garbage_is_not_a_theme() {
        for code in [
            "",
            "hello",
            "cast-theme:!!!",
            "cast-theme:e30",
            r#"{ "name": "no version" }"#,
            r#"{ "v": 1, "colors": { "frame": "blue" } }"#,
        ] {
            let error = decode(code).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "code {code:?}");
        }
    }

    #[test]
    fn a_code_from_a_newer_launcher_is_unsupported() {
        let error = decode(r##"{ "v": 2, "colors": { "frame": "#000000" } }"##).unwrap_err();

        assert_eq!(error.code, ErrorCode::Unsupported);
    }
}
