use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".local/share")
        })
        .join("bench")
}

pub fn benches_dir() -> PathBuf {
    data_dir().join("benches")
}

pub fn tools_dir() -> PathBuf {
    data_dir().join("tools")
}

pub fn runtime_dir() -> PathBuf {
    data_dir().join("runtime")
}

pub fn ensure_tools_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(tools_dir())
}

pub fn ensure_runtime_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(runtime_dir())
}

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(benches_dir())?;
    ensure_tools_dir()?;
    ensure_runtime_dir()
}

pub fn bench_path(name: &str) -> PathBuf {
    benches_dir().join(format!("{}.yml", name))
}

pub fn tool_path(name: &str) -> PathBuf {
    let sanitized = sanitize_name(name);
    tools_dir().join(format!("{}.yml", sanitized))
}

pub fn tool_runtime_path(name: &str) -> PathBuf {
    let sanitized = sanitize_name(name);
    tools_dir().join(format!("{}.runtime.json", sanitized))
}

pub fn bench_runtime_path(name: &str) -> PathBuf {
    let sanitized = sanitize_name(name);
    runtime_dir().join(format!("{}.json", sanitized))
}

pub fn active_bench_path() -> PathBuf {
    runtime_dir().join(".active_bench")
}

fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .map(|c| if matches!(c, '/' | '\\') { '_' } else { c })
        .collect()
}
