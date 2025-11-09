use std::collections::{BTreeMap, HashSet};

use anyhow::Result;

use crate::model::{AssembledBench, Bench};
use crate::storage;
use crate::sway;

/// Collect all windows that belong to a bench
pub fn collect_bench_windows(bench: &Bench) -> Result<HashSet<String>> {
    let mut window_ids = HashSet::new();

    // Collect all tools defined in the bench
    let mut tool_names = HashSet::new();
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            tool_names.insert(tool_name.as_str());
        }
    }

    // Look up window IDs for each tool
    for tool_name in tool_names {
        if let Some(assembled) = storage::read_assembled_tool(tool_name)? {
            if sway::container_exists(&assembled.window_id)? {
                window_ids.insert(assembled.window_id);
            }
        }
    }

    Ok(window_ids)
}

/// Get list of windows that should be stowed (moved to temp workspace)
/// Returns windows that are not part of the bench and not already stowed
pub fn get_windows_to_stow(bench_window_ids: &HashSet<String>) -> Result<Vec<sway::WindowInfo>> {
    let all_windows = sway::current_windows()?;
    let mut windows_to_stow = Vec::new();

    for window in all_windows {
        // Skip windows that belong to the bench
        if bench_window_ids.contains(&window.id) {
            continue;
        }

        // Skip windows already in stowed workspaces (temp/scratchpad)
        if let Some(ref ws) = window.workspace {
            if crate::bench_ops::is_stowed_workspace(ws) {
                continue;
            }
        }

        windows_to_stow.push(window);
    }

    Ok(windows_to_stow)
}

/// Restore windows to their saved layout
/// Takes an AssembledBench and moves each window back to its saved workspace
pub fn restore_bench_layout(assembled: &AssembledBench) -> Result<()> {
    // For each workspace, restore its windows
    for (workspace, window_ids) in &assembled.bay_windows {
        for window_id in window_ids {
            // Check if window still exists
            if sway::container_exists(window_id)? {
                // Move to the saved workspace
                sway::move_container_to_workspace(window_id, workspace)?;
            }
        }

        // Make sure the workspace is visible if it has windows
        if !window_ids.is_empty() {
            sway::ensure_workspace_visible(workspace)?;
        }
    }

    Ok(())
}

/// Capture current window positions into AssembledBench structure
/// Captures ALL windows grouped by their current workspace
/// This preserves the entire workspace state, including untracked windows
pub fn capture_current_layout() -> Result<AssembledBench> {
    let mut bay_windows: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Get all current windows with their workspace info
    let all_windows = sway::current_windows()?;

    // Group windows by their workspace
    for window in all_windows {
        if let Some(workspace) = window.workspace {
            // Skip stowed workspaces (temp/scratchpad)
            if !crate::bench_ops::is_stowed_workspace(&workspace) {
                bay_windows
                    .entry(workspace)
                    .or_insert_with(Vec::new)
                    .push(window.id);
            }
        }
    }

    Ok(AssembledBench { bay_windows })
}

/// Move a specific window to a workspace
#[allow(dead_code)]
pub fn place_window(window_id: &str, workspace: &str) -> Result<()> {
    sway::move_container_to_workspace(window_id, workspace)?;
    Ok(())
}
