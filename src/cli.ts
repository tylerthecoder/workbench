import * as Service from "./service";
import { $ } from "bun";

const command = process.argv[2];

function printHelp() {
  console.log("Commands:");
  console.log("  open <workspace>");
  console.log("  list-workspaces");
  console.log("  read-state");
  console.log("  read-config");
  console.log("  sync");
  console.log("  new <workspace>");
  console.log("  logs");
}

switch (command) {
  case "open":
    const workspaceName = process.argv[3];
    await Service.selectWorkspace(workspaceName);
    break;
  case "list-workspaces":
    const worksapces = await Service.getAllWorkspaces();
    console.log(worksapces.join("\n"));
    break;
  case "read-state":
    const state = await Service.getFromFs();
    console.log(JSON.stringify(state, null, 2));
    break;
  case "sync":
    await Service.sync();
    break;
  case "sync-loop":
    await Service.syncLoop();
    break;
  case "read-config":
    const config = Service.getConfig();
    console.log(JSON.stringify(config, null, 2));
    break;
  case "new": {
    const workspaceName = process.argv[3];
    await Service.newWorkspace(workspaceName);
    break;
  }
  case "logs":
    await $`sudo journalctl -u bench-sync.service`;
    break;
  default:
    printHelp();
}
