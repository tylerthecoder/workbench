import { getConfig } from "./config";
import { waitForNewWindow } from "./sway.service";
import { ChromeApp, type ChromeAppState } from "./tools/chrome/chrome";

export type Tool<
    GToolType extends ToolKind = ToolKind,
> = {
    name: string;
    type: GToolType;
    state: ToolStateMap[GToolType];
}

/** Tool kinds (capabilities). */
export type ToolKind = "browser" | "obsidian" | "terminal";

/** Tool state payloads per kind. */
export type BrowserState = {
    urls: string[];
}
export type ObsidianState = {
    note_id: string;   // kept snake_case to match your data, feel free to switch to noteId
    vault?: string;    // optional, if you want it
}
export type TerminalState = {
    cwd?: string;
    command?: string[]; // or a single string if you prefer
}

export type ToolStateMap = {
    browser: BrowserState;
    obsidian: ObsidianState;
    terminal: TerminalState;
};

export type OpenedToolStateMap = {
    browser: {
        chromeWindowId: string;
    };
    obsidian: {
        obsidianWindowId: string;
    };
    terminal: {
        terminalWindowId: string;
    };
}

export type OpenedTool<
    GToolType extends ToolKind = ToolKind,
    OpenedState extends OpenedToolStateMap[GToolType] = OpenedToolStateMap[GToolType]
> = Tool<GToolType> & {
    containerId: string;
    openedState: OpenedState;
}

const isOpened = (tool: Tool): tool is OpenedTool => {
    return "containerId" in tool;
}

export async function getAllTools(): Promise<Tool[]> {
    const config = getConfig();
    const allToolsFile = config.stateDir + "/tools.json";
    const allTools = await Bun.file(allToolsFile).json() as Tool[];
    return allTools;
}

export async function getOpenedTools(): Promise<OpenedTool[]> {
    const allTools = await getAllTools();
    return allTools.filter(isOpened);
}


export const launchTool = async (tool: Tool): Promise<OpenedTool> => {
    const openedTools = await getOpenedTools();
    const openedTool = openedTools.find(t => t.name === tool.name);
    if (openedTool) {
        return openedTool;
    }

    if (tool.type === "browser") {
        const chromeAppState = tool.state as ChromeAppState;
        const chromeAppOpenedState = await ChromeApp.openApp(chromeAppState);
        const containerId = await waitForNewWindow(ChromeApp.windowName);
        return {
            ...tool,
            containerId,
            openedState: chromeAppOpenedState
        };
    }

    throw new Error(`Tool ${tool.name} not found`);
}