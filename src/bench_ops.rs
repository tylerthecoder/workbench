use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};

use crate::apps::{self, BenchTool, Tool};
use crate::model::{Bench, BenchRuntime, CapturedBay, ToolDefault, ToolDefinition};
use crate::runtime::{self, ToolRuntimeState};
use crate::storage;
use crate::sway;
use crate::sway::WindowInfo;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct ToolRuntimeStatus {
    pub name: String,
    pub default_bay: u32,
    pub actual_bay: Option<u32>,
    pub container_id: Option<String>,
    pub debug_port: Option<u16>,
    pub last_opened: Option<OffsetDateTime>,
}

impl ToolRuntimeStatus {
    pub fn is_drifted(&self) -> bool {
        match self.actual_bay {
            Some(actual) => actual != self.default_bay,
            None => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchRuntimeReport {
    pub bench: Bench,
    pub runtime: BenchRuntime,
    pub tool_statuses: Vec<ToolRuntimeStatus>,
}

pub fn create_bench(name: &str, benches_dir: &Path) -> Result<Bench> {
    storage::ensure_dirs()?;
    let bench = Bench {
        name: name.to_string(),
        tool_defaults: Vec::new(),
    };
    let path = resolve_bench_path(name, benches_dir);
    if path.exists() {
        return Err(anyhow!(
            "bench {} already exists at {}",
            name,
            path.display()
        ));
    }
    save_bench(&path, &bench)?;
    Ok(bench)
}

pub fn current_runtime_snapshot() -> Result<BenchRuntime> {
    let windows = sway::current_windows()?;
    let mut workspace_windows: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for window in &windows {
        if let Some(ws) = &window.workspace {
            if let Some(num) = parse_workspace_number(ws) {
                workspace_windows
                    .entry(num)
                    .or_default()
                    .push(window.id.clone());
            }
        }
    }

    if workspace_windows.is_empty() {
        return Ok(BenchRuntime {
            name: "current".to_string(),
            captured_bays: Vec::new(),
        });
    }

    let tree = sway::get_tree()?;
    let workspace_nums: BTreeSet<u32> = workspace_windows.keys().copied().collect();
    let snapshots = sway::capture_workspace_snapshots(&tree, &workspace_nums);

    let mut captured = Vec::new();
    for bay in workspace_nums {
        let mut window_ids = workspace_windows.remove(&bay).unwrap_or_default();
        window_ids.sort();
        let snapshot = snapshots.get(&bay).cloned().unwrap_or_default();
        captured.push(CapturedBay {
            bay,
            name: snapshot.name,
            window_ids,
        });
    }

    Ok(BenchRuntime {
        name: "current".to_string(),
        captured_bays: captured,
    })
}

pub fn default_data_dir() -> PathBuf {
    let home = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".local/share")
        });
    home.join("bench")
}

pub fn benches_dir_or_default(p: Option<PathBuf>) -> PathBuf {
    p.unwrap_or_else(|| default_data_dir().join("benches"))
}

pub fn resolve_bench_path(target: &str, benches_dir: &Path) -> PathBuf {
    let as_path = PathBuf::from(target);
    if as_path.is_file() {
        as_path
    } else {
        benches_dir.join(format!("{}.yml", target))
    }
}

pub fn load_bench(path: &Path) -> Result<Bench> {
    let data = std::fs::read_to_string(path)?;
    let bench: Bench = serde_yaml::from_str(&data)?;
    Ok(bench)
}

pub fn save_bench(path: &Path, bench: &Bench) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create benches dir {}", parent.display()))?;
    }
    let data = serde_yaml::to_string(bench)?;
    std::fs::write(path, data)?;
    Ok(())
}

pub fn list_benches(dir: &Path) -> Result<Vec<String>> {
    let mut entries = vec![];
    if dir.is_dir() {
        for e in std::fs::read_dir(dir)? {
            let e = e?;
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("yml") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    entries.push(stem.to_string());
                }
            }
        }
    }
    entries.sort();
    Ok(entries)
}

pub fn set_active_bench(name: &str, benches_dir: &Path) -> Result<()> {
    let path = resolve_bench_path(name, benches_dir);
    if !path.exists() {
        return Err(anyhow!("bench {} not found at {}", name, path.display()));
    }
    runtime::set_active_bench(name)
}

pub fn assemble_active_bench(benches_dir: &Path) -> Result<BenchRuntimeReport> {
    storage::ensure_dirs()?;
    let (bench, mut bench_runtime) = load_active_bench(benches_dir)?;

    ensure_workspace_labels(&bench.tool_defaults)?;
    restore_captured_windows(&bench_runtime)?;

    for default in &bench.tool_defaults {
        let workspace = default.bay.to_string();
        sway::ensure_workspace_visible(&workspace)?;
        for tool_name in &default.tool_names {
            let tool_def = load_tool_definition(tool_name)?;
            let tool = instantiate_tool(&tool_def, default.bay);
            let mut runtime_state = runtime::load_tool_runtime(tool_name)?;

            let mut needs_launch = true;
            if let Some(mut state) = runtime_state.take() {
                if sway::container_exists(&state.container_id)? {
                    sway::move_container_to_workspace(&state.container_id, &workspace)?;
                    state.touch();
                    runtime::save_tool_runtime(tool_name, &state)?;
                    needs_launch = false;
                } else {
                    runtime::remove_tool_runtime(tool_name)?;
                }
            }

            if needs_launch {
                let (cid, debug_port) = launch_and_place_tool(&tool)?;
                runtime::save_tool_runtime(tool_name, &ToolRuntimeState::new(cid, debug_port))?;
            }
        }
    }

    let statuses = gather_tool_statuses(&bench)?;
    bench_runtime.captured_bays = capture_captured_bays(&statuses)?;
    runtime::save_bench_runtime(&bench_runtime)?;

    Ok(BenchRuntimeReport {
        bench,
        runtime: bench_runtime,
        tool_statuses: statuses,
    })
}

pub fn stow_active_bench(benches_dir: &Path) -> Result<BenchRuntimeReport> {
    storage::ensure_dirs()?;
    let (bench, mut bench_runtime) = load_active_bench(benches_dir)?;

    for default in &bench.tool_defaults {
        for tool_name in &default.tool_names {
            let tool_def = load_tool_definition(tool_name)?;
            let tool = instantiate_tool(&tool_def, default.bay);

            let mut runtime_state = runtime::load_tool_runtime(tool_name)?;
            if let Some(mut state) = runtime_state.take() {
                if sway::container_exists(&state.container_id)? {
                    let _ = sway::move_container_to_scratchpad(&state.container_id);
                    state.touch();
                    runtime::save_tool_runtime(tool_name, &state)?;
                    continue;
                } else {
                    runtime::remove_tool_runtime(tool_name)?;
                }
            }

            if let Some(cid) = sway::matching_container_ids(tool.sway_patterns())?
                .into_iter()
                .next()
            {
                let _ = sway::move_container_to_scratchpad(&cid);
                runtime::save_tool_runtime(tool_name, &ToolRuntimeState::new(cid, None))?;
            }
        }
    }

    let statuses = gather_tool_statuses(&bench)?;
    bench_runtime.captured_bays = capture_captured_bays(&statuses)?;
    runtime::save_bench_runtime(&bench_runtime)?;

    Ok(BenchRuntimeReport {
        bench,
        runtime: bench_runtime,
        tool_statuses: statuses,
    })
}

pub fn snapshot_current_as_bench(name: &str, benches_dir: &Path) -> Result<Bench> {
    storage::ensure_dirs()?;
    let (active_bench, _) = load_active_bench(benches_dir)?;
    let statuses = gather_tool_statuses(&active_bench)?;

    let mut workspace_tools: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for status in &statuses {
        if let Some(actual) = status.actual_bay {
            workspace_tools
                .entry(actual)
                .or_default()
                .push(status.name.clone());
        }
    }

    let workspace_nums: BTreeSet<u32> = workspace_tools.keys().copied().collect();
    let snapshots = if workspace_nums.is_empty() {
        BTreeMap::new()
    } else {
        let tree = sway::get_tree()?;
        sway::capture_workspace_snapshots(&tree, &workspace_nums)
    };

    let mut defaults = Vec::new();
    for bay in workspace_nums {
        let mut tool_names = workspace_tools.remove(&bay).unwrap_or_default();
        tool_names.sort();
        let snapshot = snapshots.get(&bay).cloned().unwrap_or_default();
        defaults.push(ToolDefault {
            bay,
            name: snapshot.name,
            tool_names,
        });
    }

    defaults.sort_by_key(|d| d.bay);
    let bench = Bench {
        name: name.to_string(),
        tool_defaults: defaults,
    };

    let path = resolve_bench_path(name, benches_dir);
    save_bench(&path, &bench)?;
    Ok(bench)
}

fn load_active_bench(benches_dir: &Path) -> Result<(Bench, BenchRuntime)> {
    match load_active_bench_optional(benches_dir)? {
        Some(data) => Ok(data),
        None => Err(anyhow!(
            "no active bench is set; run `bench activate <name>` to mark one active"
        )),
    }
}

fn load_active_bench_optional(benches_dir: &Path) -> Result<Option<(Bench, BenchRuntime)>> {
    let Some(name) = runtime::get_active_bench()? else {
        return Ok(None);
    };
    let path = resolve_bench_path(&name, benches_dir);
    if !path.exists() {
        return Err(anyhow!(
            "active bench {} not found at {}; set a new bench with `bench assemble <name>`",
            name,
            path.display()
        ));
    }
    let bench = load_bench(&path)?;
    let runtime_state = runtime::load_bench_runtime(&bench.name)?;
    Ok(Some((bench, runtime_state)))
}

fn ensure_workspace_labels(defaults: &[ToolDefault]) -> Result<()> {
    for default in defaults {
        let workspace = default.bay.to_string();
        sway::ensure_workspace_visible(&workspace)?;
        if let Some(name) = default.name.as_deref() {
            if !name.is_empty() {
                sway::rename_workspace(&workspace, name)?;
            }
        }
    }
    Ok(())
}

fn restore_captured_windows(runtime: &BenchRuntime) -> Result<()> {
    for bay in &runtime.captured_bays {
        let workspace = bay.bay.to_string();
        sway::ensure_workspace_visible(&workspace)?;
        if let Some(name) = bay.name.as_deref() {
            if !name.is_empty() {
                sway::rename_workspace(&workspace, name)?;
            }
        }
        for window_id in &bay.window_ids {
            if sway::container_exists(window_id)? {
                sway::move_container_to_workspace(window_id, &workspace)?;
            }
        }
    }
    Ok(())
}

fn gather_tool_statuses(bench: &Bench) -> Result<Vec<ToolRuntimeStatus>> {
    let windows = sway::current_windows()?;
    let id_to_workspace = map_windows_to_workspaces(&windows);

    let mut statuses = Vec::new();
    for default in &bench.tool_defaults {
        for tool_name in &default.tool_names {
            let runtime_state = runtime::load_tool_runtime(tool_name)?;
            let (container_id, debug_port, last_opened) = if let Some(state) = runtime_state {
                (
                    Some(state.container_id.clone()),
                    state.debug_port,
                    Some(state.last_opened),
                )
            } else {
                (None, None, None)
            };
            let actual_bay = container_id
                .as_ref()
                .and_then(|cid| id_to_workspace.get(cid).copied());
            statuses.push(ToolRuntimeStatus {
                name: tool_name.clone(),
                default_bay: default.bay,
                actual_bay,
                container_id,
                debug_port,
                last_opened,
            });
        }
    }

    statuses.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(statuses)
}

fn capture_captured_bays(statuses: &[ToolRuntimeStatus]) -> Result<Vec<CapturedBay>> {
    let mut workspace_windows: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for status in statuses {
        if let (Some(bay), Some(cid)) = (status.actual_bay, &status.container_id) {
            workspace_windows.entry(bay).or_default().push(cid.clone());
        }
    }

    if workspace_windows.is_empty() {
        return Ok(Vec::new());
    }

    let tree = sway::get_tree()?;
    let workspace_nums: BTreeSet<u32> = workspace_windows.keys().copied().collect();
    let snapshots = sway::capture_workspace_snapshots(&tree, &workspace_nums);

    let mut captured = Vec::new();
    for bay in workspace_nums {
        let mut window_ids = workspace_windows.remove(&bay).unwrap_or_default();
        window_ids.sort();
        let snapshot = snapshots.get(&bay).cloned().unwrap_or_default();
        captured.push(CapturedBay {
            bay,
            name: snapshot.name,
            window_ids,
        });
    }
    Ok(captured)
}

fn load_tool_definition(name: &str) -> Result<ToolDefinition> {
    storage::ensure_tools_dir()?;
    let path = storage::tool_path(name);
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read tool definition {}", path.display()))?;
    let def: ToolDefinition = serde_yaml::from_str(&data)
        .with_context(|| format!("failed to parse tool definition {}", path.display()))?;
    Ok(def)
}

fn instantiate_tool(def: &ToolDefinition, bay: u32) -> Tool {
    Tool {
        name: def.name.clone(),
        kind: def.kind,
        bay,
        state: def.state.clone(),
    }
}

fn launch_and_place_tool(tool: &Tool) -> Result<(String, Option<u16>)> {
    let workspace = tool.bay().to_string();
    let before = sway::matching_container_ids(tool.sway_patterns())?;

    let debug_port = match tool.kind() {
        apps::ToolKind::Browser => {
            let port = reserve_local_port()?;
            let config = tool.browser_config()?;
            apps::browser::launch(&config, port)?;
            Some(port)
        }
        apps::ToolKind::Terminal => {
            let config = tool.terminal_config()?;
            apps::terminal::launch(&config)?;
            None
        }
        apps::ToolKind::Zed => {
            let config = tool.zed_config()?;
            apps::zed::launch(&config)?;
            None
        }
    };

    let cid = sway::wait_for_new_container(tool.sway_patterns(), &before, Duration::from_secs(10))?;
    sway::move_container_to_workspace(&cid, &workspace)?;
    Ok((cid, debug_port))
}

fn reserve_local_port() -> Result<u16> {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

fn map_windows_to_workspaces(windows: &[WindowInfo]) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    for window in windows {
        if let Some(ws) = &window.workspace {
            if let Some(num) = parse_workspace_number(ws) {
                out.insert(window.id.clone(), num);
            }
        }
    }
    out
}

fn parse_workspace_number(label: &str) -> Option<u32> {
    let head = label.split(':').next().unwrap_or(label).trim();
    if head.is_empty() {
        return None;
    }
    head.parse::<u32>().ok()
}
