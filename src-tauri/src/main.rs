// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

const VERSION_MANIFEST_LINK: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

const ASSETS_LINK: &str =
    "https://resources.download.minecraft.net/%A/%B";

const FABRIC_LOADERS_BY_GAME_VERSION_LINK: &str =
    "https://meta.fabricmc.net/v2/versions/loader/%A";

const FABRIC_LOADER_LINK: &str =
    "https://meta.fabricmc.net/v2/versions/loader/%A/%B/profile/json";

fn main() {
    cast_launcher_lib::run()
}
