import type { Server, ServerWebSocket } from "bun";
import type { App } from "../../service";


export type ChromeAppState = {
  urls: string[];
}

export type ChromeAppOpenedState = {
  chromeWindowId: string;
};

type TabsMessage = Record<string, string[]>;

const port = 3149;

// Will be called by a daemon
async function startServer() {
  let tabs: TabsMessage | null = null;
  Bun.serve({
    port,
    async fetch(req, server) {
      const url = new URL(req.url);

      console.log("Request", url.pathname);

      if (url.pathname === "/tabs") {
        if (tabs) {
          return new Response("OK");
        }
        const receivedTabs = JSON.parse(await req.text()) as TabsMessage;
        console.log("Received tabs");
        tabs = receivedTabs;
        return new Response("OK");
      } else if (url.pathname == "/get-tabs") {
        // wait for fresh tabs
        tabs = null;
        while (!tabs) {
          await new Promise((resolve) => setTimeout(resolve, 200));
        }

        return new Response(JSON.stringify(tabs ?? {}));
      }
    },
    websocket: {
      message(ws, message) { }, // a message is received
      open(ws) { }, // a socket is opened
      close(ws, code, message) { }, // a socket is closed
      drain(ws) { }, // the socket is ready to receive more data
    },
  });
}

const getTabs = async (): Promise<TabsMessage> => {
  const tabsRes = await fetch(`http://localhost:${port}/get-tabs`);
  const tabs = (await tabsRes.json()) as TabsMessage;
  return tabs;
};

async function openApp(appState: ChromeAppState): Promise<ChromeAppOpenedState> {
  const oldTabs = await getTabs();

  const command = ["chromium", "--new-window", ...appState.urls];
  Bun.spawnSync(command);

  const newTabs = await getTabs();

  const oldWindowIds = Object.keys(oldTabs);
  const newWindowIds = Object.keys(newTabs);

  const diffWindowIds = newWindowIds.filter((id) => !oldWindowIds.includes(id));

  console.log("Diff window ids", diffWindowIds);

  if (diffWindowIds?.length > 1) {
    throw new Error("Too many new windows");
  }
  const newWindowId = diffWindowIds[0];

  console.log("Found new browser window", newWindowId);

  if (!newWindowId) {
    throw new Error("No new window found");
  }

  return {
    chromeWindowId: newWindowId
  };
}

async function syncApp(app: App<AppState>) {
  console.log("Syncing app", app.name);
  const tabDict = await getTabs();
  const myTabs = tabDict[app.data.chromeWindowId] ?? [];
  console.log("Got tabs", myTabs);
  app.data.urls = myTabs;
}

export const ChromeApp = {
  windowName: "Chromium",
  openApp,
  syncApp,
  startServer
};
