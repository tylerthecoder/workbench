use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};

use crate::apps::{self, Tool};
use crate::model::{AssembledBench, AssembledTool, BaySpec, Bench, ToolDefinition};
use crate::sway::WindowInfo;
use crate::{storage, sway};

#[derive(Debug, Clone)]
pub struct ToolStatus {
    pub name: String,
    pub bay: String,
    pub window_id: Option<String>,
    pub workspace: Option<String>,
    pub launched: bool,
}

#[derive(Debug, Clone)]
pub struct AssemblyOutcome {
    pub assembled_bench: AssembledBench,
    pub tool_records: BTreeMap<String, AssembledTool>,
    pub statuses: Vec<ToolStatus>,
}

pub fn assemble_bench(
    bench: &Bench,
    mut assembled_bench: AssembledBench,
) -> Result<AssemblyOutcome> {
    let mut tool_records = BTreeMap::new();
    let mut statuses = Vec::new();
    let mut observed_bays = BTreeSet::new();

    for bay in &bench.bays {
        observed_bays.insert(bay.name.clone());
        let entry = assembled_bench
            .bay_windows
            .entry(bay.name.clone())
            .or_insert_with(Vec::new);
        prune_missing_windows(entry)?;

        for tool_name in &bay.tool_names {
            let definition = storage::read_tool(tool_name).with_context(|| {
                format!(
                    "failed to read tool definition for '{}'; ensure it exists",
                    tool_name
                )
            })?;
            let mut tool = instantiate_tool(&definition, &bay.name);
            let mut launched = false;
            let mut window_id = lookup_tracked_window(tool_name)?;

            if window_id
                .as_ref()
                .map(|id| sway::container_exists(id))
                .transpose()?
                == Some(true)
            {
                // window is still alive
            } else {
                window_id = find_existing_window(&tool)?;
            }

            if window_id.is_none() {
                window_id = Some(launch_tool(&mut tool)?);
                launched = true;
            }

            let Some(window_id) = window_id else {
                return Err(anyhow!(
                    "failed to locate or launch tool '{}' for bay '{}'",
                    tool_name,
                    bay.name
                ));
            };

            if !entry.contains(&window_id) {
                entry.push(window_id.clone());
            }

            tool_records.insert(
                tool_name.clone(),
                AssembledTool {
                    window_id: window_id.clone(),
                },
            );

            statuses.push(ToolStatus {
                name: tool_name.clone(),
                bay: bay.name.clone(),
                window_id: Some(window_id.clone()),
                workspace: None,
                launched,
            });
        }
    }

    assembled_bench
        .bay_windows
        .retain(|bay, _| observed_bays.contains(bay));

    enrich_status_workspaces(&mut statuses)?;

    Ok(AssemblyOutcome {
        assembled_bench,
        tool_records,
        statuses,
    })
}

fn instantiate_tool(def: &ToolDefinition, bay: &str) -> Tool {
    Tool {
        name: def.name.clone(),
        kind: def.kind,
        bay: bay.to_string(),
        state: def.state.clone(),
    }
}

fn lookup_tracked_window(tool_name: &str) -> Result<Option<String>> {
    let stored = storage::read_assembled_tool(tool_name)?;
    Ok(stored.map(|tool| tool.window_id))
}

fn find_existing_window(tool: &Tool) -> Result<Option<String>> {
    let ids = sway::matching_container_ids(tool.sway_patterns())?;
    Ok(ids.into_iter().next())
}

fn prune_missing_windows(windows: &mut Vec<String>) -> Result<()> {
    windows.retain(|id| match sway::container_exists(id) {
        Ok(true) => true,
        Ok(false) => false,
        Err(_) => false,
    });
    Ok(())
}

fn enrich_status_workspaces(statuses: &mut [ToolStatus]) -> Result<()> {
    let mut map = HashMap::new();
    for window in sway::current_windows()? {
        map.insert(window.id.clone(), window);
    }

    for status in statuses {
        if let Some(id) = status.window_id.as_ref() {
            status.workspace = map.get(id).and_then(|info| info.workspace.clone());
        }
    }
    Ok(())
}

fn launch_tool(tool: &mut Tool) -> Result<String> {
    let patterns = tool.sway_patterns();
    let before = sway::matching_container_ids(patterns)?;

    match tool.kind() {
        apps::ToolKind::Browser => {
            let port = debug_port_for_tool(tool);
            let config = tool.browser_config()?;
            apps::browser::launch(&config, port)?;
        }
        apps::ToolKind::Terminal => {
            let config = tool.terminal_config()?;
            apps::terminal::launch(&config)?;
        }
        apps::ToolKind::Zed => {
            let config = tool.zed_config()?;
            apps::zed::launch(&config)?;
        }
    }

    sway::wait_for_new_container(patterns, &before, Duration::from_secs(15))
}

pub fn stow_bench(
    bench: &Bench,
    assembled_bench: &mut AssembledBench,
    tool_records: &BTreeMap<String, AssembledTool>,
) -> Result<Vec<ToolStatus>> {
    let mut moved = HashSet::new();
    for windows in assembled_bench.bay_windows.values_mut() {
        let mut refreshed = Vec::new();
        for window_id in windows.iter() {
            if sway::container_exists(window_id)? {
                if moved.insert(window_id.clone()) {
                    sway::move_container_to_scratchpad(window_id)?;
                }
                refreshed.push(window_id.clone());
            }
        }
        *windows = refreshed;
    }

    let mut statuses = Vec::new();
    let window_map = build_window_index()?;

    for bay in &bench.bays {
        for tool_name in &bay.tool_names {
            let record = tool_records.get(tool_name);
            let window_id = record.map(|r| r.window_id.clone());
            if let Some(id) = window_id.as_ref() {
                if moved.insert(id.clone()) && sway::container_exists(id)? {
                    sway::move_container_to_scratchpad(id)?;
                }
            }
            statuses.push(ToolStatus {
                name: tool_name.clone(),
                bay: bay.name.clone(),
                workspace: window_id
                    .as_ref()
                    .and_then(|id| window_map.get(id))
                    .and_then(|info| info.workspace.clone()),
                window_id,
                launched: false,
            });
        }
    }

    Ok(statuses)
}

pub fn focus_bench(
    bench: &Bench,
    assembled_bench: &mut AssembledBench,
    tool_records: &BTreeMap<String, AssembledTool>,
) -> Result<Vec<ToolStatus>> {
    let mut statuses = Vec::new();
    let mut seen_windows = HashSet::new();

    let target_bays: BTreeSet<_> = bench.bays.iter().map(|b| b.name.clone()).collect();
    assembled_bench
        .bay_windows
        .retain(|bay, _| target_bays.contains(bay));

    for bay in &bench.bays {
        sway::ensure_workspace_visible(&bay.name)?;

        let entry = assembled_bench
            .bay_windows
            .entry(bay.name.clone())
            .or_insert_with(Vec::new);
        prune_missing_windows(entry)?;

        for id in entry.iter() {
            if sway::container_exists(id)? {
                sway::move_container_to_workspace(id, &bay.name)?;
                seen_windows.insert(id.clone());
            }
        }

        for tool_name in &bay.tool_names {
            let record = tool_records.get(tool_name);
            let window_id = record.map(|r| r.window_id.clone());
            if let Some(id) = window_id.as_ref() {
                if sway::container_exists(id)? {
                    sway::move_container_to_workspace(id, &bay.name)?;
                    if !entry.contains(id) {
                        entry.push(id.clone());
                    }
                    seen_windows.insert(id.clone());
                }
            }
            statuses.push(ToolStatus {
                name: tool_name.clone(),
                bay: bay.name.clone(),
                window_id,
                workspace: Some(bay.name.clone()),
                launched: false,
            });
        }
    }

    stow_non_bench_windows(&seen_windows)?;
    Ok(statuses)
}

fn stow_non_bench_windows(bench_windows: &HashSet<String>) -> Result<()> {
    let windows = build_window_index()?;
    for (id, info) in windows {
        if bench_windows.contains(&id) {
            continue;
        }
        if let Some(ws) = info.workspace {
            // Skip scratchpad or empty workspace markers
            if ws == "__i3_scratch" {
                continue;
            }
            sway::move_container_to_scratchpad(&id)?;
        }
    }
    Ok(())
}

fn build_window_index() -> Result<HashMap<String, WindowInfo>> {
    let mut map = HashMap::new();
    for window in sway::current_windows()? {
        map.insert(window.id.clone(), window);
    }
    Ok(map)
}

pub fn browser_debug_port(tool_name: &str) -> u16 {
    stable_debug_port(tool_name)
}

fn debug_port_for_tool(tool: &Tool) -> u16 {
    stable_debug_port(&tool.name)
}

fn stable_debug_port(key: &str) -> u16 {
    const BASE_PORT: u16 = 45000;
    const PORT_SPAN: u16 = 1000;

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();
    BASE_PORT + (hash % PORT_SPAN as u64) as u16
}

pub fn assign_tool_to_bay(tool_name: &str, bay: &BaySpec) -> Result<(String, bool)> {
    let definition = storage::read_tool(tool_name).with_context(|| {
        format!(
            "failed to read tool definition for '{}'; ensure it exists",
            tool_name
        )
    })?;
    let mut tool = instantiate_tool(&definition, &bay.name);
    let mut window_id = lookup_tracked_window(tool_name)?;

    if window_id
        .as_ref()
        .map(|id| sway::container_exists(id))
        .transpose()?
        != Some(true)
    {
        window_id = find_existing_window(&tool)?;
    }

    let mut launched = false;
    if window_id.is_none() {
        window_id = Some(launch_tool(&mut tool)?);
        launched = true;
    }

    window_id
        .map(|id| (id, launched))
        .ok_or_else(|| anyhow!("failed to launch tool '{}'", tool_name))
}
