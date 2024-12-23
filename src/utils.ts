
import { spawn } from "bun";

export function wait(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}


export interface Option {
  id: string;
  label: string;
}

export function rofiPicker(options: Option[]): string | null {
  // Prepare the input for rofi
  const rofiInput = options.map(option => option.label).join("\n");

  // Spawn rofi command
  const rofiProcess = spawn([
    "rofi",
    "-dmenu",
    "-i",
    "-p", "Select an option"
  ], {
    stdin: "pipe"
  });

  const enc = new TextEncoder();

  rofiProcess.stdin.write(enc.encode(rofiInput));

  // Get the selected label
  const selectedLabel = rofiProcess.stdout.toString().trim();

  // If no selection was made, return null
  if (!selectedLabel) {
    return null;
  }

  // Find the corresponding option
  const selectedOption = options.find(option => option.label === selectedLabel);

  // Return the id of the selected option, or null if not found
  return selectedOption ? selectedOption.id : null;
}

