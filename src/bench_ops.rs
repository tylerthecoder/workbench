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
    pub launched: bool,
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
    pub active: bool,
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

    // Use tool_ops to find or launch the tool
    let (window_id, launched) = tool_ops::ensure_tool_window(tool_name, bay)?;

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
        launched,
    })
}

pub fn info(bench_name: &str) -> Result<BenchInfo> {
    storage::ensure_dirs()?;
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;
    let active = storage::read_active_bench()?;
    let is_active = active.as_deref() == Some(&bench.name);

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
                launched: false,
            });
        }
    }

    Ok(BenchInfo {
        bench,
        assembled,
        active: is_active,
        statuses,
    })
}

pub fn focus(bench_name: &str) -> Result<BenchReport> {
    storage::ensure_dirs()?;

    // 1. Load the target bench
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;

    // 2. Save current bench state before switching
    if let Some(current) = storage::read_active_bench()? {
        if current != bench_name {
            // Sync the current bench's layout to disk before we switch
            let _ = sync_layout(); // Ignore errors if there's nothing to sync
        }
    }

    // 3. Ensure all tools for this bench exist
    let mut statuses = Vec::new();
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            let (window_id, launched) = tool_ops::ensure_tool_window(tool_name, &bay.name)?;

            statuses.push(ToolStatus {
                name: tool_name.clone(),
                bay: bay.name.clone(),
                window_id: Some(window_id.clone()),
                workspace: None,
                launched,
            });
        }
    }

    // 4. Collect all bench windows
    let bench_windows = layout_ops::collect_bench_windows(&bench)?;

    // 5. Stow everything else
    layout_ops::stow_foreign_windows(&bench_windows)?;

    // 6. Restore bench layout
    let assembled = storage::read_assembled_bench(bench_name)?.unwrap_or_default();
    layout_ops::restore_bench_layout(&bench, &assembled)?;

    // 7. Enrich statuses with current workspace info
    enrich_status_workspaces(&mut statuses)?;

    // 8. Mark as active
    storage::write_active_bench(bench_name)?;

    Ok(BenchReport {
        bench,
        assembled,
        statuses,
    })
}

pub fn stow(bench_name: &str) -> Result<BenchReport> {
    storage::ensure_dirs()?;
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;

    // Get all windows for this bench
    let bench_windows = layout_ops::collect_bench_windows(&bench)?;

    // Move them all to scratchpad
    for window_id in &bench_windows {
        sway::move_container_to_scratchpad(window_id)?;
    }

    // Clear active if this was active
    if storage::read_active_bench()? == Some(bench_name.to_string()) {
        storage::write_active_bench("")?;
    }

    let assembled = storage::read_assembled_bench(bench_name)?.unwrap_or_default();
    let statuses = build_statuses(&bench)?;

    Ok(BenchReport {
        bench,
        assembled,
        statuses,
    })
}

pub fn sync_layout() -> Result<AssembledBench> {
    storage::ensure_dirs()?;

    // Get the currently active bench
    let bench_name =
        storage::read_active_bench()?.ok_or_else(|| anyhow!("no active bench to sync"))?;

    // Load the bench definition to know what tools/bays exist
    let bench = storage::read_bench(&bench_name)?;

    // Capture current window state from sway
    let assembled = layout_ops::capture_current_layout(&bench)?;

    // Write to storage
    storage::write_assembled_bench(&bench_name, &assembled)?;

    Ok(assembled)
}

pub fn sync_tool_state() -> Result<()> {
    let active = storage::read_active_bench()?;
    let name = active.ok_or_else(|| anyhow!("no active bench is set"))?;
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

pub fn active_bench() -> Result<Option<String>> {
    storage::read_active_bench()
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

fn build_statuses(bench: &Bench) -> Result<Vec<ToolStatus>> {
    let tool_records = read_tool_records(bench)?;
    let window_map = sway::current_windows()?
        .into_iter()
        .map(|w| (w.id.clone(), w))
        .collect::<std::collections::HashMap<_, _>>();

    let mut statuses = Vec::new();
    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            let record = tool_records.get(tool_name);
            let window_id = record.map(|r| r.window_id.clone());
            let workspace = window_id
                .as_ref()
                .and_then(|id| window_map.get(id))
                .and_then(|info| info.workspace.clone());

            statuses.push(ToolStatus {
                name: tool_name.clone(),
                bay: bay.name.clone(),
                window_id,
                workspace,
                launched: false,
            });
        }
    }
    Ok(statuses)
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
