const scriptDir = new URL(".", import.meta.url);
const packageRoot = new URL("../", scriptDir);
const repoRoot = new URL("../../", packageRoot);
const specPath = new URL("schema/flow-server.openapi.json", repoRoot);
const outputPath = new URL(
  "../contracts/src/generated/flow_server_openapi.ts",
  packageRoot,
);

await Deno.mkdir(new URL("../contracts/src/generated", packageRoot), {
  recursive: true,
});

const command = new Deno.Command("deno", {
  cwd: packageRoot,
  args: [
    "run",
    "-A",
    "npm:openapi-typescript",
    specPath.pathname,
    "-o",
    outputPath.pathname,
  ],
  stdout: "inherit",
  stderr: "inherit",
});

const output = await command.output();
if (output.code !== 0) {
  throw new Error(`openapi-typescript failed with exit code ${output.code}`);
}
