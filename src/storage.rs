use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::model::{AssembledBench, AssembledTool, Bench, ToolDefinition};

pub fn data_dir() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".local/share")
        })
        .join("yard")
}

pub fn benches_dir() -> PathBuf {
    data_dir().join("benches")
}

pub fn tools_dir() -> PathBuf {
    data_dir().join("tools")
}

pub fn focused_bench_path() -> PathBuf {
    data_dir().join("focused-bench")
}

pub fn ensure_dirs() -> Result<()> {
    fs::create_dir_all(benches_dir()).context("failed to create benches directory")?;
    fs::create_dir_all(tools_dir()).context("failed to create tools directory")?;
    Ok(())
}

pub fn bench_path(name: &str) -> PathBuf {
    benches_dir().join(format!("{}.json", sanitize_name(name)))
}

pub fn tool_path(name: &str) -> PathBuf {
    tools_dir().join(format!("{}.json", sanitize_name(name)))
}

pub fn read_bench(name: &str) -> Result<Bench> {
    let path = bench_path(name);
    read_json(&path)
}

pub fn write_bench(bench: &Bench) -> Result<()> {
    let path = bench_path(&bench.name);
    write_json(&path, bench)
}

pub fn list_bench_names() -> Result<Vec<String>> {
    let mut benches = Vec::new();
    if let Ok(entries) = fs::read_dir(benches_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    benches.push(name.to_string());
                }
            }
        }
    }
    benches.sort();
    Ok(benches)
}

pub fn list_tool_names() -> Result<Vec<String>> {
    let mut tools = Vec::new();
    if let Ok(entries) = fs::read_dir(tools_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    tools.push(name.to_string());
                }
            }
        }
    }
    tools.sort();
    Ok(tools)
}

pub fn read_tool(name: &str) -> Result<ToolDefinition> {
    let path = tool_path(name);
    read_json(&path)
}

pub fn write_tool(def: &ToolDefinition) -> Result<()> {
    let path = tool_path(&def.name);
    write_json(&path, def)
}

// Compatibility functions for accessing assembled state
pub fn read_assembled_bench(name: &str) -> Result<Option<AssembledBench>> {
    let bench = read_bench(name)?;
    Ok(Some(bench.assembled))
}

pub fn write_assembled_bench(name: &str, assembled: &AssembledBench) -> Result<()> {
    let mut bench = read_bench(name)?;
    bench.assembled = assembled.clone();
    write_bench(&bench)
}

pub fn read_assembled_tool(name: &str) -> Result<Option<AssembledTool>> {
    let tool = read_tool(name)?;
    Ok(tool.assembled)
}

pub fn write_assembled_tool(name: &str, assembled: &AssembledTool) -> Result<()> {
    let mut tool = read_tool(name)?;
    tool.assembled = Some(assembled.clone());
    tool.last_assembled_at = Some(time::OffsetDateTime::now_utc());
    write_tool(&tool)
}

pub fn read_focused_bench() -> Result<Option<String>> {
    let path = focused_bench_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path)
        .with_context(|| format!("failed to read focused bench {}", path.display()))?;
    let name = data.trim().to_string();
    if name.is_empty() {
        Ok(None)
    } else {
        Ok(Some(name))
    }
}

pub fn write_focused_bench(name: &str) -> Result<()> {
    let path = focused_bench_path();
    ensure_parent(&path)?;
    fs::write(&path, name)
        .with_context(|| format!("failed to write focused bench {}", path.display()))
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON {}", path.display()))?;
    serde_json::from_str(&data).with_context(|| format!("failed to parse JSON {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    ensure_parent(path)?;
    let data = serde_json::to_string_pretty(value).context("failed to serialize JSON value")?;
    fs::write(path, data).with_context(|| format!("failed to write JSON {}", path.display()))
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(())
}

fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .map(|c| if matches!(c, '/' | '\\') { '_' } else { c })
        .collect()
}
