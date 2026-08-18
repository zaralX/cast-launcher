pub mod library;
pub mod mojang;
pub mod texture;

use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::account::{Account, AccountStore, AccountType};
use crate::error::{CommandError, CommandResult};

pub use library::{SkinEntry, SkinLibrary, SkinSource};
pub use texture::SkinVariant;

const ACTIVE: &str = "ACTIVE";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSkin {
    pub id: String,
    pub state: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_key: Option<String>,
    pub variant: SkinVariant,
}

impl ProfileSkin {
    pub fn is_active(&self) -> bool {
        self.state.eq_ignore_ascii_case(ACTIVE)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCape {
    pub id: String,
    pub state: String,
    pub url: String,
    pub alias: String,
}

impl ProfileCape {
    pub fn is_active(&self) -> bool {
        self.state.eq_ignore_ascii_case(ACTIVE)
    }
}

pub fn profile_skins(values: &[Value]) -> Vec<ProfileSkin> {
    values
        .iter()
        .filter_map(|value| {
            Some(ProfileSkin {
                id: text(value, "id")?,
                state: text(value, "state").unwrap_or_else(|| "INACTIVE".into()),
                url: text(value, "url").unwrap_or_default(),
                texture_key: text(value, "textureKey"),
                variant: SkinVariant::from_model(value.get("variant").and_then(Value::as_str)),
            })
        })
        .collect()
}

pub fn profile_capes(values: &[Value]) -> Vec<ProfileCape> {
    values
        .iter()
        .filter_map(|value| {
            let id = text(value, "id")?;

            Some(ProfileCape {
                alias: text(value, "alias").unwrap_or_else(|| id.clone()),
                id,
                state: text(value, "state").unwrap_or_else(|| "INACTIVE".into()),
                url: text(value, "url").unwrap_or_default(),
            })
        })
        .collect()
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|found| !found.is_empty())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapeView {
    pub id: String,
    pub alias: String,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountLook {
    pub uuid: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin_id: Option<String>,
    pub variant: SkinVariant,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cape_id: Option<String>,
    pub capes: Vec<CapeView>,
    pub library: SkinLibrary,
    pub stale: bool,
}

pub async fn look(
    accounts: &AccountStore,
    dir: &Path,
    uuid: &str,
    refresh: bool,
) -> CommandResult<AccountLook> {
    let mut account = accounts.licensed(uuid).await?;
    let mut stale = false;

    if refresh {
        match refresh_profile(accounts, &account).await {
            Ok(updated) => account = updated,
            Err(error) if error.code == "AUTH_EXPIRED" => return Err(error),
            Err(_) => stale = true,
        }
    }

    build_look(dir, &account, stale).await
}

pub async fn apply_skin(
    accounts: &AccountStore,
    dir: &Path,
    uuid: &str,
    id: &str,
) -> CommandResult<AccountLook> {
    let account = accounts.licensed(uuid).await?;
    let token = token_of(&account)?;

    let entry = library::load(dir)
        .await
        .find(id)
        .cloned()
        .ok_or_else(|| CommandError::fs("Набор не найден в библиотеке"))?;

    let bytes = library::read(dir, &entry.texture).await?;
    let profile = mojang::upload_skin(&token, bytes, entry.variant).await?;

    let account = accounts
        .set_textures(uuid, profile.skins, profile.capes)
        .await?;

    build_look(dir, &account, false).await
}

pub async fn reset_skin(
    accounts: &AccountStore,
    dir: &Path,
    uuid: &str,
) -> CommandResult<AccountLook> {
    let account = accounts.licensed(uuid).await?;
    let token = token_of(&account)?;

    mojang::reset_skin(&token).await?;

    let account = refresh_profile(accounts, &account).await?;

    build_look(dir, &account, false).await
}

pub async fn apply_cape(
    accounts: &AccountStore,
    dir: &Path,
    uuid: &str,
    cape_id: Option<&str>,
) -> CommandResult<AccountLook> {
    let account = accounts.licensed(uuid).await?;
    let token = token_of(&account)?;

    let account = match cape_id {
        Some(cape_id) => {
            let profile = mojang::set_cape(&token, cape_id).await?;
            accounts
                .set_textures(uuid, profile.skins, profile.capes)
                .await?
        }
        None => {
            mojang::clear_cape(&token).await?;
            refresh_profile(accounts, &account).await?
        }
    };

    build_look(dir, &account, false).await
}

pub async fn import_player(dir: &Path, name: &str) -> CommandResult<SkinEntry> {
    let player = mojang::player_skin(name).await?;
    let bytes = library::remote_bytes(dir, &player.url).await?;

    library::add(
        dir,
        &player.name,
        &bytes,
        SkinSource::Player,
        Some(player.variant),
    )
    .await
}

async fn build_look(dir: &Path, account: &Account, stale: bool) -> CommandResult<AccountLook> {
    let skins = profile_skins(&account.skins);
    let capes = profile_capes(&account.capes);

    let mut views = Vec::with_capacity(capes.len());
    let mut cape_id = None;

    for cape in &capes {
        if cape.is_active() {
            cape_id = Some(cape.id.clone());
        }

        views.push(CapeView {
            id: cape.id.clone(),
            alias: cape.alias.clone(),
            active: cape.is_active(),
            texture: library::remote_data_url(dir, &cape.url).await.ok(),
        });
    }

    let mut active_texture = None;
    let mut variant = SkinVariant::Classic;

    for skin in &skins {
        if skin.url.is_empty() {
            continue;
        }

        let name = skin
            .texture_key
            .as_deref()
            .map(|key| format!("Профиль {}", key.chars().take(8).collect::<String>()))
            .unwrap_or_else(|| format!("Скин {}", account.name));

        let Ok(bytes) = library::remote_bytes(dir, &skin.url).await else {
            continue;
        };

        let Ok(entry) = library::add(dir, &name, &bytes, SkinSource::Profile, Some(skin.variant)).await
        else {
            continue;
        };

        if skin.is_active() {
            active_texture = Some(entry.texture);
            variant = skin.variant;
        }
    }

    let library = library::load(dir).await;

    let skin_id = active_texture.and_then(|texture| {
        let matching: Vec<_> = library
            .skins
            .iter()
            .filter(|entry| entry.texture == texture)
            .collect();

        matching
            .iter()
            .find(|entry| entry.cape_id == cape_id)
            .or_else(|| matching.first())
            .map(|entry| entry.id.clone())
    });

    Ok(AccountLook {
        uuid: account.uuid.clone().unwrap_or_default(),
        name: account.name.clone(),
        skin_id,
        variant,
        cape_id,
        capes: views,
        library,
        stale,
    })
}

async fn refresh_profile(accounts: &AccountStore, account: &Account) -> CommandResult<Account> {
    let token = token_of(account)?;
    let uuid = account
        .uuid
        .clone()
        .ok_or_else(|| CommandError::no_account("У аккаунта нет идентификатора"))?;

    let profile = crate::account::microsoft::profile(&token).await?;

    accounts.set_textures(&uuid, profile.skins, profile.capes).await
}

fn token_of(account: &Account) -> CommandResult<String> {
    if account.account_type != AccountType::Microsoft {
        return Err(CommandError::no_account(
            "Скин можно менять только у аккаунта Microsoft",
        ));
    }

    account
        .access_token
        .clone()
        .ok_or_else(|| CommandError::auth_expired("Нужно войти в аккаунт заново"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn profile_skins_are_read_leniently() {
        let values = vec![
            json!({
                "id": "one",
                "state": "ACTIVE",
                "url": "https://textures/one",
                "textureKey": "keykeykey",
                "variant": "SLIM"
            }),
            json!({ "id": "two" }),
            json!({ "state": "ACTIVE" }),
        ];

        let skins = profile_skins(&values);

        assert_eq!(skins.len(), 2);
        assert!(skins[0].is_active());
        assert_eq!(skins[0].variant, SkinVariant::Slim);
        assert_eq!(skins[0].texture_key.as_deref(), Some("keykeykey"));

        assert!(!skins[1].is_active());
        assert_eq!(skins[1].variant, SkinVariant::Classic);
        assert_eq!(skins[1].url, "");
    }

    #[test]
    fn capes_fall_back_to_the_id_when_there_is_no_alias() {
        let values = vec![
            json!({ "id": "cape-1", "alias": "Migrator", "state": "ACTIVE" }),
            json!({ "id": "cape-2" }),
        ];

        let capes = profile_capes(&values);

        assert_eq!(capes[0].alias, "Migrator");
        assert!(capes[0].is_active());
        assert_eq!(capes[1].alias, "cape-2");
        assert!(!capes[1].is_active());
    }

    #[test]
    fn offline_accounts_have_no_token_to_change_skins_with() {
        let offline = Account {
            account_type: AccountType::Offline,
            name: "Steve".into(),
            uuid: Some("uuid".into()),
            access_token: Some("token".into()),
            expires_at: None,
            refresh_token: None,
            xbl_hash: None,
            skins: Vec::new(),
            capes: Vec::new(),
        };

        let error = token_of(&offline).unwrap_err();
        assert_eq!(error.code, "NO_ACCOUNT");

        let microsoft = Account {
            account_type: AccountType::Microsoft,
            access_token: None,
            ..offline.clone()
        };

        assert_eq!(token_of(&microsoft).unwrap_err().code, "AUTH_EXPIRED");

        let ready = Account {
            account_type: AccountType::Microsoft,
            ..offline
        };

        assert_eq!(token_of(&ready).unwrap(), "token");
    }
}
