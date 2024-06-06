const log = (...args: any[]) => console.log("I3:", ...args);

export function getTree(): TreeNode {
  const command = ["i3-msg", "-t", "get_tree"];
  const res = Bun.spawnSync(command);
  const data = JSON.parse(res.stdout.toString());
  return data as TreeNode;
}

export function findNodeIdsByClass(
  treeNode: TreeNode,
  className: string,
): number[] {
  let result: number[] = [];

  function traverse(node: TreeNode) {
    if (node.window_properties && node.window_properties.class === className) {
      result.push(node.id);
    }
    node.nodes.forEach(traverse);
    node.floating_nodes.forEach(traverse);
  }

  traverse(treeNode);
  return result;
}

export function getAllWindowIds(): string[] {
  const command = ["xdotool", "search", "$"];
  const data = Bun.spawnSync(command);
  const windowIds = data.stdout.toString().split("\n");
  return windowIds;
}

export function moveToWindowWorkspace(windowId: number, workspaceId: string) {
  const command = [
    "i3-msg",
    `[con_id=${windowId}]`,
    `move container to workspace ${workspaceId}`,
  ];

  const { stdout } = Bun.spawnSync(command);
  log(
    "Moved window",
    windowId,
    "to workspace",
    workspaceId,
    "Result:",
    stdout.toString(),
  );
}

export function moveWindowToScratchPad(windowId: number) {
  const command = [
    "i3-msg",
    `[id=${windowId}]`,
    "move container to scratchpad",
  ];
  const { stdout } = Bun.spawnSync(command);
  log("Moved window", windowId, "to scratchpad Result:", stdout.toString());
}

export interface TreeNode {
  id: number;
  type: string;
  orientation: string;
  scratchpad_state: string;
  percent: number;
  urgent: boolean;
  marks: any[];
  focused: boolean;
  output?: string;
  layout: string;
  workspace_layout: string;
  last_split_layout: string;
  border: string;
  current_border_width: number;
  rect: Rect;
  deco_rect: Rect;
  window_rect: Rect;
  geometry: Rect;
  name?: string;
  window_icon_padding: number;
  window?: number;
  window_type?: string;
  nodes: TreeNode[];
  floating_nodes: TreeNode[];
  focus: number[];
  fullscreen_mode: number;
  sticky: boolean;
  floating: string;
  swallows: any[];
  gaps?: Gaps;
  window_properties?: WindowProperties;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Gaps {
  inner: number;
  outer: number;
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export interface WindowProperties {
  class: string;
  instance: string;
  window_role?: string;
  machine: string;
  title: string;
  transient_for?: any;
}

export interface Swallow {
  dock: number;
  insert_where: number;
}
