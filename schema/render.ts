import { default as hb } from "npm:handlebars";

function include(path: string) {
  hb.registerPartial(
    path,
    hb.compile(Deno.readTextFileSync(path)),
  );
}

include("flow.schema.json");
include("value.schema.json");
include("node-v2.schema.json");

const template = hb.compile(
  Deno.readTextFileSync("context.md"),
);
Deno.writeTextFileSync("context-rendered.md", template({}));
