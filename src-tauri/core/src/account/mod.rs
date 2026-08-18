pub mod microsoft;
pub mod oauth;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{read_json_opt, write_json_atomic};

const EXPIRY_MARGIN_SECS: u64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Offline,
    Microsoft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(rename = "type")]
    pub account_type: AccountType,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xbl_hash: Option<String>,
    #[serde(default)]
    pub skins: Vec<Value>,
    #[serde(default)]
    pub capes: Vec<Value>,
}

impl Account {
    pub fn user_type(&self) -> &'static str {
        match self.account_type {
            AccountType::Microsoft => "msa",
            AccountType::Offline => "legacy",
        }
    }

    pub fn is_expired(&self) -> bool {
        match (self.account_type, self.expires_at) {
            (AccountType::Microsoft, Some(expires_at)) => now() + EXPIRY_MARGIN_SECS >= expires_at,
            (AccountType::Microsoft, None) => true,
            (AccountType::Offline, _) => false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountConfig {
    #[serde(default)]
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub selected: Option<usize>,
}

impl AccountConfig {
    pub fn active(&self) -> Option<&Account> {
        let index = self.selected.unwrap_or(0);
        self.accounts.get(index).or_else(|| self.accounts.first())
    }

    fn backfill_offline_uuids(&mut self) {
        for account in &mut self.accounts {
            if account.account_type == AccountType::Offline && account.uuid.is_none() {
                account.uuid = Some(offline_uuid(&account.name));
            }
        }
    }
}

pub struct AccountStore {
    file: RwLock<PathBuf>,
    config: RwLock<AccountConfig>,
}

impl AccountStore {
    pub async fn load(file: PathBuf) -> Self {
        Self {
            config: RwLock::new(read(&file).await),
            file: RwLock::new(file),
        }
    }

    pub async fn relocate(&self, file: PathBuf) {
        let loaded = read(&file).await;

        let mut current = self.file.write().await;
        *self.config.write().await = loaded;
        *current = file;
    }

    pub async fn config(&self) -> AccountConfig {
        self.config.read().await.clone()
    }

    pub async fn select(&self, index: usize) -> CommandResult<AccountConfig> {
        self.mutate(|config| {
            config.selected = Some(index.min(config.accounts.len().saturating_sub(1)));
        })
        .await
    }

    pub async fn remove(&self, uuid: &str) -> CommandResult<AccountConfig> {
        self.mutate(|config| {
            config.accounts.retain(|account| account.uuid.as_deref() != Some(uuid));

            if let Some(selected) = config.selected {
                if selected >= config.accounts.len() {
                    config.selected = config.accounts.len().checked_sub(1);
                }
            }
        })
        .await
    }

    pub async fn add_offline(&self, name: &str) -> CommandResult<AccountConfig> {
        let name = name.trim();

        if name.is_empty() {
            return Err(CommandError::no_account("Укажите никнейм"));
        }

        self.upsert(Account {
            account_type: AccountType::Offline,
            name: name.to_string(),
            uuid: Some(offline_uuid(name)),
            access_token: None,
            expires_at: None,
            refresh_token: None,
            xbl_hash: None,
            skins: Vec::new(),
            capes: Vec::new(),
        })
        .await
    }

    pub async fn upsert(&self, account: Account) -> CommandResult<AccountConfig> {
        self.mutate(|config| {
            let existing = config
                .accounts
                .iter()
                .position(|candidate| candidate.uuid == account.uuid && account.uuid.is_some());

            match existing {
                Some(index) => config.accounts[index] = account,
                None => {
                    config.accounts.push(account);
                    config.selected.get_or_insert(config.accounts.len() - 1);
                }
            }
        })
        .await
    }

    pub async fn active_for_launch(&self) -> CommandResult<Account> {
        let account = self
            .config()
            .await
            .active()
            .cloned()
            .ok_or_else(|| CommandError::no_account("Аккаунт не выбран"))?;

        if !account.is_expired() {
            return Ok(account);
        }

        let uuid = account
            .uuid
            .clone()
            .ok_or_else(|| CommandError::auth_expired("У аккаунта нет идентификатора"))?;

        self.refresh(&uuid).await
    }

    pub async fn find(&self, uuid: &str) -> CommandResult<Account> {
        self.config()
            .await
            .accounts
            .into_iter()
            .find(|account| account.uuid.as_deref() == Some(uuid))
            .ok_or_else(|| CommandError::no_account("Аккаунт не найден"))
    }

    pub async fn licensed(&self, uuid: &str) -> CommandResult<Account> {
        let account = self.find(uuid).await?;

        if account.account_type != AccountType::Microsoft {
            return Err(CommandError::no_account(format!(
                "{} - оффлайн-аккаунт, у него нет профиля Mojang",
                account.name
            )));
        }

        if account.is_expired() {
            return self.refresh(uuid).await;
        }

        Ok(account)
    }
    
    pub async fn set_textures(
        &self,
        uuid: &str,
        skins: Vec<Value>,
        capes: Vec<Value>,
    ) -> CommandResult<Account> {
        let mut updated = None;

        self.mutate(|config| {
            if let Some(account) = config
                .accounts
                .iter_mut()
                .find(|account| account.uuid.as_deref() == Some(uuid))
            {
                account.skins = skins;
                account.capes = capes;
                updated = Some(account.clone());
            }
        })
        .await?;

        updated.ok_or_else(|| CommandError::no_account("Аккаунт не найден"))
    }

    pub async fn refresh(&self, uuid: &str) -> CommandResult<Account> {
        let account = self
            .config()
            .await
            .accounts
            .into_iter()
            .find(|account| account.uuid.as_deref() == Some(uuid))
            .ok_or_else(|| CommandError::no_account("Аккаунт не найден"))?;

        let refresh_token = account.refresh_token.clone().ok_or_else(|| {
            CommandError::auth_expired(format!(
                "Для аккаунта {} нет refresh-токена. Войдите заново.",
                account.name
            ))
        })?;

        let tokens = microsoft::refresh(&refresh_token).await?;
        let refreshed = complete_login(tokens).await?;

        self.upsert(refreshed.clone()).await?;

        Ok(refreshed)
    }

    async fn mutate<F>(&self, apply: F) -> CommandResult<AccountConfig>
    where
        F: FnOnce(&mut AccountConfig),
    {
        let updated = {
            let mut config = self.config.write().await;
            apply(&mut config);
            config.clone()
        };

        let file = self.file.read().await.clone();
        write_json_atomic(&file, &updated).await?;

        Ok(updated)
    }
}

async fn read(file: &Path) -> AccountConfig {
    let mut config: AccountConfig = read_json_opt(file).await.unwrap_or_default();
    config.backfill_offline_uuids();
    config
}

/// Microsoft -> Xbox Live -> XSTS -> Minecraft -> Profile.
pub async fn complete_login(tokens: microsoft::MicrosoftTokens) -> CommandResult<Account> {
    let xbox = microsoft::xbox_live(&tokens.access_token).await?;

    let user_hash = xbox
        .user_hash()
        .ok_or_else(|| CommandError::auth("Xbox Live вернул ответ без идентификатора пользователя"))?
        .to_string();

    let xsts = microsoft::xsts(&xbox.token).await?;
    let minecraft = microsoft::minecraft_login(&user_hash, &xsts.token).await?;
    let profile = microsoft::profile(&minecraft.access_token).await?;

    Ok(Account {
        account_type: AccountType::Microsoft,
        name: profile.name,
        uuid: Some(profile.id),
        access_token: Some(minecraft.access_token),
        expires_at: Some(now() + minecraft.expires_in.unwrap_or(86_400)),
        refresh_token: tokens.refresh_token,
        xbl_hash: Some(user_hash),
        skins: profile.skins,
        capes: profile.capes,
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn offline_uuid(name: &str) -> String {
    let digest = md5(format!("OfflinePlayer:{name}").as_bytes());

    let mut bytes = digest;
    bytes[6] = (bytes[6] & 0x0f) | 0x30; // версия 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // вариант RFC 4122

    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();

    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn md5(input: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    let k: Vec<u32> = (0..64)
        .map(|i| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32)
        .collect();

    let mut message = input.to_vec();
    let bit_length = (input.len() as u64).wrapping_mul(8);

    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_le_bytes());

    let mut state: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];

    for chunk in message.chunks(64) {
        let words: Vec<u32> = chunk
            .chunks(4)
            .map(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
            .collect();

        let [mut a, mut b, mut c, mut d] = state;

        for i in 0..64 {
            let (mut f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };

            f = f
                .wrapping_add(a)
                .wrapping_add(k[i])
                .wrapping_add(words[g]);

            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }

    let mut digest = [0_u8; 16];
    for (index, word) in state.iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }

    digest
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn microsoft_account(expires_at: Option<u64>) -> Account {
        Account {
            account_type: AccountType::Microsoft,
            name: "Steve".into(),
            uuid: Some("uuid".into()),
            access_token: Some("token".into()),
            expires_at,
            refresh_token: Some("refresh".into()),
            xbl_hash: Some("hash".into()),
            skins: Vec::new(),
            capes: Vec::new(),
        }
    }

    #[test]
    fn backfill_gives_legacy_offline_accounts_a_uuid() {
        let mut config: AccountConfig = serde_json::from_value(json!({
            "accounts": [
                {"type": "offline", "name": "aboba"},
                {"type": "offline", "name": "Notch", "uuid": "оставить как есть"},
                {"type": "microsoft", "name": "Steve"}
            ]
        }))
        .unwrap();

        config.backfill_offline_uuids();

        assert_eq!(config.accounts[0].uuid.as_deref(), Some(offline_uuid("aboba").as_str()));
        assert_eq!(config.accounts[1].uuid.as_deref(), Some("оставить как есть"));
        assert_eq!(config.accounts[2].uuid, None);
    }

    #[test]
    fn accounts_json_format_is_unchanged() {
        let account: Account = serde_json::from_value(json!({
            "type": "microsoft",
            "name": "Steve",
            "uuid": "abc",
            "accessToken": "token",
            "expiresAt": 100,
            "refreshToken": "refresh",
            "xblHash": "hash",
            "skins": [],
            "capes": []
        }))
        .unwrap();

        assert_eq!(account.account_type, AccountType::Microsoft);
        assert_eq!(account.xbl_hash.as_deref(), Some("hash"));

        let written = serde_json::to_value(&account).unwrap();
        assert_eq!(written["type"], "microsoft");
        assert_eq!(written["xblHash"], "hash");
    }

    #[test]
    fn expiry_uses_a_safety_margin() {
        assert!(microsoft_account(Some(now() + 10)).is_expired());
        assert!(!microsoft_account(Some(now() + 3600)).is_expired());
        assert!(microsoft_account(None).is_expired());
    }

    #[test]
    fn offline_accounts_never_expire() {
        let offline = Account {
            account_type: AccountType::Offline,
            expires_at: Some(0),
            ..microsoft_account(None)
        };

        assert!(!offline.is_expired());
        assert_eq!(offline.user_type(), "legacy");
    }

    #[test]
    fn md5_matches_reference_vectors() {
        let hex = |input: &[u8]| md5(input).iter().map(|b| format!("{b:02x}")).collect::<String>();

        assert_eq!(hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            hex(b"The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
        assert_eq!(hex(&[b'a'; 56]), "3b0c8ac703f828b04c6c197006d17218");
    }

    #[test]
    fn offline_uuid_matches_vanilla_algorithm() {
        assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
        assert_eq!(offline_uuid("jeb_"), "a762f560-4fce-3236-812a-b80efff0b62b");
        assert_eq!(offline_uuid("Steve"), offline_uuid("Steve"));
        assert_ne!(offline_uuid("Steve"), offline_uuid("Alex"));
    }

    #[test]
    fn active_falls_back_to_first_account() {
        let config = AccountConfig {
            accounts: vec![microsoft_account(None)],
            selected: Some(5),
        };

        assert_eq!(config.active().unwrap().name, "Steve");
        assert!(AccountConfig::default().active().is_none());
    }
}
