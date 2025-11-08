use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::model::BenchRuntime;
use crate::storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRuntimeState {
    pub container_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub last_opened: OffsetDateTime,
}

impl ToolRuntimeState {
    pub fn new(container_id: String) -> Self {
        Self {
            container_id,
            last_opened: OffsetDateTime::now_utc(),
        }
    }

    pub fn touch(&mut self) {
        self.last_opened = OffsetDateTime::now_utc();
    }
}

fn ensure_tool_runtime_dir() -> Result<PathBuf> {
    storage::ensure_tools_dir().context("failed to ensure tools directory")?;
    Ok(storage::tools_dir())
}

fn tool_runtime_file(tool_name: &str) -> PathBuf {
    storage::tool_runtime_path(tool_name)
}

pub fn load_tool_runtime(tool_name: &str) -> Result<Option<ToolRuntimeState>> {
    let _ = ensure_tool_runtime_dir()?;
    let path = tool_runtime_file(tool_name);
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path)
        .with_context(|| format!("failed to read runtime state from {}", path.display()))?;
    let state = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse runtime state {}", path.display()))?;
    Ok(Some(state))
}

pub fn save_tool_runtime(tool_name: &str, state: &ToolRuntimeState) -> Result<()> {
    let dir = ensure_tool_runtime_dir()?;
    let path = tool_runtime_file(tool_name);
    if let Some(parent) = path.parent() {
        if parent != dir {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    let data =
        serde_json::to_string_pretty(state).context("failed to serialize tool runtime state")?;
    fs::write(&path, data)
        .with_context(|| format!("failed to write runtime state to {}", path.display()))?;
    Ok(())
}

pub fn remove_tool_runtime(tool_name: &str) -> Result<()> {
    let path = tool_runtime_file(tool_name);
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove runtime state {}", path.display()))?;
    }
    Ok(())
}

fn ensure_runtime_dir() -> Result<PathBuf> {
    storage::ensure_runtime_dir().context("failed to ensure runtime directory")?;
    Ok(storage::runtime_dir())
}

fn bench_runtime_file(bench_name: &str) -> PathBuf {
    storage::bench_runtime_path(bench_name)
}

pub fn load_bench_runtime(bench_name: &str) -> Result<BenchRuntime> {
    let _ = ensure_runtime_dir()?;
    let path = bench_runtime_file(bench_name);
    if !path.exists() {
        return Ok(new_bench_runtime(bench_name));
    }
    let data = fs::read_to_string(&path)
        .with_context(|| format!("failed to read bench runtime from {}", path.display()))?;
    let runtime = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse bench runtime {}", path.display()))?;
    Ok(runtime)
}

pub fn save_bench_runtime(runtime: &BenchRuntime) -> Result<()> {
    let _ = ensure_runtime_dir()?;
    let path = bench_runtime_file(&runtime.name);
    let data =
        serde_json::to_string_pretty(runtime).context("failed to serialize bench runtime state")?;
    fs::write(&path, data)
        .with_context(|| format!("failed to write bench runtime to {}", path.display()))?;
    Ok(())
}

pub fn new_bench_runtime(name: &str) -> BenchRuntime {
    BenchRuntime {
        name: name.to_string(),
        captured_bays: Vec::new(),
    }
}

pub fn set_active_bench(name: &str) -> Result<()> {
    let dir = ensure_runtime_dir()?;
    let path = storage::active_bench_path();
    if let Some(parent) = path.parent() {
        if parent != dir {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    fs::write(&path, name)
        .with_context(|| format!("failed to write active bench to {}", path.display()))?;
    Ok(())
}

pub fn get_active_bench() -> Result<Option<String>> {
    let path = storage::active_bench_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path)
        .with_context(|| format!("failed to read active bench from {}", path.display()))?;
    let name = data.trim().to_string();
    if name.is_empty() {
        Ok(None)
    } else {
        Ok(Some(name))
    }
}

pub fn clear_active_bench() -> Result<()> {
    let path = storage::active_bench_path();
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove active bench {}", path.display()))?;
    }
    Ok(())
}
