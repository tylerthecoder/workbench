use std::collections::{BTreeMap, BTreeSet};

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
    };
    storage::write_bench(&bench)?;
    Ok(bench)
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
    let window_index = sway::current_windows()?
        .into_iter()
        .map(|w| (w.id.clone(), w))
        .collect::<std::collections::HashMap<_, _>>();

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

    Ok(BenchInfo {
        bench,
        assembled,
        focused: is_focused,
        statuses,
    })
}

pub fn focus(bench_name: &str, stow_others: bool) -> Result<BenchReport> {
    storage::ensure_dirs()?;

    // 1. Load the target bench
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;

    // 2. Save current bench state before switching
    if let Some(current) = storage::read_focused_bench()? {
        if current != bench_name {
            // Sync the current bench's layout to disk before we switch
            let _ = sync_layout(); // Ignore errors if there's nothing to sync
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
        layout_ops::stow_foreign_windows(&bench_windows)?;
    }

    // 6. Restore bench layout
    let assembled = storage::read_assembled_bench(bench_name)?.unwrap_or_default();
    layout_ops::restore_bench_layout(&bench, &assembled)?;

    // 7. Enrich statuses with current workspace info
    enrich_status_workspaces(&mut statuses)?;

    // 8. Mark as focused
    storage::write_focused_bench(bench_name)?;

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
    let all_windows = sway::current_windows()?;
    let mut has_stowed = false;
    for window in &all_windows {
        if !bench_windows.contains(&window.id) {
            if let Some(ref ws) = window.workspace {
                if ws != "__i3_scratch" {
                    output.push_str(&format!("  → Window {} from workspace {}\n", window.id, ws));
                    has_stowed = true;
                }
            }
        }
    }
    if !has_stowed {
        output.push_str("  (none)\n");
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

pub fn sync_layout() -> Result<AssembledBench> {
    storage::ensure_dirs()?;

    // Get the currently focused bench
    let bench_name =
        storage::read_focused_bench()?.ok_or_else(|| anyhow!("no focused bench to sync"))?;

    // Load the bench definition to know what tools/bays exist
    let bench = storage::read_bench(&bench_name)?;

    // Capture current window state from sway
    let assembled = layout_ops::capture_current_layout(&bench)?;

    // Write to storage
    storage::write_assembled_bench(&bench_name, &assembled)?;

    Ok(assembled)
}

pub fn sync_tool_state() -> Result<()> {
    let focused = storage::read_focused_bench()?;
    let name = focused.ok_or_else(|| anyhow!("no focused bench is set"))?;
    let bench = storage::read_bench(&name)?;
    let mut processed = BTreeSet::new();
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            if !processed.insert(tool_name.clone()) {
                continue;
            }
            let mut definition = storage::read_tool(tool_name)?;
            match definition.kind {
                crate::apps::ToolKind::Browser => {
                    let port = tool_ops::browser_debug_port(tool_name);
                    if let Ok(urls) = crate::apps::browser::list_tabs(port) {
                        definition.state = Some(crate::apps::ToolState::Browser(
                            crate::apps::browser::Config { urls },
                        ));
                        storage::write_tool(&definition)?;
                    }
                }
                _ => {
                    // No dynamic state to sync for terminal/zed at the moment.
                }
            }
        }
    }

    Ok(())
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
        state,
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
