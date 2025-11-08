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
mod layout_ops;
mod model;
mod storage;
mod sway;
mod tool_ops;

use bench_ops::{
    active_bench, assemble_tool, craft_tool, create_bench, focus, info, list_benches, list_tools,
    stow, sync_layout, sync_tool_state, BenchInfo, BenchReport, ToolStatus,
};
use clap::{Parser, Subcommand, ValueEnum};
use owo_colors::OwoColorize;

use crate::apps::ToolKind;

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
    /// List known tools
    ListTools,
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
                "{} {}",
                "✨".bold().bright_magenta(),
                format!("Bench '{}' created!", bench.name).bold()
            );
            println!(
                "  {} {}",
                "📄".bright_cyan(),
                format!("Saved to {}", path.display()).italic()
            );
            println!(
                "  {} {}",
                "✏️".bright_yellow(),
                "Edit the YAML to add bays whenever you're ready!".dimmed()
            );
        }
        Commands::ListBenches => {
            let benches = list_benches()?;
            println!(
                "{} {}",
                "📚".bold().bright_magenta(),
                "Available benches".bold()
            );
            if benches.is_empty() {
                println!(
                    "  {}",
                    "No benches found. Try `bench create <name>` to get started!".dimmed()
                );
            } else {
                for (idx, bench) in benches.iter().enumerate() {
                    println!(
                        "  {} {}",
                        "•".bright_cyan(),
                        format!("{:>2}. {}", idx + 1, bench).bold()
                    );
                }
            }
        }
        Commands::ListTools => {
            let tools = list_tools()?;
            println!(
                "{} {}",
                "🧰".bold().bright_magenta(),
                "Available tools".bold()
            );
            if tools.is_empty() {
                println!(
                    "  {}",
                    "No tools found. Try `bench craft-tool <kind> <name>` to scaffold one."
                        .dimmed()
                );
            } else {
                for (idx, tool) in tools.iter().enumerate() {
                    println!(
                        "  {} {}",
                        "•".bright_cyan(),
                        format!("{:>2}. {}", idx + 1, tool).bold()
                    );
                }
            }
        }
        Commands::Stow { bench } => {
            println!(
                "{} {}",
                "🧳".bold().bright_blue(),
                format!("Stowing bench '{}' into the scratchpad…", bench).bold()
            );
            let report = stow(&bench)?;
            println!(
                "{} {}",
                "✨".bold().bright_magenta(),
                format!("Bench '{}' tucked away!", bench).bold()
            );
            print_bench_report(&report);
        }
        Commands::Focus { bench } => {
            println!(
                "{} {}",
                "🎯".bold().bright_green(),
                format!("Bringing bench '{}' into focus…", bench).bold()
            );
            let report = focus(&bench)?;
            println!(
                "{} {}",
                "👀".bold().bright_cyan(),
                format!("Bench '{}' is front-and-center!", bench).bold()
            );
            print_bench_report(&report);
        }
        Commands::AssembleTool { tool, bay } => {
            println!(
                "{} {}",
                "🔁".bold().bright_yellow(),
                format!("Ensuring tool '{}' is running…", tool).bold()
            );
            let bay_name = bay.as_deref().unwrap_or("default");
            let status = assemble_tool(&tool, bay_name)?;
            println!("{} {}", "✅".bold().bright_green(), "Tool status:".bold());
            print_tool_status(&status);
        }
        Commands::SyncLayout => {
            let _assembled = sync_layout()?;
            let bench_name = active_bench()?.ok_or_else(|| anyhow::anyhow!("no active bench"))?;
            println!(
                "{} {}",
                "🧭".bold().bright_cyan(),
                format!("Captured current layout for '{}'.", bench_name).bold()
            );
        }
        Commands::SyncToolState => {
            sync_tool_state()?;
            println!(
                "{} {}",
                "🔄".bold().bright_green(),
                "Captured tool state from running apps.".bold()
            );
        }
        Commands::CraftTool { kind, name } => {
            let definition = craft_tool(ToolKind::from(kind), &name)?;
            println!(
                "{} {}",
                "🪄".bold().bright_magenta(),
                format!(
                    "Crafted tool '{}' ({:?}).",
                    definition.name, definition.kind
                )
                .bold()
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
            Some(name) => println!(
                "{} {}",
                "🎯".bold().bright_green(),
                format!("Active bench: {}", name).bold()
            ),
            None => println!(
                "{} {}",
                "💤".bold().bright_black(),
                "No bench is currently active.".dimmed()
            ),
        },
    }
    Ok(())
}

fn print_bench_report(report: &BenchReport) {
    println!();
    heading("🧰", &format!("Bench {}", report.bench.name));

    if report.assembled.bay_windows.is_empty() {
        note_line(
            "📭",
            "No tracked bays yet. Focus or assemble a bench to capture window IDs.",
        );
    } else {
        section("🗂️", "Tracked Bays");
        for (bay, windows) in &report.assembled.bay_windows {
            let bay_label = format!(
                "{} ({} window{})",
                bay,
                windows.len(),
                if windows.len() == 1 { "" } else { "s" }
            );
            println!("  {} {}", "•".bright_cyan(), bay_label.bold());
            for window in windows {
                println!("     {}", format!("window {}", window).dimmed());
            }
        }
    }

    if report.statuses.is_empty() {
        note_line("🛠️", "No tools registered with this bench yet.");
    } else {
        section("🛠️", "Tools");
        for status in &report.statuses {
            print_tool_status(status);
        }
    }
}

fn print_bench_info(info: &BenchInfo) {
    println!();
    heading("🧾", &format!("Bench {}", info.bench.name));

    state_line(
        if info.active { "🎯" } else { "💤" },
        if info.active {
            "Active: yes"
        } else {
            "Active: no"
        },
        info.active,
    );
    state_line(
        if info.assembled { "✅" } else { "⚠️" },
        if info.assembled {
            "Assembled: yes"
        } else {
            "Assembled: no"
        },
        info.assembled,
    );

    if info.statuses.is_empty() {
        note_line("🛠️", "No tools tracked yet.");
        return;
    }

    section("🛠️", "Tools");
    for status in &info.statuses {
        print_tool_status(status);
    }
}

fn print_tool_status(status: &ToolStatus) {
    let has_window = status.window_id.is_some();
    let icon = if has_window {
        if status.launched {
            format!("{}", "🚀".bold().bright_green())
        } else {
            format!("{}", "✅".bold().bright_green())
        }
    } else {
        format!("{}", "⚠️".bold().bright_red())
    };

    let name = format!("{}", status.name.as_str().bold());
    let bay = format!("{}", status.bay.as_str().bold().bright_cyan());
    let at = format!("{}", "@".bright_black());
    let arrow = format!("{}", "→".bright_black());
    let window_id = status.window_id.as_deref().unwrap_or("<missing>");
    let window_text = if has_window {
        format!("{}", window_id.bright_green())
    } else {
        format!("{}", window_id.bold().bright_red())
    };
    let workspace = status.workspace.as_deref().unwrap_or("<unplaced>");
    let workspace_text = format!("{}", workspace.italic().bright_blue());

    println!(
        "  {} {} {} {} {} {} ({})",
        icon, name, at, bay, arrow, window_text, workspace_text
    );

    if status.launched {
        println!(
            "    {}",
            "✨ Launched during this command; we'll reuse it next time.".dimmed()
        );
    } else if !has_window {
        println!(
            "    {}",
            "🔁 Window missing; the next assemble will relaunch this tool.".dimmed()
        );
    }
}

fn heading(icon: &str, text: &str) {
    println!(
        "{} {}",
        icon.bold().bright_magenta(),
        text.bold().underline()
    );
}

fn section(icon: &str, text: &str) {
    println!("{} {}", icon.bold().bright_cyan(), text.bold());
}

fn note_line(icon: &str, text: &str) {
    println!("  {} {}", icon.bright_black(), text.dimmed());
}

fn state_line(icon: &str, text: &str, highlight: bool) {
    if highlight {
        println!("  {} {}", icon.bold().bright_green(), text.bold());
    } else {
        println!("  {} {}", icon.bold().bright_black(), text.dimmed());
    }
}
