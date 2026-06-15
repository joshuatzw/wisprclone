use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SttProvider {
    Openai,
    Groq,
}

impl Default for SttProvider {
    fn default() -> Self {
        SttProvider::Openai
    }
}

impl SttProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            SttProvider::Openai => "openai",
            SttProvider::Groq => "groq",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyCombo {
    CtrlWin,
    RightAlt,
    CtrlShift,
    CtrlAlt,
}

impl Default for HotkeyCombo {
    fn default() -> Self {
        HotkeyCombo::CtrlWin
    }
}

impl HotkeyCombo {
    pub fn to_u8(&self) -> u8 {
        match self {
            HotkeyCombo::CtrlWin => 0,
            HotkeyCombo::RightAlt => 1,
            HotkeyCombo::CtrlShift => 2,
            HotkeyCombo::CtrlAlt => 3,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HotkeyCombo::CtrlWin => "ctrl_win",
            HotkeyCombo::RightAlt => "right_alt",
            HotkeyCombo::CtrlShift => "ctrl_shift",
            HotkeyCombo::CtrlAlt => "ctrl_alt",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    #[serde(default = "default_true")]
    pub cleanup_enabled: bool,
    #[serde(default)]
    pub stt_provider: SttProvider,
    #[serde(default = "default_lang")]
    pub language: String,
    #[serde(default)]
    pub hotkey: HotkeyCombo,
    #[serde(default = "default_true")]
    pub context_awareness_enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_lang() -> String {
    "en".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            cleanup_enabled: true,
            stt_provider: SttProvider::Openai,
            language: "en".to_string(),
            hotkey: HotkeyCombo::CtrlWin,
            context_awareness_enabled: true,
        }
    }
}

pub fn load(dir: &Path) -> AppConfig {
    let path = dir.join("config.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, config: &AppConfig) {
    std::fs::create_dir_all(dir).ok();
    let path = dir.join("config.json");
    if let Ok(json) = serde_json::to_string_pretty(config) {
        std::fs::write(path, json).ok();
    }
}
