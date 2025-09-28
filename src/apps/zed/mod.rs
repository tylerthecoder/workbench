use std::process::Command;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub path: Option<String>,
}

pub fn launch(config: &Config) -> Result<()> {
    let mut cmd = Command::new("zed");
    if let Some(path) = config.path.as_ref() {
        cmd.arg(expand_tilde(path));
    }
    let _ = cmd.spawn()?;
    Ok(())
}

fn expand_tilde(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, stripped);
        }
    }
    path.to_string()
}
