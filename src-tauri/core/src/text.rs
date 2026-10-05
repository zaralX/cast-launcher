use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Text shown to the user: an i18n key from the frontend locales plus its parameters.
/// Rust never produces user-facing wording itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiText {
    pub key: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, Param>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Param {
    Value(String),
    Text(UiText),
}

impl UiText {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn param(mut self, name: &str, value: impl fmt::Display) -> Self {
        self.params
            .insert(name.to_string(), Param::Value(value.to_string()));
        self
    }

    pub fn param_text(mut self, name: &str, text: UiText) -> Self {
        self.params.insert(name.to_string(), Param::Text(text));
        self
    }

    /// Whether the key or any parameter mentions `word`, case-insensitively.
    #[cfg(test)]
    pub fn mentions(&self, word: &str) -> bool {
        serde_json::to_string(self)
            .map(|json| json.to_lowercase().contains(&word.to_lowercase()))
            .unwrap_or(false)
    }
}

/// `key {name: value, ...}`, for logs.
impl fmt::Display for UiText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key)?;

        if self.params.is_empty() {
            return Ok(());
        }

        f.write_str(" {")?;
        for (index, (name, value)) in self.params.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            match value {
                Param::Value(value) => write!(f, "{name}: {value}")?,
                Param::Text(text) => write!(f, "{name}: {text}")?,
            }
        }
        f.write_str("}")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use regex::Regex;

    use super::*;

    #[test]
    fn params_serialize_as_strings_or_nested_texts() {
        let text = UiText::new("outer")
            .param("name", "Vanilla")
            .param_text("reason", UiText::new("inner").param("count", 3));

        assert_eq!(
            serde_json::to_value(&text).unwrap(),
            serde_json::json!({
                "key": "outer",
                "params": {
                    "name": "Vanilla",
                    "reason": { "key": "inner", "params": { "count": "3" } },
                },
            })
        );
    }

    #[test]
    fn a_text_without_params_is_just_a_key() {
        assert_eq!(
            serde_json::to_value(UiText::new("install.phase.java")).unwrap(),
            serde_json::json!({ "key": "install.phase.java" })
        );
    }

    #[test]
    fn logs_show_the_key_and_the_params() {
        let text = UiText::new("outer")
            .param("name", "Vanilla")
            .param_text("reason", UiText::new("inner").param("count", 3));

        assert_eq!(
            text.to_string(),
            "outer {name: Vanilla, reason: inner {count: 3}}"
        );
        assert_eq!(UiText::new("bare").to_string(), "bare");
    }

    #[test]
    fn nested_texts_survive_a_round_trip() {
        let text = UiText::new("outer").param_text("reason", UiText::new("inner").param("a", "b"));
        let back: UiText = serde_json::from_value(serde_json::to_value(&text).unwrap()).unwrap();

        assert_eq!(back, text);
    }

    #[test]
    fn every_key_rust_sends_exists_in_both_locales() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let key = Regex::new(
            r#""((?:error\.reason|install\.(?:phase|message)|settings\.import\.step|import\.blocked|catalog\.(?:unsupported|provider))\.[A-Za-z0-9_.-]+)""#,
        )
        .unwrap();
        let phase = Regex::new(r#"Phase::new\("([a-z-]+)""#).unwrap();

        let mut used = BTreeSet::from(["install.phase.prepare".to_string()]);
        for dir in ["src-tauri/core/src", "src-tauri/src"] {
            for source in rust_files(&repo.join(dir)) {
                let text = std::fs::read_to_string(&source).unwrap();
                let code = text.split("#[cfg(test)]").next().unwrap_or_default();

                used.extend(key.captures_iter(code).map(|found| found[1].to_string()));
                used.extend(
                    phase
                        .captures_iter(code)
                        .map(|found| format!("install.phase.{}", &found[1])),
                );
            }
        }

        assert!(used.len() > 100, "the scan found only {used:?}");

        for locale in ["ru", "en"] {
            let path = repo.join(format!("i18n/locales/{locale}.json"));
            let messages: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

            let missing: Vec<_> = used
                .iter()
                .filter(|key| !messages.contains_key(*key))
                .collect();
            assert!(missing.is_empty(), "{locale}.json lacks {missing:?}");
        }
    }

    fn rust_files(dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(rust_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }

        files
    }
}
