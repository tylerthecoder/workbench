# Bench

## Dev

```bash
bun install
```

To run:

```bash
bun run build.ts

./out/bench
```

## Bench.yml

```yml
name: Home

tool:
- name: Home Browser
- bay: 1

tool:
- name: Home Note
- bay: 2

tool:
- name: Home Terminal
- window: 3

tool:
- name: music
- window: 9

tool:
- name: comms
- window: 10
```

Stored in `./local/share/bench/benches/<bench-name>.yml`

## Tool.yml

```yml
name: Home Browser
type: browser
state:
  urls:
    - https://tylertracy.com
```

```yml
name: Home Note
type: note
state:
  noteId: 1234
```

Stored in `./local/share/bench/benches/apps/<app-name>.yml`

## Types

- Bench
  - name: String
  - apps: [App]

- App
  - id: String (optional)
  - type: String
  - space: Int
  - state: Dict (optional)
  - last-opened: DateTime (auto)
  - is-open: Bool (auto)

- AppInstance
  - app: App
  - bench: Bench

- AppType
  - name: String
  - default-space: Int (optional)

## Actions

- Launch
  - (bench) -> ()
  - Open all apps in bench and moving them to their configured space.

- Start
  - (bench) -> ()
  - Opening all apps in bench and but don't move them.

- Rename
  - (bench, new-name) -> ()
  - Rename a bench.

- Add a app
  - (bench, space, app) -> ()
  - Adds an application to a space

- Move an app
  - (bench, app, new-workspace) -> ()
  - Moves an application to a different workspace

- Close
  - (bench) -> ()
  - Closes all the apps in a bench.

- Delete
  - (bench) -> ()
  - Deletes a bench. Asks for confirmation.

## How to build a bench with keyboard shortcuts

### Launcher (Super+M)

Open a menu to manage and navigate workspaces

- you see:
  A list of workspaces

Type a workspace name the list filters.

- Press (tab, down arrow) to go down the list.
- Press (Shift-Tab, up) arrow to go up the list.
- Press (Esc, ctrl+Space) to return to search.
- Press (ctrl+m) to focus on current workspace.

While hovering over a workspace, you see the workspace control panel:

- Displays:
  - Name
  - Number and name of open windows
  - Last opened
  - Option
- Keys:
  - Enter: Launch the selected workspace
  - r: Rename the selected workspace
  - e: Edit the selected workspace
  - a: Add an app to the selected workspace
  - c: Close the selected workspace

### Add (Super+A)

Open a menu to add an app to the current workspace

- you see:
  A list of apps to add

Type an app name the list filters.

Press (tab, down arrow) to go down the list.
Press (Shift-Tab, up) arrow to go up the list.
Press (Esc, ctrl+Space) to return to search.

While hovering over app type, you see:

- All instances of that app type in all benches sorted by most recently used

you can press:

- Enter: Instantiate a new version of the app type in the current workspace. Auto name BENCH-APP-NUM
- Start typing to select any copy version of that app
  - Press (tab, down arrow) to go down the list.
  - Press (Shift-Tab, up) arrow to go up the list.
  - Press (Esc, ctrl+Space) to hover over app type again (yield to the app type list)
  - Press (Enter) to instantiate the selected app in the current workspace.

## Tool Types

### Browser

Launches a chrome browser with the remote debugging port enabled.

Every minute we query the open tabs and store them in the app state.

State:

```rust
struct Browser {
  urls: String[]; // list of urls to open
}
```

### Terminal

Launches a terminal with the cwd in the state.

State:

```rust

enum TerminalCommand {
  Tmux(Path),
  Command(String),
}

struct TerminalPaneState {
  cwd: Path; // cwd for this pane
  command: String; // command to run
struct Terminal {
  cwd: Path; // default cwd for new panes
  panes: TerminalPaneState[];
}
```

### Obsidian

Launches obsidian with the vault and noteId in the state.

State:

```rust
struct Obsidian {
  vault: Path; // path to vault
  noteId: String; // id of note to open
}
```
