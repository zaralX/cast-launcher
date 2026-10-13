use serde::Deserialize;

use crate::assets;
use crate::config::DiscordConfig;
use crate::instance::Instance;
use crate::launch::game::{GameStatus, RunningGame};

pub const APP_NAME: &str = "Cast Launcher";

pub const LOGO: &str = "logo";

pub const SITE: &str = "https://cast.zaralx.ru";

const MIN_TEXT: usize = 2;

const MAX_TEXT: usize = 128;

const MAX_BUTTON: usize = 32;

const MAX_IMAGE: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PresenceText {
    pub menu: String,
    pub playing: String,
    pub button: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playing {
    pub name: String,
    pub version: String,
    pub started_at: u64,
    pub icon_url: Option<String>,
}

impl Playing {
    pub fn new(instance: &Instance, started_at: u64, icon_url: Option<String>) -> Self {
        Self {
            name: instance.name.clone(),
            version: format!("{} {}", instance.loader.label(), instance.minecraft_version),
            started_at,
            icon_url,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    pub details: Option<String>,
    pub state: Option<String>,
    pub large_image: String,
    pub large_text: String,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
    pub started_at: Option<u64>,
    pub button: Option<String>,
}

pub fn latest(running: &[RunningGame]) -> Option<&RunningGame> {
    running
        .iter()
        .filter(|game| matches!(game.status, GameStatus::Starting | GameStatus::Running))
        .max_by_key(|game| game.started_at)
}

pub fn build(
    config: DiscordConfig,
    text: &PresenceText,
    game: Option<&Playing>,
) -> Option<Presence> {
    if !config.enabled {
        return None;
    }

    let mut presence = Presence {
        details: fit(&text.menu, MAX_TEXT),
        state: None,
        large_image: LOGO.into(),
        large_text: APP_NAME.into(),
        small_image: None,
        small_text: None,
        started_at: None,
        button: fit(&text.button, MAX_BUTTON),
    };

    let Some(game) = game else {
        return Some(presence);
    };

    presence.details = fit(&text.playing, MAX_TEXT);
    presence.started_at = Some(game.started_at);

    if !config.show_instance {
        return Some(presence);
    }

    presence.state = fit(&game.version, MAX_TEXT);

    if let Some(name) = fit(&game.name, MAX_TEXT) {
        if let Some(url) = game.icon_url.clone().filter(|url| url.len() <= MAX_IMAGE) {
            presence.large_image = url;
            presence.large_text = name.clone();
            presence.small_image = Some(LOGO.into());
            presence.small_text = Some(APP_NAME.into());
        }

        presence.details = Some(name);
    }

    Some(presence)
}

pub fn icon_url(icon: &str, remembered: Option<String>) -> Option<String> {
    remembered.or_else(|| {
        assets::item_of_icon_file(icon).and_then(|item| assets::item_icon_url(item).ok())
    })
}

fn fit(text: &str, max: usize) -> Option<String> {
    let text = text.trim();

    if text.encode_utf16().count() < MIN_TEXT {
        return None;
    }

    let mut used = 0;

    Some(
        text.chars()
            .take_while(|symbol| {
                used += symbol.len_utf16();
                used <= max
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text() -> PresenceText {
        PresenceText {
            menu: "В лаунчере".into(),
            playing: "В игре".into(),
            button: "Скачать лаунчер".into(),
        }
    }

    fn config(enabled: bool, show_instance: bool) -> DiscordConfig {
        DiscordConfig {
            enabled,
            show_instance,
        }
    }

    fn playing(icon_url: Option<&str>) -> Playing {
        Playing {
            name: "TFG Modern".into(),
            version: "Forge 1.20.1".into(),
            started_at: 7,
            icon_url: icon_url.map(str::to_string),
        }
    }

    fn game(run_id: &str, started_at: u64, status: GameStatus) -> RunningGame {
        RunningGame {
            run_id: run_id.into(),
            instance_id: run_id.into(),
            instance_name: run_id.into(),
            pid: None,
            started_at,
            status,
        }
    }

    #[test]
    fn nothing_is_shown_when_the_status_is_off() {
        let off = config(false, true);

        assert_eq!(build(off, &text(), None), None);
        assert_eq!(build(off, &text(), Some(&playing(None))), None);
    }

    #[test]
    fn the_launcher_alone_shows_the_menu_line_without_a_timer() {
        let presence = build(config(true, true), &text(), None).unwrap();

        assert_eq!(presence.details.as_deref(), Some("В лаунчере"));
        assert_eq!(presence.state, None);
        assert_eq!(presence.large_image, LOGO);
        assert_eq!(presence.started_at, None);
        assert_eq!(presence.button.as_deref(), Some("Скачать лаунчер"));
    }

    #[test]
    fn a_hidden_instance_shows_only_that_the_game_runs() {
        let presence = build(
            config(true, false),
            &text(),
            Some(&playing(Some("https://cdn/a.png"))),
        )
        .unwrap();

        assert_eq!(presence.details.as_deref(), Some("В игре"));
        assert_eq!(presence.state, None);
        assert_eq!(presence.large_image, LOGO);
        assert_eq!(presence.started_at, Some(7));
    }

    #[test]
    fn a_shown_instance_carries_its_name_version_and_icon() {
        let presence = build(
            config(true, true),
            &text(),
            Some(&playing(Some("https://cdn/a.png"))),
        )
        .unwrap();

        assert_eq!(presence.details.as_deref(), Some("TFG Modern"));
        assert_eq!(presence.state.as_deref(), Some("Forge 1.20.1"));
        assert_eq!(presence.large_image, "https://cdn/a.png");
        assert_eq!(presence.large_text, "TFG Modern");
        assert_eq!(presence.small_image.as_deref(), Some(LOGO));
        assert_eq!(presence.started_at, Some(7));
        assert_eq!(presence.button.as_deref(), Some("Скачать лаунчер"));
    }

    #[test]
    fn a_missing_or_too_long_icon_link_shows_the_logo() {
        let long = format!("https://cdn/{}.png", "a".repeat(MAX_IMAGE));

        for icon in [None, Some(long.as_str())] {
            let presence = build(config(true, true), &text(), Some(&playing(icon))).unwrap();

            assert_eq!(presence.large_image, LOGO);
            assert_eq!(presence.small_image, None);
        }
    }

    #[test]
    fn a_one_letter_name_falls_back_to_the_playing_line() {
        let mut game = playing(Some("https://cdn/a.png"));
        game.name = " Я ".into();

        let presence = build(config(true, true), &text(), Some(&game)).unwrap();

        assert_eq!(presence.details.as_deref(), Some("В игре"));
        assert_eq!(presence.large_image, LOGO);
    }

    #[test]
    fn texts_are_cut_by_utf16_length() {
        assert_eq!(
            fit(&"Сборка".repeat(40), MAX_TEXT).unwrap().chars().count(),
            MAX_TEXT
        );
        assert_eq!(
            fit(&"🎮".repeat(100), MAX_TEXT).unwrap().chars().count(),
            MAX_TEXT / 2
        );
        assert_eq!(
            fit(&"Скачать".repeat(10), MAX_BUTTON)
                .unwrap()
                .chars()
                .count(),
            MAX_BUTTON
        );
    }

    #[test]
    fn the_version_line_names_the_loader() {
        let instance: Instance = serde_json::from_value(serde_json::json!({
            "id": "a",
            "name": "Ванилла",
            "minecraftVersion": "1.21.1",
            "type": "vanilla"
        }))
        .unwrap();

        assert_eq!(Playing::new(&instance, 5, None).version, "Vanilla 1.21.1");
    }

    #[test]
    fn the_latest_running_game_wins() {
        let running = [
            game("old", 10, GameStatus::Running),
            game("new", 30, GameStatus::Running),
            game("crashed", 40, GameStatus::Crashed),
        ];

        assert_eq!(latest(&running).unwrap().run_id, "new");
        assert!(latest(&[game("gone", 1, GameStatus::Exited)]).is_none());
    }

    #[test]
    fn a_remembered_link_wins_and_item_icons_need_none() {
        assert_eq!(
            icon_url("modrinth-a.png", Some("https://cdn/a.png".into())).as_deref(),
            Some("https://cdn/a.png")
        );
        assert_eq!(
            icon_url(&assets::item_icon_file("diamond"), None),
            assets::item_icon_url("diamond").ok()
        );
        assert_eq!(icon_url("mc-server.png", None), None);
        assert_eq!(icon_url("Моя-иконка.png", None), None);
    }
}
