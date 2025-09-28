mod apps;
mod bench_ops;
#[cfg(feature = "launcher-ui")]
mod launcher_ui;
#[cfg(not(feature = "launcher-ui"))]
mod launcher_ui {
    use anyhow::{anyhow, Result};

    pub fn run() -> Result<()> {
        Err(anyhow!(
            "launcher UI disabled; rebuild with `--features launcher-ui`"
        ))
    }
}
mod model;
mod runtime;
mod storage;
mod sway;

use bench_ops::{
    assemble_active_bench, benches_dir_or_default, create_bench, current_runtime_snapshot,
    list_benches, set_active_bench, snapshot_current_as_bench, stow_active_bench,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::model::BenchRuntime;

#[derive(Parser, Debug)]
#[command(name = "bench")]
#[command(about = "Bench: Sway bench/bay/tool manager (Rust)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a bench from the current window layout (adds defaults only)
    Create {
        name: String,
        #[arg(long)]
        benches_dir: Option<PathBuf>,
    },
    /// Assemble the currently active bench (will activate the chosen bench)
    Assemble {
        bench: String,
        #[arg(long)]
        benches_dir: Option<PathBuf>,
    },
    /// Stow the currently active bench
    Stow {
        bench: String,
        #[arg(long)]
        benches_dir: Option<PathBuf>,
    },
    /// Print the current runtime snapshot
    Current,
    /// Snapshot the current window layout into a new bench file
    SnapshotCurrent {
        name: String,
        #[arg(long)]
        benches_dir: Option<PathBuf>,
    },
    /// List bench YAMLs under the benches directory
    ListBenches {
        #[arg(long)]
        benches_dir: Option<PathBuf>,
    },
    /// List current Sway workspaces
    ListWorkspaces,
    /// Launch the optional GTK launcher UI
    Launcher,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Create { name, benches_dir } => {
            let benches_dir = benches_dir_or_default(benches_dir);
            let bench = create_bench(&name, &benches_dir)?;
            println!("created bench '{}'", bench.name);
        }
        Commands::Assemble { bench, benches_dir } => {
            let benches_dir = benches_dir_or_default(benches_dir);
            set_active_bench(&bench, &benches_dir)?;
            let report = assemble_active_bench(&benches_dir)?;
            print_report(report);
        }
        Commands::Stow { bench, benches_dir } => {
            let benches_dir = benches_dir_or_default(benches_dir);
            set_active_bench(&bench, &benches_dir)?;
            let report = stow_active_bench(&benches_dir)?;
            print_report(report);
        }
        Commands::Current => {
            let runtime = current_runtime_snapshot()?;
            print_runtime(runtime);
        }
        Commands::SnapshotCurrent { name, benches_dir } => {
            let benches_dir = benches_dir_or_default(benches_dir);
            let bench = snapshot_current_as_bench(&name, &benches_dir)?;
            println!(
                "saved new bench '{}' with {} defaults",
                bench.name,
                bench.tool_defaults.len()
            );
        }
        Commands::ListBenches { benches_dir } => {
            let benches_dir = benches_dir_or_default(benches_dir);
            for bench in list_benches(&benches_dir)? {
                println!("{}", bench);
            }
        }
        Commands::ListWorkspaces => {
            let names = sway::list_workspaces()?;
            for n in names {
                println!("{}", n);
            }
        }
        Commands::Launcher => {
            launcher_ui::run()?;
        }
    }
    Ok(())
}

fn print_runtime(runtime: BenchRuntime) {
    println!("Bench runtime snapshot for {}", runtime.name);
    if runtime.captured_bays.is_empty() {
        println!("Captured bays: <none>");
        return;
    }
    println!("Captured bays:");
    for bay in runtime.captured_bays {
        let title = bay.name.clone().unwrap_or_default();
        println!("  Bay {} {}", bay.bay, title);
        for id in bay.window_ids {
            println!("    window id {}", id);
        }
    }
}

fn print_report(report: bench_ops::BenchRuntimeReport) {
    println!("Bench: {}", report.bench.name);
    if report.runtime.captured_bays.is_empty() {
        println!("Captured bays: <none>");
    } else {
        println!("Captured bays:");
        for bay in &report.runtime.captured_bays {
            let title = bay.name.clone().unwrap_or_default();
            println!("  Bay {} {}", bay.bay, title);
            for id in &bay.window_ids {
                println!("    window id {}", id);
            }
        }
    }

    if report.tool_statuses.is_empty() {
        println!("Tools: <none>");
        return;
    }

    println!("Tools:");
    for status in &report.tool_statuses {
        let drift = if status.is_drifted() { "*" } else { " " };
        let actual = status
            .actual_bay
            .map(|n| format!("{}", n))
            .unwrap_or_else(|| "(missing)".to_string());
        let mut details = Vec::new();
        if let Some(cid) = &status.container_id {
            details.push(format!("cid {}", cid));
        }
        if let Some(port) = status.debug_port {
            details.push(format!("debug {}", port));
        }
        if let Some(time) = status.last_opened {
            details.push(format!("last_opened {}", time));
        }
        let details = if details.is_empty() {
            String::new()
        } else {
            format!(" [{}]", details.join(", "))
        };
        println!(
            "{} {} default={} actual={}{}",
            drift, status.name, status.default_bay, actual, details
        );
    }
}
