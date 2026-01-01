# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Yard is a Rust CLI application for Sway window manager workspace orchestration. It treats GUI workspaces like tmux treats terminal sessions - users define "benches" (project contexts) containing multiple "tools" (browser, terminal, editor) and can instantly switch between them with automatic window layout restoration and application state persistence.

## Build & Development Commands

```bash
cargo build --release           # Build for release
cargo build                     # Debug build
cargo install --path .          # Install to ~/.cargo/bin/
cargo run -- <subcommand>       # Run directly (e.g., cargo run -- bench list)
```

## Architecture

### Core Concepts

- **Bench**: A named workspace collection (project context) containing multiple bays
- **Bay**: A Sway workspace within a bench, holding one or more tools
- **Tool**: A reusable application instance (Browser, Terminal, or Zed) with saved configuration and runtime state

### Module Structure

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI entry point with clap subcommands |
| `model.rs` | Data structures (Bench, Tool, AssembledBench) |
| `storage.rs` | XDG Base Directory persistence (JSON files) |
| `runtime.rs` | Runtime state management |
| `bench_ops.rs` | Bench operations (create, focus, sync, info) |
| `tool_ops.rs` | Tool assembly and state synchronization |
| `layout_ops.rs` | Window layout capture/restore |
| `sway.rs` | Sway WM integration via swaymsg subprocess |
| `apps/*` | Application launchers (browser, terminal, zed) |

### Data Flow

```
CLI Input → main.rs (clap parsing)
    ↓
bench_ops.rs / tool_ops.rs (business logic)
    ↓
storage.rs (JSON persistence)
    ↓
sway.rs (swaymsg calls)
    ↓
layout_ops.rs (window collection/placement)
    ↓
apps/* (application launchers)
```

### State Persistence

All state is stored under `$XDG_DATA_HOME/yard/` (defaults to `~/.local/share/yard/`):

- `benches/<name>.json` - Bench definitions with layout state
- `tools/<name>.json` - Tool definitions with runtime state (window IDs, browser tabs, etc.)
- `focused-bench` - Text file tracking currently active bench

### Key Operations

**Focus Bench**: Saves current layout → assembles missing tools → stows non-bench windows to "temp" workspace → restores layout

**Assemble Tool**: Checks if window exists → launches app if missing (15s timeout) → saves window ID

**Sync**: Captures window-to-workspace mappings and fetches application state (browser tabs via Chrome DevTools Protocol)

## External Dependencies

Requires these applications installed:
- Sway window manager + `swaymsg`
- Chromium browser (DevTools Protocol for tab state)
- Kitty terminal
- Zed editor

## CLI Commands

```bash
yard bench create <name>    # Create empty bench
yard bench focus <name>     # Activate bench (restore layout)
yard bench sync             # Save current layout
yard bench info <name>      # Show status
yard bench list             # List all benches
yard b <cmd>                # Shorthand

yard tool craft <kind> <name>   # Create tool (browser/terminal/zed)
yard tool assemble <name>       # Launch/activate tool
yard tool sync                  # Update tool states from running apps
yard tool info <name>           # Show details
yard tool list                  # List all tools
```
