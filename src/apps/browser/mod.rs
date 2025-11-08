use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub urls: Vec<String>,
}

pub fn launch(config: &Config, debug_port: u16) -> Result<()> {
    let mut cmd = Command::new("chromium");
    cmd.arg("--new-window");
    cmd.arg(format!("--remote-debugging-port={}", debug_port));
    // tmp
    cmd.arg(format!("--user-data-dir=/tmp/chromium-{}", debug_port));
    for url in &config.urls {
        cmd.arg(url);
    }

    // Redirect stdout and stderr to null to avoid cluttering the terminal
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let _ = cmd.spawn()?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct TargetDescriptor {
    #[serde(default)]
    url: String,
    #[serde(rename = "type")]
    #[serde(default)]
    target_type: String,
}

/// Fetch a list of active tab URLs from the Chromium DevTools endpoint.
pub fn list_tabs(port: u16) -> Result<Vec<String>> {
    let endpoint = format!("http://127.0.0.1:{}/json/list", port);
    let response = ureq::get(&endpoint)
        .timeout(std::time::Duration::from_millis(800))
        .call()
        .with_context(|| format!("failed to reach Chromium DevTools endpoint at {endpoint}"))?;

    let targets: Vec<TargetDescriptor> = response
        .into_json()
        .context("failed to parse DevTools tab JSON")?;

    let mut urls = Vec::new();
    for target in targets {
        if target.target_type == "page" && !target.url.is_empty() {
            urls.push(target.url);
        }
    }
    Ok(urls)
}
