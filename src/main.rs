mod apps;
mod assembly;
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
mod storage;
mod sway;

use bench_ops::{
    active_bench, assemble, assemble_tool, craft_tool, create_bench, focus, info, list_benches,
    stow, sync_layout, sync_tool_state, BenchInfo, BenchReport,
};
use clap::{Parser, Subcommand, ValueEnum};

use crate::apps::ToolKind;
use crate::assembly::ToolStatus;

#[derive(Parser, Debug)]
#[command(name = "bench")]
#[command(about = "Bench: Sway bench/bay/tool manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create an empty bench specification
    Create { name: String },
    /// List known benches
    ListBenches,
    /// Assemble a bench, ensuring windows exist and tracking them
    Assemble { bench: String },
    /// Stow a bench's windows into the scratchpad
    Stow { bench: String },
    /// Focus a bench, restoring its layout
    Focus { bench: String },
    /// Ensure a single tool is running
    #[command(name = "assemble-tool")]
    AssembleTool {
        tool: String,
        #[arg(long)]
        bay: Option<String>,
    },
    /// Sync the active bench layout back to disk
    #[command(name = "sync-layout")]
    SyncLayout,
    /// Sync tool state back to disk (tabs, etc.)
    #[command(name = "sync-tool-state")]
    SyncToolState,
    /// Scaffold a new tool definition
    #[command(name = "craft-tool")]
    CraftTool { kind: ToolKindArg, name: String },
    /// Display bench details and runtime status
    Info { bench: String },
    /// Launch the optional GTK launcher UI
    Launcher,
    /// Print the currently active bench name, if any
    Active,
}

#[derive(Clone, Debug, ValueEnum)]
enum ToolKindArg {
    Browser,
    Terminal,
    Zed,
}

impl From<ToolKindArg> for ToolKind {
    fn from(value: ToolKindArg) -> Self {
        match value {
            ToolKindArg::Browser => ToolKind::Browser,
            ToolKindArg::Terminal => ToolKind::Terminal,
            ToolKindArg::Zed => ToolKind::Zed,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Create { name } => {
            let bench = create_bench(&name)?;
            let path = crate::storage::bench_path(&bench.name);
            println!(
                "created bench '{}' at {}; edit the YAML to add bays",
                bench.name,
                path.display()
            );
        }
        Commands::ListBenches => {
            for bench in list_benches()? {
                println!("{}", bench);
            }
        }
        Commands::Assemble { bench } => {
            let report = assemble(&bench)?;
            print_bench_report(&report);
        }
        Commands::Stow { bench } => {
            let report = stow(&bench)?;
            print_bench_report(&report);
        }
        Commands::Focus { bench } => {
            let report = focus(&bench)?;
            print_bench_report(&report);
        }
        Commands::AssembleTool { tool, bay } => {
            let status = assemble_tool(&tool, bay)?;
            print_tool_status(&status);
        }
        Commands::SyncLayout => {
            let bench = sync_layout()?;
            println!("synced layout for bench '{}'", bench.name);
        }
        Commands::SyncToolState => {
            sync_tool_state()?;
            println!("synced tool state");
        }
        Commands::CraftTool { kind, name } => {
            let definition = craft_tool(ToolKind::from(kind), &name)?;
            println!(
                "wrote tool definition '{}' ({:?})",
                definition.name, definition.kind
            );
        }
        Commands::Info { bench } => {
            let details = info(&bench)?;
            print_bench_info(&details);
        }
        Commands::Launcher => {
            launcher_ui::run()?;
        }
        Commands::Active => match active_bench()? {
            Some(name) => println!("{}", name),
            None => println!("<no active bench>"),
        },
    }
    Ok(())
}

fn print_bench_report(report: &BenchReport) {
    println!("Bench: {}", report.bench.name);
    if report.assembled.bay_windows.is_empty() {
        println!("Tracked bays: <none>");
    } else {
        println!("Tracked bays:");
        for (bay, windows) in &report.assembled.bay_windows {
            println!("  {}:", bay);
            for window in windows {
                println!("    window {}", window);
            }
        }
    }

    if report.statuses.is_empty() {
        println!("Tools: <none>");
    } else {
        println!("Tools:");
        for status in &report.statuses {
            print_tool_status(status);
        }
    }
}

fn print_bench_info(info: &BenchInfo) {
    println!("Bench: {}", info.bench.name);
    println!("Active: {}", if info.active { "yes" } else { "no" });
    println!("Assembled: {}", if info.assembled { "yes" } else { "no" });
    if info.statuses.is_empty() {
        println!("Tools: <none>");
        return;
    }
    println!("Tools:");
    for status in &info.statuses {
        print_tool_status(status);
    }
}

fn print_tool_status(status: &ToolStatus) {
    let window = status
        .window_id
        .as_ref()
        .map(|id| id.as_str())
        .unwrap_or("<missing>");
    let workspace = status
        .workspace
        .as_ref()
        .map(|ws| ws.as_str())
        .unwrap_or("<unplaced>");
    let flag = if status.launched { "*" } else { " " };
    println!(
        "{} {} @ {} -> window {} (workspace {})",
        flag, status.name, status.bay, window, workspace
    );
}
