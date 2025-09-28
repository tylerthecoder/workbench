use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{anyhow, Context, Result};

use crate::apps::{self, ToolKind};
use crate::assembly::{self, AssemblyOutcome, ToolStatus};
use crate::model::{AssembledBench, AssembledTool, BaySpec, Bench, ToolDefinition};
use crate::storage;
use crate::sway;

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

pub fn assemble(bench_name: &str) -> Result<BenchReport> {
    storage::ensure_dirs()?;
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;

    let existing = storage::read_assembled_bench(&bench.name)?.unwrap_or_default();
    let outcome = assembly::assemble_bench(&bench, existing)?;
    persist_assembly(&bench, &outcome)?;
    storage::write_active_bench(&bench.name)?;

    Ok(BenchReport {
        bench,
        assembled: outcome.assembled_bench,
        statuses: outcome.statuses,
    })
}

pub fn info(bench_name: &str) -> Result<BenchInfo> {
    storage::ensure_dirs()?;
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;
    let active = storage::read_active_bench()?;
    let is_active = active.as_deref() == Some(&bench.name);

    let tool_records = read_tool_records(&bench)?;
    let mut window_index = HashMap::new();
    for window in sway::current_windows()? {
        window_index.insert(window.id.clone(), window);
    }

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

pub fn stow(bench_name: &str) -> Result<BenchReport> {
    storage::ensure_dirs()?;
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;

    let existing = storage::read_assembled_bench(&bench.name)?.unwrap_or_default();
    let mut outcome = assembly::assemble_bench(&bench, existing)?;
    persist_assembly(&bench, &outcome)?;

    let statuses =
        assembly::stow_bench(&bench, &mut outcome.assembled_bench, &outcome.tool_records)?;
    storage::write_assembled_bench(&bench.name, &outcome.assembled_bench)?;
    storage::write_active_bench(&bench.name)?;

    Ok(BenchReport {
        bench,
        assembled: outcome.assembled_bench,
        statuses,
    })
}

pub fn focus(bench_name: &str) -> Result<BenchReport> {
    storage::ensure_dirs()?;
    let bench = storage::read_bench(bench_name)
        .with_context(|| format!("failed to load bench '{}'", bench_name))?;

    let existing = storage::read_assembled_bench(&bench.name)?.unwrap_or_default();
    let mut outcome = assembly::assemble_bench(&bench, existing)?;
    persist_assembly(&bench, &outcome)?;

    let statuses =
        assembly::focus_bench(&bench, &mut outcome.assembled_bench, &outcome.tool_records)?;
    storage::write_assembled_bench(&bench.name, &outcome.assembled_bench)?;
    storage::write_active_bench(&bench.name)?;

    Ok(BenchReport {
        bench,
        assembled: outcome.assembled_bench,
        statuses,
    })
}

pub fn assemble_tool(tool_name: &str, bay_override: Option<String>) -> Result<ToolStatus> {
    storage::ensure_dirs()?;

    let active = storage::read_active_bench()?;
    let (bench, bay) = match active {
        Some(name) => {
            let bench = storage::read_bench(&name)
                .with_context(|| format!("failed to load active bench '{}'", name))?;
            if let Some(spec) = find_bay_for_tool(&bench, tool_name) {
                (Some(bench), spec)
            } else if let Some(name) = bay_override.clone() {
                (
                    Some(bench),
                    BaySpec {
                        name,
                        tool_names: vec![tool_name.to_string()],
                    },
                )
            } else {
                anyhow::bail!(
                    "tool '{}' is not part of the active bench '{}'; provide --bay to override",
                    tool_name,
                    name
                );
            }
        }
        None => {
            let bay_name = bay_override.ok_or_else(|| {
                anyhow!(
                    "no active bench set; use --bay to choose a Sway bay for '{}'",
                    tool_name
                )
            })?;
            (
                None,
                BaySpec {
                    name: bay_name,
                    tool_names: vec![tool_name.to_string()],
                },
            )
        }
    };

    let (window_id, launched) = assembly::assign_tool_to_bay(tool_name, &bay)?;
    storage::write_assembled_tool(
        tool_name,
        &AssembledTool {
            window_id: window_id.clone(),
        },
    )?;

    if let Some(bench) = bench {
        let mut assembled = storage::read_assembled_bench(&bench.name)?.unwrap_or_default();
        let entry = assembled
            .bay_windows
            .entry(bay.name.clone())
            .or_insert_with(Vec::new);
        if !entry.contains(&window_id) {
            entry.push(window_id.clone());
        }
        storage::write_assembled_bench(&bench.name, &assembled)?;
    }

    let workspace = sway::current_windows()?
        .into_iter()
        .find(|info| info.id == window_id)
        .and_then(|info| info.workspace);

    Ok(ToolStatus {
        name: tool_name.to_string(),
        bay: bay.name,
        window_id: Some(window_id),
        workspace,
        launched,
    })
}

pub fn sync_layout() -> Result<Bench> {
    let active = storage::read_active_bench()?;
    let name = active.ok_or_else(|| anyhow!("no active bench is set"))?;
    let mut bench = storage::read_bench(&name)?;
    let assembled = storage::read_assembled_bench(&name)?.unwrap_or_default();
    let tool_records = read_tool_records(&bench)?;
    let window_to_bay = invert_assembled(&assembled);

    let mut new_bays = Vec::new();
    for bay in &bench.bays {
        let mut tools = Vec::new();
        for (tool, record) in &tool_records {
            if window_to_bay
                .get(&record.window_id)
                .map(|name| name == &bay.name)
                .unwrap_or(false)
            {
                tools.push(tool.clone());
            }
        }

        // Preserve tools without active windows in their existing bays.
        for tool in &bay.tool_names {
            if !tools.contains(tool) && !tool_records.contains_key(tool) {
                tools.push(tool.clone());
            }
        }

        let mut spec = bay.clone();
        spec.tool_names = tools;
        new_bays.push(spec);
    }

    bench.bays = new_bays;
    storage::write_bench(&bench)?;
    Ok(bench)
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
                    let port = assembly::browser_debug_port(tool_name);
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

fn persist_assembly(bench: &Bench, outcome: &AssemblyOutcome) -> Result<()> {
    for (tool_name, record) in &outcome.tool_records {
        storage::write_assembled_tool(tool_name, record)?;
    }
    storage::write_assembled_bench(&bench.name, &outcome.assembled_bench)?;
    Ok(())
}

fn find_bay_for_tool<'a>(bench: &'a Bench, tool: &str) -> Option<BaySpec> {
    for bay in &bench.bays {
        if bay.tool_names.iter().any(|name| name == tool) {
            return Some(bay.clone());
        }
    }
    None
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

fn invert_assembled(assembled: &AssembledBench) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (bay, windows) in &assembled.bay_windows {
        for window in windows {
            map.insert(window.clone(), bay.clone());
        }
    }
    map
}
