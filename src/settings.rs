use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSettings {
    pub general: GeneralSettings,
    pub privacy: PrivacySettings,
    pub appearance: AppearanceSettings,
    pub search: SearchSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub homepage: String,
    pub new_tab_url: String,
    pub restore_session: bool,
    pub download_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub javascript_enabled: bool,
    pub cookies_enabled: bool,
    pub block_third_party_cookies: bool,
    pub do_not_track: bool,
    pub clear_on_close: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub sidebar_position: SidebarPosition,
    pub show_status_bar: bool,
    pub zoom_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSettings {
    pub engine: SearchEngine,
    pub custom_url: Option<String>,
    pub suggest: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SidebarPosition {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchEngine {
    Brave,
    DuckDuckGo,
    Google,
    Startpage,
    Custom,
}

impl SearchEngine {
    pub fn search_url(&self, query: &str, custom_url: Option<&str>) -> String {
        let encoded = urlencoding::encode(query);
        match self {
            Self::Brave      => format!("https://search.brave.com/search?q={encoded}"),
            Self::DuckDuckGo => format!("https://duckduckgo.com/?q={encoded}"),
            Self::Google     => format!("https://www.google.com/search?q={encoded}"),
            Self::Startpage  => format!("https://www.startpage.com/search?q={encoded}"),
            Self::Custom     => custom_url
                .map(|u| u.replace("%s", &encoded))
                .unwrap_or_else(|| format!("https://search.brave.com/search?q={encoded}")),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Brave      => "Brave Search",
            Self::DuckDuckGo => "DuckDuckGo",
            Self::Google     => "Google",
            Self::Startpage  => "Startpage",
            Self::Custom     => "Custom",
        }
    }
}

// ── Defaults ──────────────────────────────────────────────────────────────────

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            general:    GeneralSettings::default(),
            privacy:    PrivacySettings::default(),
            appearance: AppearanceSettings::default(),
            search:     SearchSettings::default(),
        }
    }
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            homepage:        "https://start.page".into(),
            new_tab_url:     "https://start.page".into(),
            restore_session: true,
            download_path:   dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("~/Downloads")),
        }
    }
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            javascript_enabled:       true,
            cookies_enabled:          true,
            block_third_party_cookies: true,
            do_not_track:             true,
            clear_on_close:           false,
        }
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme:            Theme::System,
            sidebar_position: SidebarPosition::Left,
            show_status_bar:  true,
            zoom_level:       1.0,
        }
    }
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            engine:     SearchEngine::Brave,
            custom_url: None,
            suggest:    true,
        }
    }
}

// ── Persistence ───────────────────────────────────────────────────────────────

impl BrowserSettings {
    pub fn load() -> Self {
        match config_path() {
            Some(path) if path.exists() => {
                fs::read_to_string(&path)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default()
            }
            _ => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = config_path() else { return };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| {
        d.join("cosmic-browser").join("settings.json")
    })
}
