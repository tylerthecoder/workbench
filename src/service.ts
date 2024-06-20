import { launchApp, syncApp } from "./apps/chrome/chrome.ts";
import * as I3Service from "./i3.service.ts";

export type Config = {
  stateFile: string;
};

export type BaseApp<T> = {
  name: string;
  i3Workspace: string;
  data: T;
};

export type OpenedApp<T> = BaseApp<T> & {
  i3WindowId: number;
};

export type App<T> = BaseApp<T> | OpenedApp<T>;

const isOpened = <T>(app: App<T>): app is OpenedApp<T> => {
  return "i3WindowId" in app;
};

export type Workspace = {
  name: string;
  isOpened: boolean;
  apps: App<any>[];
};

export function getConfig(): Config {
  return {
    stateFile: "/home/tylord/docs/bench.json",
  };
}

export async function getFromFs(): Promise<Workspace[]> {
  const { stateFile } = getConfig();

  const currentTabState = await (async () => {
    const file = Bun.file(stateFile);
    if (!(await file.exists())) {
      console.log("File doesn't exist");
      return [];
    }
    return await file.json();
  })();

  return currentTabState;
}

async function saveToFs(workspaces: Workspace[]) {
  const { stateFile } = getConfig();
  const file = Bun.file(stateFile);
  await Bun.write(file, JSON.stringify(workspaces, null, 2));
}

export async function getAllWorkspaces() {
  const workspaces = await getFromFs();
  return workspaces.map((w) => w.name);
}

export async function getOpenedWorkspace() {
  const workspaces = await getFromFs();
  return workspaces.find((w) => w.isOpened);
}

export async function selectWorkspace(workspaceName: string) {
  console.log("Selecting workspace", workspaceName);
  const workspaces = await getFromFs();

  const workspace = workspaces.find((w) => w.name === workspaceName);

  if (!workspace) {
    console.error(`Workspace ${workspaceName} not found`);
    return;
  }

  const currentlyOpenedWorkspace = workspaces.find((w) => w.isOpened);

  if (currentlyOpenedWorkspace) {
    currentlyOpenedWorkspace.isOpened = false;

    for (const app of currentlyOpenedWorkspace.apps) {
      if (!isOpened(app)) {
        return;
      }
      I3Service.moveWindowToScratchPad(app.i3WindowId);
    }
  }

  workspace.isOpened = true;

  for (const app of workspace.apps) {
    let openedApp: OpenedApp<any>;
    openedApp = await openApp(app);

    await new Promise((resolve) => setTimeout(resolve, 100));

    I3Service.moveToWindowWorkspace(
      openedApp.i3WindowId,
      openedApp.i3Workspace,
    );
  }

  saveToFs(workspaces);
}

async function openApp<T>(app: App<T>): Promise<OpenedApp<T>> {
  const prevWindowIds = I3Service.findNodeIdsByClass(
    I3Service.getTree(),
    "Chromium",
  );

  if (isOpened(app)) {
    const isActuallyOpened = prevWindowIds.find((w) => w == app.i3WindowId);
    if (isActuallyOpened) {
      return app;
    } else {
      console.log("App is opened but window is not found. Removing id.");
      (app as any).i3WindowId = undefined;
    }
  }

  if (app.name === "chrome") {
    await launchApp(app as any);
  }

  const nextWindowIds = I3Service.findNodeIdsByClass(
    I3Service.getTree(),
    "Chromium",
  );

  const newWindows = nextWindowIds.filter((w) => !prevWindowIds.includes(w));

  if (newWindows.length > 1) {
    console.log(newWindows);
    throw new Error("Too many new i3 windows");
  }

  if (newWindows.length === 0) {
    throw new Error("No new i3 windows");
  }

  const newWindowId = newWindows[0];

  console.log("Found new I3 window", newWindowId);

  const openedApp = app as OpenedApp<T>;
  openedApp.i3WindowId = newWindowId;
  return openedApp;
}

export async function sync() {
  console.log("Syncing workspaces");
  const workspaces = await getFromFs();

  const openededWorkspaces = workspaces.filter((w) => w.isOpened);

  if (openededWorkspaces.length !== 1) {
    console.log(
      "Expected 1 opened workspace, found",
      openededWorkspaces.length,
    );
  }

  const openedWorkspace = openededWorkspaces[0];

  for (const app of openedWorkspace.apps) {
    if (app.name === "chrome") {
      await syncApp(app as any);
    }
  }

  saveToFs(workspaces);
}

export async function syncLoop() {
  while (true) {
    await sync();
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
}

export async function newWorkspace(name: string) {
  const workspaces = await getFromFs();

  // Check if workspace already exists
  if (workspaces.find((w) => w.name === name)) {
    console.error(`Workspace ${name} already exists`);
    return;
  }

  workspaces.push({
    name,
    isOpened: false,
    apps: [],
  });

  saveToFs(workspaces);
}
