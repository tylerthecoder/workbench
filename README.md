# Bench (Rust)

Sway-first reimplementation of Bench in Rust. Manages “benches” of tools and places them into named Sway bays. Supports Chromium, Kitty, and Zed.

## Requirements

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

## Terms

- **Bench**: A named collection of bays and the tools assigned to each bay.
- **Bay**: A named target in Sway (for example `1: Home`) where Bench places windows.
- **Tool**: A reusable description of how to launch an application (browser, terminal, Zed, etc.).

## Config

Bench follows the XDG base directory spec for persisted data. We refer to the resolved data directory as `$BENCH_STATE` throughout the docs (defaults to `~/.local/share/bench`).

## Bench

Bench specifications live under `$BENCH_STATE/benches/<name>.yml`. With the default `$BENCH_STATE=~/.local/share/bench`, you might have:

```yml
name: Home
bays:
  - name: "1: Home"
    tool_names:
      - home-browser
      - home-chat
  - name: "2: Notes"
    tool_names:
      - notes-zed
  - name: "3: Terminal"
    tool_names:
      - work-term
```

Each bay name maps directly to a named target in Sway; numeric prefixes are optional but recommended when you want consistent ordering. Benches do not embed tool definitions—they reference shared tool names. When you focus a bench, every bay name is re-applied by renaming the corresponding bays so the layout stays synchronized.

Bench assembly data is stored as JSON under `$BENCH_STATE/assembled-benches/<bench>.json`. Each record captures a mapping from bay names to the window IDs currently associated with that bay. This lets Bench remember which containers belong to the bench between sessions without duplicating the bench specification itself.

Use `bench create <bench-name>` to scaffold a new bench specification and `bench list-benches` to discover existing ones. Inspect a specific spec with `bench info <bench-name>`, which prints the bench definition along with whether it is currently assembled or focused.

## Tool

Each tool is defined once under `$BENCH_STATE/tools/<tool>.yml`. These files map directly to the Rust `Tool` struct and can be reused across benches.

Example tool definition (`$BENCH_STATE/tools/home-browser.yml`):

```yml
name: home-browser
kind: browser
state:
  urls:
    - "https://tylertracy.com"
    - "https://mail.google.com/"
```

Terminal and Zed tools follow the same format, using their respective configuration payloads.

Assembled tool data is stored as JSON under `$BENCH_STATE/assembled-tools/<tool>.json`. Each record currently tracks the observed `window_id` for that tool so we can reuse the same container instead of relaunching it—no other metadata is stored. Launch or refresh a single tool with `bench assemble-tool <tool-name>`. This command updates the currently focused bench if one is set. If the tool isn’t assembled yet, the file is absent until we observe it.

## Crafting a Tool

Use `bench craft-tool <tool-kind> <name>` to create a YAML stub in `$BENCH_STATE/tools/<name>.yml` with sensible defaults for the requested kind (`browser`, `terminal`, or `zed`).

## Assembling a Tool

Assembling a tool means the tool has an active window on your machine with a known Sway `container_id`. When you open a tool through Bench, the tool’s JSON file in `$BENCH_STATE/assembled-tools/` is updated with its `window_id` so it can be reused instead of relaunched.

## Assemble a Bench

Assembling a bench means ensuring each tool defined in the bench has an open window. During assembly we:

- Launch tools that aren’t currently assembled.
- Reuse existing tool containers when possible.
- Record a map of `tool_name -> window_id` under `$BENCH_STATE/assembled-tools/` for quick reuse.
- Update `$BENCH_STATE/assembled-benches/<bench>.json` with the list of window IDs currently tied to each bay. This includes unmanaged windows that you grouped with the bench manually.

Run `bench assemble <bench-name>` to perform these steps. Assembling does *not* move windows; it only guarantees they exist and that their IDs are tracked.

## Stowing a Bench

Stowing a window simply moves it into the Sway scratchpad while leaving the process running. `bench stow <bench-name>` applies that to every window tied to the bench and keeps the JSON records in `$BENCH_STATE/assembled-benches/` up to date.

## Focusing a Bench

Focusing a bench:

- Stows whatever windows are currently visible, except for windows that already belong to the bench and are already in the right bay.
- Ensures the target bench is assembled (launching any missing tools).
- Moves each window recorded in the bench’s assembled JSON back to its bay, preserving the layout instead of collapsing into a single view.
- Renames those bays to match the bench’s bay names.
- Marks this bench as the active bench so future sync commands know which layout to observe.

Bring a bench into view with `bench focus <bench-name>`. Check its current state at any time with `bench info <bench-name>`.

## Syncing Benches

You can sync the current state of the bench to the bench specification. Syncing gathers data from running tools (for example, capturing Chromium tabs) and merges it into the bench YAML. In practice, it’s helpful to save every few minutes. You can choose to sync just the layout (bay assignments) or the tool state payloads. Use `bench sync-layout` or `bench sync-tool-state` against the currently focused bench.

## Launcher UI

`bench launcher` (built with `--features launcher-ui`) opens a fast, keyboard-first GTK window for switching benches:

- Search: start typing to filter bench names; `Esc` clears or closes.
- Navigation: arrow keys move the selection; `Tab` toggles between search and list.
- Actions: `Enter` assembles the selected bench, `Shift+Enter` stows it, and `Ctrl+S` syncs the current arrangement back to disk.
- Status: the footer shows the last action result; errors bubble up in the same bar.

When running from the repository, you can start the launcher with:

```bash
cargo run --features launcher-ui -- launcher
```

