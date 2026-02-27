import * as client from "../src/mod.ts";
import * as dotenv from "@std/dotenv";
import { getEnv } from "./utils.ts";

dotenv.loadSync({
    export: true,
});

const anonKey = getEnv("ANON_KEY");

Deno.test("test export", async () => {
    const c = new client.Client({
        host: "http://localhost:8080",
        anonKey,
        token: getEnv("APIKEY"),
    });

    await c.export();
});
