use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MojangOs {
    Windows,
    Osx,
    Linux,
    Unknown,
}

impl MojangOs {
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Osx
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Unknown
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Osx => "osx",
            Self::Linux => "linux",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeContext {
    pub os: MojangOs,
    pub arch: String,
    pub os_version: String,
}

impl RuntimeContext {
    pub fn new(arch: &str, os_version: &str) -> Self {
        Self {
            os: MojangOs::current(),
            arch: normalize_arch(arch),
            os_version: os_version.to_string(),
        }
    }

    pub fn bits(&self) -> &'static str {
        match self.arch.as_str() {
            "x86" | "arm32" => "32",
            _ => "64",
        }
    }

    pub fn classifier(&self) -> String {
        format!("{}-{}", self.os.as_str(), self.arch)
    }
}

pub fn normalize_arch(arch: &str) -> String {
    match arch.trim().to_ascii_lowercase().as_str() {
        "amd64" | "x86_64" | "x64" => "x86_64".to_string(),
        "i386" | "i486" | "i586" | "i686" | "x86" => "x86".to_string(),
        "aarch64" | "arm64" => "arm64".to_string(),
        "arm" | "armhf" | "arm32" => "arm32".to_string(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

pub type Features = HashMap<String, bool>;

pub fn check_rules(rules: Option<&[Rule]>, ctx: &RuntimeContext, features: &Features) -> bool {
    let Some(rules) = rules else { return true };

    if rules.is_empty() {
        return true;
    }

    let mut allowed = false;

    for rule in rules {
        if !rule_matches(rule, ctx, features) {
            continue;
        }
        allowed = rule.action == RuleAction::Allow;
    }

    allowed
}

fn rule_matches(rule: &Rule, ctx: &RuntimeContext, features: &Features) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if !os_name_matches(name, ctx) {
                return false;
            }
        }

        if let Some(arch) = &os.arch {
            if normalize_arch(arch) != ctx.arch {
                return false;
            }
        }

        if let Some(version) = &os.version {
            if !os_version_matches(version, &ctx.os_version) {
                return false;
            }
        }
    }

    if let Some(required) = &rule.features {
        for (key, expected) in required {
            if features.get(key).copied().unwrap_or(false) != *expected {
                return false;
            }
        }
    }

    true
}

fn os_name_matches(name: &str, ctx: &RuntimeContext) -> bool {
    name == ctx.os.as_str() || name == ctx.classifier()
}

fn os_version_matches(pattern: &str, current: &str) -> bool {
    match Regex::new(pattern) {
        Ok(regex) => regex.is_match(current),
        Err(error) => {
            eprintln!("Некорректное правило по версии ОС '{pattern}': {error}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(os: MojangOs, arch: &str, version: &str) -> RuntimeContext {
        RuntimeContext {
            os,
            arch: normalize_arch(arch),
            os_version: version.to_string(),
        }
    }

    fn rules(value: serde_json::Value) -> Vec<Rule> {
        serde_json::from_value(value).unwrap()
    }

    fn allows(value: serde_json::Value, ctx: &RuntimeContext) -> bool {
        check_rules(Some(&rules(value)), ctx, &Features::new())
    }

    fn mac_arm() -> RuntimeContext {
        ctx(MojangOs::Osx, "aarch64", "15.1")
    }

    fn win_x64() -> RuntimeContext {
        ctx(MojangOs::Windows, "amd64", "10.0")
    }

    fn win_x86() -> RuntimeContext {
        ctx(MojangOs::Windows, "x86", "6.1")
    }

    #[test]
    fn arch_names_are_normalized_to_mojang_vocabulary() {
        assert_eq!(normalize_arch("amd64"), "x86_64");
        assert_eq!(normalize_arch("i686"), "x86");
        assert_eq!(normalize_arch("aarch64"), "arm64");
        assert_eq!(normalize_arch("ARM"), "arm32");
        assert_eq!(normalize_arch("riscv64"), "riscv64");
    }

    #[test]
    fn bits_follow_the_jvm_architecture() {
        assert_eq!(win_x64().bits(), "64");
        assert_eq!(win_x86().bits(), "32");
        assert_eq!(mac_arm().bits(), "64");
        assert_eq!(ctx(MojangOs::Linux, "arm", "6.8").bits(), "32");
    }

    #[test]
    fn osx_rules_match_on_macos() {
        let osx_only = json!([{ "action": "allow", "os": { "name": "osx" } }]);

        assert!(allows(osx_only.clone(), &mac_arm()));
        assert!(!allows(osx_only, &win_x64()));
    }

    #[test]
    fn last_matching_rule_wins() {
        let not_osx = json!([
            { "action": "allow" },
            { "action": "disallow", "os": { "name": "osx" } }
        ]);

        assert!(!allows(not_osx.clone(), &mac_arm()));
        assert!(allows(not_osx, &win_x64()));
    }

    #[test]
    fn arch_inside_os_is_honoured() {
        let x86_only = json!([{ "action": "allow", "os": { "name": "windows", "arch": "x86" } }]);

        assert!(allows(x86_only.clone(), &win_x86()));
        assert!(!allows(x86_only, &win_x64()));
    }

    #[test]
    fn os_version_is_a_regex() {
        let win10 = json!([{ "action": "allow", "os": { "name": "windows", "version": "^10\\." } }]);

        assert!(allows(win10.clone(), &win_x64()));
        assert!(!allows(win10, &win_x86()));
    }

    #[test]
    fn broken_version_regex_does_not_match() {
        let broken = json!([{ "action": "allow", "os": { "name": "windows", "version": "(((" } }]);
        assert!(!allows(broken, &win_x64()));
    }

    #[test]
    fn classifier_style_os_names_are_supported() {
        let arm_only = json!([{ "action": "allow", "os": { "name": "osx-arm64" } }]);

        assert!(allows(arm_only.clone(), &mac_arm()));
        assert!(!allows(arm_only, &ctx(MojangOs::Osx, "x86_64", "13.0")));
    }

    #[test]
    fn missing_feature_counts_as_disabled() {
        let demo = rules(json!([{ "action": "allow", "features": { "is_demo_user": true } }]));
        let not_demo = rules(json!([{ "action": "allow", "features": { "is_demo_user": false } }]));

        let mut enabled = Features::new();
        enabled.insert("is_demo_user".into(), true);

        assert!(!check_rules(Some(&demo), &win_x64(), &Features::new()));
        assert!(check_rules(Some(&demo), &win_x64(), &enabled));
        assert!(check_rules(Some(&not_demo), &win_x64(), &Features::new()));
    }

    #[test]
    fn absent_rules_allow_everything() {
        assert!(check_rules(None, &win_x64(), &Features::new()));
        assert!(check_rules(Some(&[]), &win_x64(), &Features::new()));
    }
}
