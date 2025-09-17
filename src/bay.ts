export type Bay = {
  isActive: boolean;
  name: string;
  window_name: string;
  key_shortcut: string;
  layout: "stacking" | "tabbed" | "floating";
  windows: Window[];
}

export type Window = {
  containerId: string;
}

// Gets a bay from the state folder by name
export const getStoredBay = (name: string): Bay | undefined => {
  throw new Error("Not implemented");
}

// Gets the active bay from sway
export const getActiveBay = (name: string): Bay | undefined => {
  throw new Error("Not implemented");
}

export async function getTrueBays(): Promise<Bay[]> {
  throw new Error("Not implemented");
}

// Moves a window to a bay
export async function moveWindowToBay(window: Window, bay: string) {
  throw new Error("Not implemented");
}

// Creates the sway layout
export async function buildBay(bay: Bay) {
  throw new Error("Not implemented");
}

export const moveBayToScratchpad = (bay: Bay) => {
  throw new Error("Not implemented");
}
