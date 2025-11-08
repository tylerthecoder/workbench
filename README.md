# Yard

A workspace manager system for Sway.

## Why?

Your work involves multiple projects—a game you're coding, a paper you're researching, a side project you're building. Each project needs specific tools in specific arrangements: browser tabs for documentation, terminals in the right directories, editors with the right files open. Traditional window managers make you rebuild these layouts manually every time you switch between projects.

**Yard treats GUI workspaces like tmux treats terminal sessions.**

Define a "bench" for each project or context. Add tools to it. Move them around with normal sway commands. Switch to a different bench and the previous bench's windows are automatically stowed to the scratchpad while the new bench's windows are restored. One bench is always focused—switching between them is instant.

**Tool state persists to disk.** When you focus a bench, missing tools are relaunched with their saved state—your browser reopens with the exact same tabs, your terminals land in the right directories, your editors load the right projects. Close everything, restart your machine, focus the bench again—it all comes back. You're not just saving window positions; you're saving the entire application state.

**No manual window management.** You define your contexts once, then switch between them with a single command. The system handles launching, positioning, and tracking everything. Your mental energy goes into your work, not into arranging windows.

## Quick Start

```bash
# Build and install
cargo install --path .

# Create a bench
yard bench create my-project
# or use the shortcut:
yard b create my-project

# Create some tools
yard tool craft browser my-browser
yard tool craft terminal my-terminal

# Edit the bench config to add tools to bays
vim ~/.local/share/yard/benches/my-project.json

# Focus the bench (assembles tools, restores layout)
yard b focus my-project

# ... do work, move windows around ...

# Sync your current layout back to disk
yard b sync

# Sync tool state (browser tabs, etc.)
yard tool sync

# See what tools are running
yard b info my-project

# View tool details and live state
yard tool info my-browser

# Switch to another bench
yard b focus other-project
```

## Requirements

- Sway and `swaymsg` on PATH
- Chromium (`chromium` binary), Kitty (`kitty`), Zed (`zed`)
- (Optional for the launcher) GTK4 development libraries and `pkg-config`

## Build

```bash
cargo build --release
```

Install
```bash
cargo install --path .
```

## Config

Yard follows the XDG base directory spec for persisted data. We refer to the resolved data directory as `$YARD_STATE` throughout the docs (defaults to `~/.local/share/yard`).

All state lives under `$YARD_STATE`:
- `benches/` - Bench definitions with embedded assembly state (JSON)
- `tools/` - Tool definitions with embedded assembly state (JSON)
- `focused-bench` - Currently focused bench name (text file)

**Note:** Benches and tools now store everything in a single JSON file per entity. This includes:
- Configuration (bays, tool assignments)
- Assembly state (window IDs, layouts)
- Metadata (created_at, last_focused_at, last_assembled_at timestamps)

---

## Tools

A **tool** is a reusable application configuration. It defines:
- What kind of app it is (browser, terminal, or zed)
- What state it should launch with (URLs, working directory, project path, etc.)

Tools are defined once and can be reused across multiple benches. They live in `$YARD_STATE/tools/<tool-name>.json`.

### Example Tool Definitions

**Browser tool** (`$YARD_STATE/tools/research-browser.json`):
```json
{
  "name": "research-browser",
  "kind": "browser",
  "created_at": "2025-11-08T12:00:00Z",
  "last_assembled_at": null,
  "state": {
    "browser": {
      "urls": [
        "https://scholar.google.com",
        "https://arxiv.org",
        "https://github.com/myorg/research-notes"
      ]
    }
  },
  "assembled": null
}
```

**Terminal tool** (`$YARD_STATE/tools/dev-terminal.json`):
```json
{
  "name": "dev-terminal",
  "kind": "terminal",
  "created_at": "2025-11-08T12:00:00Z",
  "last_assembled_at": null,
  "state": {
    "terminal": {
      "working_directory": "/home/user/projects/myapp",
      "command": "nvim src/main.rs"
    }
  },
  "assembled": null
}
```

### Crafting Tools

Create a new tool with sensible defaults:

```bash
yard tool craft browser research-browser
yard tool craft terminal dev-terminal
yard tool craft zed code-editor
```

This scaffolds a JSON file in `$YARD_STATE/tools/` with default state for that tool kind. Edit the file to customize the tool's behavior.

**GUI:** The launcher UI (if built with `--features launcher-ui`) doesn't currently support creating tools—use the CLI for now.

### Assembling Tools

**Assembling** a tool means ensuring it has a running window with a known Sway container ID.

When you assemble a tool:
1. Yard checks if the tool already has a tracked window that still exists (from its embedded `assembled` state)
2. If none found, it launches the tool with its saved state
3. The window ID is saved to the tool's JSON file along with the `last_assembled_at` timestamp

Assemble a single tool:

```bash
yard tool assemble research-browser --bay "1: Research"
```

**GUI:** Not directly exposed—tools are assembled automatically when you focus a bench.

### Tool State Sync

As you work, your tools accumulate state changes—browser tabs open/close, terminal directories change. Capture this state back to the tool definitions:

```bash
yard tool sync
```

This updates the JSON files in `$YARD_STATE/tools/` with current runtime state. For browsers, it captures all open tabs via the Chrome DevTools Protocol. For terminals and Zed, state syncing isn't implemented yet but is planned.

**GUI:** Press `Ctrl+Shift+S` in the launcher to sync tool state.

### Tool Info

View detailed information about a tool, including live state:

```bash
yard tool info research-browser
```

This shows:
- Tool metadata (name, kind, timestamps)
- Current assembly status (window ID, whether it's still running)
- Live state (e.g., current browser tabs fetched via DevTools)
- Saved state from the JSON file

### Where Tool Data Lives

Everything lives in a single JSON file: `$YARD_STATE/tools/<tool-name>.json`

This file contains:
- Tool configuration (name, kind)
- Timestamps (created_at, last_assembled_at)
- Saved state (URLs, directories, etc.)
- Assembly tracking (window ID)

---

## Benches

A **bench** is a collection of tools organized into **bays**. A bay is a named Sway workspace where tools are placed.

Benches are defined in `$YARD_STATE/benches/<bench-name>.json`.

### Example Bench

**`$YARD_STATE/benches/research.json`:**
```json
{
  "name": "research",
  "bays": [
    {
      "name": "1: Browser",
      "tool_names": ["research-browser"]
    },
    {
      "name": "2: Notes",
      "tool_names": ["notes-zed", "notes-terminal"]
    },
    {
      "name": "3: Papers",
      "tool_names": ["pdf-viewer"]
    }
  ],
  "created_at": "2025-11-08T12:00:00Z",
  "last_focused_at": null,
  "assembled": {
    "bay_windows": {}
  }
}
```

Each bay name corresponds to a Sway workspace. Tools listed under a bay will be launched and placed in that workspace when you focus the bench.

### Creating Benches

```bash
yard bench create research
# or use the shortcut:
yard b create research
```

This creates an empty bench specification at `$YARD_STATE/benches/research.json`. Edit the file to add bays and tools.

**GUI:** The launcher shows all available benches but doesn't support creating new ones—use the CLI.

### Listing Benches

```bash
yard bench list
# or:
yard b list
```

**GUI:** The launcher displays all benches in a filterable list.

### Viewing Bench Info

```bash
yard bench info research
# or:
yard b info research
```

Shows:
- Whether the bench is currently focused
- Whether all tools are assembled (running)
- Status of each tool: window ID, workspace location, whether it was recently assembled
- Timestamps (created_at, last_focused_at)

**GUI:** Select a bench in the launcher and press `i` (planned feature).

### Planning a Focus

See what will happen when you focus a bench without actually doing it:

```bash
yard bench focus-plan research
# or:
yard b focus-plan research
```

Shows:
- Which tools will be assembled (because they aren't running yet)
- Which windows will be moved where
- Which windows from other benches will be stowed to scratchpad

This is useful for understanding what changes before committing to them.

**GUI:** Not yet available.

### Focusing a Bench

**Focusing** is the main operation. It switches to a bench and brings all its tools into view.

```bash
yard bench focus research
# or:
yard b focus research
```

What happens:
1. **Save current state:** If another bench is focused, its current layout is saved to disk
2. **Ensure tools exist:** All tools defined in the target bench are assembled if not already running
3. **Stow other windows:** Windows not belonging to the target bench are moved to the scratchpad
4. **Restore layout:** Target bench windows are moved to their designated bay workspaces
5. **Mark focused:** This bench becomes the focused bench, and its `last_focused_at` timestamp is updated

**Keep existing windows visible:**

```bash
yard b focus research --no-stow
```

With `--no-stow`, the bench's windows are brought up but other windows remain visible. This is useful when you want to add a bench's tools to your current workspace without hiding everything else.

**GUI:** Select a bench and press `Enter`, or double-click.

### Syncing Bench Layout

As you rearrange windows, move them between workspaces, or open new tools manually, you'll want to capture the current arrangement:

```bash
yard bench sync
# or:
yard b sync
```

This saves the current window-to-bay mapping to the bench's JSON file. Next time you focus the bench, windows will be restored to these positions.

**Note:** This only syncs which windows are in which bays. To sync tool state (browser tabs, etc.), use `yard tool sync`.

**GUI:** Press `Ctrl+S` in the launcher.

### Focused Bench

Check which bench is currently focused:

```bash
yard bench focused
# or:
yard b focused
```

**GUI:** The focused bench is highlighted in the launcher.

### Where Bench Data Lives

Everything lives in a single JSON file: `$YARD_STATE/benches/<bench-name>.json`

This file contains:
- Bench configuration (name, bays, tool assignments)
- Timestamps (created_at, last_focused_at)
- Assembled layout (current window-to-bay mapping)

The currently focused bench name is stored in: `$YARD_STATE/focused-bench`

---

## Launcher UI

`yard launcher` (built with `--features launcher-ui`) opens a fast, keyboard-first GTK window for switching benches:

- **Search:** Start typing to filter bench names; `Esc` clears or closes
- **Navigation:** Arrow keys move the selection; `Tab` toggles between search and list
- **Actions:**
  - `Enter` - Focus the selected bench
  - `Ctrl+S` - Sync the current focused bench layout
- **Status:** The footer shows the last action result; errors bubble up in the same bar

When running from the repository:

```bash
cargo run --features launcher-ui -- launcher
```

Bind this to a hotkey in your Sway config for instant access:

```
bindsym $mod+Space exec /path/to/yard launcher
```

---

## Command Reference

### Bench Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `yard bench list` | `yard b list` | List all benches |
| `yard bench create <name>` | `yard b create <name>` | Create a new bench |
| `yard bench focus <name>` | `yard b focus <name>` | Focus a bench (assemble tools, restore layout) |
| `yard bench focus <name> --no-stow` | `yard b focus <name> --no-stow` | Focus bench without stowing other windows |
| `yard bench focus-plan <name>` | `yard b focus-plan <name>` | Preview what focusing will do |
| `yard bench info <name>` | `yard b info <name>` | Show bench status and tool information |
| `yard bench sync` | `yard b sync` | Sync current layout to disk |
| `yard bench focused` | `yard b focused` | Show currently focused bench |

### Tool Commands

| Command | Description |
|---------|-------------|
| `yard tool list` | List all tools |
| `yard tool craft <kind> <name>` | Create a new tool (kind: browser, terminal, zed) |
| `yard tool info <name>` | Show tool details and live state |
| `yard tool assemble <name> [--bay <bay>]` | Ensure a tool is running |
| `yard tool sync` | Sync all tool states to disk |

### Other Commands

| Command | Description |
|---------|-------------|
| `yard launcher` | Open the GTK launcher UI |
