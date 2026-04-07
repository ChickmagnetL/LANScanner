use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{is_preset_username, username_from_id};

const CONFIG_DIRECTORY: &str = ".lanscanner";
const CONFIG_FILE: &str = "config.json";
const KNOWN_HOSTS_FILE: &str = "known_hosts";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CredentialConfig {
    pub usernames: Vec<String>,
    pub passwords: BTreeMap<String, String>,
    pub removed_presets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppPaths {
    pub vscode: Option<String>,
    pub mobaxterm: Option<String>,
    pub vncviewer: Option<String>,
    pub rustdesk: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKind {
    Vscode,
    Mobaxterm,
    VncViewer,
    RustDesk,
}

impl ToolKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vscode => "VS Code",
            Self::Mobaxterm => "MobaXterm",
            Self::VncViewer => "VNC Viewer",
            Self::RustDesk => "RustDesk",
        }
    }
}

impl AppPaths {
    pub fn path_for(&self, tool: ToolKind) -> Option<&str> {
        let value = match tool {
            ToolKind::Vscode => self.vscode.as_deref(),
            ToolKind::Mobaxterm => self.mobaxterm.as_deref(),
            ToolKind::VncViewer => self.vncviewer.as_deref(),
            ToolKind::RustDesk => self.rustdesk.as_deref(),
        }?;

        let value = value.trim();
        (!value.is_empty()).then_some(value)
    }

    pub fn path_buf_for(&self, tool: ToolKind) -> Option<PathBuf> {
        self.path_for(tool).map(PathBuf::from)
    }

    pub fn set_path(&mut self, tool: ToolKind, path: Option<String>) {
        let normalized = path.and_then(|value| {
            let value = value.trim();
            (!value.is_empty()).then_some(value.to_owned())
        });

        match tool {
            ToolKind::Vscode => self.vscode = normalized,
            ToolKind::Mobaxterm => self.mobaxterm = normalized,
            ToolKind::VncViewer => self.vncviewer = normalized,
            ToolKind::RustDesk => self.rustdesk = normalized,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppConfig {
    pub credentials: CredentialConfig,
    pub app_paths: AppPaths,
}

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Parse(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Parse(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn load_config() -> Result<AppConfig, StoreError> {
    let path = config_path();

    if !path.exists() {
        let config = AppConfig::default();
        save_config(&config)?;
        return Ok(config);
    }

    let raw = fs::read_to_string(&path)?;

    if raw.trim().is_empty() {
        let config = AppConfig::default();
        save_config(&config)?;
        return Ok(config);
    }

    AppConfig::from_json_str(&raw)
}

pub fn save_config(config: &AppConfig) -> Result<(), StoreError> {
    let directory = ensure_data_dir()?;
    let path = directory.join(CONFIG_FILE);

    fs::write(path, config.to_json_string())?;

    Ok(())
}

pub fn save_app_path(tool: ToolKind, path: Option<&Path>) -> Result<AppConfig, StoreError> {
    let mut config = load_config()?;
    config
        .app_paths
        .set_path(tool, path.map(|value| value.to_string_lossy().into_owned()));
    save_config(&config)?;
    Ok(config)
}

pub fn add_credential(username: &str, password: Option<&str>) -> Result<AppConfig, StoreError> {
    let username = username.trim();

    if username.is_empty() {
        return Err(StoreError::Parse(String::from(
            "credential username cannot be empty",
        )));
    }

    let mut config = load_config()?;
    let password = password.map(str::trim).filter(|value| !value.is_empty());

    if !is_preset_username(username)
        && !config
            .credentials
            .usernames
            .iter()
            .any(|saved| saved == username)
    {
        config.credentials.usernames.push(username.to_owned());
        config.credentials.usernames.sort();
    }

    match password {
        Some(password) => {
            config
                .credentials
                .passwords
                .insert(username.to_owned(), password.to_owned());

            if is_preset_username(username) {
                config
                    .credentials
                    .removed_presets
                    .retain(|removed| removed != username);
            }
        }
        None => {
            config.credentials.passwords.remove(username);
        }
    }

    normalize_removed_presets(&mut config.credentials.removed_presets);
    save_config(&config)?;

    Ok(config)
}

pub fn update_credential_password(username: &str, password: &str) -> Result<AppConfig, StoreError> {
    let username = username.trim();
    let password = password.trim();

    if username.is_empty() {
        return Err(StoreError::Parse(String::from(
            "credential username cannot be empty",
        )));
    }

    if password.is_empty() {
        return Err(StoreError::Parse(String::from(
            "credential password cannot be empty",
        )));
    }

    let mut config = load_config()?;
    let credential_exists = is_preset_username(username)
        || config
            .credentials
            .usernames
            .iter()
            .any(|saved| saved == username);

    if !credential_exists {
        return Err(StoreError::Parse(format!(
            "credential `{username}` does not exist"
        )));
    }

    config
        .credentials
        .passwords
        .insert(username.to_owned(), password.to_owned());

    if is_preset_username(username) {
        config
            .credentials
            .removed_presets
            .retain(|removed| removed != username);
    }

    normalize_removed_presets(&mut config.credentials.removed_presets);
    save_config(&config)?;

    Ok(config)
}

pub fn remove_credential(id: &str) -> Result<AppConfig, StoreError> {
    let username = username_from_id(id);

    if username.is_empty() {
        return Err(StoreError::Parse(String::from(
            "credential id cannot be empty",
        )));
    }

    let mut config = load_config()?;

    if is_preset_username(username) {
        if !config
            .credentials
            .removed_presets
            .iter()
            .any(|removed| removed == username)
        {
            config.credentials.removed_presets.push(username.to_owned());
        }
    } else {
        config
            .credentials
            .usernames
            .retain(|saved| saved != username);
    }

    config.credentials.passwords.remove(username);
    normalize_removed_presets(&mut config.credentials.removed_presets);

    save_config(&config)?;

    Ok(config)
}

pub fn config_path() -> PathBuf {
    data_dir().join(CONFIG_FILE)
}

pub fn known_hosts_path() -> Result<PathBuf, StoreError> {
    Ok(ensure_data_dir()?.join(KNOWN_HOSTS_FILE))
}

pub fn system_ssh_dir() -> Result<PathBuf, StoreError> {
    let directory = home_dir().join(".ssh");
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

pub fn system_ssh_config_path() -> Result<PathBuf, StoreError> {
    Ok(system_ssh_dir()?.join("config"))
}

pub fn system_known_hosts_path() -> Result<PathBuf, StoreError> {
    Ok(system_ssh_dir()?.join(KNOWN_HOSTS_FILE))
}

pub fn ensure_data_dir() -> Result<PathBuf, StoreError> {
    let directory = data_dir();
    fs::create_dir_all(&directory)?;

    Ok(directory)
}

fn data_dir() -> PathBuf {
    home_dir().join(CONFIG_DIRECTORY)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| {
            let drive = std::env::var_os("HOMEDRIVE")?;
            let path = std::env::var_os("HOMEPATH")?;

            Some(Path::new(&drive).join(path))
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

impl AppConfig {
    fn from_json_str(input: &str) -> Result<Self, StoreError> {
        let value = JsonParser::new(input).parse()?;
        let root = value
            .as_object()
            .ok_or_else(|| StoreError::Parse(String::from("config root must be an object")))?;

        let credentials = root
            .get("credentials")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| {
                StoreError::Parse(String::from("config.credentials must be an object"))
            })?;
        let usernames = credentials
            .get("usernames")
            .map(parse_string_array)
            .transpose()?
            .unwrap_or_default();
        let passwords = credentials
            .get("passwords")
            .map(parse_string_map)
            .transpose()?
            .unwrap_or_default();
        let removed_presets = credentials
            .get("removed_presets")
            .map(parse_removed_presets)
            .transpose()?
            .unwrap_or_default();

        let app_paths = root
            .get("app_paths")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| StoreError::Parse(String::from("config.app_paths must be an object")))?;

        Ok(Self {
            credentials: CredentialConfig {
                usernames,
                passwords,
                removed_presets,
            },
            app_paths: AppPaths {
                vscode: parse_nullable_string(app_paths.get("vscode"))?,
                mobaxterm: parse_nullable_string(app_paths.get("mobaxterm"))?,
                vncviewer: parse_nullable_string(app_paths.get("vncviewer"))?,
                rustdesk: parse_nullable_string(app_paths.get("rustdesk"))?,
            },
        })
    }

    fn to_json_string(&self) -> String {
        let usernames = self
            .credentials
            .usernames
            .iter()
            .map(|username| format!("      {}", quoted(username)))
            .collect::<Vec<_>>();
        let passwords = self
            .credentials
            .passwords
            .iter()
            .map(|(username, password)| format!("      {}: {}", quoted(username), quoted(password)))
            .collect::<Vec<_>>();
        let removed_presets = self
            .credentials
            .removed_presets
            .iter()
            .map(|username| format!("      {}", quoted(username)))
            .collect::<Vec<_>>();

        let usernames = if usernames.is_empty() {
            String::from("[]")
        } else {
            format!("[\n{}\n    ]", usernames.join(",\n"))
        };
        let passwords = if passwords.is_empty() {
            String::from("{}")
        } else {
            format!("{{\n{}\n    }}", passwords.join(",\n"))
        };
        let removed_presets = if removed_presets.is_empty() {
            String::from("[]")
        } else {
            format!("[\n{}\n    ]", removed_presets.join(",\n"))
        };

        format!(
            "{{\n  \"credentials\": {{\n    \"usernames\": {usernames},\n    \"passwords\": {passwords},\n    \"removed_presets\": {removed_presets}\n  }},\n  \"app_paths\": {{\n    \"vscode\": {},\n    \"mobaxterm\": {},\n    \"vncviewer\": {},\n    \"rustdesk\": {}\n  }}\n}}\n",
            nullable_string(self.app_paths.vscode.as_deref()),
            nullable_string(self.app_paths.mobaxterm.as_deref()),
            nullable_string(self.app_paths.vncviewer.as_deref()),
            nullable_string(self.app_paths.rustdesk.as_deref()),
        )
    }
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", escape_json(value))
}

fn nullable_string(value: Option<&str>) -> String {
    value.map_or_else(|| String::from("null"), quoted)
}

fn escape_json(value: &str) -> String {
    let mut output = String::new();

    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0C}' => output.push_str("\\f"),
            ch if ch.is_control() => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }

    output
}

fn parse_string_array(value: &JsonValue) -> Result<Vec<String>, StoreError> {
    let Some(values) = value.as_array() else {
        return Err(StoreError::Parse(String::from(
            "config.credentials.usernames must be an array",
        )));
    };

    let mut usernames = Vec::with_capacity(values.len());

    for value in values {
        let Some(username) = value.as_string() else {
            return Err(StoreError::Parse(String::from(
                "config.credentials.usernames entries must be strings",
            )));
        };

        usernames.push(username.to_owned());
    }

    usernames.sort();
    usernames.dedup();

    Ok(usernames)
}

fn parse_string_map(value: &JsonValue) -> Result<BTreeMap<String, String>, StoreError> {
    let Some(values) = value.as_object() else {
        return Err(StoreError::Parse(String::from(
            "config.credentials.passwords must be an object",
        )));
    };

    let mut passwords = BTreeMap::new();

    for (username, value) in values {
        let Some(password) = value.as_string() else {
            return Err(StoreError::Parse(String::from(
                "config.credentials.passwords values must be strings",
            )));
        };

        passwords.insert(username.clone(), password.to_owned());
    }

    Ok(passwords)
}

fn parse_removed_presets(value: &JsonValue) -> Result<Vec<String>, StoreError> {
    let mut usernames = parse_string_array(value)?;
    normalize_removed_presets(&mut usernames);

    Ok(usernames)
}

fn normalize_removed_presets(usernames: &mut Vec<String>) {
    usernames.retain(|username| is_preset_username(username));
    usernames.sort();
    usernames.dedup();
}

fn parse_nullable_string(value: Option<&JsonValue>) -> Result<Option<String>, StoreError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(StoreError::Parse(String::from(
            "config.app_paths values must be a string or null",
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonValue {
    Null,
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(values) => Some(values),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    input: &'a [u8],
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            index: 0,
        }
    }

    fn parse(mut self) -> Result<JsonValue, StoreError> {
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();

        if self.index != self.input.len() {
            return Err(StoreError::Parse(String::from(
                "unexpected trailing content in config",
            )));
        }

        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, StoreError> {
        self.skip_whitespace();

        let Some(byte) = self.peek() else {
            return Err(StoreError::Parse(String::from("unexpected end of config")));
        };

        match byte {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => self.parse_string().map(JsonValue::String),
            b'n' => self.parse_null(),
            _ => Err(StoreError::Parse(format!(
                "unsupported JSON token '{}'",
                byte as char
            ))),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, StoreError> {
        self.consume(b'{')?;
        self.skip_whitespace();

        let mut values = BTreeMap::new();

        if self.peek() == Some(b'}') {
            self.index += 1;
            return Ok(JsonValue::Object(values));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.consume(b':')?;
            let value = self.parse_value()?;
            values.insert(key, value);
            self.skip_whitespace();

            match self.peek() {
                Some(b',') => {
                    self.index += 1;
                }
                Some(b'}') => {
                    self.index += 1;
                    break;
                }
                _ => {
                    return Err(StoreError::Parse(String::from(
                        "expected ',' or '}' while parsing object",
                    )));
                }
            }
        }

        Ok(JsonValue::Object(values))
    }

    fn parse_array(&mut self) -> Result<JsonValue, StoreError> {
        self.consume(b'[')?;
        self.skip_whitespace();

        let mut values = Vec::new();

        if self.peek() == Some(b']') {
            self.index += 1;
            return Ok(JsonValue::Array(values));
        }

        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();

            match self.peek() {
                Some(b',') => {
                    self.index += 1;
                }
                Some(b']') => {
                    self.index += 1;
                    break;
                }
                _ => {
                    return Err(StoreError::Parse(String::from(
                        "expected ',' or ']' while parsing array",
                    )));
                }
            }
        }

        Ok(JsonValue::Array(values))
    }

    fn parse_null(&mut self) -> Result<JsonValue, StoreError> {
        if self.take_exact(b"null") {
            Ok(JsonValue::Null)
        } else {
            Err(StoreError::Parse(String::from("invalid JSON literal")))
        }
    }

    fn parse_string(&mut self) -> Result<String, StoreError> {
        self.consume(b'"')?;

        let mut output = String::new();

        while let Some(byte) = self.next() {
            match byte {
                b'"' => return Ok(output),
                b'\\' => {
                    let Some(escaped) = self.next() else {
                        return Err(StoreError::Parse(String::from(
                            "unterminated escape sequence",
                        )));
                    };

                    match escaped {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{08}'),
                        b'f' => output.push('\u{0C}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let codepoint = self.parse_unicode_escape()?;
                            let Some(ch) = char::from_u32(codepoint) else {
                                return Err(StoreError::Parse(String::from(
                                    "invalid unicode escape in config",
                                )));
                            };
                            output.push(ch);
                        }
                        _ => {
                            return Err(StoreError::Parse(String::from(
                                "unsupported string escape in config",
                            )));
                        }
                    }
                }
                byte => output.push(byte as char),
            }
        }

        Err(StoreError::Parse(String::from("unterminated JSON string")))
    }

    fn parse_unicode_escape(&mut self) -> Result<u32, StoreError> {
        let digits = self.take_bytes(4)?;
        let value = std::str::from_utf8(digits)
            .map_err(|_| StoreError::Parse(String::from("unicode escape must be valid UTF-8")))?
            .to_owned();

        u32::from_str_radix(&value, 16)
            .map_err(|_| StoreError::Parse(String::from("unicode escape must be hexadecimal")))
    }

    fn consume(&mut self, expected: u8) -> Result<(), StoreError> {
        match self.next() {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(StoreError::Parse(format!(
                "expected '{}', found '{}'",
                expected as char, actual as char
            ))),
            None => Err(StoreError::Parse(String::from("unexpected end of config"))),
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn take_exact(&mut self, expected: &[u8]) -> bool {
        if self.input.get(self.index..self.index + expected.len()) == Some(expected) {
            self.index += expected.len();
            true
        } else {
            false
        }
    }

    fn take_bytes(&mut self, count: usize) -> Result<&'a [u8], StoreError> {
        let end = self.index + count;
        let Some(slice) = self.input.get(self.index..end) else {
            return Err(StoreError::Parse(String::from("unexpected end of config")));
        };

        self.index = end;
        Ok(slice)
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.index += 1;
        Some(byte)
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, AppPaths, CredentialConfig};
    use std::collections::BTreeMap;

    #[test]
    fn round_trips_json_config() {
        let config = AppConfig {
            credentials: CredentialConfig {
                usernames: vec![String::from("alice"), String::from("bob")],
                passwords: BTreeMap::from([
                    (String::from("alice"), String::from("secret")),
                    (String::from("root"), String::from("admin123")),
                ]),
                removed_presets: vec![String::from("jetson"), String::from("ubuntu")],
            },
            app_paths: AppPaths {
                vscode: Some(String::from("/opt/code")),
                mobaxterm: None,
                vncviewer: Some(String::from("/opt/vnc")),
                rustdesk: Some(String::from("/opt/rustdesk")),
            },
        };

        let json = config.to_json_string();
        let parsed = AppConfig::from_json_str(&json).expect("config should parse");

        assert_eq!(parsed, config);
    }

    #[test]
    fn parses_sample_shape() {
        let parsed = AppConfig::from_json_str(
            r#"{
  "credentials": {
    "usernames": ["custom_user1"],
    "passwords": {"custom_user1": "pass123"}
  },
  "app_paths": {
    "vscode": null,
    "mobaxterm": null,
    "vncviewer": null,
    "rustdesk": null
  }
}"#,
        )
        .expect("sample config should parse");

        assert_eq!(
            parsed.credentials.usernames,
            vec![String::from("custom_user1")]
        );
        assert_eq!(
            parsed.credentials.passwords.get("custom_user1"),
            Some(&String::from("pass123"))
        );
        assert!(parsed.credentials.removed_presets.is_empty());
        assert_eq!(parsed.app_paths, AppPaths::default());
    }

    #[test]
    fn filters_unknown_removed_presets() {
        let parsed = AppConfig::from_json_str(
            r#"{
  "credentials": {
    "usernames": [],
    "passwords": {},
    "removed_presets": ["pi", "root", "ubuntu", "pi"]
  },
  "app_paths": {
    "vscode": null,
    "mobaxterm": null,
    "vncviewer": null,
    "rustdesk": null
  }
}"#,
        )
        .expect("config should parse");

        assert_eq!(
            parsed.credentials.removed_presets,
            vec![
                String::from("pi"),
                String::from("root"),
                String::from("ubuntu")
            ]
        );
    }
}
