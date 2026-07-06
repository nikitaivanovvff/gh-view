use crate::github::DEFAULT_GH_COMMAND_TIMEOUT_SECONDS;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub gh_timeout_seconds: u64,
    pub nerd_fonts: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let Some(path) = config_path() else {
            return Ok(Self::default());
        };
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        let file: ConfigFile = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file {}", path.display()))?;
        Ok(Self {
            gh_timeout_seconds: file
                .gh_timeout_seconds
                .unwrap_or(DEFAULT_GH_COMMAND_TIMEOUT_SECONDS),
            nerd_fonts: file.nerd_fonts.unwrap_or(false),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gh_timeout_seconds: DEFAULT_GH_COMMAND_TIMEOUT_SECONDS,
            nerd_fonts: false,
        }
    }
}

#[derive(Deserialize)]
struct ConfigFile {
    gh_timeout_seconds: Option<u64>,
    nerd_fonts: Option<bool>,
}

fn config_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GH_VIEW_CONFIG") {
        return Some(PathBuf::from(path));
    }
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(path).join("gh-view/config.toml"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/gh-view/config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_timeout_is_thirty_seconds() {
        assert_eq!(Config::default().gh_timeout_seconds, 30);
    }

    #[test]
    fn nerd_fonts_default_to_disabled() {
        assert!(!Config::default().nerd_fonts);
    }
}
