# Bench (Rust)

Sway-first reimplementation of Bench in Rust. Manages “benches” of tools and places them into numbered Sway workspaces (“bays”). Supports Chromium, Kitty, and Zed.

Requirements:

- Sway and `swaymsg` on PATH
- Chromium (`chromium` binary), Kitty (`kitty`), Zed (`zed`)
- (Optional for the launcher) GTK4 development libraries and `pkg-config`

## Build

```bash
cargo build --release
```

Binary: `target/release/bench`

To install system-wide:

```bash
cargo install --path .
```

## Bench YAML

Benches live in `~/.local/share/bench/benches/<name>.yml`. They are intentionally lightweight and only describe which tools belong in which bays (Sway workspaces) by way of `tool_defaults`.

Example `~/.local/share/bench/benches/home.yml`:

```yml
name: Home
tool_defaults:
  - bay: 1
    name: "1: Home"
    tool_names:
      - home-browser
      - home-chat
  - bay: 2
    name: "2: Notes"
    tool_names:
      - notes-zed
  - bay: 3
    name: "3: Terminal"
    tool_names:
      - work-term
```

Notice that the bench file does **not** contain inline tool definitions. Tools are shared across benches and stored separately.

## Tool Definitions

Each tool is defined once under `~/.local/share/bench/tools/<tool>.yml`. These files map directly to the Rust `Tool` struct and can be reused across benches.

Example `~/.local/share/bench/tools/home-browser.yml`:

```yml
name: home-browser
kind: browser
state:
  urls:
    - "https://tylertracy.com"
    - "https://mail.google.com/"
```

Terminal and Zed tools follow the same format, using their respective configuration payloads.

Per-tool runtime data (current container ID, last-opened timestamp, browser debug port) is stored alongside the definition in `~/.local/share/bench/tools/<tool>.runtime.json`.

## Runtime Layout

Bench-specific runtime data is written to `~/.local/share/bench/runtime/<bench>.json`. These snapshots keep track of the untracked window IDs that should be pulled out of the scratchpad when the bench is assembled. The currently active bench name is recorded in `~/.local/share/bench/runtime/.active_bench`; commands that require an active bench will error if nothing is marked active.

## Workflow Tips

1. Create or update tool definitions in `~/.local/share/bench/tools/`.
2. Create or edit a bench YAML under `~/.local/share/bench/benches/` listing the desired `tool_defaults`.
3. Set the active bench with `bench activate <name>` (writes `.active_bench`).
4. Run `bench assemble` to launch/move shared tools into their bays using the active bench.
5. Use `bench active` at any time to inspect the active bench, runtime state, and whether any tools have drifted from their default bays.
6. When you like the current layout, run `bench snapshot-current <new-name>` to capture it into a fresh bench YAML (untracked windows are ignored).
7. `bench stow` moves the active bench’s windows back to the scratchpad without killing their processes.

## CLI

Commands:

- `bench activate <bench-name>`: Mark a bench as the active bench (writes `.active_bench`).
- `bench assemble`: Launch tools or pull them from the scratchpad for the active bench.
- `bench stow`: Move the active bench’s windows back to the scratchpad.
- `bench active`: Print the active bench configuration plus runtime details and any bay drift.
- `bench snapshot-current <bench-name>`: Generate a new bench YAML from the current window state.
- `bench list-workspaces`: Show current Sway workspaces.
- `bench list-benches`: List YAML benches under `~/.local/share/bench/benches`.
- `bench launcher`: Launch the GTK search UI (build with `--features launcher-ui`).

When running from the repository, you can start the launcher with:

```bash
cargo run --features launcher-ui -- launcher
```

## Types

- Bench
  - name: String
  - tool_defaults: [ToolDefault]
  - tools: [Tool]
  - active_bays: [ActiveBay] (captured by `bench sync`)
  - is_open: Bool (optional hint)

- Tool
  - name: String
  - kind: `browser | terminal | zed`
  - bay: Int
  - state: Dict (kind-specific payload)

- ToolDefault
  - bay: Int
  - name: String (optional workspace title shown in Sway)
  - tool_names: [String]

- ActiveBay
  - bay: Int
  - name: String (optional workspace title)
  - window_ids: [String] (Sway container IDs parked in the scratchpad)
  - title: String (optional)
  - workspace: String

## Launcher UI

`bench launcher` (built with `--features launcher-ui`) opens a fast, keyboard-first GTK window for switching benches:

- Search: start typing to filter bench names; `Esc` clears or closes.
- Navigation: arrow keys move the selection; `Tab` toggles between search and list.
- Actions: `Enter` assembles the selected bench, `Shift+Enter` stows it, and `Ctrl+S` syncs the current arrangement back to disk.
- Status: the footer shows the last action result; errors bubble up in the same bar.

## Notes

- Uses Sway via `swaymsg` (no i3 dependencies).
- Supported tools: Chromium (`kind: browser`), Kitty (`kind: terminal`), Zed (`kind: zed`).
- Windows are detected via the Sway tree. On assemble, tools are launched or reused; stored `active_bays` window IDs are moved from the scratchpad back to their workspaces.
- Runtime data about open tools lives in the runtime directory; the bench YAML stays declarative.
- Browser tools expose a DevTools websocket via the recorded debug port so you can query tabs or other state.
- Stowing moves bench windows into the Sway scratchpad; assembling first tries to reuse those containers before launching anything new.
- Build with `--features launcher-ui` to enable the GTK launcher; without it, the CLI still works headless.
