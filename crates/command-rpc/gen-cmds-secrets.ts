import { encodeHex } from "jsr:@std/encoding@1.0.10";
import { existsSync } from "jsr:@std/fs@1.0.21";

const cmdsServerConfig = {
  "$schema":
    "https://schema.spaceoperator.com/command-server-config.schema.json",
  "secret_key": "",
  "flow_server": [
    {
      // for kubernetes
      "url": "http://flow-server-1:8080/",
    },
  ],
};
cmdsServerConfig["secret_key"] = encodeHex(
  crypto.getRandomValues(new Uint8Array(32)),
);
const json = JSON.stringify(cmdsServerConfig, null, 2);
const path = Deno.args[0];
if (path && !existsSync(path)) {
  Deno.writeFileSync(path, new TextEncoder().encode(json));
} else {
  console.log(json);
}
