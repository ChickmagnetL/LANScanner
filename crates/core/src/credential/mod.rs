pub mod store;

use store::AppConfig;

const PRESET_USERNAMES: [&str; 6] = ["root", "admin", "pi", "jetson", "ubuntu", "sunrise"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub id: String,
    pub username: String,
    pub password: Option<String>,
    pub is_preset: bool,
    pub can_delete: bool,
}

pub fn preset_usernames() -> &'static [&'static str] {
    &PRESET_USERNAMES
}

pub fn preset_credentials() -> Vec<Credential> {
    PRESET_USERNAMES
        .iter()
        .map(|username| Credential {
            id: credential_id(username, true),
            username: (*username).to_owned(),
            password: None,
            is_preset: true,
            can_delete: true,
        })
        .collect()
}

pub fn load_credentials() -> Result<Vec<Credential>, store::StoreError> {
    store::load_config().map(|config| credentials_from_config(&config))
}

pub fn credentials_from_config(config: &AppConfig) -> Vec<Credential> {
    let mut credentials = preset_credentials()
        .into_iter()
        .filter(|credential| {
            !(credential.can_delete
                && config
                    .credentials
                    .removed_presets
                    .iter()
                    .any(|removed| removed == &credential.username))
        })
        .collect::<Vec<_>>();

    for credential in &mut credentials {
        credential.password = config
            .credentials
            .passwords
            .get(&credential.username)
            .cloned();
    }

    let mut custom_usernames = config.credentials.usernames.clone();
    custom_usernames.sort();
    custom_usernames.dedup();

    credentials.extend(custom_usernames.into_iter().map(|username| Credential {
        id: credential_id(&username, false),
        password: config.credentials.passwords.get(&username).cloned(),
        username,
        is_preset: false,
        can_delete: true,
    }));

    credentials
}

pub fn find_by_username<'a>(
    credentials: &'a [Credential],
    username: &str,
) -> Option<&'a Credential> {
    credentials
        .iter()
        .find(|credential| credential.username == username)
}

pub fn is_preset_username(username: &str) -> bool {
    PRESET_USERNAMES.contains(&username)
}

pub fn is_removable_preset(username: &str) -> bool {
    is_preset_username(username)
}

pub fn credential_id(username: &str, is_preset: bool) -> String {
    let prefix = if is_preset { "preset" } else { "custom" };

    format!("{prefix}:{username}")
}

pub fn username_from_id(id: &str) -> &str {
    id.split_once(':').map_or(id, |(_, username)| username)
}
