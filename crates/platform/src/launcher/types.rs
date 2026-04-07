use std::fmt;
use std::io;

use ssh_core::ssh::config;

#[derive(Debug)]
pub enum LaunchError {
    Io(io::Error),
    Config(config::ConfigError),
    Unsupported(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VncLaunchOutcome {
    pub warning: Option<String>,
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Config(error) => write!(f, "{error}"),
            Self::Unsupported(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for LaunchError {}

impl From<io::Error> for LaunchError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<config::ConfigError> for LaunchError {
    fn from(error: config::ConfigError) -> Self {
        Self::Config(error)
    }
}
