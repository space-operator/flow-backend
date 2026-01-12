import { Application, Router } from "@oak/oak";
const router = new Router();
router.post("/webhook", async (ctx) => {
  const info = await ctx.request.body.json();
  const url = info.url!;
  const output = info.extra?.output ?? { "S": "hello" };
  const resp = await fetch(url, {
    method: "POST",
    headers: [["content-type", "application/json"]],
    body: JSON.stringify({ value: output }),
  });
  await resp.text();
  ctx.response.body = "ok";
});
const app = new Application();
app.use(router.routes());
app.use(router.allowedMethods());
app.listen({
  port: 80,
});
