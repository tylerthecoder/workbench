
const swayMoveToolToBay = (tool: WieldedTool, bay: Bay) => {
    const command = ['swaymsg', `[con_id="${tool.containerId}"]`, 'move', 'container', 'to', 'workspace', bay.name];

    Bun.spawnSync(command, {
        stdout: 'inherit',
        stderr: 'inherit',
    });
}



// Waits for a new window to appear, optionally filtering by window name.
export async function waitForNewWindow(windowName?: string): Promise<string> {


}

const moveAllWindowsToScratchpad = () => {
    const command = ['swaymsg', 'move', 'container', 'to', 'workspace', 'scratchpad'];
    Bun.spawnSync(command, {
        stdout: 'inherit',
        stderr: 'inherit',
    });
}
