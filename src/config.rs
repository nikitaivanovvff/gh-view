use crate::github::DEFAULT_GH_COMMAND_TIMEOUT_SECONDS;
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::path::PathBuf;
use toml_edit::{DocumentMut, Item, Table, value};

const DEFAULT_DASHBOARD_PRS_PER_REPO_PAGE: usize = 3;
const DEFAULT_THEME: &str = "default";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub gh_timeout_seconds: u64,
    pub nerd_fonts: bool,
    pub ui: UiConfig,
    pub dashboard: DashboardConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiConfig {
    pub theme: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DashboardConfig {
    pub prs_per_repo_page: usize,
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
        Ok(Self::from_file(file))
    }

    fn from_file(file: ConfigFile) -> Self {
        let dashboard = file.dashboard.unwrap_or_default();
        let ui = file.ui.unwrap_or_default();
        Self {
            gh_timeout_seconds: file
                .gh_timeout_seconds
                .unwrap_or(DEFAULT_GH_COMMAND_TIMEOUT_SECONDS),
            nerd_fonts: file.nerd_fonts.unwrap_or(false),
            ui: UiConfig {
                theme: ui.theme.unwrap_or_else(|| DEFAULT_THEME.to_owned()),
            },
            dashboard: DashboardConfig {
                prs_per_repo_page: dashboard
                    .prs_per_repo_page
                    .filter(|page_size| *page_size > 0)
                    .unwrap_or(DEFAULT_DASHBOARD_PRS_PER_REPO_PAGE),
            },
        }
    }

    pub fn save_theme(&mut self, theme: &str) -> Result<()> {
        let path = config_path().ok_or_else(|| {
            anyhow!("cannot save theme because no config directory could be determined")
        })?;
        save_theme_to_path(&path, theme)?;
        self.ui.theme = theme.to_owned();
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gh_timeout_seconds: DEFAULT_GH_COMMAND_TIMEOUT_SECONDS,
            nerd_fonts: false,
            ui: UiConfig::default(),
            dashboard: DashboardConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_owned(),
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            prs_per_repo_page: DEFAULT_DASHBOARD_PRS_PER_REPO_PAGE,
        }
    }
}

#[derive(Deserialize)]
struct ConfigFile {
    gh_timeout_seconds: Option<u64>,
    nerd_fonts: Option<bool>,
    ui: Option<UiConfigFile>,
    dashboard: Option<DashboardConfigFile>,
}

#[derive(Default, Deserialize)]
struct UiConfigFile {
    theme: Option<String>,
}

#[derive(Default, Deserialize)]
struct DashboardConfigFile {
    prs_per_repo_page: Option<usize>,
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

fn save_theme_to_path(path: &std::path::Path, theme: &str) -> Result<()> {
    let contents = if path.exists() {
        std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?
    } else {
        String::new()
    };
    let mut document = if contents.trim().is_empty() {
        DocumentMut::new()
    } else {
        contents
            .parse::<DocumentMut>()
            .with_context(|| format!("failed to parse config file {}", path.display()))?
    };

    if !document.contains_key("ui") {
        document["ui"] = Item::Table(Table::new());
    }
    document["ui"]["theme"] = value(theme);

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }
    std::fs::write(path, document.to_string())
        .with_context(|| format!("failed to save theme to config file {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn default_timeout_is_thirty_seconds() {
        assert_eq!(Config::default().gh_timeout_seconds, 30);
    }

    #[test]
    fn nerd_fonts_default_to_disabled() {
        assert!(!Config::default().nerd_fonts);
    }

    #[test]
    fn ui_theme_defaults_to_default() {
        assert_eq!(Config::default().ui.theme, "default");
    }

    #[test]
    fn dashboard_pr_page_size_defaults_to_three() {
        assert_eq!(Config::default().dashboard.prs_per_repo_page, 3);
    }

    #[test]
    fn parses_dashboard_pr_page_size_from_config_file() {
        let config =
            Config::from_file(toml::from_str("[dashboard]\nprs_per_repo_page = 4").unwrap());

        assert_eq!(config.dashboard.prs_per_repo_page, 4);
    }

    #[test]
    fn parses_ui_theme_from_config_file() {
        let config = Config::from_file(toml::from_str("[ui]\ntheme = 'catppuccin-mocha'").unwrap());

        assert_eq!(config.ui.theme, "catppuccin-mocha");
    }

    #[test]
    fn ignores_zero_dashboard_pr_page_size() {
        let config =
            Config::from_file(toml::from_str("[dashboard]\nprs_per_repo_page = 0").unwrap());

        assert_eq!(config.dashboard.prs_per_repo_page, 3);
    }

    #[test]
    fn saves_theme_without_rewriting_unrelated_config() {
        let path = temp_config_path();
        std::fs::write(
            &path,
            "# keep this comment\nnerd_fonts = true\n\n[ui]\ntheme = \"default\"\n\n[dashboard]\nprs_per_repo_page = 5\n",
        )
        .unwrap();

        save_theme_to_path(&path, "github-light").unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("# keep this comment"));
        assert!(saved.contains("nerd_fonts = true"));
        assert!(saved.contains("theme = \"github-light\""));
        assert!(saved.contains("prs_per_repo_page = 5"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn creates_config_and_ui_table_when_missing() {
        let path = temp_config_path();

        save_theme_to_path(&path, "solarized-light").unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, "[ui]\ntheme = \"solarized-light\"\n");
        std::fs::remove_file(path).unwrap();
    }

    fn temp_config_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "gh-view-config-{}-{}.toml",
            std::process::id(),
            TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
