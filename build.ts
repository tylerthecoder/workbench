import fs from "node:fs/promises";
import { $ } from "bun";

const outDir = "./out";

const assertDir = async (dir: string) => {
  const dirExists = await fs.exists(dir);

  if (!dirExists) {
    await fs.mkdir(dir, { recursive: true });
  } else {
    await fs.rm(dir, { recursive: true });

    await fs.mkdir(dir);
  }
};

const buildCli = async () => {
  await assertDir(outDir);
  await $`bun build ./src/cli.ts --compile --outfile ./${outDir}/bench`;
  console.log("Cli built");
};

const extensionOutDir = "./extension";
const extensionSourceDir = "./src/apps/chrome/extension";

const buildExtension = async () => {
  await assertDir(extensionOutDir);

  await Bun.write(
    `${extensionOutDir}/manifest.json`,
    Bun.file(`${extensionSourceDir}/manifest.json`),
  );
  await Bun.write(
    `${extensionOutDir}/popup.html`,
    Bun.file(`${extensionSourceDir}/popup.html`),
  );

  const buildOut = await Bun.build({
    outdir: extensionOutDir,
    entrypoints: [`${extensionSourceDir}/background.ts`],
  });

  console.log("Extension built", buildOut);
};

buildCli();
buildExtension();
