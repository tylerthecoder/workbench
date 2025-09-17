import { spawnSync } from 'bun';
import type { App } from "../../service";

export type AppState = {
  directory: string;
};

function getSessionName(directory: string): string {
  return directory.split('/').pop() || 'tmux-session';
}

async function openApp(app: App<AppState>): Promise<void> {
  try {
    const sessionName = getSessionName(app.data.directory);
    const terminal = process.env.TERMINAL ?? 'xterm';

    // Check if the session already exists
    const checkSession = spawnSync(['tmux', 'has-session', '-t', sessionName]);
    const sessionExists = checkSession.exitCode === 0;

    const tmuxCommand = sessionExists
      ? `tmux attach-session -t ${sessionName}`
      : `cd ${app.data.directory} && tmux new-session -s ${sessionName}`;

    const command = [
      terminal,
      '-e',
      'bash',
      '-c',
      tmuxCommand
    ];

    const result = spawnSync(command);

    if (result.success) {
      console.log(`${sessionExists ? 'Attached to' : 'Launched'} tmux session '${sessionName}' in ${app.data.directory}`);
    } else {
      throw new Error(`Failed to ${sessionExists ? 'attach to' : 'launch'} tmux session: ${result.stderr.toString()}`);
    }
  } catch (error) {
    console.error(`Error with tmux session: ${error}`);
    throw error;
  }
}

export const TmuxApp = {
  openApp
};






