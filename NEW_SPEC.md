Here’s a full README.md in markdown that includes everything: the new ontology, TypeScript examples everywhere, and all the detailed menu/keyboard flows you originally wrote. I kept the fun “workshop” theme while making it consistent.

# 🛠️ Bench: A Workshop for Your Digital Tools

Bench is a playful way to manage your workspaces.
Think of it like a workshop:

- A **Bench** is your whole setup.
- A **Bay** is one numbered workspace in your WM.
- A **Jig** is a template (a tool spec with defaults).
- A **Rig** is a configured instance of a Jig placed on a Bench.

That’s all you need to describe and launch your environment.

---

## 📐 Core Concepts

### Bench

A Bench is a named setup of rigs and bays. It combines your *plan* with runtime state.

```ts
export interface Bench {
  name: string;
  bays?: Record<number, BayConfig>;
  rigs: Rig[];
  is_open?: boolean; // true when assembled
}

Bay

A Bay is one workspace/slot in your WM.

export interface BayConfig {
  name: string;
  window_name: string;
  key_shortcut: string;
  layout: "stacking" | "tabbed" | "floating";
}

Jig

A Jig is a template for a kind of tool with defaults.

export interface Jig<K extends ToolKind = ToolKind> {
  name: string;
  kind: K;
  defaults?: ToolStateMap[K];
}

Rig

A Rig is a Jig placed on a Bench.

export interface Rig<K extends ToolKind = ToolKind> {
  id?: string;
  jig: string;   // Jig.name
  kind: K;
  bay: number;
  state?: ToolStateMap[K];
  last_opened?: string;
  is_open?: boolean;
}

🔌 ToolKinds and State

Supported tool kinds:

export type ToolKind = "browser" | "obsidian" | "terminal";

export interface BrowserState {
  urls: string[];
}

export interface ObsidianState {
  note_id: string;
  vault?: string;
}

export interface TerminalState {
  cwd?: string;
  command?: string[];
}

export type ToolStateMap = {
  browser: BrowserState;
  obsidian: ObsidianState;
  terminal: TerminalState;
};

🏗️ Building Jigs

Jigs are templates you’ll reuse to create rigs.

export const JIG_HOME_BROWSER: Jig<"browser"> = {
  name: "Home Browser",
  kind: "browser",
  defaults: { urls: ["https://tylertracy.com"] },
};

export const JIG_HOME_NOTE: Jig<"obsidian"> = {
  name: "Home Note",
  kind: "obsidian",
  defaults: { vault: "~/Notes", note_id: "1234" },
};

export const JIG_HOME_TERMINAL: Jig<"terminal"> = {
  name: "Home Terminal",
  kind: "terminal",
  defaults: {
    cwd: "~",
    command: ["tmux", "new", "-A", "-s", "work"],
  },
};

🔧 Making Rigs

Rigs are created from Jigs.

function rigFromJig<K extends ToolKind>(
  jig: Jig<K>,
  bay: number,
  overrides?: Partial<ToolStateMap[K]>,
  id?: string,
): Rig<K> {
  const merged = { ...(jig.defaults ?? {}), ...(overrides ?? {}) } as ToolStateMap[K];
  return { id, jig: jig.name, kind: jig.kind, bay, state: merged, is_open: false };
}

const rig1 = rigFromJig(JIG_HOME_BROWSER, 1, undefined, "home-browser-1");

🪑 A Bench in Action

Here’s a full “Home” bench:

export const BENCH_HOME: Bench = {
  name: "Home",
  is_open: false,
  bays: {
    1: { name: "Browse",  window_name: "1: Home",   key_shortcut: "Super+1", layout: "tabbed" },
    2: { name: "Notes",   window_name: "2: Notes",  key_shortcut: "Super+2", layout: "tabbed" },
    3: { name: "Terminal",window_name: "3: Term",   key_shortcut: "Super+3", layout: "stacking" },
    9: { name: "Music",   window_name: "9: Music",  key_shortcut: "Super+9", layout: "floating" },
    10:{ name: "Comms",   window_name: "10: Comms", key_shortcut: "Super+0", layout: "tabbed" },
  },
  rigs: [
    rigFromJig(JIG_HOME_BROWSER,   1, undefined, "home-browser-1"),
    rigFromJig(JIG_HOME_NOTE,      2, undefined, "home-note-1"),
    rigFromJig(JIG_HOME_TERMINAL,  3, undefined, "home-term-1"),
    rigFromJig({ name: "music", kind: "browser", defaults: { urls: ["https://music.youtube.com/"] } }, 9, undefined, "music-1"),
    rigFromJig({ name: "comms", kind: "browser", defaults: { urls: ["https://discord.com/app", "https://mail.google.com/"] } }, 10, undefined, "comms-1"),
  ],
};

🚀 Actions
export function assembleBench(bench: Bench): Bench {
  return {
    ...bench,
    is_open: true,
    rigs: bench.rigs.map(r => ({ ...r, is_open: true, last_opened: new Date().toISOString() })),
  };
}

export function stowBench(bench: Bench): Bench {
  return {
    ...bench,
    is_open: false,
    rigs: bench.rigs.map(r => ({ ...r, is_open: false })),
  };
}


Assemble: open everything in its bay.

Stow: close all rigs.

Rebay: move a rig to another bay.

Relabel: rename a bench.

Mount: add a new rig to a bay.

Scrap: delete a bench.

🎹 Keyboard Shortcuts & Menus
Launcher (Super+M)

Open a menu to manage and navigate benches.

You see: a list of benches.

Type to filter by bench name.

Keys:

Tab / Down: move down list

Shift+Tab / Up: move up list

Esc / Ctrl+Space: return to search

Ctrl+M: focus current bench

While hovering over a bench:

Displays:

Name

Bays and rigs

Last opened

Options

Keys:

Enter: Assemble bench

r: Relabel bench

e: Edit bench

a: Mount a rig

c: Stow bench

Add (Super+A)

Open a menu to add a rig to the current bay.

You see: a list of Jigs.

Type to filter by jig name.

Keys:

Tab / Down: move down

Shift+Tab / Up: move up

Esc / Ctrl+Space: return to search

While hovering a Jig:

Shows all rigs of that Jig across benches, sorted by most recent.

Enter: mount a new rig into the current bay (auto-id BENCH-JIG-N).

Or: pick an existing rig instance.

🧭 Summary

Define Jigs (templates).

Drop them as Rigs into Bays.

Group everything in a Bench.

Use Assemble/Stow to bring it to life.

Navigate with keyboard-driven menus.
