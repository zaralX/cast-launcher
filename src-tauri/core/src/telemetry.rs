use serde_json::{Map, Number, Value};

use crate::error::CommandError;
use crate::install::phases::Source;
use crate::instance::{Instance, LocalPackKind, PackProvider};

pub const MAX_KEY_LEN: usize = 40;
pub const MAX_VALUE_LEN: usize = 200;
pub const MAX_NAME_LEN: usize = 40;

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    name: String,
    props: Map<String, Value>,
}

impl Event {
    pub fn new(name: &str) -> Self {
        Self {
            name: clamp(name, MAX_NAME_LEN),
            props: Map::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn props(&self) -> &Map<String, Value> {
        &self.props
    }

    pub fn into_props(self) -> Value {
        Value::Object(self.props)
    }

    pub fn text(mut self, key: &str, value: impl AsRef<str>) -> Self {
        let value = clamp(value.as_ref(), MAX_VALUE_LEN);

        if !value.is_empty() {
            self.props.insert(clamp(key, MAX_KEY_LEN), Value::String(value));
        }

        self
    }

    pub fn maybe(self, key: &str, value: Option<impl AsRef<str>>) -> Self {
        match value {
            Some(value) => self.text(key, value),
            None => self,
        }
    }

    pub fn num(mut self, key: &str, value: impl Into<f64>) -> Self {
        if let Some(number) = Number::from_f64(round(value.into())) {
            self.props.insert(clamp(key, MAX_KEY_LEN), Value::Number(number));
        }

        self
    }

    pub fn flag(self, key: &str, value: bool) -> Self {
        self.num(key, u8::from(value))
    }

    pub fn instance(self, instance: &Instance) -> Self {
        self.text("loader", instance.loader.key())
            .text("mc_version", &instance.minecraft_version)
            .text("source", source_key(instance))
            .maybe("loader_version", instance.loader_version.as_deref())
    }

    pub fn error(self, error: &CommandError) -> Self {
        self.text("code", error.code)
    }
}

pub fn source_key(instance: &Instance) -> &'static str {
    match Source::of(instance) {
        Source::Plain => "plain",
        Source::Pack(PackProvider::Modrinth) => "modrinth",
        Source::Pack(PackProvider::CurseForge) => "curseforge",
        Source::LocalPack(LocalPackKind::Modrinth) => "file:modrinth",
        Source::LocalPack(LocalPackKind::CurseForge) => "file:curseforge",
        Source::LocalPack(LocalPackKind::MultiMc) => "file:multimc",
        Source::CastPack(None) => "castpack",
        Source::CastPack(Some(PackProvider::Modrinth)) => "castpack:modrinth",
        Source::CastPack(Some(PackProvider::CurseForge)) => "castpack:curseforge",
    }
}

const CRASH_SIGNATURES: &[(&str, &str)] = &[
    ("java.lang.outofmemoryerror", "out_of_memory"),
    ("exception_access_violation", "native_crash"),
    ("sigsegv", "native_crash"),
    ("sigbus", "native_crash"),
    ("unsupportedclassversionerror", "java_version"),
    ("unsatisfiedlinkerror", "natives"),
    ("failed to load a library", "natives"),
    ("mixin apply failed", "mixin"),
    ("org.spongepowered.asm.mixin", "mixin"),
    ("duplicatemodsfound", "duplicate_mods"),
    ("modresolutionexception", "missing_deps"),
    ("missing or unsupported mandatory dependencies", "missing_deps"),
    ("incompatible mods found", "missing_deps"),
    ("nosuchmethoderror", "linkage"),
    ("noclassdeffounderror", "linkage"),
    ("nosuchfielderror", "linkage"),
    ("failed to create window", "graphics"),
    ("glfw error", "graphics"),
    ("pixel format not accelerated", "graphics"),
    ("could not create context", "graphics"),
    ("invalid or corrupt jarfile", "corrupt_files"),
    ("java.util.zip.zipexception", "corrupt_files"),
    ("ticking entity", "ticking"),
    ("ticking block entity", "ticking"),
    ("exception in server tick loop", "ticking"),
    ("unknownhostexception", "network"),
    ("java.net.connectexception", "network"),
];

pub fn classify_crash(log_tail: &str) -> &'static str {
    let tail = log_tail.to_ascii_lowercase();

    CRASH_SIGNATURES
        .iter()
        .find(|(needle, _)| tail.contains(needle))
        .map(|(_, reason)| *reason)
        .unwrap_or("unknown")
}

pub fn host_of(url: &str) -> String {
    url::Url::parse(url.trim())
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .unwrap_or_default()
}

pub fn seconds_between(started_at: u64, ended_at: u64) -> f64 {
    ended_at.saturating_sub(started_at) as f64 / 1000.0
}

pub fn minutes(seconds: u64) -> f64 {
    seconds as f64 / 60.0
}

pub fn megabytes(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn clamp(value: &str, limit: usize) -> String {
    let value = value.trim();

    match value.char_indices().nth(limit) {
        Some((at, _)) => value[..at].to_string(),
        None => value.to_string(),
    }
}

fn round(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }

    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::LoaderType;
    use serde_json::json;

    fn instance(extra: Value) -> Instance {
        let mut base = json!({
            "id": "abc",
            "name": "Моя сборка",
            "minecraftVersion": "1.20.1",
            "type": "fabric",
            "loaderVersion": "0.16.9"
        });

        if let (Some(base), Some(extra)) = (base.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                base.insert(key.clone(), value.clone());
            }
        }

        serde_json::from_value(base).unwrap()
    }

    #[test]
    fn strings_are_cut_by_characters_not_bytes() {
        let long = "я".repeat(MAX_VALUE_LEN + 50);
        let event = Event::new("test").text("key", &long);

        let value = event.props()["key"].as_str().unwrap();
        assert_eq!(value.chars().count(), MAX_VALUE_LEN);
    }

    #[test]
    fn empty_values_are_dropped() {
        let event = Event::new("test").text("a", "  ").text("b", "").maybe("c", None::<&str>);

        assert!(event.props().is_empty());
    }

    #[test]
    fn long_event_names_and_keys_are_clamped() {
        let event = Event::new(&"n".repeat(80)).text(&"k".repeat(80), "v");

        assert_eq!(event.name().len(), MAX_NAME_LEN);
        assert_eq!(event.props().keys().next().unwrap().len(), MAX_KEY_LEN);
    }

    #[test]
    fn flags_become_numbers() {
        let event = Event::new("test").flag("on", true).flag("off", false);

        assert_eq!(event.props()["on"], json!(1.0));
        assert_eq!(event.props()["off"], json!(0.0));
    }

    #[test]
    fn broken_numbers_are_skipped() {
        let event = Event::new("test").num("nan", f64::NAN).num("ok", 12.345);

        assert_eq!(event.props()["nan"], json!(0.0));
        assert_eq!(event.props()["ok"], json!(12.35));
    }

    #[test]
    fn an_instance_never_leaks_its_name_or_id() {
        let event = Event::new("test").instance(&instance(json!({})));
        let props = event.into_props().to_string();

        assert!(!props.contains("Моя сборка"));
        assert!(!props.contains("abc"));
        assert!(props.contains("1.20.1"));
        assert!(props.contains("fabric"));
    }

    #[test]
    fn an_error_carries_only_its_code() {
        let error = CommandError::download("Не скачался мод")
            .with_details("C:\\Users\\vasya\\mods\\secret.jar");

        let props = Event::new("test").error(&error).into_props().to_string();

        assert!(props.contains("DOWNLOAD_FAILED"));
        assert!(!props.contains("vasya"));
        assert!(!props.contains("мод"));
    }

    #[test]
    fn every_kind_of_source_has_its_own_key() {
        assert_eq!(source_key(&instance(json!({}))), "plain");

        let pack = json!({"provider": "modrinth", "projectId": "p", "versionId": "v", "fileUrl": "https://x"});
        assert_eq!(source_key(&instance(json!({"pack": pack.clone()}))), "modrinth");

        let local = json!({"kind": "multimc", "name": "TFG", "version": ""});
        assert_eq!(source_key(&instance(json!({"localPack": local}))), "file:multimc");

        let castpack = json!({"catalogId": "rpg", "manifestUrl": "https://x/m.json"});
        assert_eq!(source_key(&instance(json!({"castpack": castpack.clone()}))), "castpack");
        assert_eq!(
            source_key(&instance(json!({"castpack": castpack, "pack": pack}))),
            "castpack:modrinth"
        );
    }

    #[test]
    fn loader_keys_stay_lowercase() {
        for loader in LoaderType::ALL {
            assert_eq!(loader.key(), loader.key().to_ascii_lowercase());
        }
    }

    #[test]
    fn crashes_are_classified_by_the_first_matching_signature() {
        assert_eq!(
            classify_crash("java.lang.OutOfMemoryError: Java heap space"),
            "out_of_memory"
        );
        assert_eq!(
            classify_crash("# EXCEPTION_ACCESS_VIOLATION (0xc0000005)"),
            "native_crash"
        );
        assert_eq!(
            classify_crash("Mixin apply failed sodium.mixins.json"),
            "mixin"
        );
        assert_eq!(classify_crash("GLFW error 65542: WGL"), "graphics");
        assert_eq!(classify_crash("всё хорошо, игра закрылась"), "unknown");
    }

    #[test]
    fn running_out_of_memory_wins_over_the_stack_trace_below_it() {
        let tail = "java.lang.OutOfMemoryError: Java heap space\n\tat org.spongepowered.asm.mixin.Foo";

        assert_eq!(classify_crash(tail), "out_of_memory");
    }

    #[test]
    fn only_the_host_survives_a_url() {
        assert_eq!(host_of("https://cdn.modrinth.com/data/x/y.jar?v=2"), "cdn.modrinth.com");
        assert_eq!(host_of("  https://s3.zaralx.ru/launcher/latest.json "), "s3.zaralx.ru");
        assert_eq!(host_of("не ссылка"), "");
    }

    #[test]
    fn durations_are_counted_forward_only() {
        assert_eq!(seconds_between(1_000, 4_500), 3.5);
        assert_eq!(seconds_between(4_500, 1_000), 0.0);
        assert_eq!(minutes(90), 1.5);
        assert_eq!(megabytes(1024 * 1024 * 3), 3.0);
    }
}
