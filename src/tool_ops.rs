use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::apps::{self, Tool, ToolKind};
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

/// Find an existing window for a tool by matching sway patterns
pub fn find_tool_window(tool_def: &ToolDefinition) -> Result<Option<String>> {
    let patterns = tool_def.kind.sway_patterns();
    let ids = sway::matching_container_ids(patterns)?;
    Ok(ids.into_iter().next())
}

/// Launch a tool and return its window ID
pub fn launch_tool(tool_def: &ToolDefinition, bay: &str) -> Result<String> {
    println!("Launching tool '{}'...", tool_def.name);

    let tool = Tool {
        name: tool_def.name.clone(),
        kind: tool_def.kind,
        bay: bay.to_string(),
        state: tool_def.state.clone(),
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
    Ok(window_id)
}

/// Generate a consistent debug port for a browser tool
pub fn browser_debug_port(tool_name: &str) -> u16 {
    let mut hasher = DefaultHasher::new();
    tool_name.hash(&mut hasher);
    let hash = hasher.finish();

    // Port range: 9222-10222
    9222 + (hash % 1000) as u16
}

/// Combined: find existing or launch new tool window
/// Returns (window_id, was_launched)
pub fn ensure_tool_window(tool_name: &str, bay: &str) -> Result<(String, bool)> {
    // First check if we have a tracked window that still exists
    if let Some(window_id) = tool_window_exists(tool_name)? {
        return Ok((window_id, false));
    }

    // Load the tool definition
    let definition =
        storage::read_tool(tool_name).with_context(|| format!("tool '{}' not found", tool_name))?;

    // Try to find an existing window by pattern matching
    if let Some(window_id) = find_tool_window(&definition)? {
        // Save it for future tracking
        storage::write_assembled_tool(
            tool_name,
            &crate::model::AssembledTool {
                window_id: window_id.clone(),
            },
        )?;
        return Ok((window_id, false));
    }

    // Launch the tool
    let window_id = launch_tool(&definition, bay)?;

    // Save the window ID
    storage::write_assembled_tool(
        tool_name,
        &crate::model::AssembledTool {
            window_id: window_id.clone(),
        },
    )?;

    Ok((window_id, true))
}
