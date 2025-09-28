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
    pub bay: u32,
    #[serde(default)]
    pub state: Option<ToolState>,
}

pub trait BenchTool {
    fn name(&self) -> &str;
    fn bay(&self) -> u32;
    fn set_bay(&mut self, bay: u32);
    fn kind(&self) -> ToolKind;
    fn identifier(&self) -> String;
    fn sway_patterns(&self) -> &'static [&'static str];
    fn browser_config(&self) -> Result<browser::Config>;
    fn terminal_config(&self) -> Result<terminal::Config>;
    fn zed_config(&self) -> Result<zed::Config>;
    fn set_browser_urls(&mut self, urls: Vec<String>);
}

impl BenchTool for Tool {
    fn name(&self) -> &str {
        &self.name
    }

    fn bay(&self) -> u32 {
        self.bay
    }

    fn set_bay(&mut self, bay: u32) {
        self.bay = bay;
    }

    fn kind(&self) -> ToolKind {
        self.kind
    }

    fn identifier(&self) -> String {
        if self.name.trim().is_empty() {
            format!("{}_bay{}", self.kind.as_str(), self.bay)
        } else {
            self.name.clone()
        }
    }

    fn sway_patterns(&self) -> &'static [&'static str] {
        self.kind.sway_patterns()
    }

    fn browser_config(&self) -> Result<browser::Config> {
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

    fn terminal_config(&self) -> Result<terminal::Config> {
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

    fn zed_config(&self) -> Result<zed::Config> {
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

    fn set_browser_urls(&mut self, urls: Vec<String>) {
        if self.kind != ToolKind::Browser {
            return;
        }
        let mut config = match &self.state {
            Some(ToolState::Browser(cfg)) => cfg.clone(),
            _ => browser::Config::default(),
        };
        config.urls = urls;
        self.state = Some(ToolState::Browser(config));
    }
}
