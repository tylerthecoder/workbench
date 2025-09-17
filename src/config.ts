
export type Config = {
    stateFile: string;
    stateDir: string;
};

export function getConfig(): Config {
    return {
        stateFile: "/home/tylord/docs/bench.json",
    };
}