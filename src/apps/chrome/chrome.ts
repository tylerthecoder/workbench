import type { Server, ServerWebSocket } from "bun";
import type { App } from "../../service";

type AppState = {
  urls: string[];
  chromeWindowId: string;
};

type TabsMessage = Record<string, string[]>;

const port = 3149;

const getTabs = async (): Promise<TabsMessage> => {
  const id = Math.random().toString();
  console.log("Getting tabs", id);
  let tabs: TabsMessage | null = null;
  const bunServer = Bun.serve({
    port,
    async fetch(req, server) {
      const url = new URL(req.url);

      console.log("Request", url.pathname, id);

      if (url.pathname === "/tabs") {
        if (tabs) {
          return new Response("OK");
        }
        const receivedTabs = JSON.parse(await req.text()) as TabsMessage;
        console.log("Received tabs", id);
        tabs = receivedTabs;
        return new Response("OK");
      }
    },
    websocket: {
      message(ws, message) {}, // a message is received
      open(ws) {}, // a socket is opened
      close(ws, code, message) {}, // a socket is closed
      drain(ws) {}, // the socket is ready to receive more data
    },
  });

  let timeout: Timer | undefined;
  let interval: Timer | undefined;
  // wait for the extension to connect
  await new Promise<void>((resolve) => {
    timeout = setTimeout(() => {
      console.log("extension connection timeout", id);
      resolve();
    }, 3000);
    interval = setInterval(() => {
      if (tabs) {
        clearInterval(interval);
        resolve();
      }
    }, 500);
  });
  clearInterval(interval);
  clearTimeout(timeout);

  if (!tabs) {
    console.error("Extension did not connect", id);
    return {};
  }

  bunServer.stop(true);

  return tabs;
};

export async function launchApp(app: App<AppState>) {
  const oldTabs = await getTabs();

  const command = ["chromium", "--new-window", ...app.data.urls];
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

  app.data.chromeWindowId = newWindowId;
}

// Request the chrome tabs and update them
export async function syncApp(app: App<AppState>) {
  console.log("Syncing app", app.name);
  const tabDict = await getTabs();
  const myTabs = tabDict[app.data.chromeWindowId] ?? [];
  console.log("Got tabs", myTabs);
  app.data.urls = myTabs;
}

export async function close() {
  bunServer?.stop();
}
