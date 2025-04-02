use crate::prelude::*;
use anyhow::anyhow;
use flow_lib::command::builder::{BuildResult, BuilderCache};
use reqwest::{dns::Name as DomainName, Url};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tracing::info;
use value::crud::path::Path as JsonPath;

const HTTP_REQUEST: &str = "http_request";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("http.json"))?.check_name(HTTP_REQUEST)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(HTTP_REQUEST, |_| build()));

fn default_method() -> String {
    "GET".to_owned()
}

fn default_retry_delay() -> u64 {
    100 // 100ms initial delay
}

fn default_backoff_factor() -> f64 {
    1.2 // exponential backoff factor
}

fn default_max_retries() -> usize {
    3
}

fn default_timeout_ms() -> Option<u64> {
    None // Default is no timeout
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BasicAuth {
    pub user: String,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JsonCondition {
    Equals(serde_json::Value),
    NotEquals(serde_json::Value),
    Exists,
    NotExists,
    IsNull,
    NotNull,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RetryConfig {
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    pub retry_status_codes: Vec<u16>,
    #[serde(default = "default_retry_delay")]
    pub initial_delay_ms: u64,
    #[serde(default = "default_backoff_factor")]
    pub backoff_factor: f64,
    pub retry_json_path: Option<String>,
    pub retry_condition: Option<JsonCondition>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub url: Url,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    pub basic_auth: Option<BasicAuth>,
    #[serde(default)]
    pub query_params: Vec<(String, String)>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    #[serde(default)]
    pub form: Option<Vec<(String, String)>>,
    pub retry: Option<RetryConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    body: Value,
    headers: HashMap<String, String>,
}

struct Resolver;

fn is_global(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => Ipv4Ext::is_global(ip),
        IpAddr::V6(ip) => Ipv6Ext::is_global(ip),
    }
}

impl Resolver {
    fn resolve_impl(&self, name: String) -> reqwest::dns::Resolving {
        Box::pin(async move {
            tracing::debug!("resolving {}", name.as_str());
            let host = name + ":0";

            let addrs = tokio::net::lookup_host(host).await?.collect::<Vec<_>>();
            if let Some(addr) = addrs.iter().find(|addr| !is_global(&addr.ip())) {
                return Err(format!("IP address not allowed: {}", addr.ip()).into());
            }
            let addrs: Box<dyn Iterator<Item = SocketAddr> + Send> = Box::new(addrs.into_iter());
            Ok(addrs)
        })
    }
}

impl reqwest::dns::Resolve for Resolver {
    fn resolve(&self, name: DomainName) -> reqwest::dns::Resolving {
        self.resolve_impl(name.as_str().to_owned())
    }
}

// copied from nightly rust
trait Ipv4Ext {
    fn is_global(&self) -> bool;
    fn is_shared(&self) -> bool;
    fn is_benchmarking(&self) -> bool;
    fn is_reserved(&self) -> bool;
}

impl Ipv4Ext for Ipv4Addr {
    fn is_global(&self) -> bool {
        !(self.octets()[0] == 0 // "This network"
            || self.is_private()
            || Ipv4Ext::is_shared(self)
            || self.is_loopback()
            || self.is_link_local()
            // addresses reserved for future protocols (`192.0.0.0/24`)
            ||(self.octets()[0] == 192 && self.octets()[1] == 0 && self.octets()[2] == 0)
            || self.is_documentation()
            || Ipv4Ext::is_benchmarking(self)
            || Ipv4Ext::is_reserved(self)
            || self.is_broadcast())
    }

    fn is_shared(&self) -> bool {
        self.octets()[0] == 100 && (self.octets()[1] & 0b1100_0000 == 0b0100_0000)
    }

    fn is_benchmarking(&self) -> bool {
        self.octets()[0] == 198 && (self.octets()[1] & 0xfe) == 18
    }

    fn is_reserved(&self) -> bool {
        self.octets()[0] & 240 == 240 && !self.is_broadcast()
    }
}

trait Ipv6Ext {
    fn is_global(&self) -> bool;
    fn is_documentation(&self) -> bool;
    fn is_unique_local(&self) -> bool;
    fn is_unicast_link_local(&self) -> bool;
}

impl Ipv6Ext for Ipv6Addr {
    fn is_global(&self) -> bool {
        !(self.is_unspecified()
            || self.is_loopback()
            // IPv4-mapped Address (`::ffff:0:0/96`)
            || matches!(self.segments(), [0, 0, 0, 0, 0, 0xffff, _, _])
            // IPv4-IPv6 Translat. (`64:ff9b:1::/48`)
            || matches!(self.segments(), [0x64, 0xff9b, 1, _, _, _, _, _])
            // Discard-Only Address Block (`100::/64`)
            || matches!(self.segments(), [0x100, 0, 0, 0, _, _, _, _])
            // IETF Protocol Assignments (`2001::/23`)
            || (matches!(self.segments(), [0x2001, b, _, _, _, _, _, _] if b < 0x200)
                && !(
                    // Port Control Protocol Anycast (`2001:1::1`)
                    u128::from_be_bytes(self.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0001
                    // Traversal Using Relays around NAT Anycast (`2001:1::2`)
                    || u128::from_be_bytes(self.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0002
                    // AMT (`2001:3::/32`)
                    || matches!(self.segments(), [0x2001, 3, _, _, _, _, _, _])
                    // AS112-v6 (`2001:4:112::/48`)
                    || matches!(self.segments(), [0x2001, 4, 0x112, _, _, _, _, _])
                    // ORCHIDv2 (`2001:20::/28`)
                    || matches!(self.segments(), [0x2001, b, _, _, _, _, _, _] if (0x20..=0x2F).contains(&b))
                ))
            || Ipv6Ext::is_documentation(self)
            || Ipv6Ext::is_unique_local(self)
            || Ipv6Ext::is_unicast_link_local(self))
    }

    fn is_documentation(&self) -> bool {
        (self.segments()[0] == 0x2001) && (self.segments()[1] == 0xdb8)
    }

    fn is_unique_local(&self) -> bool {
        (self.segments()[0] & 0xfe00) == 0xfc00
    }

    fn is_unicast_link_local(&self) -> bool {
        (self.segments()[0] & 0xffc0) == 0xfe80
    }
}

async fn execute_request(
    req: reqwest::RequestBuilder,
    input: &Input,
) -> Result<Output, CommandError> {
    // Attempt to send the request, catching reqwest timeouts and network errors
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) if e.is_timeout() => {
            return Err(anyhow::anyhow!(
                "timeout: connection\nRequest timed out while connecting to server"
            ));
        }
        Err(e) => return Err(anyhow::anyhow!("network error: \n{}", e.to_string())),
    };
    let status = resp.status();

    if status.is_success() {
        info!("response: {:?}", &resp.url());

        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_lowercase(),
                    String::from_utf8_lossy(v.as_bytes()).into_owned(),
                )
            })
            .collect::<HashMap<String, String>>();

        let ct = headers
            .get("content-type")
            .map(String::as_str)
            .unwrap_or("text/plain");

        let body: Value = if ct.starts_with("text/") {
            resp.text()
                .await
                .map_err(|e| {
                    anyhow::anyhow!("body error: parsing\nFailed to parse response text: {}", e)
                })?
                .into()
        } else if ct.contains("json") {
            resp.json::<serde_json::Value>().await?.into()
        } else {
            resp.bytes().await?.into()
        };

        // Check if we should retry based on response content
        if let Some(retry_config) = &input.retry {
            if let Err(e) = should_retry_based_on_content(&body, retry_config) {
                return Err(e);
            }
        }

        Ok(Output { headers, body })
    } else {
        let body = resp.text().await.ok();
        Err(anyhow::anyhow!(
            "status code: {}\n{}",
            status.as_u16(),
            body.unwrap_or_default()
        ))
    }
}

fn should_retry_based_on_content(
    body: &Value,
    retry_config: &RetryConfig,
) -> Result<(), CommandError> {
    // If no JSONPath or condition is specified, no content-based retry
    if let (Some(json_path_str), Some(condition)) = (
        retry_config.retry_json_path.as_ref(),
        retry_config.retry_condition.as_ref(),
    ) {
        // Parse the JSONPath using the existing Path implementation
        let json_path = match JsonPath::parse(json_path_str) {
            Ok(path) => path,
            Err(e) => return Err(anyhow::anyhow!("Invalid JSON path: {}", e).into()),
        };

        // Extract value at JSONPath using path.segments directly
        let value_ref = value::crud::get(body, &json_path.segments);

        let should_retry = match condition {
            JsonCondition::Equals(expected) => {
                // Convert serde_json::Value to Value for comparison
                let expected_value: Value = serde_json::to_string(expected)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(Value::Null);
                // Retry if value doesn't equal expected value
                value_ref.map_or(true, |v| v != &expected_value)
            }
            JsonCondition::NotEquals(expected) => {
                // Convert serde_json::Value to Value for comparison
                let expected_value: Value = serde_json::to_string(expected)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(Value::Null);
                // Retry if value equals expected value
                value_ref.map_or(false, |v| v == &expected_value)
            }
            JsonCondition::Exists => {
                // Retry if value doesn't exist
                value_ref.is_none()
            }
            JsonCondition::NotExists => {
                // Retry if value exists
                value_ref.is_some()
            }
            JsonCondition::IsNull => {
                // Retry if value is not null (using matches for null check)
                value_ref.map_or(false, |v| !matches!(v, Value::Null))
            }
            JsonCondition::NotNull => {
                // Retry if value is null or doesn't exist
                value_ref.map_or(true, |v| matches!(v, Value::Null))
            }
        };

        if should_retry {
            Err(anyhow::anyhow!(
                "Retryable condition: JSONPath '{}' condition not satisfied",
                json_path_str
            )
            .into())
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

// Extract HTTP status code from error if it's a status error
fn extract_status_code(err: &CommandError) -> Option<u16> {
    if let Some(err_str) = err.to_string().strip_prefix("status code: ") {
        if let Some(code_str) = err_str.split('\n').next() {
            return code_str.parse::<u16>().ok();
        }
    }
    None
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    let is_test = cfg!(test);

    match input.url.host() {
        Some(url::Host::Domain(domain)) => {
            if !is_test {
                let _ = Resolver
                    .resolve_impl(domain.to_owned())
                    .await
                    .map_err(|error| anyhow!(error))?;
            }
            if is_test {
                tracing::debug!("Test mode: Skipping domain validation for {}", domain);
            }
        }
        Some(url::Host::Ipv4(ip)) => {
            if !is_test && !Ipv4Ext::is_global(&ip) {
                return Err(anyhow::anyhow!("IP address not allowed: {}", ip));
            }
            if is_test {
                tracing::debug!("Test mode: Skipping IP address validation for {}", ip);
            }
        }
        Some(url::Host::Ipv6(ip)) => {
            if !is_test && !Ipv6Ext::is_global(&ip) {
                return Err(anyhow::anyhow!("IP address not allowed: {}", ip));
            }
            if is_test {
                tracing::debug!("Test mode: Skipping IP address validation for {}", ip);
            }
        }
        None => return Err(anyhow::anyhow!("URL has no host")),
    }

    let client = ctx.http;
    let mut attempt = 0;
    let max_attempts = input.retry.as_ref().map(|r| r.max_retries + 1).unwrap_or(1);

    // Track start time for timeout calculations
    let start_time = std::time::Instant::now();
    let timeout_ms = input.retry.as_ref().and_then(|r| r.timeout_ms);

    loop {
        attempt += 1;
        let is_last_attempt = attempt >= max_attempts;

        // Check if we've exceeded the timeout (if specified)
        if let Some(timeout) = timeout_ms {
            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            if elapsed_ms >= timeout {
                info!("Request timed out after {}ms", elapsed_ms);
                return Err(anyhow::anyhow!(
                    "timeout: {}ms\nTimeout reached after {} attempts",
                    timeout,
                    attempt
                ));
            }
        }

        let mut req = client.request(input.method.parse()?, input.url.clone());

        if !input.query_params.is_empty() {
            req = req.query(&input.query_params);
        }

        for (k, v) in &input.headers {
            req = req.header(k, v);
        }

        if let Some(basic) = &input.basic_auth {
            let passwd = basic.password.as_ref().filter(|p| !p.is_empty());
            req = req.basic_auth(&basic.user, passwd);
        }

        if let Some(ref body) = input.body {
            req = req.json(body);
        }

        if let Some(ref form) = input.form {
            let mut multiform = reqwest::multipart::Form::new();
            for (k, v) in form {
                multiform = multiform.text(k.clone(), v.clone());
            }
            req = req.multipart(multiform);
        }

        match execute_request(req, &input).await {
            Ok(output) => return Ok(output),
            Err(err) => {
                // If this is the last attempt or no retry config, return the error
                // For max_retries: If timeout is specified and max_retries is not (max_retries=0),
                // we'll continue until timeout is reached
                let should_stop_retrying = (is_last_attempt
                    && input.retry.as_ref().map_or(true, |r| r.max_retries > 0))
                    || input.retry.is_none();

                if should_stop_retrying {
                    return Err(err);
                }

                let retry_config = input.retry.as_ref().unwrap();

                // Check if we should retry based on status code
                let should_retry = if let Some(status_code) = extract_status_code(&err) {
                    retry_config.retry_status_codes.contains(&status_code)
                } else {
                    // For non-status code errors (including JSONPath condition failures)
                    // we'll retry if it's not the last attempt
                    true
                };

                if !should_retry {
                    return Err(err);
                }

                // Calculate backoff delay
                let mut delay = retry_config.initial_delay_ms as f64
                    * retry_config.backoff_factor.powf((attempt - 1) as f64);

                // Adjust delay if it would exceed timeout
                if let Some(timeout) = timeout_ms {
                    let elapsed_ms = start_time.elapsed().as_millis() as u64;
                    let remaining_ms = timeout.saturating_sub(elapsed_ms);

                    // If remaining time is less than calculated delay, use remaining time instead
                    if remaining_ms < delay as u64 {
                        delay = remaining_ms as f64;
                        if delay <= 0.0 {
                            info!("Timeout reached, no more retries");
                            return Err(anyhow::anyhow!(
                                "timeout: {}ms\nTimeout reached during retry backoff after {} attempts",
                                timeout,
                                attempt
                            ));
                        }
                    }
                }

                info!(
                    "Request failed, retrying ({}/{}), waiting for {}ms: {}",
                    attempt,
                    if retry_config.max_retries > 0 {
                        max_attempts - 1
                    } else {
                        usize::MAX
                    },
                    delay as u64,
                    err
                );

                // Sleep with backoff before retrying
                tokio::time::sleep(tokio::time::Duration::from_millis(delay as u64)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    // update to simulate since added is_test=cfg!(test) to run
    #[tokio::test]
    async fn test_local() {
        async fn validate_ip(url_str: &str) -> Result<(), String> {
            let url = url::Url::parse(url_str).unwrap();

            match url.host() {
                Some(url::Host::Domain(domain)) => {
                    let _ = Resolver
                        .resolve_impl(domain.to_owned())
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Some(url::Host::Ipv4(ip)) => {
                    if !Ipv4Ext::is_global(&ip) {
                        return Err(format!("IP address not allowed: {}", ip));
                    }
                }
                Some(url::Host::Ipv6(ip)) => {
                    if !Ipv6Ext::is_global(&ip) {
                        return Err(format!("IP address not allowed: {}", ip));
                    }
                }
                None => return Err("URL has no host".to_string()),
            }

            Ok(())
        }

        async fn test(url: &str) {
            let result = validate_ip(url).await;
            assert!(result.is_err(), "Expected validation to fail for {}", url);

            let err = result.unwrap_err();
            assert!(
                err.contains("IP address not allowed"),
                "Expected 'IP address not allowed' error, got: {}",
                err
            );
        }

        // local networks are not allowed because of security reason
        test("http://localhost").await;
        test("http://127.0.0.1:8080").await;
        test("http://169.254.169.254/latest/api/token").await;
        test("http://255.255.255.255").await;
    }

    #[tokio::test]
    async fn test_retry_with_timeout() {
        // Use mockito with its existing runtime
        let mut server = mockito::Server::new_async().await;
        let server_url = server.url();
        println!("Server started at: {}", server_url);

        // Create a URL for our test endpoint
        let endpoint = "/api/resource";
        let url = format!("{}{}", server_url, endpoint);

        // Track the number of requests received
        let request_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let request_count_clone = request_count.clone();

        let start = std::time::Instant::now();
        println!("Starting request execution");

        // First two requests should fail
        let retry_mock = server
            .mock("GET", endpoint)
            .with_status(503)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Service Unavailable"}"#)
            .expect(1) // Only match once
            .match_request(move |req| {
                let count =
                    request_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

                // Show elapsed time since test started
                let elapsed = start.elapsed();

                println!("Request #{} received at {:?} since start", count, elapsed);
                println!("Request path: {}", req.path());
                true // Always match
            })
            .create_async()
            .await;

        // Second retry with a different error code (502 Bad Gateway)
        let retry_mock2 = server
            .mock("GET", endpoint)
            .with_status(502) // Changed from 503 to 502
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Bad Gateway"}"#) // Updated error message
            .expect(1) // Only match once
            .create_async()
            .await;

        // Final success response
        let success_mock = server
            .mock("GET", endpoint)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "ok", "data": "success"}"#)
            .expect(1) // Should be matched once
            .create_async()
            .await;

        // Retry configuration
        let retry_config = RetryConfig {
            max_retries: 3,
            retry_status_codes: vec![500, 502, 503, 504],
            initial_delay_ms: 100, // Short delay for test
            backoff_factor: 1.2,
            retry_json_path: None,
            retry_condition: None,
            timeout_ms: Some(3000), // 3 second timeout
        };

        println!("Retry configuration:");
        println!("  Max retries: {}", retry_config.max_retries);
        println!(
            "  Status codes to retry: {:?}",
            retry_config.retry_status_codes
        );
        println!("  Initial delay: {}ms", retry_config.initial_delay_ms);
        println!("  Backoff factor: {}", retry_config.backoff_factor);
        println!("  Timeout: {:?}ms", retry_config.timeout_ms);

        // Input for the request with retry configuration
        let input = Input {
            url: url.parse().unwrap(),
            method: "GET".to_string(),
            headers: vec![],
            basic_auth: None,
            query_params: vec![],
            body: None,
            form: None,
            retry: Some(retry_config),
        };

        // Create the context and execute the request
        let ctx = Context::default();

        // Track timing for debugging
        let start = std::time::Instant::now();
        println!("Starting request execution",);

        let result = run(ctx, input).await;

        let duration = start.elapsed();
        println!("Request completed in {:?}", duration);

        // Verify the result was successful
        assert!(result.is_ok(), "Request failed: {:?}", result.err());

        // Verify the mocks were called as expected
        retry_mock.assert_async().await;
        retry_mock2.assert_async().await;
        success_mock.assert_async().await;

        // Print final stats
        println!(
            "Total requests made: {}",
            request_count.load(std::sync::atomic::Ordering::SeqCst)
        );
        println!("Test completed successfully!");
    }
}

// Retry configuration examples

// Example 1: Timeout only (retry until timeout)
// {
//       "retry_status_codes": [429, 500, 502, 503, 504],
//       "initial_delay_ms": 1000,
//       "max_retries": 0,
//       "timeout_ms": 60000
// }

// Example 2: Both timeout and max retries (timeout overrides retries)
// {
//      "max_retries": 5,
//      "retry_status_codes": [429, 500, 502, 503, 504],
//      "initial_delay_ms": 1000,
//      "timeout_ms": 30000
// }

// Example 3: Max retries only (timeout ignored)
// {
//      "max_retries": 3,
//      "retry_status_codes": [429, 500, 502, 503, 504],
//      "initial_delay_ms": 1000
// }

// Example 4: Check if data is not null. e.g Wait for RPC to fill data
// {
//     "retry_status_codes": [
//         200
//     ],
//     "initial_delay_ms": 1000,
//     "max_retries": 0,
//     "timeout_ms": 60000,
//     "retry_json_path": "data",
//     "retry_condition": "not_null"
// }
