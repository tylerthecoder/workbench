use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};

use crate::apps::{self, Tool, ToolKind, ToolState};
use crate::model::ToolDefinition;
use crate::storage;
use crate::sway;

/// Check if a tool window exists and return its window ID if valid
pub fn tool_window_exists(tool_name: &str) -> Result<Option<String>> {
    let stored = storage::read_assembled_tool(tool_name)?;

    if let Some(tool) = stored {
        // Check if the window ID still exists in sway
        if sway::container_exists(&tool.window_id)? {
            return Ok(Some(tool.window_id));
        }
    }

    Ok(None)
}

/// Generate a consistent debug port for a browser tool
pub fn browser_debug_port(tool_name: &str) -> u16 {
    let mut hasher = DefaultHasher::new();
    tool_name.hash(&mut hasher);
    let hash = hasher.finish();

    // Port range: 9222-10222
    9222 + (hash % 1000) as u16
}

/// Assemble a tool: ensure it has a running window
/// Returns (window_id, was_assembled_now)
pub fn assemble_tool(tool_name: &str, bay: &str) -> Result<(String, bool)> {
    // First check if we have a tracked window that still exists
    if let Some(window_id) = tool_window_exists(tool_name)? {
        println!(
            "  ✓ {} - already assembled (window {})",
            tool_name, window_id
        );
        return Ok((window_id, false));
    }

    // Load the tool definition
    let definition =
        storage::read_tool(tool_name).with_context(|| format!("tool '{}' not found", tool_name))?;

    // Assemble the tool by starting its process
    println!("  → {} - assembling now...", tool_name);

    let tool = Tool {
        name: definition.name.clone(),
        kind: definition.kind,
        bay: bay.to_string(),
        state: definition.state.clone(),
    };

    let patterns = tool.sway_patterns();
    let before = sway::matching_container_ids(patterns)?;

    match tool.kind() {
        ToolKind::Browser => {
            let port = browser_debug_port(&tool.name);
            let config = tool.browser_config()?;
            apps::browser::launch(&config, port)?;
        }
        ToolKind::Terminal => {
            let config = tool.terminal_config()?;
            apps::terminal::launch(&config)?;
        }
        ToolKind::Zed => {
            let config = tool.zed_config()?;
            apps::zed::launch(&config)?;
        }
    }

    let window_id = sway::wait_for_new_container(patterns, &before, Duration::from_secs(15))?;
    sway::move_container_to_workspace(&window_id, bay)?;

    // Save the window ID
    storage::write_assembled_tool(
        tool_name,
        &crate::model::AssembledTool {
            window_id: window_id.clone(),
        },
    )?;

    Ok((window_id, true))
}

/// Fetch live state from a running tool
fn fetch_live_state(tool: &ToolDefinition) -> Result<Option<ToolState>> {
    match tool.kind {
        ToolKind::Browser => {
            let port = browser_debug_port(&tool.name);
            match apps::browser::list_tabs(port) {
                Ok(urls) => Ok(Some(ToolState::Browser(apps::browser::Config { urls }))),
                Err(_) => Ok(None),
            }
        }
        ToolKind::Terminal | ToolKind::Zed => {
            // Not yet implemented for these tool types
            Ok(None)
        }
    }
}

/// Fetch live state for display (returns Result with error message)
fn fetch_live_state_display(tool: &ToolDefinition) -> Result<String, String> {
    match tool.kind {
        ToolKind::Browser => {
            let port = browser_debug_port(&tool.name);
            match apps::browser::list_tabs(port) {
                Ok(urls) => {
                    let mut output = String::new();
                    output.push_str(&format!("Open Tabs ({}):\n", urls.len()));
                    for (idx, url) in urls.iter().enumerate() {
                        output.push_str(&format!("  {}. {}\n", idx + 1, url));
                    }
                    Ok(output)
                }
                Err(e) => Err(format!("Could not fetch tabs: {}", e)),
            }
        }
        ToolKind::Terminal | ToolKind::Zed => {
            Ok("(Live state fetching not yet implemented for this tool type)\n".to_string())
        }
    }
}

/// Display detailed information about a tool, including live state
pub fn tool_info(tool_name: &str) -> Result<String> {
    storage::ensure_dirs()?;

    let tool = storage::read_tool(tool_name)?;

    let mut output = String::new();
    output.push_str(&format!("Tool: {}\n", tool.name));
    output.push_str(&format!("Kind: {:?}\n", tool.kind));
    output.push_str(&format!(
        "Created: {}\n",
        tool.created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| tool.created_at.to_string())
    ));

    if let Some(last_assembled) = tool.last_assembled_at {
        output.push_str(&format!(
            "Last Assembled: {}\n",
            last_assembled
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| last_assembled.to_string())
        ));
    } else {
        output.push_str("Last Assembled: Never\n");
    }

    let is_running = if let Some(ref assembled) = tool.assembled {
        output.push_str(&format!("\nAssembled Window ID: {}\n", assembled.window_id));

        // Check if window still exists
        if sway::container_exists(&assembled.window_id)? {
            output.push_str("Window Status: ✓ Running\n");
            true
        } else {
            output.push_str("Window Status: ✗ Not found (stale)\n");
            false
        }
    } else {
        output.push_str("\nAssembled Window: None\n");
        false
    };

    // Only fetch live state if the tool is currently running
    if is_running {
        output.push_str("\n--- Live State ---\n");
        match fetch_live_state_display(&tool) {
            Ok(state) => output.push_str(&state),
            Err(e) => output.push_str(&format!("{}\n", e)),
        }
    }

    // Show saved state
    output.push_str("\n--- Saved State ---\n");
    if let Some(ref state) = tool.state {
        output.push_str(&format!("{:#?}\n", state));
    } else {
        output.push_str("No saved state\n");
    }

    Ok(output)
}

/// Sync a single tool's state from live to disk
pub fn sync_tool(tool_name: &str) -> Result<bool> {
    let mut definition = storage::read_tool(tool_name)?;

    if let Some(live_state) = fetch_live_state(&definition)? {
        definition.state = Some(live_state);
        storage::write_tool(&definition)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Sync all tools in the focused bench
pub fn sync_all_tools() -> Result<()> {
    use std::collections::BTreeSet;

    let focused = storage::read_focused_bench()?;
    let name = focused.ok_or_else(|| anyhow!("no focused bench is set"))?;
    let bench = storage::read_bench(&name)?;

    let mut processed = BTreeSet::new();
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            if !processed.insert(tool_name.clone()) {
                continue;
            }
            sync_tool(tool_name)?;
        }
    }

    Ok(())
}
