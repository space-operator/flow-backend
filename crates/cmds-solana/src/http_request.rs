use crate::prelude::*;
use flow_lib::command::builder::{BuildResult, BuilderCache};
use hyper::client::connect::dns::Name as DomainName;
use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

const HTTP_REQUEST: &str = "http_request";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(include_str!("../../../node-definitions/http.json"))?
            .check_name(HTTP_REQUEST)
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(HTTP_REQUEST, |_| build()));

fn default_method() -> String {
    "GET".to_owned()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BasicAuth {
    pub user: String,
    pub password: Option<String>,
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

impl reqwest::dns::Resolve for Resolver {
    fn resolve(&self, name: DomainName) -> reqwest::dns::Resolving {
        Box::pin(async move {
            tracing::debug!("resolving {}", name.as_str());
            let host = name.as_str().to_owned() + ":0";

            let addrs = tokio::net::lookup_host(host).await?.collect::<Vec<_>>();
            if let Some(addr) = addrs.iter().find(|addr| !is_global(&addr.ip())) {
                return Err(format!("IP address not allowed: {}", addr.ip()).into());
            }
            let addrs: Box<dyn Iterator<Item = SocketAddr> + Send> = Box::new(addrs.into_iter());
            Ok(addrs)
        })
    }
}

// copied from nightly rust
trait Ipv4Ext {
    fn is_global(&self) -> bool;
    fn is_shared(&self) -> bool;
    fn is_benchmarking(&self) -> bool;
    fn is_documentation(&self) -> bool;
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

    fn is_documentation(&self) -> bool {
        matches!(
            self.octets(),
            [192, 0, 2, _] | [198, 51, 100, _] | [203, 0, 113, _]
        )
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

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    match input.url.host() {
        Some(url::Host::Domain(_)) => {}
        Some(url::Host::Ipv4(ip)) => {
            if !Ipv4Ext::is_global(&ip) {
                return Err(anyhow::anyhow!("IP address not allowed: {}", ip));
            }
        }
        Some(url::Host::Ipv6(ip)) => {
            if !Ipv6Ext::is_global(&ip) {
                return Err(anyhow::anyhow!("IP address not allowed: {}", ip));
            }
        }
        None => return Err(anyhow::anyhow!("URL has no host")),
    }

    let client = reqwest::Client::builder()
        .dns_resolver(Arc::new(Resolver))
        .build()?;

    let mut req = client.request(input.method.parse()?, input.url);

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

    if let Some(body) = input.body {
        req = req.json(&body);
    }

    if let Some(form) = input.form {
        let mut multiform = reqwest::multipart::Form::new();
        for (k, v) in form {
            multiform = multiform.text(k, v);
        }
        req = req.multipart(multiform);
    }

    let resp = req.send().await?;

    let status = resp.status();

    if status.is_success() {
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
            resp.text().await?.into()
        } else if ct.contains("json") {
            resp.json::<serde_json::Value>().await?.into()
        } else {
            resp.bytes().await?.into()
        };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_local() {
        async fn test(url: &str) {
            let c = Context::default();
            let e = run(
                c.clone(),
                value::from_map(value::map! {"url" => url}).unwrap(),
            )
            .await
            .unwrap_err()
            .to_string();
            assert!(e.contains("IP address not allowed"));
        }

        // local networks are not allowed because of security reason
        test("http://localhost").await;
        test("http://127.0.0.1:8080").await;
        test("http://169.254.169.254/latest/api/token").await;
        test("http://255.255.255.255").await;
    }
}
