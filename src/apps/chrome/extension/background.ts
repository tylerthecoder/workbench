const port = 3149;

async function sendData() {
  const tabs = await getTabs();

  try {
      await fetch(`http://localhost:${port}/tabs`, {
        method: "POST",
        body: JSON.stringify(tabs),
      });
      console.log("SENT DATA");
    } catch (e) {
        console.log("Failed to connect");
    }

}

const connectOnLoop = async () => {
  while (true) {
    console.log("Attempting to connect to service");
    await sendData();
    await wait(1000);
  }
};

async function getTabs() {
  const windows = await chrome.windows.getAll({ populate: true });

  console.log("Got windows", windows);

  const tabs = windows.reduce(
    (acc, window) => {
      const windowId = window.id?.toString();
      const urls = window.tabs?.map((tab) => tab.url ?? "");
      if (!windowId || !urls) {
        return acc;
      }
      acc[windowId] = urls;
      return acc;
    },
    {} as Record<string, string[]>,
  );

  return tabs;
}

connectOnLoop();

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

// Listen for the popup script
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log("Got message", message);
  sendResponse({ message: "Got message" });
});
