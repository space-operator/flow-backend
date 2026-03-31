const scriptDir = new URL(".", import.meta.url);
const packageRoot = new URL("../", scriptDir);
const repoRoot = new URL("../../", packageRoot);

async function run(
  command: string,
  args: string[],
  cwd: URL = repoRoot,
): Promise<void> {
  const output = await new Deno.Command(command, {
    cwd,
    args,
    stdout: "inherit",
    stderr: "inherit",
  }).output();

  if (output.code !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed with exit code ${output.code}`,
    );
  }
}

await run("cargo", ["run", "-p", "generate-schema"]);
await run("deno", ["task", "generate:openapi-types"], packageRoot);

const diff = await new Deno.Command("git", {
  cwd: repoRoot,
  args: [
    "diff",
    "--exit-code",
    "--",
    "schema/flow-server.openapi.json",
    "@space-operator/contracts/src/generated/flow_server_openapi.ts",
  ],
  stdout: "inherit",
  stderr: "inherit",
}).output();

if (diff.code !== 0) {
  throw new Error(
    "generated OpenAPI artifacts are out of date; run `cargo run -p generate-schema` and `deno task generate:openapi-types`",
  );
}
