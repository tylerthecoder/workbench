# Bench

A workspace manager system for sway.

## Why?

Your work involves multiple projects—a game you're coding, a paper you're researching, a side project you're building. Each project needs specific tools in specific arrangements: browser tabs for documentation, terminals in the right directories, editors with the right files open. Traditional window managers make you rebuild these layouts manually every time you switch between projects.

**Bench treats GUI workspaces like tmux treats terminal sessions.**

Define a "bench" for each project or context. Add tools to it by launching them from the menu. Move them around with normal sway commands. Switch to a different bench and the window state is saved into the scratchpad. Reboot your computer without loosing the state of the windows!

**Tool state persists to disk.** When you focus a bench, missing tools are relaunched with their saved state—your browser reopens with the exact same tabs, your terminals land in the right directories, your editors load the right projects. Close everything, restart your machine, focus the bench again—it all comes back. You're not just saving window positions; you're saving the entire application state.

**No manual window management.** You define your contexts once, then switch between them with a single command. The system handles launching, positioning, and tracking everything. Your mental energy goes into your work, not into arranging windows.

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

Bench follows the XDG base directory spec for persisted data. We refer to the resolved data directory as `$BENCH_STATE` throughout the docs (defaults to `~/.local/share/bench`).

All state lives under `$BENCH_STATE`:
- `benches/` - Bench definitions (YAML)
- `tools/` - Tool definitions (YAML)
- `assembled-benches/` - Current window layouts (JSON)
- `assembled-tools/` - Tracked window IDs (JSON)
- `active-bench` - Currently active bench name (text file)

---

## Tools

A **tool** is a reusable application configuration. It defines:
- What kind of app it is (browser, terminal, or zed)
- What state it should launch with (URLs, working directory, project path, etc.)

Tools are defined once and can be reused across multiple benches. They live in `$BENCH_STATE/tools/<tool-name>.yml`.

### Example Tool Definitions

**Browser tool** (`$BENCH_STATE/tools/research-browser.yml`):
```yml
name: research-browser
kind: browser
state:
  urls:
    - "https://scholar.google.com"
    - "https://arxiv.org"
    - "https://github.com/myorg/research-notes"
```

**Terminal tool** (`$BENCH_STATE/tools/dev-terminal.yml`):
```yml
name: dev-terminal
kind: terminal
state:
  working_directory: "/home/user/projects/myapp"
  command: "nvim src/main.rs"
```

**Zed tool** (`$BENCH_STATE/tools/code-editor.yml`):
```yml
name: code-editor
kind: zed
state:
  projects:
    - "/home/user/projects/myapp"
```

### Crafting Tools

Create a new tool with sensible defaults:

```bash
bench craft-tool browser research-browser
bench craft-tool terminal dev-terminal
bench craft-tool zed code-editor
```

This scaffolds a YAML file in `$BENCH_STATE/tools/` with default state for that tool kind. Edit the file to customize the tool's behavior.

**GUI:** The launcher UI (if built with `--features launcher-ui`) doesn't currently support creating tools—use the CLI for now.

### Assembling Tools

**Assembling** a tool means ensuring it has a running window with a known Sway container ID.

When you assemble a tool:
1. Bench checks if the tool already has a tracked window that still exists
2. If not, it searches for an existing window matching the tool's pattern
3. If none found, it launches the tool with its saved state
4. The window ID is saved to `$BENCH_STATE/assembled-tools/<tool-name>.json`

Assemble a single tool:

```bash
bench assemble-tool research-browser --bay "1: Research"
```

**GUI:** Not directly exposed—tools are assembled automatically when you focus a bench.

### Tool State Sync

As you work, your tools accumulate state changes—browser tabs open/close, terminal directories change. Capture this state back to the tool definitions:

```bash
bench sync-tool-state
```

This updates the YAML files in `$BENCH_STATE/tools/` with current runtime state. For browsers, it captures all open tabs via the Chrome DevTools Protocol. For terminals and Zed, state syncing isn't implemented yet but is planned.

**GUI:** Press `Ctrl+Shift+S` in the launcher to sync tool state.

### Where Tool Data Lives

- **Definition:** `$BENCH_STATE/tools/<tool-name>.yml` - What the tool is and its persistent state
- **Assembly tracking:** `$BENCH_STATE/assembled-tools/<tool-name>.json` - Which window ID currently represents this tool (e.g., `{"window_id": "12345"}`)

The assembly tracking file exists only while the tool is running. If you close the tool, the file remains but the window ID becomes stale. Next time you focus a bench that needs this tool, Bench will detect the stale ID and relaunch.

---

## Benches

A **bench** is a collection of tools organized into **bays**. A bay is a named Sway workspace where tools are placed.

Benches are defined in `$BENCH_STATE/benches/<bench-name>.yml`.

### Example Bench

**`$BENCH_STATE/benches/research.yml`:**
```yml
name: research
bays:
  - name: "1: Browser"
    tool_names:
      - research-browser
  - name: "2: Notes"
    tool_names:
      - notes-zed
      - notes-terminal
  - name: "3: Papers"
    tool_names:
      - pdf-viewer
```

Each bay name corresponds to a Sway workspace. Tools listed under a bay will be launched and placed in that workspace when you focus the bench.

### Creating Benches

```bash
bench create research
```

This creates an empty bench specification at `$BENCH_STATE/benches/research.yml`. Edit the file to add bays and tools.

**GUI:** The launcher shows all available benches but doesn't support creating new ones—use the CLI.

### Listing Benches

```bash
bench list-benches
```

**GUI:** The launcher displays all benches in a filterable list.

### Viewing Bench Info

```bash
bench info research
```

Shows:
- Whether the bench is currently active
- Whether all tools are assembled (running)
- Status of each tool: window ID, workspace location, whether it was recently launched

**GUI:** Select a bench in the launcher and press `i` (planned feature).

### Focusing a Bench

**Focusing** is the main operation. It activates a bench and brings all its tools into view.

```bash
bench focus research
```

What happens:
1. **Save current state:** If another bench is active, its current layout is saved to disk
2. **Ensure tools exist:** All tools defined in the target bench are launched if not already running
3. **Stow other windows:** Windows not belonging to the bench are moved to the scratchpad
4. **Restore layout:** Windows are moved to their designated bay workspaces
5. **Mark active:** This bench becomes the active bench

**GUI:** Select a bench and press `Enter`, or double-click.

### Stowing a Bench

**Stowing** hides a bench's windows without closing them.

```bash
bench stow research
```

All windows belonging to the bench are moved to the scratchpad. The processes keep running—you can focus the bench again later to restore them.

**GUI:** Select a bench and press `Shift+Enter`.

### Syncing Bench Layout

As you rearrange windows, move them between workspaces, or open new tools manually, you'll want to capture the current arrangement:

```bash
bench sync-layout
```

This saves the current window-to-bay mapping to `$BENCH_STATE/assembled-benches/<bench-name>.json`. Next time you focus the bench, windows will be restored to these positions.

**Note:** This only syncs which windows are in which bays. To sync tool state (browser tabs, etc.), use `bench sync-tool-state`.

**GUI:** Press `Ctrl+S` in the launcher.

### Active Bench

Check which bench is currently active:

```bash
bench active
```

**GUI:** The active bench is highlighted in the launcher.

### Where Bench Data Lives

- **Definition:** `$BENCH_STATE/benches/<bench-name>.yml` - The bench structure: which tools go in which bays
- **Assembled layout:** `$BENCH_STATE/assembled-benches/<bench-name>.json` - Current window arrangement (e.g., `{"1: Browser": ["12345", "67890"], "2: Notes": ["11111"]}`)
- **Active bench:** `$BENCH_STATE/active-bench` - Name of the currently focused bench

---

## Launcher UI

`bench launcher` (built with `--features launcher-ui`) opens a fast, keyboard-first GTK window for switching benches:

- **Search:** Start typing to filter bench names; `Esc` clears or closes
- **Navigation:** Arrow keys move the selection; `Tab` toggles between search and list
- **Actions:**
  - `Enter` - Focus the selected bench
  - `Shift+Enter` - Stow the selected bench
  - `Ctrl+S` - Sync the current layout
- **Status:** The footer shows the last action result; errors bubble up in the same bar

When running from the repository:

```bash
cargo run --features launcher-ui -- launcher
```

Bind this to a hotkey in your Sway config for instant access:

```
bindsym $mod+Space exec /path/to/bench launcher
```
