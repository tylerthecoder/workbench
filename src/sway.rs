use serde_json::Value;
use std::process::Command;
use std::time::{Duration, Instant};

fn run_sway<I, S>(args: I) -> anyhow::Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let output = Command::new("swaymsg")
        .args(args.into_iter().map(|s| s.as_ref().to_string()))
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "swaymsg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn get_tree() -> anyhow::Result<Value> {
    let out = run_sway(["-t", "get_tree"])?;
    let v: Value = serde_json::from_str(&out)?;
    Ok(v)
}

pub fn ensure_workspace_visible(name: &str) -> anyhow::Result<()> {
    let _ = run_sway(["workspace", name])?;
    Ok(())
}

pub fn move_container_to_workspace(container_id: &str, workspace: &str) -> anyhow::Result<()> {
    let selector = format!("[con_id=\"{}\"]", container_id);
    let _ = run_sway([
        selector.as_str(),
        "move",
        "container",
        "to",
        "workspace",
        workspace,
    ])?;
    Ok(())
}

pub fn move_container_to_scratchpad(container_id: &str) -> anyhow::Result<()> {
    let selector = format!("[con_id=\"{}\"]", container_id);
    let _ = run_sway([selector.as_str(), "move", "container", "to", "scratchpad"])?;
    Ok(())
}

pub fn container_exists(container_id: &str) -> anyhow::Result<bool> {
    let tree = get_tree()?;
    Ok(container_in_tree(&tree, container_id))
}

fn container_in_tree(node: &Value, target_id: &str) -> bool {
    if let Some(id) = node.get("id").and_then(|x| x.as_i64()) {
        if id.to_string() == target_id {
            return true;
        }
    }
    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        if children
            .iter()
            .any(|child| container_in_tree(child, target_id))
        {
            return true;
        }
    }
    if let Some(children) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        if children
            .iter()
            .any(|child| container_in_tree(child, target_id))
        {
            return true;
        }
    }
    false
}

fn collect_ids_from_tree(v: &Value, patterns: &[&str], out: &mut Vec<String>) {
    if let Some(app_id) = v.get("app_id").and_then(|x| x.as_str()) {
        if patterns.iter().any(|p| app_id.eq_ignore_ascii_case(p)) {
            if let Some(id) = v.get("id").and_then(|x| x.as_i64()) {
                out.push(id.to_string());
            }
        }
    }
    if let Some(cls) = v
        .get("window_properties")
        .and_then(|wp| wp.get("class"))
        .and_then(|x| x.as_str())
    {
        if patterns.iter().any(|p| cls.eq_ignore_ascii_case(p)) {
            if let Some(id) = v.get("id").and_then(|x| x.as_i64()) {
                out.push(id.to_string());
            }
        }
    }

    if let Some(nodes) = v.get("nodes").and_then(|x| x.as_array()) {
        for n in nodes {
            collect_ids_from_tree(n, patterns, out);
        }
    }
    if let Some(fnodes) = v.get("floating_nodes").and_then(|x| x.as_array()) {
        for n in fnodes {
            collect_ids_from_tree(n, patterns, out);
        }
    }
}

pub fn matching_container_ids(patterns: &[&str]) -> anyhow::Result<Vec<String>> {
    let tree = get_tree()?;
    let mut ids = vec![];
    collect_ids_from_tree(&tree, patterns, &mut ids);
    Ok(ids)
}

pub fn wait_for_new_container(
    patterns: &[&str],
    before: &[String],
    timeout: Duration,
) -> anyhow::Result<String> {
    let start = Instant::now();
    loop {
        let after = matching_container_ids(patterns)?;
        for id in &after {
            if !before.contains(id) {
                return Ok(id.clone());
            }
        }
        if start.elapsed() > timeout {
            anyhow::bail!(
                "Timed out waiting for new container for patterns: {:?}",
                patterns
            );
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: String,
    pub workspace: Option<String>,
}

fn collect_windows(v: &Value, current_ws: &mut Option<String>, out: &mut Vec<WindowInfo>) {
    if let Some(t) = v.get("type").and_then(|x| x.as_str()) {
        if t == "workspace" {
            *current_ws = v
                .get("name")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());
        }
    }
    let id = v.get("id").and_then(|x| x.as_i64()).map(|x| x.to_string());
    let app_id = v
        .get("app_id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let class = v
        .get("window_properties")
        .and_then(|wp| wp.get("class"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    if let Some(id) = id {
        // Heuristic: consider a node a window if it has title and (app_id or class) or has window field
        let title = v
            .get("name")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let is_window =
            v.get("window").is_some() || title.is_some() && (app_id.is_some() || class.is_some());
        if is_window {
            out.push(WindowInfo {
                id,
                workspace: current_ws.clone(),
            });
        }
    }
    if let Some(nodes) = v.get("nodes").and_then(|x| x.as_array()) {
        for n in nodes {
            let mut ws = current_ws.clone();
            collect_windows(n, &mut ws, out);
        }
    }
    if let Some(fnodes) = v.get("floating_nodes").and_then(|x| x.as_array()) {
        for n in fnodes {
            let mut ws = current_ws.clone();
            collect_windows(n, &mut ws, out);
        }
    }
}

pub fn current_windows() -> anyhow::Result<Vec<WindowInfo>> {
    let tree = get_tree()?;
    let mut windows = vec![];
    let mut ws = None;
    collect_windows(&tree, &mut ws, &mut windows);
    Ok(windows)
}
