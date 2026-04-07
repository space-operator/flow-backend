import { assertEquals } from "@std/assert";
import { dirname, fromFileUrl, join } from "jsr:@std/path@^1.1.2";

type BrowserTestGlobal = typeof globalThis & {
  __clientError?: string;
  __clientResult?: unknown;
};

const shouldRunPlaywright = (() => {
  try {
    return Deno.env.get("RUN_SPACE_OPERATOR_PLAYWRIGHT_TESTS") === "1";
  } catch {
    return false;
  }
})();

const denoMajor = Number(Deno.version.deno.split(".")[0]);

if (shouldRunPlaywright && denoMajor < 2) {
  throw new Error(
    `tests/playwright requires Deno 2+. Found Deno ${Deno.version.deno}.`,
  );
}

Deno.test({
  name: "playwright browser websocket smoke test",
  ignore: !shouldRunPlaywright,
  sanitizeOps: false,
  sanitizeResources: false,
  async fn() {
    const { chromium } = await import("@playwright/test");
    const { denoPlugins } = await import("npm:@oazmi/esbuild-plugin-deno@^0.4.4");
    const { build, stop } = await import("npm:esbuild@^0.25.12");
    const root = dirname(dirname(fromFileUrl(import.meta.url)));
    const packageRoot = dirname(root);
    const contractsRoot = join(dirname(packageRoot), "contracts");
    const clientNodeModulesPath = join(packageRoot, "node_modules");
    const contractsNodeModulesPath = join(contractsRoot, "node_modules");
    const tmpDir = await Deno.makeTempDir({
      prefix: "client-next-playwright-",
    });
    const bundlePath = join(tmpDir, "client-next.bundle.js");
    let createdContractsNodeModulesLink = false;

    try {
      await Deno.stat(contractsNodeModulesPath);
    } catch {
      await Deno.symlink(
        clientNodeModulesPath,
        contractsNodeModulesPath,
        { type: "dir" },
      );
      createdContractsNodeModulesLink = true;
    }

    const [
      entryPlugin,
      httpPlugin,
      jsrPlugin,
      npmPlugin,
      resolverPipelinePlugin,
    ] = denoPlugins({
      scanAncestralWorkspaces: true,
      initialPluginData: {
        runtimePackage: "./deno.json",
        resolverConfig: {
          useNodeModules: false,
        },
      },
    });

    await build({
      absWorkingDir: packageRoot,
      bundle: true,
      entryPoints: ["./src/mod.ts"],
      format: "esm",
      outfile: bundlePath,
      platform: "browser",
      target: ["chrome120"],
      logLevel: "silent",
      plugins: [
        entryPlugin,
        httpPlugin,
        jsrPlugin,
        npmPlugin,
        resolverPipelinePlugin,
      ],
    });

    const bundleSource = await Deno.readTextFile(bundlePath);
    const server = Deno.serve({ hostname: "127.0.0.1", port: 0 }, (
      request: Request,
    ) => {
      const url = new URL(request.url);
      if (url.pathname === "/bundle.js") {
        return new Response(bundleSource, {
          headers: { "content-type": "application/javascript; charset=utf-8" },
        });
      }

      return new Response(
        `<!doctype html>
<meta charset="utf-8" />
<script>
globalThis.fetch = async () => new Response(JSON.stringify({ success: true }), {
  headers: { "content-type": "application/json" },
});

globalThis.WebSocket = class MockBrowserWebSocket {
  constructor(url) {
    this.url = url;
    this.onopen = null;
    this.onmessage = null;
    this.onerror = null;
    this.onclose = null;
    queueMicrotask(() => this.onopen && this.onopen({}));
  }

  send(data) {
    const message = JSON.parse(data);
    if (message.method === "Authenticate") {
      queueMicrotask(() =>
        this.onmessage && this.onmessage({
          data: JSON.stringify({ id: message.id, Ok: { user_id: "user-1" } }),
        })
      );
      return;
    }
    if (message.method === "SubscribeFlowRunEvents") {
      queueMicrotask(() =>
        this.onmessage && this.onmessage({
          data: JSON.stringify({ id: message.id, Ok: { stream_id: 2 } }),
        })
      );
      queueMicrotask(() =>
        this.onmessage && this.onmessage({
          data: JSON.stringify({
            stream_id: 2,
            event: "FlowFinish",
            data: {
              flow_run_id: "run-1",
              time: "now",
              not_run: [],
              output: { M: { ok: { B: true } } },
            },
          }),
        })
      );
    }
  }

  close(code, reason) {
    queueMicrotask(() =>
      this.onclose && this.onclose({ code, reason: reason ?? "closed" })
    );
  }
};
</script>
<script type="module">
import { bearerAuth, createClient } from "/bundle.js";

const client = createClient({
  baseUrl: "http://example.test",
  auth: bearerAuth("jwt-1"),
});

try {
  const subscription = await client.events.subscribeFlowRun("run-1");
  const event = await subscription.next();
  await subscription.close();
  globalThis.__clientResult = {
    event: event.value?.event,
    output: event.value?.data.output.toJSObject(),
  };
} catch (error) {
  globalThis.__clientError = String(error);
}
</script>`,
        {
          headers: { "content-type": "text/html; charset=utf-8" },
        },
      );
    });
    const port = (server.addr as Deno.NetAddr).port;

    const browser = await chromium.launch();
    try {
      const page = await browser.newPage();
      await page.goto(`http://127.0.0.1:${port}`);
      await page.waitForFunction(() =>
        Boolean((globalThis as BrowserTestGlobal).__clientResult)
      );
      const result = await page.evaluate(() =>
        (globalThis as BrowserTestGlobal).__clientResult
      );
      assertEquals(result, {
        event: "FlowFinish",
        output: { ok: true },
      });
    } finally {
      stop();
      await browser.close();
      await server.shutdown();
      if (createdContractsNodeModulesLink) {
        await Deno.remove(contractsNodeModulesPath).catch(() => undefined);
      }
      await Deno.remove(tmpDir, { recursive: true }).catch(() => undefined);
    }
  },
});
