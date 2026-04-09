const scriptDir = new URL(".", import.meta.url);
const packageRoot = new URL("../", scriptDir);
const repoRoot = new URL("../../", packageRoot);

async function pathExists(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      return false;
    }
    throw error;
  }
}

async function resolveCommand(command: string): Promise<string> {
  if (command === "deno") {
    return Deno.execPath();
  }

  if (command === "cargo") {
    const candidates = [
      Deno.env.get("CARGO"),
      Deno.env.get("CARGO_HOME")
        ? `${Deno.env.get("CARGO_HOME")}/bin/cargo`
        : undefined,
      Deno.env.get("HOME") ? `${Deno.env.get("HOME")}/.cargo/bin/cargo` : undefined,
    ].filter((candidate): candidate is string => Boolean(candidate));

    for (const candidate of candidates) {
      if (await pathExists(candidate)) {
        return candidate;
      }
    }
  }

  return command;
}

async function run(
  command: string,
  args: string[],
  cwd: URL = repoRoot,
): Promise<void> {
  const executable = await resolveCommand(command);
  let output: Deno.CommandOutput;

  try {
    output = await new Deno.Command(executable, {
      cwd,
      args,
      stdout: "inherit",
      stderr: "inherit",
    }).output();
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      throw new Error(
        `failed to find executable for ${command}; tried ${executable}`,
        { cause: error },
      );
    }
    throw error;
  }

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
