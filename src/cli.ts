import * as Service from "./service";

const command = process.argv[2];

function printHelp() {
  console.log("Commands:");
  console.log("  open <workspace>");
  console.log("  list-workspaces");
  console.log("  read-state");
  console.log("  read-config");
  console.log("  sync");
}

if (command === "open") {
  const workspaceName = process.argv[3];
  await Service.selectWorkspace(workspaceName);
} else if (command == "list-workspaces") {
  const worksapces = await Service.getAllWorkspaces();
  console.log(worksapces.join("\n"));
} else if (command == "read-state") {
  const state = await Service.getFromFs();
  console.log(JSON.stringify(state, null, 2));
} else if (command == "read-config") {
  const config = Service.getConfig();
  console.log(JSON.stringify(config, null, 2));
} else if (command == "sync") {
  Service.sync();
} else {
  printHelp();
}
