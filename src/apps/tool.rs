use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::{browser, terminal, zed};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ToolKind {
    Browser,
    Terminal,
    Zed,
}

impl ToolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolKind::Browser => "browser",
            ToolKind::Terminal => "terminal",
            ToolKind::Zed => "zed",
        }
    }

    pub fn sway_patterns(&self) -> &'static [&'static str] {
        match self {
            ToolKind::Browser => &[
                "chromium",
                "Chromium",
                "chromium-browser",
                "Chromium-browser",
            ],
            ToolKind::Terminal => &["kitty", "Kitty"],
            ToolKind::Zed => &["zed", "Zed", "dev.zed.Zed"],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolState {
    Browser(browser::Config),
    Terminal(terminal::Config),
    Zed(zed::Config),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub kind: ToolKind,
    pub bay: String,
    #[serde(default)]
    pub state: Option<ToolState>,
}

impl Tool {
    pub fn kind(&self) -> ToolKind {
        self.kind
    }

    pub fn sway_patterns(&self) -> &'static [&'static str] {
        self.kind.sway_patterns()
    }

    pub fn browser_config(&self) -> Result<browser::Config> {
        match (&self.kind, &self.state) {
            (ToolKind::Browser, Some(ToolState::Browser(cfg))) => Ok(cfg.clone()),
            (ToolKind::Browser, None) => Ok(browser::Config::default()),
            (ToolKind::Browser, Some(other)) => Err(anyhow!(
                "invalid state {:?} for browser tool {}",
                other,
                self.identifier()
            )),
            _ => Err(anyhow!(
                "tool {} is not a browser (kind={})",
                self.identifier(),
                self.kind.as_str()
            )),
        }
    }

    pub fn terminal_config(&self) -> Result<terminal::Config> {
        match (&self.kind, &self.state) {
            (ToolKind::Terminal, Some(ToolState::Terminal(cfg))) => Ok(cfg.clone()),
            (ToolKind::Terminal, None) => Ok(terminal::Config::default()),
            (ToolKind::Terminal, Some(other)) => Err(anyhow!(
                "invalid state {:?} for terminal tool {}",
                other,
                self.identifier()
            )),
            _ => Err(anyhow!(
                "tool {} is not a terminal (kind={})",
                self.identifier(),
                self.kind.as_str()
            )),
        }
    }

    pub fn zed_config(&self) -> Result<zed::Config> {
        match (&self.kind, &self.state) {
            (ToolKind::Zed, Some(ToolState::Zed(cfg))) => Ok(cfg.clone()),
            (ToolKind::Zed, None) => Ok(zed::Config::default()),
            (ToolKind::Zed, Some(other)) => Err(anyhow!(
                "invalid state {:?} for zed tool {}",
                other,
                self.identifier()
            )),
            _ => Err(anyhow!(
                "tool {} is not a zed tool (kind={})",
                self.identifier(),
                self.kind.as_str()
            )),
        }
    }

    pub fn identifier(&self) -> String {
        if self.name.trim().is_empty() {
            format!("{}_{}", self.kind.as_str(), self.bay)
        } else {
            self.name.clone()
        }
    }
}
