# Bench Alignment Plan

## 1. Terminology & Identifier Updates
- Represent bays with their full string names (for example `1: Home`) throughout models and logic.
- Update `ToolDefault`, `CapturedBay`, `BenchRuntime`, and related types to store bay names as strings instead of numeric IDs.
- Replace helpers that parse numeric workspace IDs; provide string-based sway helpers for ensuring/renaming/moving bays.

## 2. Storage Layout & Persistence Helpers
- Extend `storage.rs` so it exposes helpers for every persisted artifact: bench specs, tool specs, assembled benches, assembled tools, and the active bench marker.
- Ensure directory creation (`ensure_dirs`, etc.) covers the new paths under `$BENCH_STATE/`.
- Keep bench/tool definitions as YAML; write assembled bench/tool data as JSON in the new locations via `storage.rs`.

## 3. Tool Runtime Tracking
- Replace `ToolRuntimeState` with a minimal structure that stores only the tracked `window_id`.
- Adjust `launch_and_place_tool` and reuse logic to persist and consume this simplified state through `storage.rs` helpers.
- Remove drift/debug metadata that relied on container IDs, debug ports, or timestamps.

## 4. Bench Assembly & Focus Flow
- Rework `assemble_active_bench` to launch/reuse tools, record `tool_name -> window_id`, and update the assembled bench JSON without moving windows.
- Update `stow_active_bench` to move the bench’s windows to the scratchpad and refresh stored mappings.
- Implement a `focus` command that stows non-bench windows, ensures assembly, restores each bay’s windows, renames bays, and marks the bench active.

## 5. CLI Surface Alignment
- Align the CLI with the README: `assemble`, `stow`, `focus`, `assemble-tool`, `sync-layout`, `sync-tool-state`, `craft-tool`, `list-benches`, and `info` (with `bench info <bench>` showing whether the bench is assembled or focused).
- Refresh command descriptions/help text to use bay terminology.
- Update CLI output to reflect the simplified runtime state (e.g., showing tracked `window_id` per tool).

## 6. Model & Helper Adjustments
- Update sway helpers to work with bay names: ensure visibility, rename, move containers using strings.
- Revise layout capture (`snapshot_current_as_bench`, sync commands) to store bay names and tool lists based on the new model.
- Replace `gather_tool_statuses`/`capture_captured_bays` plumbing with versions that operate on string bay keys.

## 7. Sync Commands
- Define logic for `bench sync-layout` (update bench YAML bay order/names from current Sway state).
- Define logic for `bench sync-tool-state` (pull current tool payloads—e.g., browser tabs—and merge into tool definitions).
- Ensure both commands operate on the currently focused bench and handle missing focus gracefully.

## 8. Migration & Compatibility
- Detect legacy runtime files (with old metadata) and refresh or replace them on the next assemble cycle.
- Document the migration behavior inline to avoid confusing users upgrading from earlier builds.

## 9. Core Data Structures
- `model::Bench`: YAML-backed spec containing `name` and an ordered list of `BaySpec` entries (see below).
- `model::BaySpec` (new): replaces `ToolDefault`, stores `{ name: String, tool_names: Vec<String> }` to align with README.
- `model::AssembledBench` (new): JSON-serializable structure mapping bay names to window ID lists plus optional metadata.
- `model::AssembledTool` (new): JSON holding `{ window_id: String }` for each assembled tool.
- Active bench tracking becomes a simple string persisted via `storage.rs`; no standalone runtime module.

## 10. File Layout
- `README.md`: authoritative description (already updated).
- `PLAN.md`: this plan document.
- Rust modules:
  - `src/model.rs`: define `Bench`, `BaySpec`, `AssembledBench`, `AssembledTool` structures.
  - `src/storage.rs`: single source for resolving `$BENCH_STATE` paths and reading/writing bench specs, tool specs, assembled benches, assembled tools, and the active bench marker.
  - `src/assembly.rs` (new): helpers coordinating assemble/stow/focus using `storage` + sway utilities (replaces `runtime.rs`).
  - `src/bench_ops.rs`: high-level operations for assemble/stow/focus/sync delegating to `assembly` and `storage`.
  - `src/apps/*`: launchers remain but adjust signatures to work with new runtime metadata.
  - `src/main.rs`: CLI wiring aligned with updated commands.
- JSON/YAML output in `$BENCH_STATE` mirrors README examples (no extra metadata beyond what’s documented).
