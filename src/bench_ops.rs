use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};

use crate::apps::{self, ToolKind};
use crate::layout_ops;
use crate::model::{AssembledBench, AssembledTool, Bench, ToolDefinition};
use crate::storage;
use crate::sway;
use crate::tool_ops;

#[derive(Debug, Clone)]
pub struct ToolStatus {
    pub name: String,
    pub bay: String,
    pub window_id: Option<String>,
    pub workspace: Option<String>,
    pub assembled: bool,
}

#[derive(Debug, Clone)]
pub struct BenchReport {
    pub bench: Bench,
    pub assembled: AssembledBench,
    pub statuses: Vec<ToolStatus>,
}

#[derive(Debug, Clone)]
pub struct BenchInfo {
    pub bench: Bench,
    pub assembled: bool,
    pub focused: bool,
    pub statuses: Vec<ToolStatus>,
    pub current_windows: Vec<sway::WindowInfo>,
    pub saved_layout: Option<AssembledBench>,
}

#[derive(Debug, Clone)]
pub struct LayoutDiff {
    pub added_windows: Vec<(String, String)>, // (workspace, window_id)
    pub removed_windows: Vec<(String, String)>, // (workspace, window_id)
}

pub fn create_bench(name: &str) -> Result<Bench> {
    storage::ensure_dirs()?;
    let path = storage::bench_path(name);
    if path.exists() {
        anyhow::bail!("bench '{}' already exists", name);
    }

    let bench = Bench {
        name: name.to_string(),
        bays: Vec::new(),
        created_at: time::OffsetDateTime::now_utc(),
        last_focused_at: None,
        assembled: crate::model::AssembledBench::default(),
    };
    storage::write_bench(&bench)?;
    Ok(bench)
}

pub fn add_tool_to_bench(bench_name: &str, tool_name: &str, bay_name: &str) -> Result<()> {
    storage::ensure_dirs()?;

    // Verify the tool exists
    let _tool =
        storage::read_tool(tool_name).with_context(|| format!("tool '{}' not found", tool_name))?;

    // Load the bench
    let mut bench = storage::read_bench(bench_name)
        .with_context(|| format!("bench '{}' not found", bench_name))?;

    // Find or create the bay
    let bay = bench.bays.iter_mut().find(|b| b.name == bay_name);

    if let Some(bay) = bay {
        // Check if tool is already in this bay
        if bay.tool_names.contains(&tool_name.to_string()) {
            anyhow::bail!("tool '{}' is already in bay '{}'", tool_name, bay_name);
        }
        bay.tool_names.push(tool_name.to_string());
    } else {
        // Create new bay with the tool
        bench.bays.push(crate::model::BaySpec {
            name: bay_name.to_string(),
            tool_names: vec![tool_name.to_string()],
        });
    }

    storage::write_bench(&bench)?;
    Ok(())
}

pub fn list_benches() -> Result<Vec<String>> {
    storage::ensure_dirs()?;
    storage::list_bench_names()
}

pub fn list_tools() -> Result<Vec<String>> {
    storage::ensure_dirs()?;
    storage::list_tool_names()
}

pub fn assemble_tool(tool_name: &str, bay: &str) -> Result<ToolStatus> {
    storage::ensure_dirs()?;

    // Load tool definition to verify it exists
    let _definition =
        storage::read_tool(tool_name).with_context(|| format!("tool '{}' not found", tool_name))?;

    // Use tool_ops to assemble the tool
    let (window_id, assembled) = tool_ops::assemble_tool(tool_name, bay)?;

    // Build status response with current workspace info
    let workspace = sway::current_windows()?
        .into_iter()
        .find(|w| w.id == window_id)
        .and_then(|w| w.workspace);

    Ok(ToolStatus {
        name: tool_name.to_string(),
        bay: bay.to_string(),
        window_id: Some(window_id),
        workspace,
        assembled,
    })
}

pub fn info(bench_name: &str) -> Result<BenchInfo> {
    storage::ensure_dirs()?;
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;
    let focused = storage::read_focused_bench()?;
    let is_focused = focused.as_deref() == Some(&bench.name);

    let tool_records = read_tool_records(&bench)?;
    let all_windows = sway::current_windows()?;
    let window_index: std::collections::HashMap<_, _> = all_windows
        .iter()
        .map(|w| (w.id.clone(), w.clone()))
        .collect();

    let mut statuses = Vec::new();
    let mut assembled = true;
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            let record = tool_records.get(tool_name);
            let window_id = record.map(|r| r.window_id.clone());
            let workspace = window_id
                .as_ref()
                .and_then(|id| window_index.get(id))
                .and_then(|info| info.workspace.clone());
            let present = window_id
                .as_ref()
                .map(|id| window_index.contains_key(id))
                .unwrap_or(false);
            if !present {
                assembled = false;
            }
            statuses.push(ToolStatus {
                name: tool_name.clone(),
                bay: bay.name.clone(),
                window_id,
                workspace,
                assembled: false,
            });
        }
    }

    // Filter to only include "active" windows (not in temp or scratchpad)
    let active_windows = all_windows
        .into_iter()
        .filter(|w| {
            if let Some(ref ws) = w.workspace {
                !is_stowed_workspace(ws)
            } else {
                false
            }
        })
        .collect();

    // Load saved layout if it exists
    let saved_layout = storage::read_assembled_bench(bench_name)?;

    Ok(BenchInfo {
        bench,
        assembled,
        focused: is_focused,
        statuses,
        current_windows: active_windows,
        saved_layout,
    })
}

pub(crate) fn is_stowed_workspace(workspace: &str) -> bool {
    workspace == "temp" || workspace == "__i3_scratch"
}

pub fn focus(bench_name: &str, stow_others: bool) -> Result<BenchReport> {
    storage::ensure_dirs()?;

    // 1. Load the target bench
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;

    // 2. Save current bench state before switching
    if let Some(current) = storage::read_focused_bench()? {
        if current != bench_name {
            println!("Saving layout for focused bench '{}' to disk", current);
            // Sync the current bench's layout to disk before we switch
            let layout_diff = sync_layout()?; // Ignore errors if there's nothing to sync
            for (workspace, window_id) in layout_diff.added_windows {
                println!("Adding window {} to workspace {}", window_id, workspace);
            }
            for (workspace, window_id) in layout_diff.removed_windows {
                println!("Removing window {} from workspace {}", window_id, workspace);
            }
        }
    }

    // 3. Ensure all tools for this bench exist
    println!("\nAssembling tools:");
    let mut statuses = Vec::new();
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            let (window_id, assembled) = tool_ops::assemble_tool(tool_name, &bay.name)?;

            statuses.push(ToolStatus {
                name: tool_name.clone(),
                bay: bay.name.clone(),
                window_id: Some(window_id.clone()),
                workspace: None,
                assembled,
            });
        }
    }

    // 4. Collect all bench windows
    let bench_windows = layout_ops::collect_bench_windows(&bench)?;

    // 5. Stow everything else (if requested)
    if stow_others {
        let windows_to_stow = layout_ops::get_windows_to_stow(&bench_windows)?;
        for window in windows_to_stow {
            sway::move_container_to_workspace(&window.id, "temp")?;
        }
    }

    // 6. Restore bench layout
    let assembled = storage::read_assembled_bench(bench_name)?.unwrap_or_default();
    layout_ops::restore_bench_layout(&assembled)?;

    // 7. Enrich statuses with current workspace info
    enrich_status_workspaces(&mut statuses)?;

    // 8. Mark as focused and update timestamp
    storage::write_focused_bench(bench_name)?;

    // Update last_focused_at timestamp
    let mut bench = storage::read_bench(bench_name)?;
    bench.last_focused_at = Some(time::OffsetDateTime::now_utc());
    storage::write_bench(&bench)?;

    Ok(BenchReport {
        bench,
        assembled,
        statuses,
    })
}

pub fn focus_plan(bench_name: &str) -> Result<String> {
    storage::ensure_dirs()?;

    // Load the target bench
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;

    let mut output = String::new();
    output.push_str(&format!("Plan for focusing bench '{}'\n\n", bench_name));

    // Check which tools need to be assembled
    output.push_str("Tools:\n");
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            match tool_ops::tool_window_exists(tool_name)? {
                Some(window_id) => {
                    output.push_str(&format!(
                        "  ✓ {} (window {}) - already assembled\n",
                        tool_name, window_id
                    ));
                }
                None => {
                    output.push_str(&format!("  ✗ {} - will be assembled\n", tool_name));
                }
            }
        }
    }

    // Check which windows will be stowed
    output.push_str("\nWindows to stow:\n");
    let bench_windows = layout_ops::collect_bench_windows(&bench)?;
    let windows_to_stow = layout_ops::get_windows_to_stow(&bench_windows)?;
    if windows_to_stow.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for window in &windows_to_stow {
            let ws = window.workspace.as_deref().unwrap_or("<unknown>");
            output.push_str(&format!("  → Window {} from workspace {}\n", window.id, ws));
        }
    }

    // Show where bench windows will be placed
    output.push_str("\nBench window placement:\n");
    let assembled = storage::read_assembled_bench(bench_name)?.unwrap_or_default();
    if assembled.bay_windows.is_empty() {
        output.push_str("  (no saved layout - windows will be placed in their bay workspaces)\n");
    } else {
        for (bay, window_ids) in &assembled.bay_windows {
            output.push_str(&format!(
                "  Bay '{}': {} window(s)\n",
                bay,
                window_ids.len()
            ));
        }
    }

    Ok(output)
}

pub fn sync_layout() -> Result<LayoutDiff> {
    storage::ensure_dirs()?;

    // Get the currently focused bench
    let bench_name =
        storage::read_focused_bench()?.ok_or_else(|| anyhow!("no focused bench to sync"))?;

    // Load old layout if it exists
    let old_layout = storage::read_assembled_bench(&bench_name)?;

    // Capture current window state from sway
    let new_layout = layout_ops::capture_current_layout()?;

    // Calculate diff
    let mut added_windows = Vec::new();
    let mut removed_windows = Vec::new();

    // Find added windows (in new but not in old)
    for (workspace, window_ids) in &new_layout.bay_windows {
        for window_id in window_ids {
            let existed_before = old_layout
                .as_ref()
                .and_then(|old| old.bay_windows.get(workspace))
                .map(|old_windows| old_windows.contains(window_id))
                .unwrap_or(false);

            if !existed_before {
                added_windows.push((workspace.clone(), window_id.clone()));
            }
        }
    }

    // Find removed windows (in old but not in new)
    if let Some(ref old) = old_layout {
        for (workspace, window_ids) in &old.bay_windows {
            for window_id in window_ids {
                let exists_now = new_layout
                    .bay_windows
                    .get(workspace)
                    .map(|new_windows| new_windows.contains(window_id))
                    .unwrap_or(false);

                if !exists_now {
                    removed_windows.push((workspace.clone(), window_id.clone()));
                }
            }
        }
    }

    // Write to storage
    storage::write_assembled_bench(&bench_name, &new_layout)?;

    Ok(LayoutDiff {
        added_windows,
        removed_windows,
    })
}

pub fn sync_tool_state() -> Result<()> {
    tool_ops::sync_all_tools()
}

pub fn focused_bench() -> Result<Option<String>> {
    storage::read_focused_bench()
}

pub fn craft_tool(kind: ToolKind, name: &str) -> Result<ToolDefinition> {
    storage::ensure_dirs()?;
    let path = storage::tool_path(name);
    if path.exists() {
        anyhow::bail!("tool '{}' already exists", name);
    }

    let state = match kind {
        ToolKind::Browser => Some(apps::ToolState::Browser(apps::browser::Config::default())),
        ToolKind::Terminal => Some(apps::ToolState::Terminal(apps::terminal::Config::default())),
        ToolKind::Zed => Some(apps::ToolState::Zed(apps::zed::Config::default())),
    };

    let definition = ToolDefinition {
        name: name.to_string(),
        kind,
        created_at: time::OffsetDateTime::now_utc(),
        last_assembled_at: None,
        state,
        assembled: None,
    };
    storage::write_tool(&definition)?;
    Ok(definition)
}

// Helper functions

fn enrich_status_workspaces(statuses: &mut [ToolStatus]) -> Result<()> {
    let window_map = sway::current_windows()?
        .into_iter()
        .map(|w| (w.id.clone(), w))
        .collect::<std::collections::HashMap<_, _>>();

    for status in statuses {
        if let Some(id) = status.window_id.as_ref() {
            status.workspace = window_map.get(id).and_then(|info| info.workspace.clone());
        }
    }
    Ok(())
}

fn read_tool_records(bench: &Bench) -> Result<BTreeMap<String, AssembledTool>> {
    let mut records = BTreeMap::new();
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            if records.contains_key(tool_name) {
                continue;
            }
            if let Some(record) = storage::read_assembled_tool(tool_name)? {
                records.insert(tool_name.clone(), record);
            }
        }
    }
    Ok(records)
}
