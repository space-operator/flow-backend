use crate::prelude::*;
use super::helper::{check_response, reflect_post};

pub const NAME: &str = "upload_integration_logo";
const DEFINITION: &str = flow_lib::node_definition!("reflect/upload_integration_logo.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(default)]
    pub cluster: Option<String>,
    pub image_url: String,
    pub branded_mint: String,
    pub metadata_name: String,
    pub metadata_symbol: String,
    pub metadata_description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = "/integration/upload";
    let mut req = reflect_post(&ctx, path);
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(ref val) = input.cluster {
        query.push(("cluster", val.as_str()));
    }
    if !query.is_empty() {
        req = req.query(&query);
    }
    // Download image from URL then construct multipart form
    let image_bytes = ctx.http().get(&input.image_url).send().await?.bytes().await?;
    let form = reqwest::multipart::Form::new()
        .part("image", reqwest::multipart::Part::bytes(image_bytes.to_vec()).file_name("image.png").mime_str("image/png")?)
        .text("brandedMint", input.branded_mint.clone())
        .text("metadata[name]", input.metadata_name.clone())
        .text("metadata[symbol]", input.metadata_symbol.clone())
        .text("metadata[description]", input.metadata_description.clone());
    req = req.multipart(form);
    let result = check_response(req.send().await?).await?;
    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
