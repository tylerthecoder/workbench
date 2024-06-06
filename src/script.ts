import { selectWorkspace } from "./service";

selectWorkspace("Dev");
await new Promise((resolve) => setTimeout(resolve, 5000));
selectWorkspace("Work");
