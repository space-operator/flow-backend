import http from "k6/http";
import { check } from "k6";

import {
  buildOptions,
  buildRequestParams,
  getBaseUrl,
  parseJsonEnv,
  recordOutcome,
} from "./common.js";

export const options = buildOptions("start_deployment");

function buildUrl() {
  if (!__ENV.DEPLOYMENT_ID) {
    throw new Error("DEPLOYMENT_ID is required");
  }
  return `${getBaseUrl()}/deployment/start?id=${__ENV.DEPLOYMENT_ID}`;
}

function buildBody() {
  return JSON.stringify({
    inputs: parseJsonEnv("INPUTS_JSON", {}),
  });
}

function buildHeaders() {
  const headers = {};
  if (__ENV.X_API_KEY) {
    headers["x-api-key"] = __ENV.X_API_KEY;
  }
  return headers;
}

export default function () {
  const response = http.post(
    buildUrl(),
    buildBody(),
    buildRequestParams(buildHeaders(), {
      endpoint: "deployment_start",
    }),
  );
  const ok = check(response, {
    "start deployment returned success": (res) => res.status >= 200 && res.status < 300,
  });
  recordOutcome("deployment", response, ok);
}
