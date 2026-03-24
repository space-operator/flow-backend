//! Turbo Upload File - Upload a file to Arweave via ArDrive Turbo.
//!
//! Prerequisites: The fee_payer's Turbo account must be funded
//! (use the `turbo_fund_account` node first).
//!
//! Accepts either:
//!   - `file_url`: fetch file bytes from a URL, or
//!   - `content`: provide raw content directly (string or bytes)
//!
//! Flow:
//! 1. Resolve file bytes from URL or inline content
//! 2. Create a signed ANS-104 data item using bundlr-sdk
//! 3. POST the data item to upload.ardrive.io/v1/tx/solana
//! 4. Return the arweave.net file URL
//!
//! Upload API: https://docs.ar.io/apis/turbo/upload-service/upload/

use crate::arweave::arweave_nft_upload::BundlrSigner;
use crate::prelude::*;
use bundlr_sdk::{tags::Tag, BundlrTx};
use tracing::info;

pub const NAME: &str = "turbo_upload_file";
const DEFINITION: &str = flow_lib::node_definition!("ardrive/turbo_upload_file.jsonc");

const TURBO_UPLOAD_URL: &str = "https://upload.ardrive.io/v1";
const ARWEAVE_GATEWAY: &str = "https://ar-io.dev";

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    /// URL to fetch the file from. Provide either this or `content`.
    #[serde(default)]
    pub file_url: Option<String>,
    /// Content to upload directly. Accepts plain text (JSON, HTML, etc.)
    /// or base64-encoded binary (images, PDFs). Base64 is auto-detected.
    /// Provide either this or `file_url`.
    #[serde(default)]
    pub content: Option<String>,
    /// Content type (e.g. "application/json", "image/png", "application/pdf").
    /// Auto-detected from file_url when using URL mode.
    /// Defaults to "application/octet-stream".
    #[serde(default)]
    pub content_type: Option<String>,
    /// Optional custom tags as a JSON array of {name, value} objects.
    #[serde(default)]
    pub tags: Option<Vec<CustomTag>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomTag {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub data_item_id: String,
    pub file_url: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TurboUploadResponse {
    id: String,
    // Other fields like owner, dataCaches, etc. exist but we only need the id.
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // 1. Resolve file data from URL or inline content
    let has_url = input.file_url.is_some();
    let has_content = input.content.is_some();
    if has_url == has_content {
        return Err(CommandError::msg(
            "Provide exactly one of: file_url or content",
        ));
    }

    let (data, auto_content_type) = if let Some(url) = &input.file_url {
        info!("Fetching file from {}", url);
        let file_resp = ctx.http().get(url).send().await?;
        if !file_resp.status().is_success() {
            return Err(CommandError::msg(format!(
                "Failed to fetch file: {} {}",
                file_resp.status(),
                file_resp.text().await.unwrap_or_default()
            )));
        }
        let bytes = file_resp.bytes().await?.to_vec();
        let mime = mime_guess::from_path(url)
            .first()
            .map(|m| m.to_string());
        info!("Fetched {} bytes", bytes.len());
        (bytes, mime)
    } else if let Some(content) = &input.content {
        // Auto-detect base64: try decode, use decoded bytes if result is binary
        let data = if let Ok(decoded) = base64::decode(content) {
            if std::str::from_utf8(&decoded).is_err() {
                // Decoded successfully and result is binary — treat as base64
                info!("Auto-detected base64 content, decoded to {} bytes", decoded.len());
                decoded
            } else {
                // Decoded to valid UTF-8 — treat as raw text
                info!("Using inline content ({} bytes)", content.len());
                content.as_bytes().to_vec()
            }
        } else {
            // Not valid base64 — use as raw text
            info!("Using inline content ({} bytes)", content.len());
            content.as_bytes().to_vec()
        };
        (data, None)
    } else {
        unreachable!()
    };

    // 2. Determine content type: explicit > auto-detected > default
    let content_type = input
        .content_type
        .unwrap_or_else(|| auto_content_type.unwrap_or_else(|| "application/octet-stream".to_owned()));

    // 3. Build tags
    let mut tags = vec![Tag::new("Content-Type".into(), content_type)];
    if let Some(custom_tags) = input.tags {
        for t in custom_tags {
            tags.push(Tag::new(t.name, t.value));
        }
    }

    // 4. Create and sign the ANS-104 data item
    info!("Creating signed ANS-104 data item ({} bytes, {} tags)", data.len(), tags.len());
    let signer = BundlrSigner::new(input.fee_payer, ctx.clone());
    let data_item = tokio::task::spawn_blocking(move || {
        BundlrTx::create_with_tags(data, tags, &signer)
    })
    .await
    .map_err(|e| CommandError::msg(format!("Failed to create data item: {e}")))?;

    let raw_bytes = data_item.into_inner();

    // 5. POST to Turbo upload endpoint
    let upload_url = format!("{TURBO_UPLOAD_URL}/tx/solana");
    info!("Uploading {} bytes to {}", raw_bytes.len(), upload_url);

    let resp = ctx
        .http()
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(raw_bytes)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(CommandError::msg(format!(
            "Turbo upload error: {status} {body}"
        )));
    }

    let upload_resp: TurboUploadResponse = resp.json().await?;
    let file_url = format!("{ARWEAVE_GATEWAY}/{}", upload_resp.id);

    info!("Upload successful: id={}, url={}", upload_resp.id, file_url);

    Ok(Output {
        data_item_id: upload_resp.id,
        file_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    fn test_setup() -> (CommandContext, Wallet) {
        tracing_subscriber::fmt::try_init().ok();
        let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
        dotenvy::from_path(&env_path).expect("Failed to load .env file");
        let keypair_str = std::env::var("keypair").expect("keypair not found in .env");
        let ctx = flow_lib_solana::utils::test_context_with_execute();
        let fee_payer: Wallet = Keypair::from_base58_string(&keypair_str).into();
        (ctx, fee_payer)
    }

    #[tokio::test]
    #[ignore = "hits live Turbo API — requires funded account"]
    async fn test_run_content() {
        let (ctx, fee_payer) = test_setup();
        let input = Input {
            fee_payer,
            file_url: None,
            content: Some(r#"{"test": true, "message": "hello from space-operator"}"#.to_owned()),
            content_type: Some("application/json".to_owned()),
            tags: Some(vec![CustomTag {
                name: "App-Name".to_owned(),
                value: "SpaceOperator-Test".to_owned(),
            }]),
        };

        let result = run(ctx, input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
        let output = result.unwrap();
        println!("Data item ID: {}", output.data_item_id);
        println!("File URL: {}", output.file_url);
    }

    #[tokio::test]
    #[ignore = "hits live Turbo API — requires funded account"]
    async fn test_run_url() {
        let (ctx, fee_payer) = test_setup();
        let input = Input {
            fee_payer,
            file_url: Some("https://httpbin.org/robots.txt".to_owned()),
            content: None,
            content_type: None,
            tags: Some(vec![CustomTag {
                name: "App-Name".to_owned(),
                value: "SpaceOperator-Test".to_owned(),
            }]),
        };

        let result = run(ctx, input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
        let output = result.unwrap();
        println!("Data item ID: {}", output.data_item_id);
        println!("File URL: {}", output.file_url);
    }

    #[tokio::test]
    #[ignore = "hits live Turbo API — requires funded account"]
    async fn test_run_base64_image() {
        let (ctx, fee_payer) = test_setup();

        // Minimal 1x1 red PNG (68 bytes)
        let png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

        let input = Input {
            fee_payer,
            file_url: None,
            content: Some(png_base64.to_owned()),
            content_type: Some("image/png".to_owned()),
            tags: Some(vec![CustomTag {
                name: "App-Name".to_owned(),
                value: "SpaceOperator-Test".to_owned(),
            }]),
        };

        let result = run(ctx, input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
        let output = result.unwrap();
        println!("Data item ID: {}", output.data_item_id);
        println!("File URL: {}", output.file_url);
    }
}
