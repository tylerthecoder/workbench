use std::process::{Command, Stdio};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub cwd: Option<String>,
    #[serde(default)]
    pub command: Vec<String>,
}

pub fn launch(config: &Config) -> Result<()> {
    let mut cmd = Command::new("kitty");

    if let Some(cwd) = config.cwd.as_ref() {
        cmd.current_dir(expand_tilde(cwd));
    }

    if !config.command.is_empty() {
        for part in &config.command {
            cmd.arg(part);
        }
    }

    // Redirect stdout and stderr to null to avoid cluttering the terminal
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

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
