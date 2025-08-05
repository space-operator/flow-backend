#!/usr/bin/env -S deno run --allow-read --allow-write=llm-context.txt

import { default as hb } from "npm:handlebars";

function include(path: string) {
  let text = Deno.readTextFileSync(path);
  if (!text.endsWith("\n")) {
    text += "\n";
  }
  hb.registerPartial(
    path,
    hb.compile(text),
  );
}

include("flow.schema.json");
include("value.schema.json");
include("node-v2.schema.json");

Deno.readDirSync("nodes").forEach((entry) => include(`nodes/${entry.name}`));

const template = hb.compile(
  Deno.readTextFileSync("context.md"),
);
Deno.writeTextFileSync("llm-context.txt", template({}));
