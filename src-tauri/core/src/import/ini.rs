use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Ini {
    sections: HashMap<String, HashMap<String, String>>,
}

impl Ini {
    pub fn parse(text: &str) -> Self {
        let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut current = String::from("General");

        for line in text.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }

            if let Some(name) = line.strip_prefix('[').and_then(|line| line.strip_suffix(']')) {
                current = name.trim().to_string();
                continue;
            }

            let Some((key, value)) = line.split_once('=') else { continue };

            sections
                .entry(current.clone())
                .or_default()
                .insert(key.trim().to_string(), unescape(value.trim()));
        }

        Self { sections }
    }

    pub fn section(&self, name: &str) -> Section<'_> {
        Section {
            values: self.sections.get(name),
        }
    }

    pub fn general(&self) -> Section<'_> {
        self.section("General")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Section<'a> {
    values: Option<&'a HashMap<String, String>>,
}

impl Section<'_> {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values
            .and_then(|values| values.get(key))
            .map(String::as_str)
    }

    /// Значение без пробелов по краям; отсутствующий ключ неотличим от пустого.
    pub fn string(&self, key: &str) -> String {
        self.get(key).unwrap_or_default().trim().to_string()
    }

    pub fn flag(&self, key: &str) -> bool {
        matches!(
            self.get(key).map(str::trim).map(str::to_lowercase).as_deref(),
            Some("true" | "1" | "yes")
        )
    }

    pub fn number(&self, key: &str) -> Option<u32> {
        self.get(key).map(str::trim).and_then(|value| value.parse().ok())
    }
}

/// QSettings оборачивает значения в кавычки и экранирует служебные символы,
/// когда те есть в строке. Разворачиваем ровно то, что может встретиться
/// в `name`/`notes`/`JavaPath`.
fn unescape(value: &str) -> String {
    let value = match (value.starts_with('"'), value.ends_with('"'), value.len()) {
        (true, true, length) if length >= 2 => &value[1..length - 1],
        _ => value,
    };

    if !value.contains('\\') {
        return value.to_string();
    }

    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(symbol) = chars.next() {
        if symbol != '\\' {
            out.push(symbol);
            continue;
        }

        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[General]
ConfigVersion=1.2
ManagedPack=true
iconKey=modrinth_fabulously-optimized
name=Fabulously Optimized 12.0.5
notes=
MaxMemAlloc=12544
OverrideMemory=true
AutomaticJava=true
JavaPath=C:/Users/admin/AppData/Roaming/PrismLauncher/java/java-runtime-delta/bin/javaw.exe

[UI]
mods_Page\Columns=@ByteArray(\0\0\0\xff\0\0)
"#;

    #[test]
    fn general_section_is_read_by_key() {
        let ini = Ini::parse(SAMPLE);
        let general = ini.general();

        assert_eq!(general.string("name"), "Fabulously Optimized 12.0.5");
        assert_eq!(general.string("iconKey"), "modrinth_fabulously-optimized");
        assert_eq!(general.string("notes"), "");
        assert_eq!(general.number("MaxMemAlloc"), Some(12544));
        assert!(general.flag("OverrideMemory"));
        assert!(general.flag("AutomaticJava"));
    }

    #[test]
    fn missing_keys_and_sections_are_empty_rather_than_an_error() {
        let ini = Ini::parse(SAMPLE);

        assert_eq!(ini.general().string("НетТакого"), "");
        assert_eq!(ini.general().number("name"), None);
        assert!(!ini.general().flag("НетТакого"));
        assert_eq!(ini.section("Нет").string("name"), "");
    }

    #[test]
    fn sections_do_not_leak_into_each_other() {
        let ini = Ini::parse(SAMPLE);

        assert!(ini.general().get("mods_Page\\Columns").is_none());
        assert!(ini.section("UI").get("mods_Page\\Columns").is_some());
    }

    #[test]
    fn keys_before_any_section_land_in_general() {
        let ini = Ini::parse("name=Без секции\n[UI]\nx=1");

        assert_eq!(ini.general().string("name"), "Без секции");
    }

    #[test]
    fn quoted_and_escaped_values_are_unwrapped() {
        let ini = Ini::parse("notes=\"первая\\nвторая\"\npath=C:\\\\games\\\\mc");

        assert_eq!(ini.general().string("notes"), "первая\nвторая");
        assert_eq!(ini.general().string("path"), "C:\\games\\mc");
    }

    #[test]
    fn comments_and_junk_lines_are_ignored() {
        let ini = Ini::parse("; комментарий\n# ещё\nмусор без равно\nname=Сборка");

        assert_eq!(ini.general().string("name"), "Сборка");
    }

    #[test]
    fn a_value_may_contain_equals_signs() {
        let ini = Ini::parse("JvmArgs=-Dfoo=bar -Dbaz=qux");

        assert_eq!(ini.general().string("JvmArgs"), "-Dfoo=bar -Dbaz=qux");
    }
}
