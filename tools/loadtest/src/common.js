import { Counter, Rate } from "k6/metrics";

export const startFailures = new Counter("loadtest_start_failures_total");
export const startSuccess = new Counter("loadtest_start_success_total");
export const startUnexpectedStatus = new Counter(
  "loadtest_start_unexpected_status_total",
);
export const startChecks = new Rate("loadtest_start_checks");

const loggedFailures = new Set();

function parsePositiveInt(rawValue, fallback) {
  if (!rawValue) {
    return fallback;
  }
  const parsed = Number.parseInt(rawValue, 10);
  if (Number.isFinite(parsed) && parsed > 0) {
    return parsed;
  }
  return fallback;
}

export function parseJsonEnv(name, fallback) {
  const rawValue = __ENV[name];
  if (!rawValue) {
    return fallback;
  }
  try {
    return JSON.parse(rawValue);
  } catch (error) {
    throw new Error(`${name} must be valid JSON: ${String(error)}`);
  }
}

export function buildOptions(scenarioName) {
  const vus = parsePositiveInt(__ENV.VUS, 10);
  const duration = __ENV.DURATION || "30s";
  const iterations = __ENV.ITERATIONS
    ? parsePositiveInt(__ENV.ITERATIONS, 0)
    : 0;

  if (iterations > 0) {
    return {
      scenarios: {
        [scenarioName]: {
          executor: "shared-iterations",
          vus,
          iterations,
        },
      },
      summaryTrendStats: ["avg", "min", "med", "p(95)", "p(99)", "max"],
    };
  }

  return {
    scenarios: {
      [scenarioName]: {
        executor: "constant-vus",
        vus,
        duration,
      },
    },
    summaryTrendStats: ["avg", "min", "med", "p(95)", "p(99)", "max"],
  };
}

export function buildRequestParams(extraHeaders = {}, tags = {}) {
  const headers = {
    "content-type": "application/json",
    ...extraHeaders,
  };

  if (__ENV.AUTH_TOKEN && !headers.Authorization) {
    headers.Authorization = `Bearer ${__ENV.AUTH_TOKEN}`;
  }

  return {
    headers,
    tags,
  };
}

export function getBaseUrl() {
  return __ENV.BASE_URL || "http://127.0.0.1:8080";
}

export function maybeLogFailure(kind, response) {
  const key = `${kind}:${response.status}`;
  if (loggedFailures.has(key) || loggedFailures.size >= 5) {
    return;
  }
  loggedFailures.add(key);
  console.error(
    `[${kind}] unexpected status=${response.status} body=${String(response.body).slice(0, 300)}`,
  );
}

export function recordOutcome(kind, response, ok) {
  startChecks.add(ok);
  if (ok) {
    startSuccess.add(1, { kind });
    return;
  }
  startFailures.add(1, { kind });
  startUnexpectedStatus.add(1, {
    kind,
    status: String(response.status),
  });
  maybeLogFailure(kind, response);
}
