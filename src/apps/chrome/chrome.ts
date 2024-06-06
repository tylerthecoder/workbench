import type { Server, ServerWebSocket } from "bun";
import type { App } from "../../service";

type AppState = {
  urls: string[];
  chromeWindowId: string;
};

type TabsMessage = Record<string, string[]>;

const port = 3149;

let bunServer: Server | undefined;
let extentionWs: ServerWebSocket<unknown> | undefined;
let onTabs: ((tabs: TabsMessage) => void) | undefined;

const getTabs = async (): Promise<TabsMessage> => {
  if (!bunServer) {
    bunServer = Bun.serve({
      port,
      async fetch(req, server) {
        console.log("Incoming request", req.url);
        const url = new URL(req.url);

        if (url.pathname === "/ws") {
          if (server.upgrade(req)) {
            return; // do not return a Response
          }
          return new Response("Upgrade failed :(", { status: 500 });
        }
      },
      websocket: {
        async message(_ws, message) {
          const parsed = JSON.parse(
            typeof message === "string" ? message : message.toString(),
          ) as { tabs: Record<string, string[]> };

          if (parsed.tabs) {
            const { tabs } = parsed;

            onTabs?.(tabs);
          }
        },
        open(_ws) {
          console.log("Extension connected");
          extentionWs = _ws;
        },
        close(_ws) {
          console.log("Extension disconnected");
        },
      },
    });
  }

  if (!extentionWs) {
    console.log("Extension not connected, waiting");
    // wait for the extension to connect
    await new Promise<void>((resolve) => {
      // Start a 5 second timer
      setTimeout(() => {
        console.log("extension connection timeout");
        resolve();
      }, 10000);
      const interval = setInterval(() => {
        if (extentionWs) {
          clearInterval(interval);
          resolve();
        }
      }, 1000);
    });
  }

  if (!extentionWs) {
    return {};
  }

  extentionWs.send(JSON.stringify({ getTabs: true }));

  return await new Promise<TabsMessage>((resolve) => {
    onTabs = (tabs) => {
      resolve(tabs);
    };
  });
};

export async function launchApp(app: App<AppState>) {
  const oldTabs = await getTabs();

  const command = ["chromium", "--new-window", ...app.data.urls];
  Bun.spawnSync(command);

  const newTabs = await getTabs();

  const oldWindowIds = Object.keys(oldTabs);
  const newWindowIds = Object.keys(newTabs);

  const diffWindowIds = newWindowIds.filter((id) => !oldWindowIds.includes(id));

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
