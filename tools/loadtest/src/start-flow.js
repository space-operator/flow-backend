import http from "k6/http";
import { check } from "k6";

import {
  buildOptions,
  buildRequestParams,
  getBaseUrl,
  parseJsonEnv,
  recordOutcome,
} from "./common.js";

export const options = buildOptions("start_flow");

function buildUrl() {
  if (!__ENV.FLOW_ID) {
    throw new Error("FLOW_ID is required");
  }
  return `${getBaseUrl()}/flow/start/${__ENV.FLOW_ID}`;
}

function buildBody() {
  return JSON.stringify({
    inputs: parseJsonEnv("INPUTS_JSON", {}),
    environment: parseJsonEnv("ENVIRONMENT_JSON", {}),
    output_instructions: __ENV.OUTPUT_INSTRUCTIONS === "1",
  });
}

export default function () {
  const response = http.post(buildUrl(), buildBody(), buildRequestParams({}, {
    endpoint: "flow_start",
  }));
  const ok = check(response, {
    "start flow returned success": (res) => res.status >= 200 && res.status < 300,
  });
  recordOutcome("flow", response, ok);
}
