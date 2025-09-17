import { getConfig } from "./config";
import { buildBay, getTrueBays, moveBayToScratchpad, moveWindowToBay, type Bay } from "./bay";
import { z } from "zod";

type ToolName = string;
type BayName = string;

export type Bench = {
    name: string;
    isActive: boolean;
    bays: Bay[];
    tools: Array<ToolName & { containerId: string }>;
}

const validateBench = (dict: Record<string, any>): Bench => {
    return z.object({
        name: z.string(),
        bays: z.array(z.object({
            name: z.string(),
            tools: z.array(z.object({
                name: z.string(),
                type: z.string(),
                state: z.record(z.any()),
            })),
        })),
    }).parse(dict);
}


export async function getBenches(): Promise<Bench[]> {
    const config = getConfig();
    const allBenchesFile = config.stateDir + "/benches.json";
    const allBenches = await Bun.file(allBenchesFile).json() as Record<string, any>[];
    const validatedBenches = allBenches.map(validateBench);
    return validatedBenches;
}

export async function saveBench(bench: Bench) {
    const config = getConfig();
    const allBenchesFile = config.stateDir + "/benches.json";
    await Bun.write(allBenchesFile, JSON.stringify(bench));
}

export async function getActiveBench(): Promise<Bench> {
    const config = getConfig();
    const activeBenchFile = config.stateDir + "/active-bench.json";
    const activeBench = await Bun.file(activeBenchFile).json() as Bench;
    return activeBench;
}

export async function setActiveBench(bench: Bench) {
    const config = getConfig();
    const activeBenchFile = config.stateDir + "/active-bench.json";
    await Bun.write(activeBenchFile, JSON.stringify(bench));
}

export async function launchBench(bench: Bench) {
    const trueBench = await getTrueBays();
    for (const bay of trueBench) {
        moveBayToScratchpad(bay);
    }

    for (const [toolName, bayName] of bench.tools) {
        const tool = await getTrueTool(toolName);
        await moveWindowToBay(tool, bayName);
    }

    for (const bay of bench.bays) {
        await buildBay(bay);
    }
}

export async function syncBench(bench: Bench) {
    const trueBench = await getTrueBays();
    bench.bays = trueBench;
    await saveBench(bench);
}

export async function addToolToBench(bench: Bench, tool: Tool) {
    bench.tools.push(tool);
    await saveBench(bench);
}