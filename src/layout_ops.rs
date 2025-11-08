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

/// Move all non-bench windows to scratchpad
pub fn stow_foreign_windows(bench_window_ids: &HashSet<String>) -> Result<()> {
    let all_windows = sway::current_windows()?;

    for window in all_windows {
        if bench_window_ids.contains(&window.id) {
            continue;
        }

        // Skip scratchpad windows
        if let Some(ref ws) = window.workspace {
            if ws == "__i3_scratch" {
                continue;
            }
        }

        sway::move_container_to_scratchpad(&window.id)?;
    }

    Ok(())
}

/// Restore bench windows to their saved layout
pub fn restore_bench_layout(bench: &Bench, assembled: &AssembledBench) -> Result<()> {
    // For each bay in the bench, move its windows to the bay workspace
    for bay in &bench.bays {
        if let Some(window_ids) = assembled.bay_windows.get(&bay.name) {
            for window_id in window_ids {
                // Check if window still exists
                if sway::container_exists(window_id)? {
                    // Move to the bay's workspace
                    sway::move_container_to_workspace(window_id, &bay.name)?;
                }
            }

            // Make sure the workspace is visible if it has windows
            if !window_ids.is_empty() {
                sway::ensure_workspace_visible(&bay.name)?;
            }
        }
    }

    Ok(())
}

/// Capture current window positions into AssembledBench structure
/// Queries sway for all windows, matches them to bench tools, and creates
/// an AssembledBench with bay_windows mapping bay name -> window IDs
pub fn capture_current_layout(bench: &Bench) -> Result<AssembledBench> {
    let mut bay_windows: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Get all current windows with their workspace info
    let all_windows = sway::current_windows()?;

    // Build a map of tool_name -> window_id for tools in this bench
    let mut tool_to_window = BTreeMap::new();
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            if let Some(assembled) = storage::read_assembled_tool(tool_name)? {
                if sway::container_exists(&assembled.window_id)? {
                    tool_to_window.insert(tool_name.clone(), assembled.window_id);
                }
            }
        }
    }

    // For each bay, find windows that belong to its tools and are in a workspace
    for bay in &bench.bays {
        let mut windows_in_bay = Vec::new();

        for tool_name in &bay.tool_names {
            if let Some(window_id) = tool_to_window.get(tool_name) {
                // Find this window in the current windows list
                if let Some(window_info) = all_windows.iter().find(|w| &w.id == window_id) {
                    // If it's in a real workspace (not scratchpad), include it
                    if let Some(ref ws) = window_info.workspace {
                        if ws != "__i3_scratch" {
                            windows_in_bay.push(window_id.clone());
                        }
                    }
                }
            }
        }

        if !windows_in_bay.is_empty() {
            bay_windows.insert(bay.name.clone(), windows_in_bay);
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
