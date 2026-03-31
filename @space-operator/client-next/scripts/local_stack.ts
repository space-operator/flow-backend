import { load } from "@std/dotenv";

const scriptDir = new URL(".", import.meta.url);
const packageRoot = new URL("../", scriptDir);
const repoRoot = new URL("../../", packageRoot);
const dockerDir = new URL("../../docker/", packageRoot);
const localFlowServerImage = "flow-server-local:dev";

await load({
  export: true,
  envPath: decodeURIComponent(new URL("../../docker/.flow-test.env", packageRoot).pathname),
}).catch(() => undefined);
await load({
  export: true,
  envPath: decodeURIComponent(new URL("../../docker/.env", packageRoot).pathname),
}).catch(() => undefined);
await load({
  export: true,
  envPath: decodeURIComponent(new URL(".env", repoRoot).pathname),
}).catch(() => undefined);
await load({ export: true }).catch(() => undefined);

const flowServerUrl = Deno.env.get("FLOW_SERVER_URL") ?? "http://127.0.0.1:8080";
const supabaseUrl = Deno.env.get("SUPABASE_URL") ?? "http://127.0.0.1:8000";

async function urlReachable(url: string): Promise<{ ok: boolean; detail: string }> {
  try {
    const response = await fetch(url, {
      method: "GET",
      signal: AbortSignal.timeout(5_000),
    });
    return { ok: true, detail: `${response.status} ${response.statusText}`.trim() };
  } catch (error) {
    return {
      ok: false,
      detail: error instanceof Error ? error.message : String(error),
    };
  }
}

async function currentFlowServerImage(): Promise<string | undefined> {
  const command = new Deno.Command("docker", {
    args: [
      "ps",
      "--filter",
      "label=com.docker.compose.project=flow",
      "--filter",
      "label=com.docker.compose.service=flow-server",
      "--format",
      "{{.Image}}",
    ],
    stdout: "piped",
    stderr: "null",
  });
  const result = await command.output().catch(() => undefined);
  if (!result || result.code !== 0) {
    return undefined;
  }
  const image = new TextDecoder().decode(result.stdout).trim();
  return image || undefined;
}

async function checkLocalStack(): Promise<void> {
  const flowServer = await urlReachable(new URL("healthcheck", flowServerUrl).toString());
  const supabase = await urlReachable(new URL("rest/v1/", supabaseUrl).toString());

  if (flowServer.ok && supabase.ok) {
    const image = await currentFlowServerImage();
    if (image && image !== localFlowServerImage) {
      throw new Error(
        "Local Flow Server stack is running, but it is not using the checked-out flow-server image.\n" +
          `- running image ${image}\n` +
          `- expected image ${localFlowServerImage}\n\n` +
          "Refresh the stack with:\n" +
          "deno task test:stack:up",
      );
    }
    console.log(
      `Local stack looks ready: flow-server (${flowServer.detail}), supabase (${supabase.detail})${image ? `, image (${image})` : ""}.`,
    );
    return;
  }

  const problems: string[] = [];
  if (!flowServer.ok) {
    problems.push(`flow-server ${flowServerUrl} -> ${flowServer.detail}`);
  }
  if (!supabase.ok) {
    problems.push(`supabase ${supabaseUrl} -> ${supabase.detail}`);
  }

  throw new Error(
    "Local Flow Server stack is not ready.\n" +
      problems.map((problem) => `- ${problem}`).join("\n") +
      "\n\nStart it with:\n" +
      "cd /home/amir/code/space-operator/flow-backend/docker\n" +
      "docker compose up -d --wait\n\n" +
      "Or from @space-operator/client-next run:\n" +
      "deno task test:stack:up",
  );
}

async function startLocalStack(): Promise<void> {
  const build = new Deno.Command("docker", {
    cwd: repoRoot,
    args: [
      "build",
      "-f",
      "crates/flow-server/Dockerfile",
      "-t",
      localFlowServerImage,
      ".",
    ],
    stdout: "inherit",
    stderr: "inherit",
  });
  const buildResult = await build.output();
  if (buildResult.code !== 0) {
    throw new Error(`docker build failed with exit code ${buildResult.code}`);
  }

  const up = new Deno.Command("docker", {
    cwd: dockerDir,
    args: ["compose", "up", "-d", "--wait"],
    env: {
      IMAGE: localFlowServerImage,
    },
    stdout: "inherit",
    stderr: "inherit",
  });
  const upResult = await up.output();
  if (upResult.code !== 0) {
    throw new Error(`docker compose up failed with exit code ${upResult.code}`);
  }
  await checkLocalStack();
}

const mode = Deno.args[0] ?? "check";

if (mode === "check") {
  await checkLocalStack();
} else if (mode === "up") {
  await startLocalStack();
} else {
  throw new Error(`unknown mode "${mode}", expected "check" or "up"`);
}
